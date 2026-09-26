use super::{classifier, extractor, DocumentType};
use super::extractor::ExtractionInput;
use crate::domain::{DocumentFolderItem, DocumentItem, DocumentStorageConfigItem};
use crate::entity_scope::current_legal_entity_id;
use crate::ui::{FormField, ModuleHeader};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use rust_decimal::prelude::ToPrimitive;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentOcrSummary {
    pub document_id: Uuid,
    pub status: String,
    pub ocr_status: String,
    pub classification: String,
    pub classification_confidence: f32,
    pub extraction_confidence: f32,
    pub requires_validation: bool,
}

#[cfg(feature = "server")]
fn safe_filename(input: &str) -> Result<String, String> {
    let name = std::path::Path::new(input.trim())
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .trim();
    if name.is_empty() || name == "." || name == ".." {
        return Err("Nom de fichier invalide".to_owned());
    }
    if name.contains('\0') || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Nom de fichier non sécurisé".to_owned());
    }
    Ok(name.to_owned())
}

#[cfg(feature = "server")]
fn safe_storage_key(input: &str) -> Result<std::path::PathBuf, String> {
    let raw = input.trim().replace('\\', "/");
    if raw.is_empty() || raw.starts_with('/') || raw.contains("../") || raw == ".." {
        return Err("Clé de stockage non sécurisée".to_owned());
    }
    let path = std::path::PathBuf::from(&raw);
    if path.components().any(|c| matches!(c, std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_))) {
        return Err("Clé de stockage non sécurisée".to_owned());
    }
    Ok(path)
}

#[cfg(feature = "server")]
async fn storage_config(pool: &sqlx::PgPool, entity: Uuid) -> Result<(std::path::PathBuf, Value), sqlx::Error> {
    let row = sqlx::query("SELECT root_path,folder_overrides FROM document_storage_config WHERE legal_entity_id=$1")
        .bind(entity)
        .fetch_optional(pool)
        .await?;
    let (root, overrides) = if let Some(r) = row {
        (sqlx::Row::get::<String, _>(&r, "root_path"), sqlx::Row::get::<Value, _>(&r, "folder_overrides"))
    } else {
        ("documents".to_owned(), Value::Object(Default::default()))
    };
    Ok((std::path::PathBuf::from(root), overrides))
}

#[cfg(feature = "server")]
fn folder_path(code: &str, overrides: &Value) -> String {
    overrides
        .get(code)
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| match code {
            "SOCIETE" => "01_SOCIETE",
            "ASSOCIES" => "02_ASSOCIES",
            "IMMEUBLES" => "03_IMMEUBLES",
            "BAUX" => "04_BAUX",
            "LOCATAIRES" => "05_LOCATAIRES",
            "FACTURES" => "06_FACTURES",
            "BANQUE" => "07_BANQUE",
            "FISCALITE" => "08_FISCALITE",
            "TAXES" => "09_TAXES",
            "TRAVAUX" => "11_TRAVAUX",
            "JURIDIQUE" => "12_JURIDIQUE",
            "ARCHIVES" => "99_ARCHIVES",
            _ => "99_ARCHIVES",
        })
        .to_owned()
}

#[cfg(feature = "server")]
fn category_folder(category: &str) -> &'static str {
    match category.trim().to_uppercase().as_str() {
        "SOCIETE" | "ADMINISTRATIF" => "SOCIETE",
        "ASSOCIE" | "ASSOCIES" => "ASSOCIES",
        "IMMEUBLE" | "IMMEUBLES" | "PROPERTY" => "IMMEUBLES",
        "BAIL" | "BAUX" | "AVENANT" => "BAUX",
        "LOCATAIRE" | "LOCATAIRES" => "LOCATAIRES",
        "FACTURE" | "FACTURES" | "RENT_INVOICE" => "FACTURES",
        "BANQUE" | "RELEVE" => "BANQUE",
        "FISCAL" | "FISCALITE" => "FISCALITE",
        "TAXE" | "TAXES" => "TAXES",
        "TRAVAUX" => "TRAVAUX",
        "JURIDIQUE" => "JURIDIQUE",
        _ => "ARCHIVES",
    }
}

fn document_type_label(value: &DocumentType) -> &'static str {
    match value {
        DocumentType::InvoiceSupplier => "FACTURE_FOURNISSEUR",
        DocumentType::RentInvoice => "FACTURE_LOYER",
        DocumentType::BankStatement => "RELEVE_BANCAIRE",
        DocumentType::PaymentProof => "JUSTIFICATIF_PAIEMENT",
        DocumentType::Lease => "BAIL",
        DocumentType::LeaseAmendment => "AVENANT",
        DocumentType::TaxDocument => "TAXE_FISCAL",
        DocumentType::Insurance => "ASSURANCE",
        DocumentType::Administrative => "ADMINISTRATIF",
        DocumentType::Correspondence => "COURRIER",
        DocumentType::SupportingDocument => "JUSTIFICATIF",
        DocumentType::Unknown => "INCONNU",
    }
}

#[server]
pub async fn get_document_storage_config() -> Result<DocumentStorageConfigItem, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let _ = crate::security::require_permission(pool, "DATA_READ").await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let row = sqlx::query("SELECT legal_entity_id,root_path,folder_overrides,updated_at FROM document_storage_config WHERE legal_entity_id=$1")
            .bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?;
        if let Some(r) = row {
            Ok(DocumentStorageConfigItem { legal_entity_id: sqlx::Row::get(&r,"legal_entity_id"), root_path: sqlx::Row::get(&r,"root_path"), folder_overrides: sqlx::Row::get(&r,"folder_overrides") })
        } else {
            Ok(DocumentStorageConfigItem { legal_entity_id: entity, root_path: "documents".into(), folder_overrides: serde_json::json!({}) })
        }
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("get_document_storage_config est exécutée côté serveur"))
}

#[server]
pub async fn list_document_folders() -> Result<Vec<DocumentFolderItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let _ = crate::security::require_permission(pool, "DATA_READ").await.map_err(ServerFnError::new)?;
        let cfg = get_document_storage_config().await?;
        let rows = sqlx::query("SELECT code,label,relative_path,active FROM document_folder_catalog WHERE active ORDER BY code")
            .fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| {
            let code: String = sqlx::Row::get(&r,"code");
            let default_path: String = sqlx::Row::get(&r,"relative_path");
            let configured = cfg.folder_overrides.get(&code).and_then(Value::as_str).unwrap_or(&default_path).to_owned();
            DocumentFolderItem { code, label: sqlx::Row::get(&r,"label"), relative_path: configured, active: sqlx::Row::get(&r,"active") }
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_document_folders est exécutée côté serveur"))
}

#[server]
pub async fn set_document_storage_config(root_path: String, folder_overrides: Value) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let _ = crate::security::require_permission(pool, "DATA_WRITE").await.map_err(ServerFnError::new)?;
        let root = root_path.trim();
        if root.is_empty() || root.len() > 1000 || root.contains('\0') {
            return Err(ServerFnError::new("Racine documentaire invalide"));
        }
        sqlx::query("INSERT INTO document_storage_config(legal_entity_id,root_path,folder_overrides,updated_at) VALUES($1,$2,$3,now()) ON CONFLICT(legal_entity_id) DO UPDATE SET root_path=EXCLUDED.root_path,folder_overrides=EXCLUDED.folder_overrides,updated_at=now()")
            .bind(current_legal_entity_id()).bind(root).bind(folder_overrides).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("set_document_storage_config est exécutée côté serveur"))
}

pub(crate) fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bit_len = (msg.len() as u64) * 8;
    msg.push(0x80);
    while (msg.len() % 64) != 56 { msg.push(0); }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 { w[i] = u32::from_be_bytes([chunk[i*4],chunk[i*4+1],chunk[i*4+2],chunk[i*4+3]]); }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        const K: [u32; 64] = [
            0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
            0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
            0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
            0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
            0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
            0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
            0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
            0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
        ];
        let mut a=h[0]; let mut b=h[1]; let mut c=h[2]; let mut d=h[3]; let mut e=h[4]; let mut f=h[5]; let mut g=h[6]; let mut hh=h[7];
        for i in 0..64 {
            let s1=e.rotate_right(6)^e.rotate_right(11)^e.rotate_right(25);
            let ch=(e&f)^((!e)&g);
            let t1=hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0=a.rotate_right(2)^a.rotate_right(13)^a.rotate_right(22);
            let maj=(a&b)^(a&c)^(b&c);
            let t2=s0.wrapping_add(maj);
            hh=g; g=f; f=e; e=d.wrapping_add(t1); d=c; c=b; b=a; a=t1.wrapping_add(t2);
        }
        h[0]=h[0].wrapping_add(a); h[1]=h[1].wrapping_add(b); h[2]=h[2].wrapping_add(c); h[3]=h[3].wrapping_add(d);
        h[4]=h[4].wrapping_add(e); h[5]=h[5].wrapping_add(f); h[6]=h[6].wrapping_add(g); h[7]=h[7].wrapping_add(hh);
    }
    h.iter().map(|v| format!("{v:08x}")).collect()
}

#[server]
pub async fn import_document_bytes(
    file_name: String,
    mime_type: String,
    category: String,
    title: String,
    origin: String,
    document_date: Option<chrono::NaiveDate>,
    expires_at: Option<chrono::NaiveDate>,
    bytes: Vec<u8>,
) -> Result<DocumentItem, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let _ = crate::security::require_permission(pool, "DATA_WRITE").await.map_err(ServerFnError::new)?;
        if bytes.is_empty() { return Err(ServerFnError::new("Fichier vide")); }
        if bytes.len() > 25 * 1024 * 1024 { return Err(ServerFnError::new("Fichier trop volumineux (25 Mo max)")); }
        let filename = safe_filename(&file_name).map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let hash = sha256_hex(&bytes);
        let duplicate = sqlx::query("SELECT id,storage_key FROM documents WHERE legal_entity_id=$1 AND content_hash=$2 AND archived_at IS NULL ORDER BY created_at LIMIT 1")
            .bind(entity).bind(&hash).fetch_optional(pool).await.map_err(ServerFnError::new)?;
        let (document_id, storage_key, duplicate_status, duplicate_of) = if let Some(r) = duplicate {
            (Uuid::new_v4(), sqlx::Row::get::<String,_>(&r,"storage_key"), "CERTAIN".to_owned(), Some(sqlx::Row::get::<Uuid,_>(&r,"id")))
        } else {
            let (root, overrides) = storage_config(pool, entity).await.map_err(ServerFnError::new)?;
            let folder = folder_path(category_folder(&category), &overrides);
            let key = std::path::PathBuf::from(entity.to_string()).join(folder).join(format!("{}-{}", Uuid::new_v4(), filename));
            let full = root.join(&key);
            if let Some(parent) = full.parent() { tokio::fs::create_dir_all(parent).await.map_err(ServerFnError::new)?; }
            tokio::fs::write(&full, &bytes).await.map_err(ServerFnError::new)?;
            (Uuid::new_v4(), key.to_string_lossy().replace('\\', "/"), "NEW".to_owned(), None)
        };
        let status = if duplicate_of.is_some() { "IMPORTED_DUPLICATE" } else { "IMPORTED" };
        sqlx::query("INSERT INTO documents(id,legal_entity_id,category,title,file_name,storage_key,content_hash,document_date,expires_at,origin,file_size_bytes,mime_type,status,duplicate_status,duplicate_of) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)")
            .bind(document_id).bind(entity).bind(category.trim()).bind(if title.trim().is_empty() { "Document" } else { title.trim() })
            .bind(&filename).bind(&storage_key).bind(&hash).bind(document_date).bind(expires_at).bind(if origin.trim().is_empty() { "IMPORT" } else { origin.trim() })
            .bind(bytes.len() as i64).bind(if mime_type.trim().is_empty() { "application/octet-stream" } else { mime_type.trim() }).bind(status).bind(&duplicate_status).bind(duplicate_of)
            .execute(pool).await.map_err(ServerFnError::new)?;
        if let Some(dup) = duplicate_of {
            sqlx::query("INSERT INTO document_duplicate_candidates(document_id,candidate_document_id,relation,confidence,reason) VALUES($1,$2,'CERTAIN',1.0,'Hash SHA-256 identique') ON CONFLICT DO NOTHING")
                .bind(document_id).bind(dup).execute(pool).await.map_err(ServerFnError::new)?;
        } else {
            let candidates = sqlx::query("SELECT id,file_name,file_size_bytes,title,document_date FROM documents WHERE legal_entity_id=$1 AND id<>$2 AND archived_at IS NULL AND ((file_name=$3 AND file_size_bytes=$4) OR (title=$5 AND document_date IS NOT DISTINCT FROM $6)) ORDER BY created_at DESC LIMIT 10")
                .bind(entity).bind(document_id).bind(&filename).bind(bytes.len() as i64).bind(if title.trim().is_empty() { "Document" } else { title.trim() }).bind(document_date).fetch_all(pool).await.map_err(ServerFnError::new)?;
            if !candidates.is_empty() {
                sqlx::query("UPDATE documents SET duplicate_status=CASE WHEN EXISTS(SELECT 1 FROM document_duplicate_candidates d WHERE d.document_id=$1 AND d.relation='PROBABLE') THEN 'PROBABLE' ELSE 'SIMILAR' END WHERE id=$1")
                    .bind(document_id).execute(pool).await.map_err(ServerFnError::new)?;
                for candidate in candidates {
                    let candidate_id:Uuid=sqlx::Row::get(&candidate,"id");
                    let same_filename:bool=sqlx::Row::get::<String,_>(&candidate,"file_name")==filename;
                    let relation=if same_filename && sqlx::Row::get::<i64,_>(&candidate,"file_size_bytes")==bytes.len() as i64 {"PROBABLE"} else {"SIMILAR"};
                    let confidence=if relation=="PROBABLE"{0.92}else{0.70};
                    let reason=if relation=="PROBABLE"{"Nom et taille identiques"}else{"Titre et date proches"};
                    sqlx::query("INSERT INTO document_duplicate_candidates(document_id,candidate_document_id,relation,confidence,reason) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
                        .bind(document_id).bind(candidate_id).bind(relation).bind(confidence).bind(reason).execute(pool).await.map_err(ServerFnError::new)?;
                }
            }
        }
        sqlx::query("INSERT INTO document_quality_checks(legal_entity_id,document_id,check_code,severity,status,message) VALUES($1,$2,'IMPORT_METADATA','INFO','PASS','Hash, taille, origine et chemin enregistrés')")
            .bind(entity).bind(document_id).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO document_events(legal_entity_id,document_id,event_type,payload) VALUES($1,$2,'IMPORTED',$3)")
            .bind(entity).bind(document_id).bind(serde_json::json!({"origin":origin,"hash":hash,"size":bytes.len(),"duplicate_status":duplicate_status})).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("SELECT id,category,title,file_name,storage_key,document_date,expires_at,origin,file_size_bytes,mime_type,status,ocr_status,classification_confidence,extraction_confidence,duplicate_status FROM documents WHERE id=$1")
            .bind(document_id).fetch_one(pool).await.map(|r| DocumentItem {
                id: sqlx::Row::get(&r,"id"), category: sqlx::Row::get(&r,"category"), title: sqlx::Row::get(&r,"title"), file_name: sqlx::Row::get(&r,"file_name"), storage_key: sqlx::Row::get(&r,"storage_key"), document_date: sqlx::Row::get(&r,"document_date"), expires_at: sqlx::Row::get(&r,"expires_at"), origin: sqlx::Row::get(&r,"origin"), file_size_bytes: sqlx::Row::get(&r,"file_size_bytes"), mime_type: sqlx::Row::get(&r,"mime_type"), status: sqlx::Row::get(&r,"status"), ocr_status: sqlx::Row::get(&r,"ocr_status"), classification_confidence: sqlx::Row::get(&r,"classification_confidence"), extraction_confidence: sqlx::Row::get(&r,"extraction_confidence"), duplicate_status: sqlx::Row::get(&r,"duplicate_status")
            }).map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    { let _=(file_name,mime_type,category,title,origin,document_date,expires_at,bytes); Err(ServerFnError::new("import_document_bytes est exécutée côté serveur")) }
}

#[server]
pub async fn import_document_from_path(
    path: String,
    category: String,
    title: String,
    origin: String,
    document_date: Option<chrono::NaiveDate>,
    expires_at: Option<chrono::NaiveDate>,
) -> Result<DocumentItem, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let p = std::path::PathBuf::from(path.trim());
        let metadata = tokio::fs::metadata(&p).await.map_err(ServerFnError::new)?;
        if !metadata.is_file() { return Err(ServerFnError::new("Le chemin ne désigne pas un fichier")); }
        if metadata.len() > 25 * 1024 * 1024 { return Err(ServerFnError::new("Fichier trop volumineux (25 Mo max)")); }
        let bytes = tokio::fs::read(&p).await.map_err(ServerFnError::new)?;
        let filename = p.file_name().and_then(|v| v.to_str()).unwrap_or("document.bin").to_owned();
        import_document_bytes(filename, "application/octet-stream".into(), category, title, origin, document_date, expires_at, bytes).await
    }
    #[cfg(not(feature = "server"))]
    { let _=(path,category,title,origin,document_date,expires_at); Err(ServerFnError::new("import_document_from_path est exécutée côté serveur")) }
}

#[server]
pub async fn run_document_ocr(id: Uuid) -> Result<DocumentOcrSummary, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let _ = crate::security::require_permission(pool, "DATA_WRITE").await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let row = sqlx::query("SELECT id,storage_key,file_name FROM documents WHERE id=$1 AND legal_entity_id=$2 AND archived_at IS NULL")
            .bind(id).bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?;
        let row = row.ok_or_else(|| ServerFnError::new("Document introuvable"))?;
        let (root,_) = storage_config(pool,entity).await.map_err(ServerFnError::new)?;
        let key: String = sqlx::Row::get(&row,"storage_key");
        let full = root.join(safe_storage_key(&key).map_err(ServerFnError::new)?);
        let command = std::env::var("SCI_OCR_COMMAND").ok().filter(|v| !v.trim().is_empty());
        let Some(command) = command else {
            sqlx::query("INSERT INTO document_ocr_runs(legal_entity_id,document_id,engine,status,error_message) VALUES($1,$2,'CONFIGURED_LOCAL','UNAVAILABLE','SCI_OCR_COMMAND absent')")
                .bind(entity).bind(id).execute(pool).await.map_err(ServerFnError::new)?;
            sqlx::query("UPDATE documents SET ocr_status='UNAVAILABLE',status='IMPORTED' WHERE id=$1 AND legal_entity_id=$2")
                .bind(id).bind(entity).execute(pool).await.map_err(ServerFnError::new)?;
            return Ok(DocumentOcrSummary{document_id:id,status:"IMPORTED".into(),ocr_status:"UNAVAILABLE".into(),classification:"INCONNU".into(),classification_confidence:0.0,extraction_confidence:0.0,requires_validation:false});
        };
        let output = tokio::process::Command::new(command.trim())
            .arg(&full).arg("stdout").output().await.map_err(ServerFnError::new)?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            sqlx::query("INSERT INTO document_ocr_runs(legal_entity_id,document_id,engine,status,error_message) VALUES($1,$2,'CONFIGURED_LOCAL','FAILED',$3)")
                .bind(entity).bind(id).bind(&err).execute(pool).await.map_err(ServerFnError::new)?;
            sqlx::query("UPDATE documents SET ocr_status='FAILED' WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(entity).execute(pool).await.map_err(ServerFnError::new)?;
            return Err(ServerFnError::new(format!("OCR échoué: {err}")));
        }
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        let input = classifier::ClassificationInput{document_id:id,filename:sqlx::Row::get(&row,"file_name"),text:text.clone()};
        let classified = classifier::classify(input.clone());
        let extracted = extractor::extract(ExtractionInput{document_id:id,document_type:classified.document_type.clone(),text:text.clone()});
        let confidence = if extracted.fields.is_empty(){0.0}else{extracted.fields.iter().map(|f|f.confidence).sum::<f32>() / extracted.fields.len() as f32};
        let data = serde_json::json!({"fields":extracted.fields.iter().map(|f|serde_json::json!({"name":f.name,"value":f.value,"confidence":f.confidence})).collect::<Vec<_>>()});
        sqlx::query("INSERT INTO document_ocr_runs(legal_entity_id,document_id,engine,status,mean_confidence,extracted_text) VALUES($1,$2,'CONFIGURED_LOCAL','COMPLETED',$3,$4)")
            .bind(entity).bind(id).bind(0.0f32).bind(&text).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO document_extractions(legal_entity_id,document_id,schema_version,extracted_data,confidence,validation_required) VALUES($1,$2,1,$3,$4,true)")
            .bind(entity).bind(id).bind(&data).bind(confidence).execute(pool).await.map_err(ServerFnError::new)?;
        let classification_status=if classified.confidence>=0.75{"PASS"}else{"REVIEW"};
        let extraction_status=if confidence>=0.60{"PASS"}else{"REVIEW"};
        let text_status=if text.trim().is_empty(){"BLOCK"}else{"PASS"};
        sqlx::query("INSERT INTO document_quality_checks(legal_entity_id,document_id,check_code,severity,status,message) VALUES($1,$2,'CLASSIFICATION','INFO',$3,$4),($1,$2,'EXTRACTION_CONFIDENCE',CASE WHEN $5='PASS' THEN 'INFO' ELSE 'WARNING' END,$5,$6),($1,$2,'OCR_TEXT','ERROR',$7,$8)")
            .bind(entity).bind(id).bind(classification_status).bind(format!("Confiance classification {:.0}%",classified.confidence*100.0)).bind(extraction_status).bind(format!("Confiance extraction {:.0}%",confidence*100.0)).bind(text_status).bind(if text_status=="PASS"{"Texte OCR disponible"}else{"Aucun texte OCR"}).execute(pool).await.map_err(ServerFnError::new)?;
        let label=document_type_label(&classified.document_type);
        sqlx::query("UPDATE documents SET category=CASE WHEN category='' OR category='AUTO' THEN $3 ELSE category END,status='PENDING_VALIDATION',ocr_status='COMPLETED',classification_confidence=$4,classification_reason=$5,extraction_confidence=$6 WHERE id=$1 AND legal_entity_id=$2")
            .bind(id).bind(entity).bind(label).bind(classified.confidence).bind(classified.reasons.join("; ")).bind(confidence).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO document_events(legal_entity_id,document_id,event_type,payload) VALUES($1,$2,'OCR_COMPLETED',$3)")
            .bind(entity).bind(id).bind(serde_json::json!({"classification":label,"classification_confidence":classified.confidence,"extraction_confidence":confidence,"validation_required":true})).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(DocumentOcrSummary{document_id:id,status:"PENDING_VALIDATION".into(),ocr_status:"COMPLETED".into(),classification:label.into(),classification_confidence:classified.confidence,extraction_confidence:confidence,requires_validation:true})
    }
    #[cfg(not(feature = "server"))]
    { let _=id; Err(ServerFnError::new("run_document_ocr est exécutée côté serveur")) }
}

#[server]
pub async fn validate_document_extraction(id: Uuid, actor: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let principal = crate::security::require_permission(pool, "DATA_WRITE").await.map_err(ServerFnError::new)?;
        let entity=current_legal_entity_id();
        let extraction=sqlx::query("SELECT id,extracted_data FROM document_extractions WHERE document_id=$1 AND legal_entity_id=$2 ORDER BY created_at DESC LIMIT 1")
            .bind(id).bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?;
        let extraction = extraction.ok_or_else(|| ServerFnError::new("Aucune extraction à valider"))?;
        let blocked:i64=sqlx::query_scalar("SELECT COUNT(*)::bigint FROM document_quality_checks WHERE legal_entity_id=$1 AND document_id=$2 AND status='BLOCK'")
            .bind(entity).bind(id).fetch_one(pool).await.map_err(ServerFnError::new)?;
        if blocked>0{return Err(ServerFnError::new("Validation bloquée par un contrôle documentaire"));}
        let data: Value=sqlx::Row::get(&extraction,"extracted_data");
        let validator_role = principal.role.as_deref().unwrap_or("USER");
        sqlx::query("UPDATE document_extractions SET validation_required=false,validated_at=now(),validated_by=$3 WHERE id=$1 AND legal_entity_id=$2")
            .bind(sqlx::Row::get::<Uuid,_>(&extraction,"id")).bind(entity).bind(validator_role).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("UPDATE documents SET extracted_data=$3,status='VALIDATED',validated_at=now(),validated_by=$4 WHERE id=$1 AND legal_entity_id=$2")
            .bind(id).bind(entity).bind(data).bind(validator_role).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO document_events(legal_entity_id,document_id,event_type,actor,payload) VALUES($1,$2,'OCR_VALIDATED',$3,$4)")
            .bind(entity).bind(id).bind(validator_role).bind(serde_json::json!({"business_data_committed":true})).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    { let _=(id,actor); Err(ServerFnError::new("validate_document_extraction est exécutée côté serveur")) }
}

#[server]
pub async fn archive_document_record(id: Uuid, reason: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let _ = crate::security::require_permission(pool, "DATA_WRITE").await.map_err(ServerFnError::new)?;
        let entity=current_legal_entity_id();
        let changed=sqlx::query("UPDATE documents SET status='ARCHIVED',archived_at=now(),archive_reason=$3 WHERE id=$1 AND legal_entity_id=$2 AND archived_at IS NULL")
            .bind(id).bind(entity).bind(reason.trim()).execute(pool).await.map_err(ServerFnError::new)?;
        if changed.rows_affected()==0 { return Err(ServerFnError::new("Document introuvable ou déjà archivé")); }
        sqlx::query("INSERT INTO document_events(legal_entity_id,document_id,event_type,payload) VALUES($1,$2,'ARCHIVED',$3)")
            .bind(entity).bind(id).bind(serde_json::json!({"reason":reason})).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    { let _=(id,reason); Err(ServerFnError::new("archive_document_record est exécutée côté serveur")) }
}

#[server]
pub async fn search_documents(query: String) -> Result<Vec<DocumentItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return crate::server::list_documents().await;
        }
        let docs = crate::server::list_documents().await?;
        Ok(docs.into_iter().filter(|d| {
            let haystack = format!("{} {} {} {}", d.title, d.file_name, d.category, d.origin).to_lowercase();
            haystack.contains(&q)
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    { let _=query; Err(ServerFnError::new("search_documents est exécutée côté serveur")) }
}

#[component]
pub fn DocumentsWorkflowPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let docs = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { crate::server::list_documents().await.unwrap_or_default() }
    });
    let folders = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_document_folders().await.unwrap_or_default() }
    });
    let config = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { get_document_storage_config().await.ok() }
    });

    let mut root = use_signal(|| "documents".to_owned());
    let mut path = use_signal(String::new);
    let mut title = use_signal(String::new);
    let mut category = use_signal(|| "ADMINISTRATIF".to_owned());
    let mut origin = use_signal(|| "MANUAL".to_owned());
    let mut msg = use_signal(String::new);
    let config_value = config
        .read()
        .as_ref()
        .and_then(|value| value.as_ref())
        .cloned();

    rsx! {
        section {
            class: "panel",
            h2 { "Documents externes" }
            p {
                class: "small",
                "Les fichiers physiques restent hors PostgreSQL. La base conserve métadonnées, hash, statut et historique."
            }
            if let Some(config) = config_value.as_ref() {
                input {
                    value: config.root_path.clone(),
                    oninput: move |event: FormEvent| root.set(event.value()),
                    placeholder: "Racine documentaire"
                }
            }
            button {
                class: "secondary",
                onclick: move |_| {
                    let root_value = root();
                    async move {
                        match set_document_storage_config(root_value, serde_json::json!({})).await {
                            Ok(_) => {
                                msg.set("Configuration enregistrée".into());
                                bump += 1;
                            }
                            Err(error) => msg.set(error.to_string()),
                        }
                    }
                },
                "Enregistrer la racine"
            }
            span {
                class: "save-ok",
                "{msg()}"
            }
        }

        section {
            class: "panel",
            h2 { "Importer" }
            div {
                class: "form-grid",
                FormField {
                    label: "Chemin local du fichier",
                    value: path(),
                    oninput: move |event: FormEvent| path.set(event.value())
                }
                FormField {
                    label: "Titre",
                    value: title(),
                    oninput: move |event: FormEvent| title.set(event.value())
                }
                FormField {
                    label: "Catégorie",
                    value: category(),
                    oninput: move |event: FormEvent| category.set(event.value())
                }
                FormField {
                    label: "Origine",
                    value: origin(),
                    oninput: move |event: FormEvent| origin.set(event.value())
                }
            }
            button {
                class: "primary",
                onclick: move |_| {
                    let file_path = path();
                    let document_title = title();
                    let document_category = category();
                    let document_origin = origin();
                    async move {
                        match import_document_from_path(
                            file_path,
                            document_category,
                            document_title,
                            document_origin,
                            None,
                            None,
                        )
                        .await
                        {
                            Ok(_) => {
                                msg.set("Document importé".into());
                                bump += 1;
                            }
                            Err(error) => msg.set(error.to_string()),
                        }
                    }
                },
                "Importer"
            }
        }

        section {
            class: "panel",
            h2 { "Arborescence" }
            div {
                for folder in folders.read().as_deref().unwrap_or(&[]).iter() {
                    div {
                        class: "data-row",
                        div {
                            div {
                                class: "data-title",
                                {folder.label.clone()}
                            }
                            div {
                                class: "small",
                                {format!("{} → {}", folder.code, folder.relative_path)}
                            }
                        }
                    }
                }
            }
        }

        section {
            class: "panel",
            h2 { "Pipeline OCR" }
            div {
                for document in docs.read().as_deref().unwrap_or(&[]).iter() {
                    div {
                        class: "data-row",
                        div {
                            div {
                                class: "data-title",
                                {document.title.clone()}
                            }
                            div {
                                class: "small",
                                {format!(
                                    "{} • {} • {}",
                                    document.category,
                                    document.status,
                                    document.duplicate_status
                                )}
                            }
                            div {
                                class: "small",
                                {format!(
                                    "{} • OCR {} • confiance extraction {:.0}%",
                                    document.file_name,
                                    document.ocr_status,
                                    document.extraction_confidence.to_f64().unwrap_or(0.0) * 100.0
                                )}
                            }
                        }
                        div {
                            class: "row-actions",
                            button {
                                class: "secondary",
                                onclick: {
                                    let id = document.id;
                                    move |_| async move {
                                        let _ = run_document_ocr(id).await;
                                        bump += 1;
                                    }
                                },
                                "OCR"
                            }
                            if document.status == "PENDING_VALIDATION" {
                                button {
                                    class: "primary",
                                    onclick: {
                                        let id = document.id;
                                        move |_| async move {
                                            let _ = validate_document_extraction(id, "MANAGER".into()).await;
                                            bump += 1;
                                        }
                                    },
                                    "Valider"
                                }
                            }
                            if document.status != "ARCHIVED" {
                                button {
                                    class: "secondary",
                                    onclick: {
                                        let id = document.id;
                                        move |_| async move {
                                            let _ = archive_document_record(id, "Archivage documentaire".into()).await;
                                            bump += 1;
                                        }
                                    },
                                    "Archiver"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::sha256_hex;
    #[test]
    fn sha256_known_vector() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
