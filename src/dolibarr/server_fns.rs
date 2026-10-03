use dioxus::prelude::*;
use crate::dolibarr::models::{
    DolibarrBankAccount, DolibarrBankLine, DolibarrBankLineDraft, DolibarrDocument,
    DolibarrInvoice, DolibarrPayment, DolibarrThirdParty,
};
use crate::email::EmailFacture;

#[cfg(feature = "server")]
fn parse_eur(s: &str) -> f64 {
    s.trim().replace(',', ".").parse::<f64>().unwrap_or(0.0)
}

/// Genere une ref "safe" pour Dolibarr (path + SQL + URL).
#[cfg(feature = "server")]
fn sanitize_ref(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_').to_string();
    if trimmed.is_empty() {
        format!("FOURN-{}", chrono::Utc::now().timestamp())
    } else {
        trimmed
    }
}

#[cfg(feature = "server")]
async fn write_file_in_container(
    container_dir: &str,
    container_filepath: &str,
    bytes: &[u8],
    safe_filename: &str,
) -> Result<(), ServerFnError> {
    let mkdir = tokio::process::Command::new("docker")
        .args(["exec", "sci-family-dolibarr", "mkdir", "-p", container_dir])
        .output()
        .await
        .map_err(|e| ServerFnError::new(format!("Erreur mkdir : {}", e)))?;
    if !mkdir.status.success() {
        return Err(ServerFnError::new(format!(
            "Erreur mkdir : {}",
            String::from_utf8_lossy(&mkdir.stderr)
        )));
    }

    let tmp_path = std::env::temp_dir().join(format!("doli_upload_{}", safe_filename));
    tokio::fs::write(&tmp_path, bytes)
        .await
        .map_err(|e| ServerFnError::new(format!("Erreur write temp : {}", e)))?;

    let cp = tokio::process::Command::new("docker")
        .args([
            "cp",
            &tmp_path.to_string_lossy(),
            &format!("sci-family-dolibarr:{}", container_filepath),
        ])
        .output()
        .await
        .map_err(|e| ServerFnError::new(format!("Erreur docker cp : {}", e)))?;

    let _ = tokio::fs::remove_file(&tmp_path).await;

    if !cp.status.success() {
        return Err(ServerFnError::new(format!(
            "Erreur docker cp : {}",
            String::from_utf8_lossy(&cp.stderr)
        )));
    }

    Ok(())
}

#[cfg(feature = "server")]
fn sanitize_id(s: &str) -> Result<String, ServerFnError> {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
        Ok(s.to_string())
    } else {
        Err(ServerFnError::new(format!("ID invalide : {}", s)))
    }
}

/// Verifie si un fichier existe deja dans la GED Dolibarr.
#[cfg(feature = "server")]
async fn ecm_file_exists(rel_filepath: &str, filename: &str) -> Result<bool, ServerFnError> {
    let sql = format!(
        "SELECT 1 FROM llx_ecm_files WHERE filepath = '{}' AND filename = '{}' AND entity = 1 LIMIT 1;",
        rel_filepath.replace('\'', "''"),
        filename.replace('\'', "''")
    );
    let out = tokio::process::Command::new("docker")
        .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", &sql])
        .output()
        .await
        .map_err(|e| ServerFnError::new(format!("Erreur SQL check : {}", e)))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(!stdout.is_empty())
}

/// Renvoie un nom de fichier libre (ajoute _2, _3... si necessaire).
#[cfg(feature = "server")]
async fn find_available_name(
    rel_dir: &str,
    base_filename: &str,
) -> Result<String, ServerFnError> {
    let (stem, ext) = match base_filename.rfind('.') {
        Some(i) => (&base_filename[..i], &base_filename[i..]),
        None => (base_filename, ""),
    };

    for i in 1..=999 {
        let candidate = if i == 1 {
            base_filename.to_string()
        } else {
            format!("{}_{}{}", stem, i, ext)
        };
        let full_rel = format!("{}/{}", rel_dir, candidate);
        if !ecm_file_exists(&full_rel, &candidate).await? {
            return Ok(candidate);
        }
    }
    Err(ServerFnError::new("Trop de fichiers portent ce nom (999 max)"))
}

// ============================================================
//  OCR PDF (Tesseract dans le conteneur Dolibarr)
// ============================================================

/// Convertit/OCR selon le type réel du fichier (magic bytes) :
/// - PDF → pdftoppm + tesseract
/// - JPG / PNG / WebP / TIFF / GIF → tesseract direct
/// - HEIC / HEIF → heif-convert + tesseract
/// - format inconnu → erreur claire
#[cfg(feature = "server")]
async fn ocr_pdf_internal(bytes: Vec<u8>) -> Result<String, String> {
    let (ext, mode) = detect_image_kind(&bytes);

    tracing::info!(
        "OCR: fichier reçu, taille={} octets, format détecté={}, mode={:?}",
        bytes.len(),
        ext,
        mode
    );

    let remote_input = format!("/tmp/ocr_input.{}", ext);
    let tmp_path = std::env::temp_dir().join(format!("doli_ocr_input.{}", ext));

    tokio::fs::write(&tmp_path, &bytes)
        .await
        .map_err(|e| format!("Erreur write temp OCR : {}", e))?;

    let cp = tokio::process::Command::new("docker")
        .args([
            "cp",
            &tmp_path.to_string_lossy(),
            &format!("sci-family-dolibarr:{}", remote_input),
        ])
        .output()
        .await
        .map_err(|e| format!("Erreur docker cp OCR : {}", e))?;

    let _ = tokio::fs::remove_file(&tmp_path).await;

    if !cp.status.success() {
        return Err(format!(
            "Erreur docker cp OCR : {}",
            String::from_utf8_lossy(&cp.stderr)
        ));
    }

    // Commande OCR avec --psm 1 (orientation & script detection).
    // Fallback rotation 180° via ImageMagick si le texte n'a pas assez de chiffres.
    let shell_cmd = match mode {
        OcrMode::Pdf => format!(
            "rm -f /tmp/ocr_page*.png /tmp/ocr_page*.txt && \
             pdftoppm -png -r 300 {input} /tmp/ocr_page && \
             for f in /tmp/ocr_page*.png; do \
                 tesseract \"$f\" \"${{f%.png}}\" -l fra --psm 1 2>/dev/null; \
             done && \
             cat /tmp/ocr_page*.txt 2>/dev/null | tr -d '\\f'",
            input = remote_input
        ),
        OcrMode::Image => format!(
            "rm -f /tmp/ocr_image_out.txt /tmp/ocr_image_out_rot.txt && \
             tesseract {input} /tmp/ocr_image_out -l fra --psm 1 2>/dev/null; \
             T1=$(cat /tmp/ocr_image_out.txt 2>/dev/null | tr -d '\\f'); \
             N1=$(echo \"$T1\" | grep -oE '[0-9]{{4,}}' | wc -l); \
             if [ \"$N1\" -lt 3 ]; then \
                 convert {input} -rotate 180 /tmp/ocr_image_rot.jpg 2>/dev/null && \
                 tesseract /tmp/ocr_image_rot.jpg /tmp/ocr_image_out_rot -l fra --psm 1 2>/dev/null; \
                 T2=$(cat /tmp/ocr_image_out_rot.txt 2>/dev/null | tr -d '\\f'); \
                 N2=$(echo \"$T2\" | grep -oE '[0-9]{{4,}}' | wc -l); \
                 if [ \"$N2\" -gt \"$N1\" ]; then echo \"$T2\"; else echo \"$T1\"; fi; \
             else echo \"$T1\"; fi",
            input = remote_input
        ),
        OcrMode::Heic => format!(
            "rm -f /tmp/ocr_heic.png /tmp/ocr_heic.txt /tmp/ocr_heic_rot.png /tmp/ocr_heic_rot.txt && \
             heif-convert {input} /tmp/ocr_heic.png >/dev/null 2>&1 && \
             tesseract /tmp/ocr_heic.png /tmp/ocr_heic -l fra --psm 1 2>/dev/null; \
             T1=$(cat /tmp/ocr_heic.txt 2>/dev/null | tr -d '\\f'); \
             N1=$(echo \"$T1\" | grep -oE '[0-9]{{4,}}' | wc -l); \
             if [ \"$N1\" -lt 3 ]; then \
                 convert /tmp/ocr_heic.png -rotate 180 /tmp/ocr_heic_rot.png 2>/dev/null && \
                 tesseract /tmp/ocr_heic_rot.png /tmp/ocr_heic_rot -l fra --psm 1 2>/dev/null; \
                 T2=$(cat /tmp/ocr_heic_rot.txt 2>/dev/null | tr -d '\\f'); \
                 N2=$(echo \"$T2\" | grep -oE '[0-9]{{4,}}' | wc -l); \
                 if [ \"$N2\" -gt \"$N1\" ]; then echo \"$T2\"; else echo \"$T1\"; fi; \
             else echo \"$T1\"; fi",
            input = remote_input
        ),
        OcrMode::Unknown => {
            return Err(format!(
                "Format de fichier non reconnu (détecté comme '{}'). Formats acceptés : PDF, JPG, PNG, WebP, HEIC.",
                ext
            ));
        }
    };

    let tess = tokio::process::Command::new("docker")
        .args(["exec", "sci-family-dolibarr", "sh", "-c", &shell_cmd])
        .output()
        .await
        .map_err(|e| format!("Erreur exécution OCR : {}", e))?;

    if !tess.status.success() {
        return Err(format!(
            "Erreur OCR ({} , mode {:?}) : {}",
            ext,
            mode,
            String::from_utf8_lossy(&tess.stderr)
        ));
    }

    let text = String::from_utf8_lossy(&tess.stdout).to_string();

    // Nettoyage en root (le fichier /tmp/ocr_input.* a été créé par docker cp = root)
    let cleanup_cmd = match mode {
        OcrMode::Pdf => format!(
            "rm -f /tmp/ocr_page*.png /tmp/ocr_page*.txt {}",
            remote_input
        ),
        OcrMode::Image => format!(
            "rm -f /tmp/ocr_image_out.txt /tmp/ocr_image_out_rot.txt /tmp/ocr_image_rot.jpg {}",
            remote_input
        ),
        OcrMode::Heic => format!(
            "rm -f /tmp/ocr_heic.png /tmp/ocr_heic.txt /tmp/ocr_heic_rot.png /tmp/ocr_heic_rot.txt {}",
            remote_input
        ),
        OcrMode::Unknown => String::new(),
    };
    if !cleanup_cmd.is_empty() {
        let _ = tokio::process::Command::new("docker")
            .args(["exec", "-u", "root", "sci-family-dolibarr", "sh", "-c", &cleanup_cmd])
            .output()
            .await;
    }

    Ok(text)
}



#[cfg(feature = "server")]
#[derive(Debug, Clone, Copy)]
enum OcrMode {
    Pdf,
    Image,
    Heic,
    Unknown,
}

#[cfg(feature = "server")]
fn detect_image_kind(bytes: &[u8]) -> (&'static str, OcrMode) {
    if bytes.is_empty() {
        return ("empty", OcrMode::Unknown);
    }

    // PDF : %PDF
    if bytes.starts_with(b"%PDF") {
        return ("pdf", OcrMode::Pdf);
    }

    // JPEG : FF D8 FF
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return ("jpg", OcrMode::Image);
    }

    // PNG : 89 50 4E 47 0D 0A 1A 0A
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return ("png", OcrMode::Image);
    }

    // WebP : RIFF....WEBP
    if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return ("webp", OcrMode::Image);
    }

    // TIFF : II*\0 (little endian) ou MM\0* (big endian)
    if bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00])
        || bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
    {
        return ("tiff", OcrMode::Image);
    }

    // HEIC / HEIF : octets 4-7 = "ftyp", marque interne (bytes 8-11)
    if bytes.len() > 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"heic" || brand == b"heix" || brand == b"hevc"
            || brand == b"hevx" || brand == b"mif1" || brand == b"msf1"
        {
            return ("heic", OcrMode::Heic);
        }
    }

    // GIF : GIF87a / GIF89a (rare mais possible)
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return ("gif", OcrMode::Image);
    }

    ("bin", OcrMode::Unknown)
}

#[server]
pub async fn dolibarr_ocr_pdf(bytes: Vec<u8>) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        ocr_pdf_internal(bytes).await.map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_ocr_pdf est executee cote serveur"))
}

/// OCR ciblé sur la colonne « Total des cotisations » d'un avis de taxe foncière.
/// Recette validée : auto-orient + deskew + crop NE 10%x40%+50 + resize 400% + threshold 80% + psm 6.
#[cfg(feature = "server")]
async fn ocr_right_column(bytes: Vec<u8>) -> Result<String, String> {
    let (ext, mode) = detect_image_kind(&bytes);

    if matches!(mode, OcrMode::Unknown) {
        return Err("Format non reconnu pour OCR colonne droite".into());
    }

    let remote_input = format!("/tmp/ocr_rc_input.{}", ext);
    let tmp_path = std::env::temp_dir().join(format!("doli_ocr_rc.{}", ext));

    tokio::fs::write(&tmp_path, &bytes)
        .await
        .map_err(|e| format!("Erreur write temp : {}", e))?;

    let cp = tokio::process::Command::new("docker")
        .args([
            "cp",
            &tmp_path.to_string_lossy(),
            &format!("sci-family-dolibarr:{}", remote_input),
        ])
        .output()
        .await
        .map_err(|e| format!("Erreur docker cp : {}", e))?;
    let _ = tokio::fs::remove_file(&tmp_path).await;
    if !cp.status.success() {
        return Err(format!("Erreur docker cp : {}", String::from_utf8_lossy(&cp.stderr)));
    }

    let shell_cmd = if matches!(mode, OcrMode::Pdf) {
        format!(
            "rm -f /tmp/ocr_rc_step1.jpg /tmp/ocr_rc_col.jpg /tmp/ocr_rc_out.txt && \
             pdftoppm -png -r 300 {input} /tmp/ocr_rc_page && \
             FIRST=$(ls /tmp/ocr_rc_page*.png | head -1) && \
             convert \"$FIRST\" -auto-orient -deskew 40% /tmp/ocr_rc_step1.jpg 2>/dev/null && \
             convert /tmp/ocr_rc_step1.jpg -gravity NorthEast -crop 10%x40%+50+0 +repage -resize 400% -colorspace Gray -threshold 80% /tmp/ocr_rc_col.jpg 2>/dev/null && \
             tesseract /tmp/ocr_rc_col.jpg /tmp/ocr_rc_out -l fra --psm 6 2>/dev/null && \
             cat /tmp/ocr_rc_out.txt 2>/dev/null",
            input = remote_input
        )
    } else {
        format!(
            "rm -f /tmp/ocr_rc_step1.jpg /tmp/ocr_rc_col.jpg /tmp/ocr_rc_out.txt && \
             convert {input} -auto-orient -deskew 40% /tmp/ocr_rc_step1.jpg 2>/dev/null && \
             convert /tmp/ocr_rc_step1.jpg -gravity NorthEast -crop 10%x40%+50+0 +repage -resize 400% -colorspace Gray -threshold 80% /tmp/ocr_rc_col.jpg 2>/dev/null && \
             tesseract /tmp/ocr_rc_col.jpg /tmp/ocr_rc_out -l fra --psm 6 2>/dev/null && \
             cat /tmp/ocr_rc_out.txt 2>/dev/null",
            input = remote_input
        )
    };

    let out = tokio::process::Command::new("docker")
        .args(["exec", "sci-family-dolibarr", "sh", "-c", &shell_cmd])
        .output()
        .await
        .map_err(|e| format!("Erreur exec OCR colonne droite : {}", e))?;

    if !out.status.success() {
        return Err(format!(
            "Erreur OCR colonne droite : {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    // Nettoyage root
    let cleanup = format!(
        "rm -f {} /tmp/ocr_rc_step1.jpg /tmp/ocr_rc_col.jpg /tmp/ocr_rc_out.txt /tmp/ocr_rc_page*.png",
        remote_input
    );
    let _ = tokio::process::Command::new("docker")
        .args(["exec", "-u", "root", "sci-family-dolibarr", "sh", "-c", &cleanup])
        .output()
        .await;

    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[server]
pub async fn dolibarr_ocr_right_column(bytes: Vec<u8>) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        ocr_right_column(bytes).await.map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_ocr_right_column est executee cote serveur"))
}

/// OCR ciblé sur la colonne droite d'un avis de TF (feuillet 2 - frais).
/// Recette validée : auto-orient + deskew + crop East 20%x100%+100 + resize 250% + threshold 78% + psm 6.
#[cfg(feature = "server")]
async fn ocr_fees_column(bytes: Vec<u8>) -> Result<String, String> {
    let (ext, mode) = detect_image_kind(&bytes);

    if matches!(mode, OcrMode::Unknown) {
        return Err("Format non reconnu pour OCR frais".into());
    }

    let remote_input = format!("/tmp/ocr_fees_input.{}", ext);
    let tmp_path = std::env::temp_dir().join(format!("doli_ocr_fees.{}", ext));

    tokio::fs::write(&tmp_path, &bytes)
        .await
        .map_err(|e| format!("Erreur write temp : {}", e))?;

    let cp = tokio::process::Command::new("docker")
        .args([
            "cp",
            &tmp_path.to_string_lossy(),
            &format!("sci-family-dolibarr:{}", remote_input),
        ])
        .output()
        .await
        .map_err(|e| format!("Erreur docker cp : {}", e))?;
    let _ = tokio::fs::remove_file(&tmp_path).await;
    if !cp.status.success() {
        return Err(format!("Erreur docker cp : {}", String::from_utf8_lossy(&cp.stderr)));
    }

    let shell_cmd = if matches!(mode, OcrMode::Pdf) {
        format!(
            "rm -f /tmp/ocr_fees_step1.jpg /tmp/ocr_fees_col.jpg /tmp/ocr_fees_out.txt && \
             pdftoppm -png -r 300 {input} /tmp/ocr_fees_page && \
             FIRST=$(ls /tmp/ocr_fees_page*.png | head -1) && \
             convert \"$FIRST\" -auto-orient -deskew 40% /tmp/ocr_fees_step1.jpg 2>/dev/null && \
             convert /tmp/ocr_fees_step1.jpg -gravity East -crop 20%x100%+100+0 +repage -resize 250% -colorspace Gray -threshold 78% /tmp/ocr_fees_col.jpg 2>/dev/null && \
             tesseract /tmp/ocr_fees_col.jpg /tmp/ocr_fees_out -l fra --psm 6 2>/dev/null && \
             cat /tmp/ocr_fees_out.txt 2>/dev/null",
            input = remote_input
        )
    } else {
        format!(
            "rm -f /tmp/ocr_fees_step1.jpg /tmp/ocr_fees_col.jpg /tmp/ocr_fees_out.txt && \
             convert {input} -auto-orient -deskew 40% /tmp/ocr_fees_step1.jpg 2>/dev/null && \
             convert /tmp/ocr_fees_step1.jpg -gravity East -crop 20%x100%+100+0 +repage -resize 250% -colorspace Gray -threshold 78% /tmp/ocr_fees_col.jpg 2>/dev/null && \
             tesseract /tmp/ocr_fees_col.jpg /tmp/ocr_fees_out -l fra --psm 6 2>/dev/null && \
             cat /tmp/ocr_fees_out.txt 2>/dev/null",
            input = remote_input
        )
    };

    let out = tokio::process::Command::new("docker")
        .args(["exec", "sci-family-dolibarr", "sh", "-c", &shell_cmd])
        .output()
        .await
        .map_err(|e| format!("Erreur exec OCR frais : {}", e))?;

    if !out.status.success() {
        return Err(format!("Erreur OCR frais : {}", String::from_utf8_lossy(&out.stderr)));
    }

    let cleanup = format!(
        "rm -f {} /tmp/ocr_fees_step1.jpg /tmp/ocr_fees_col.jpg /tmp/ocr_fees_out.txt /tmp/ocr_fees_page*.png",
        remote_input
    );
    let _ = tokio::process::Command::new("docker")
        .args(["exec", "-u", "root", "sci-family-dolibarr", "sh", "-c", &cleanup])
        .output()
        .await;

    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[server]
pub async fn dolibarr_ocr_fees_column(bytes: Vec<u8>) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        ocr_fees_column(bytes).await.map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_ocr_fees_column est executee cote serveur"))
}

// ============================================================
//  FACTURES
// ============================================================

#[server]
pub async fn dolibarr_list_invoices(limit: u32) -> Result<Vec<DolibarrInvoice>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client.list_invoices(limit).await.map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_list_invoices est executee cote serveur"))
}

#[server]
pub async fn dolibarr_create_invoice(
    socid: String,
    date: i64,
    lines: Vec<crate::dolibarr::models::DolibarrInvoiceLine>,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .create_invoice(&socid, date, &lines)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_create_invoice est executee cote serveur"))
}

#[server]
pub async fn dolibarr_validate_invoice(invoice_id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .validate_invoice(&invoice_id)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_validate_invoice est executee cote serveur"))
}

#[server]
pub async fn dolibarr_get_invoice_lines(
    invoice_id: String,
) -> Result<Vec<crate::dolibarr::models::DolibarrInvoiceLine>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .get_invoice_lines(&invoice_id)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_get_invoice_lines est executee cote serveur"))
}

// ============================================================
//  PAIEMENTS
// ============================================================

#[server]
pub async fn dolibarr_list_invoice_payments(
    invoice_id: String,
) -> Result<Vec<DolibarrPayment>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .list_payments_for_invoice(&invoice_id)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_list_invoice_payments est executee cote serveur"))
}

#[server]
pub async fn dolibarr_create_payment(
    invoice_id: String,
    date: i64,
    amount: f64,
    payment_id: i32,
    account_id: i32,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .create_payment(&invoice_id, date, amount, payment_id, account_id)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_create_payment est executee cote serveur"))
}

// ============================================================
//  TIERS
// ============================================================

#[server]
pub async fn dolibarr_list_third_parties(limit: u32) -> Result<Vec<DolibarrThirdParty>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client.list_third_parties(limit).await.map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_list_third_parties est executee cote serveur"))
}

#[server]
pub async fn dolibarr_create_third_party(
    name: String,
    email: String,
    phone: String,
    address: String,
    zip: String,
    town: String,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .create_third_party(&name, &email, &phone, &address, &zip, &town)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_create_third_party est executee cote serveur"))
}

#[server]
pub async fn dolibarr_update_third_party(
    id: String,
    name: String,
    email: String,
    phone: String,
    address: String,
    zip: String,
    town: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .update_third_party(&id, &name, &email, &phone, &address, &zip, &town)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_update_third_party est executee cote serveur"))
}

#[server]
pub async fn dolibarr_delete_third_party(id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .delete_third_party(&id)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_delete_third_party est executee cote serveur"))
}

// ============================================================
//  BANQUE
// ============================================================

#[server]
pub async fn dolibarr_list_bank_accounts() -> Result<Vec<DolibarrBankAccount>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client.list_bank_accounts().await.map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_list_bank_accounts est executee cote serveur"))
}

#[server]
pub async fn dolibarr_list_bank_lines(
    account_id: String,
) -> Result<Vec<DolibarrBankLine>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client.list_bank_lines(&account_id).await.map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_list_bank_lines est executee cote serveur"))
}

#[server]
pub async fn dolibarr_create_bank_line(
    account_id: String,
    dateo: i64,
    amount: f64,
    label: String,
    line_type: String,
    num_releve: String,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        let draft = DolibarrBankLineDraft {
            date: dateo,
            datev: dateo,
            amount,
            label,
            line_type,
            num_releve,
        };
        client
            .create_bank_line(&account_id, &draft)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_create_bank_line est executee cote serveur"))
}

#[server]
pub async fn dolibarr_delete_bank_lines(
    account_id: String,
    line_ids: Vec<String>,
) -> Result<usize, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if line_ids.is_empty() {
            return Ok(0);
        }

        let acc_clean = sanitize_id(&account_id)?;
        let ids_clean: Vec<String> = line_ids
            .iter()
            .map(|id| sanitize_id(id))
            .collect::<Result<Vec<_>, _>>()?;

        let ids_join = ids_clean.join(",");
        let sql = format!(
            "DELETE FROM llx_bank WHERE rowid IN ({}) AND fk_account = {} AND rappro = 0;",
            ids_join, acc_clean
        );

        let output = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-c", &sql])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur exec docker : {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ServerFnError::new(format!("Erreur SQL : {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let count = stdout
            .lines()
            .find(|l| l.trim_start().starts_with("DELETE"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(ids_clean.len());

        Ok(count)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_delete_bank_lines est executee cote serveur"))
}

// ============================================================
//  DOCUMENTS (GED)
// ============================================================

#[server]
pub async fn dolibarr_list_documents(
    modulepart: String,
    id: String,
) -> Result<Vec<DolibarrDocument>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .list_documents(&modulepart, &id)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_list_documents est executee cote serveur"))
}

#[server]
pub async fn dolibarr_delete_document(
    modulepart: String,
    original_file: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let client = crate::dolibarr::client::DolibarrClient::from_env()
            .map_err(ServerFnError::new)?;
        client
            .delete_document(&modulepart, &original_file)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_delete_document est executee cote serveur"))
}

#[server]
pub async fn dolibarr_upload_document_sql(
    modulepart: String,
    element_id: String,
    element_ref: String,
    filename: String,
    bytes: Vec<u8>,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let eid_clean = sanitize_id(&element_id)?;

        let safe_ref: String = element_ref
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
            .collect::<String>()
            .trim()
            .replace(' ', "_");
        let base_filename: String = filename
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.' || *c == ' ')
            .collect::<String>()
            .trim()
            .replace(' ', "_");

        if safe_ref.is_empty() || base_filename.is_empty() {
            return Err(ServerFnError::new("Nom de fichier ou reference invalide"));
        }

        let (rel_dir, target_type) = match modulepart.as_str() {
            "societe" => (format!("societe/{}", safe_ref), "societe"),
            "facture" => (format!("facture/{}", safe_ref), "facture"),
            "facture_fournisseur" => (format!("fournisseur/facture/{}", safe_ref), "facture_fournisseur"),
            _ => return Err(ServerFnError::new(format!("modulepart inconnu : {}", modulepart))),
        };

        let final_filename = find_available_name(&rel_dir, &base_filename).await?;
        let rel_filepath = format!("{}/{}", rel_dir, final_filename);
        let container_dir = format!("/var/www/documents/{}", rel_dir);
        let container_filepath = format!("/var/www/documents/{}", rel_filepath);

        write_file_in_container(&container_dir, &container_filepath, &bytes, &final_filename).await?;

        let esc_filename = final_filename.replace('\'', "''");
        let esc_filepath = rel_filepath.replace('\'', "''");

        let sql_ecm = format!(
            "INSERT INTO llx_ecm_files (filename, filepath, date_c, entity, gen_or_uploaded, label) \
             VALUES ('{}', '{}', NOW(), 1, 'uploaded', '{}');",
            esc_filename, esc_filepath, esc_filename
        );
        let ecm_out = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-c", &sql_ecm])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL ecm : {}", e)))?;
        if !ecm_out.status.success() {
            return Err(ServerFnError::new(format!(
                "Erreur SQL ecm : {}",
                String::from_utf8_lossy(&ecm_out.stderr)
            )));
        }

        let sql_get_id = format!(
            "SELECT rowid FROM llx_ecm_files WHERE filepath = '{}' ORDER BY rowid DESC LIMIT 1;",
            esc_filepath
        );
        let id_out = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", &sql_get_id])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL get id : {}", e)))?;
        let ecm_id_str = String::from_utf8_lossy(&id_out.stdout).trim().to_string();
        let ecm_id: i64 = ecm_id_str
            .parse()
            .map_err(|_| ServerFnError::new("ID ECM introuvable"))?;

        let sql_link = format!(
            "INSERT INTO llx_element_element (fk_source, sourcetype, fk_target, targettype) \
             VALUES ({}, 'ecmfile', {}, '{}');",
            ecm_id, eid_clean, target_type
        );
        let l_out = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-c", &sql_link])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL link : {}", e)))?;
        if !l_out.status.success() {
            return Err(ServerFnError::new(format!(
                "Erreur SQL link : {}",
                String::from_utf8_lossy(&l_out.stderr)
            )));
        }

        Ok(final_filename)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_upload_document_sql est executee cote serveur"))
}

#[server]
pub async fn dolibarr_upload_patrimoine_document(
    bien_name: String,
    lot_code: String,
    filename: String,
    bytes: Vec<u8>,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let sql_get = "SELECT rowid FROM llx_societe WHERE name_alias = 'PATRIMOINE-SCI' AND entity = 1 LIMIT 1;";
        let id_out = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", sql_get])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL : {}", e)))?;
        let eid_str = String::from_utf8_lossy(&id_out.stdout).trim().to_string();
        let element_id: i64 = eid_str
            .parse()
            .map_err(|_| ServerFnError::new("Tiers 'Patrimoine SCI' introuvable. Lance la commande SQL de creation."))?;

        let safe = |s: &str| -> String {
            s.chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
                .collect::<String>()
                .trim()
                .replace(' ', "_")
        };
        let bien_clean = safe(&bien_name);
        let lot_clean = safe(&lot_code);
        let base_filename: String = filename
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.' || *c == ' ')
            .collect::<String>()
            .trim()
            .replace(' ', "_");

        if bien_clean.is_empty() || base_filename.is_empty() {
            return Err(ServerFnError::new("Nom de bien ou de fichier invalide"));
        }

        let rel_dir = if lot_clean.is_empty() {
            format!("societe/Patrimoine_SCI/{}", bien_clean)
        } else {
            format!("societe/Patrimoine_SCI/{}/{}", bien_clean, lot_clean)
        };

        let final_filename = find_available_name(&rel_dir, &base_filename).await?;
        let rel_filepath = format!("{}/{}", rel_dir, final_filename);
        let container_dir = format!("/var/www/documents/{}", rel_dir);
        let container_filepath = format!("/var/www/documents/{}", rel_filepath);

        write_file_in_container(&container_dir, &container_filepath, &bytes, &final_filename).await?;

        let esc_filename = final_filename.replace('\'', "''");
        let esc_filepath = rel_filepath.replace('\'', "''");

        let sql_ecm = format!(
            "INSERT INTO llx_ecm_files (filename, filepath, date_c, entity, gen_or_uploaded, label) \
             VALUES ('{}', '{}', NOW(), 1, 'uploaded', '{}');",
            esc_filename, esc_filepath, esc_filename
        );
        let ecm_out = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-c", &sql_ecm])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL ecm : {}", e)))?;
        if !ecm_out.status.success() {
            return Err(ServerFnError::new(format!(
                "Erreur SQL ecm : {}",
                String::from_utf8_lossy(&ecm_out.stderr)
            )));
        }

        let sql_get_id = format!(
            "SELECT rowid FROM llx_ecm_files WHERE filepath = '{}' ORDER BY rowid DESC LIMIT 1;",
            esc_filepath
        );
        let id_out2 = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", &sql_get_id])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL get id : {}", e)))?;
        let ecm_id_str = String::from_utf8_lossy(&id_out2.stdout).trim().to_string();
        let ecm_id: i64 = ecm_id_str
            .parse()
            .map_err(|_| ServerFnError::new("ID ECM introuvable"))?;

        let sql_link = format!(
            "INSERT INTO llx_element_element (fk_source, sourcetype, fk_target, targettype) \
             VALUES ({}, 'ecmfile', {}, 'societe');",
            ecm_id, element_id
        );
        let l_out = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-c", &sql_link])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL link : {}", e)))?;
        if !l_out.status.success() {
            return Err(ServerFnError::new(format!(
                "Erreur SQL link : {}",
                String::from_utf8_lossy(&l_out.stderr)
            )));
        }

        Ok(final_filename)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_upload_patrimoine_document est executee cote serveur"))
}

#[server]
pub async fn dolibarr_list_patrimoine_documents(
    bien_name: String,
) -> Result<Vec<DolibarrDocument>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let safe_bien: String = bien_name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
            .collect::<String>()
            .trim()
            .replace(' ', "_");

        if safe_bien.is_empty() {
            return Ok(Vec::new());
        }

        let sql = format!(
            "SELECT rowid, filename, filepath, EXTRACT(EPOCH FROM date_c)::bigint \
             FROM llx_ecm_files \
             WHERE filepath LIKE 'societe/Patrimoine_SCI/{}/%' \
             ORDER BY date_c DESC LIMIT 200;",
            safe_bien
        );

        let out = tokio::process::Command::new("docker")
            .args([
                "exec", "sci-family-dolibarr-db",
                "psql", "-U", "dolibarr", "-d", "dolibarr",
                "-t", "-A", "-F", "|", "-c", &sql,
            ])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL list : {}", e)))?;

        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut docs = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() < 4 {
                continue;
            }
            let filepath = parts[2].to_string();
            let relativename = filepath
                .strip_prefix(&format!("societe/Patrimoine_SCI/{}/", safe_bien))
                .unwrap_or(&filepath)
                .to_string();
            docs.push(DolibarrDocument {
                filename: parts[1].to_string(),
                filepath: filepath.clone(),
                fullpath: format!("/var/www/documents/{}", filepath),
                modulepart: "societe".to_string(),
                size: 0,
                date: parts[3].parse().unwrap_or(0),
                mime: String::new(),
                level1name: safe_bien.clone(),
                relativename,
            });
        }
        Ok(docs)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_list_patrimoine_documents est executee cote serveur"))
}

// ============================================================
//  SCAN FACTURE FOURNISSEUR (pdf-extract + OCR fallback)
// ============================================================

use crate::dolibarr::invoice_parser::ParsedInvoice;

#[cfg(feature = "server")]
use crate::dolibarr::invoice_parser::{extract_text_from_pdf, parse_invoice_text};

/// Analyse un PDF de facture fournisseur.
/// Utilise pdf-extract d'abord. Si le texte extrait est trop court (< 200 car),
/// bascule automatiquement sur l'OCR Tesseract.
#[server]
pub async fn dolibarr_parse_invoice_pdf(
    filename: String,
    bytes: Vec<u8>,
) -> Result<ParsedInvoice, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let _ = filename;
        let direct_text = extract_text_from_pdf(&bytes).unwrap_or_default();

        let final_text = if direct_text.trim().len() < 200 {
            match ocr_pdf_internal(bytes.clone()).await {
                Ok(ocr_text) if ocr_text.trim().len() > direct_text.trim().len() => ocr_text,
                Ok(_) => direct_text,
                Err(_) => direct_text,
            }
        } else {
            direct_text
        };

        let parsed = parse_invoice_text(&final_text);
        Ok(parsed)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_parse_invoice_pdf est executee cote serveur"))
}

/// Cree une facture fournisseur dans Dolibarr.
/// Retourne une string "{invoice_id}|{invoice_ref}".
#[server]
pub async fn dolibarr_create_supplier_invoice(
    supplier_name: String,
    invoice_number: String,
    invoice_date: String,
    due_date: String,
    total_ht: String,
    total_tva: String,
    total_ttc: String,
    tva_rate: String,
    vat_lines: Vec<crate::dolibarr::invoice_parser::VatLine>,
    file_name: String,
    file_bytes: Vec<u8>,
    bien_name: String,
    lot_code: String,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let safe_supplier = supplier_name.replace('\'', "''").trim().to_string();
        if safe_supplier.is_empty() {
            return Err(ServerFnError::new("Nom du fournisseur manquant"));
        }

        let sql_find = format!(
            "SELECT rowid FROM llx_societe WHERE entity=1 AND nom = '{}' AND fournisseur = 1 LIMIT 1;",
            safe_supplier
        );
        let find_out = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", &sql_find])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL find : {}", e)))?;
        let found = String::from_utf8_lossy(&find_out.stdout).trim().to_string();

        let supplier_id: i64 = if found.is_empty() {
            let sql_create = format!(
                "INSERT INTO llx_societe (nom, entity, client, fournisseur, status, datec) \
                 VALUES ('{}', 1, 0, 1, 1, NOW()) RETURNING rowid;",
                safe_supplier
            );
            let create_out = tokio::process::Command::new("docker")
                .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", &sql_create])
                .output()
                .await
                .map_err(|e| ServerFnError::new(format!("Erreur SQL create supplier : {}", e)))?;
            let sid_str = String::from_utf8_lossy(&create_out.stdout).trim().to_string();
            if let Ok(n) = sid_str.parse::<i64>() {
                n
            } else {
                let sql_get_soc = format!(
                    "SELECT rowid FROM llx_societe WHERE entity=1 AND nom = '{}' AND fournisseur = 1 ORDER BY rowid DESC LIMIT 1;",
                    safe_supplier
                );
                let get_out = tokio::process::Command::new("docker")
                    .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", &sql_get_soc])
                    .output()
                    .await
                    .map_err(|e| ServerFnError::new(format!("Erreur SQL get soc : {}", e)))?;
                String::from_utf8_lossy(&get_out.stdout).trim().parse::<i64>()
                    .map_err(|_| ServerFnError::new("Impossible de creer le fournisseur"))?
            }
        } else {
            found.parse().map_err(|_| ServerFnError::new("ID fournisseur invalide"))?
        };

        let ht_eur = parse_eur(&total_ht);
        let tva_eur = parse_eur(&total_tva);
        let ttc_eur = parse_eur(&total_ttc);

        let mut note_parts: Vec<String> = Vec::new();
        if !bien_name.trim().is_empty() {
            note_parts.push(format!("Bien: {}", bien_name.trim()));
        }
        if !lot_code.trim().is_empty() {
            note_parts.push(format!("Lot: {}", lot_code.trim()));
        }
        note_parts.push("Source: OCR app SCI Family".to_string());
        let note_private = note_parts.join(" | ").replace('\'', "''");

        let final_ref: String = if invoice_number.trim().is_empty() {
            format!("FOURN-{}", chrono::Utc::now().timestamp())
        } else {
            sanitize_ref(invoice_number.trim())
        };
        let ref_safe = final_ref.replace('\'', "''");

        let date_safe = if invoice_date.is_empty() {
            "CURRENT_DATE".to_string()
        } else {
            format!("'{}'", invoice_date.replace('\'', "''"))
        };
        let due_date_safe = if due_date.is_empty() {
            "NULL".to_string()
        } else {
            format!("'{}'", due_date.replace('\'', "''"))
        };

        let sql_inv = format!(
            "INSERT INTO llx_facture_fourn (ref, ref_supplier, entity, fk_soc, datec, datef, date_lim_reglement, total_ht, total_tva, total_ttc, fk_statut, paye, note_private) \
             VALUES ('{}', '{}', 1, {}, NOW(), {}, {}, {}, {}, {}, 0, 0, '{}');",
            ref_safe, ref_safe, supplier_id, date_safe, due_date_safe, ht_eur, tva_eur, ttc_eur, note_private
        );
        let inv_out = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-c", &sql_inv])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL facture : {}", e)))?;

        if !inv_out.status.success() {
            return Err(ServerFnError::new(format!(
                "Erreur creation facture : {}",
                String::from_utf8_lossy(&inv_out.stderr)
            )));
        }

        let sql_get_inv = format!(
            "SELECT rowid FROM llx_facture_fourn WHERE entity=1 AND ref = '{}' ORDER BY rowid DESC LIMIT 1;",
            ref_safe
        );
        let get_out = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", &sql_get_inv])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL get inv : {}", e)))?;
        let invoice_id: i64 = String::from_utf8_lossy(&get_out.stdout).trim().parse::<i64>()
            .map_err(|_| ServerFnError::new("ID facture fournisseur introuvable"))?;

        if !vat_lines.is_empty() {
            for vl in vat_lines.iter() {
                let base_e = parse_eur(&vl.base_ht);
                let tva_e = parse_eur(&vl.amount_tva);
                let ttc_e = if !vl.amount_ttc.is_empty() {
                    parse_eur(&vl.amount_ttc)
                } else {
                    base_e + tva_e
                };
                let rate: f64 = parse_eur(&vl.rate);
                let label = format!("Ligne TVA {}%", vl.rate);

                let sql_line = format!(
                    "INSERT INTO llx_facture_fourn_det (fk_facture_fourn, description, qty, tva_tx, total_ht, total_tva, total_ttc) \
                     VALUES ({}, '{}', 1, {}, {}, {}, {});",
                    invoice_id, label.replace('\'', "''"), rate, base_e, tva_e, ttc_e
                );
                let _ = tokio::process::Command::new("docker")
                    .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-c", &sql_line])
                    .output()
                    .await;
            }
        } else {
            let rate: f64 = parse_eur(&tva_rate);
            let rate = if rate == 0.0 { 20.0 } else { rate };
            let sql_line = format!(
                "INSERT INTO llx_facture_fourn_det (fk_facture_fourn, description, qty, tva_tx, total_ht, total_tva, total_ttc) \
                 VALUES ({}, 'Ligne importee automatiquement', 1, {}, {}, {}, {});",
                invoice_id, rate, ht_eur, tva_eur, ttc_eur
            );
            let _ = tokio::process::Command::new("docker")
                .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-c", &sql_line])
                .output()
                .await;
        }

        let safe_filename: String = file_name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.' || *c == ' ')
            .collect::<String>()
            .trim()
            .replace(' ', "_");

        let safe_filename = if safe_filename.is_empty() {
            "facture.pdf".to_string()
        } else {
            safe_filename
        };

        let rel_dir = format!("fournisseur/facture/{}", final_ref);
        let container_dir = format!("/var/www/documents/{}", rel_dir);
        let container_filepath = format!("{}/{}", container_dir, safe_filename);

        write_file_in_container(&container_dir, &container_filepath, &file_bytes, &safe_filename).await?;

        let esc_filename = safe_filename.replace('\'', "''");
        let esc_filepath = format!("{}/{}", rel_dir, safe_filename).replace('\'', "''");

        let sql_ecm = format!(
            "INSERT INTO llx_ecm_files (filename, filepath, date_c, entity, gen_or_uploaded, label) \
             VALUES ('{}', '{}', NOW(), 1, 'uploaded', '{}');",
            esc_filename, esc_filepath, esc_filename
        );
        let _ = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-c", &sql_ecm])
            .output()
            .await;

        let sql_get_id = format!(
            "SELECT rowid FROM llx_ecm_files WHERE filepath = '{}' ORDER BY rowid DESC LIMIT 1;",
            esc_filepath
        );
        let id_out = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", &sql_get_id])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL get id : {}", e)))?;
        let ecm_id_str = String::from_utf8_lossy(&id_out.stdout).trim().to_string();
        if let Ok(ecm_id) = ecm_id_str.parse::<i64>() {
            let sql_link = format!(
                "INSERT INTO llx_element_element (fk_source, sourcetype, fk_target, targettype) \
                 VALUES ({}, 'ecmfile', {}, 'facture_fourn');",
                ecm_id, invoice_id
            );
            let _ = tokio::process::Command::new("docker")
                .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-c", &sql_link])
                .output()
                .await;
        }

        Ok(format!("{}|{}", invoice_id, final_ref))
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_create_supplier_invoice est executee cote serveur"))
}

#[server]
pub async fn dolibarr_link_document(
    ecm_file_id: i64,
    target_id: String,
    target_type: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let tid = sanitize_id(&target_id)?;
        let allowed = ["societe", "facture", "facture_fourn", "project", "user"];
        if !allowed.contains(&target_type.as_str()) {
            return Err(ServerFnError::new("target_type invalide"));
        }
        let sql = format!(
            "INSERT INTO llx_element_element (fk_source, sourcetype, fk_target, targettype) \
             VALUES ({}, 'ecmfile', {}, '{}') ON CONFLICT DO NOTHING;",
            ecm_file_id, tid, target_type
        );
        let out = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-c", &sql])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL : {}", e)))?;
        if !out.status.success() {
            return Err(ServerFnError::new(format!(
                "Erreur SQL : {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_link_document est executee cote serveur"))
}

#[server]
pub async fn dolibarr_get_last_ecm_id() -> Result<i64, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let sql = "SELECT rowid FROM llx_ecm_files ORDER BY rowid DESC LIMIT 1;";
        let out = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", sql])
            .output()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur SQL : {}", e)))?;
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        s.parse::<i64>().map_err(|_| ServerFnError::new("Aucun fichier ECM"))
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_get_last_ecm_id est executee cote serveur"))
}

#[server]
pub async fn dolibarr_download_from_url(
    url: String,
) -> Result<(String, Vec<u8>), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let url = url.trim();
        if url.is_empty() {
            return Err(ServerFnError::new("URL vide"));
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ServerFnError::new("URL doit commencer par http:// ou https://"));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| ServerFnError::new(format!("Erreur client HTTP : {}", e)))?;

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur telechargement : {}", e)))?;

        if !resp.status().is_success() {
            return Err(ServerFnError::new(format!(
                "Erreur HTTP {} : {}",
                resp.status(),
                url
            )));
        }

        const MAX_SIZE: usize = 25 * 1024 * 1024;
        if let Some(len) = resp.content_length() {
            if len as usize > MAX_SIZE {
                return Err(ServerFnError::new(format!(
                    "Fichier trop gros : {} Mo (max 25 Mo)",
                    len / 1024 / 1024
                )));
            }
        }

        let content_disposition = resp
            .headers()
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let filename = extract_filename_from_response(url, content_disposition.as_deref());

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ServerFnError::new(format!("Erreur lecture body : {}", e)))?
            .to_vec();

        if bytes.len() > MAX_SIZE {
            return Err(ServerFnError::new(format!(
                "Fichier trop gros : {} Mo (max 25 Mo)",
                bytes.len() / 1024 / 1024
            )));
        }

        Ok((filename, bytes))
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dolibarr_download_from_url est executee cote serveur"))
}

#[cfg(feature = "server")]
fn extract_filename_from_response(url: &str, content_disposition: Option<&str>) -> String {
    if let Some(cd) = content_disposition {
        let cd_lower = cd.to_lowercase();
        if cd_lower.contains("filename=") {
            let after = &cd[cd_lower.find("filename=").unwrap() + 9..];
            let candidate = after
                .trim_start_matches('"')
                .split('"')
                .next()
                .unwrap_or("")
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !candidate.is_empty() {
                return sanitize_filename(&candidate);
            }
        }
    }

    let without_query = url.split('?').next().unwrap_or(url);
    let segment = without_query.split('/').next_back().unwrap_or("");
    if !segment.is_empty() && segment.contains('.') {
        return sanitize_filename(segment);
    }

    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    format!("fichier_{}.pdf", ts)
}

#[cfg(feature = "server")]
fn sanitize_filename(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.' || *c == ' ')
        .collect();
    let trimmed = cleaned.trim().replace(' ', "_");
    if trimmed.is_empty() {
        "fichier.pdf".to_string()
    } else {
        trimmed
    }
}


// ============================================================
//  FLUX BANCAIRES DOLIBARR + MÉTADONNÉES LOCALES (TVA)
// ============================================================

/// Ligne bancaire enrichie avec ses métadonnées locales (taux TVA).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BankLineEnriched {
    pub dolibarr_id: String,
    pub date: String,
    pub label: String,
    pub amount: f64,
    pub vat_rate_bp: Option<i32>,
}

/// Charge toutes les lignes bancaires Dolibarr du compte principal actif,
/// jointes avec les métadonnées locales (taux TVA) depuis bank_line_metadata.
#[server]
pub async fn dolibarr_get_all_bank_lines_enriched(
) -> Result<Vec<BankLineEnriched>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::dolibarr::client::DolibarrClient;

        let client = DolibarrClient::from_env().map_err(ServerFnError::new)?;

        let accounts = client
            .list_bank_accounts()
            .await
            .map_err(ServerFnError::new)?;

        let Some(account) = accounts.iter().find(|a| a.clos == 0) else {
            return Ok(Vec::new());
        };

        let lines = client
            .list_bank_lines(&account.id)
            .await
            .map_err(ServerFnError::new)?;

        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        use sqlx::Row;
        let rows = sqlx::query(
            "SELECT dolibarr_line_id, vat_rate_bp FROM bank_line_metadata",
        )
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        let mut rates: std::collections::HashMap<String, Option<i32>> =
            std::collections::HashMap::new();
        for r in rows {
            let id: String = r.get("dolibarr_line_id");
            let rate: Option<i32> = r.get("vat_rate_bp");
            rates.insert(id, rate);
        }

        let enriched: Vec<BankLineEnriched> = lines
            .into_iter()
            .map(|l| {
                let amount: f64 = l.amount.parse().unwrap_or(0.0);
                let date = chrono::DateTime::<chrono::Utc>::from_timestamp(l.dateo, 0)
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                let rate = rates.get(&l.id).copied().flatten();
                BankLineEnriched {
                    dolibarr_id: l.id.clone(),
                    date,
                    label: l.label.clone(),
                    amount,
                    vat_rate_bp: rate,
                }
            })
            .collect();

        Ok(enriched)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "dolibarr_get_all_bank_lines_enriched est executee cote serveur",
    ))
}

/// Attribue (ou efface) un taux de TVA à une ligne bancaire Dolibarr.
#[server]
pub async fn dolibarr_set_bank_line_vat_rate(
    dolibarr_line_id: String,
    vat_rate_bp: Option<i32>,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let rate = match vat_rate_bp {
            Some(bp) if (0..=10000).contains(&bp) => Some(bp),
            Some(_) => return Err(ServerFnError::new("Taux TVA invalide (0-10000 bp)")),
            None => None,
        };
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        sqlx::query(
            "INSERT INTO bank_line_metadata (dolibarr_line_id, vat_rate_bp, updated_at) \
             VALUES ($1, $2, now()) \
             ON CONFLICT (dolibarr_line_id) DO UPDATE \
                SET vat_rate_bp = EXCLUDED.vat_rate_bp, updated_at = now()",
        )
        .bind(&dolibarr_line_id)
        .bind(rate)
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "dolibarr_set_bank_line_vat_rate est executee cote serveur",
    ))
}


// ============================================================
//  IMPORT EMAIL (Gmail IMAP)
// ============================================================

/// Scanne le dossier Gmail "Facture" et renvoie la liste des emails non lus.
#[server]
pub async fn dolibarr_scan_email_factures() -> Result<Vec<EmailFacture>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        crate::email::scan_facture_folder().map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "dolibarr_scan_email_factures est executee cote serveur",
    ))
}

/// Recupere le PDF joint du premier fichier PDF d'un email.
#[server]
pub async fn dolibarr_fetch_email_attachment(
    uid: u32,
) -> Result<(String, Vec<u8>), ServerFnError> {
    #[cfg(feature = "server")]
    {
        crate::email::fetch_attachment(uid).map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "dolibarr_fetch_email_attachment est executee cote serveur",
    ))
}