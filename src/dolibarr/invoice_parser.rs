//! Extraction et parsing de factures fournisseur depuis un PDF.
//! Pur Rust : utilise pdf-extract + regex. Aucune dépendance externe.

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VatLine {
    pub rate: String,
    pub base_ht: String,
    pub amount_tva: String,
    pub amount_ttc: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedInvoice {
    pub supplier_name: String,
    pub invoice_number: String,
    pub invoice_date: String,      // AAAA-MM-JJ
    pub due_date: String,          // AAAA-MM-JJ
    pub total_ht: String,
    pub total_tva: String,
    pub total_ttc: String,
    pub tva_rate: String,
    pub vat_lines: Vec<VatLine>,
    pub iban: String,
    pub raw_text_preview: String,
    pub confidence: i32,
}

/// Extrait le texte d'un PDF en bytes.
pub fn extract_text_from_pdf(bytes: &[u8]) -> Result<String, String> {
    pdf_extract::extract_text_from_mem(bytes)
        .map_err(|e| format!("Erreur lecture PDF : {}", e))
}

/// Parse le texte d'une facture pour en extraire les champs cles.
pub fn parse_invoice_text(text: &str) -> ParsedInvoice {
    let mut result = ParsedInvoice::default();

    let cleaned: String = text
        .replace('\u{a0}', " ")
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n");

    // ============ TTC ============
    let ttc_patterns = [
        r"(?i)montant\s+[aà]\s+r[eé]gler[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
        r"(?i)net\s+[aà]\s+payer[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
        r"(?i)total\s+ttc[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
        r"(?i)total\s+g[eé]n[eé]ral[^\d]{0,40}([\d\s]+[.,]\d{2})\s*€?",
        r"(?i)montant\s+de\s+l['']?\s*appel\s+de\s+fonds[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
        r"(?i)total\s+du\s+lot[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
        r"(?i)(?:reste\s+[aà]\s+payer|[aà]\s+payer)[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
    ];
    let mut best_ttc: f64 = 0.0;
    for pat in ttc_patterns.iter() {
        if let Ok(re) = Regex::new(pat) {
            for c in re.captures_iter(&cleaned) {
                let v = normalize_amount(&c[1]);
                if let Ok(n) = v.parse::<f64>() {
                    if n > best_ttc && n < 10_000_000.0 {
                        best_ttc = n;
                    }
                }
            }
        }
    }
    if best_ttc > 0.0 {
        result.total_ttc = format!("{:.2}", best_ttc);
    }

    // ============ HT ============
    let ht_patterns = [
        r"(?i)total\s+ht[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
        r"(?i)total\s+g[eé]n[eé]ral\s*:?[^\d]{0,20}([\d\s]+[.,]\d{2})\s*€?",
        r"(?i)base\s+ht[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
        r"(?i)montant\s+ht[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
    ];
    let mut best_ht: f64 = 0.0;
    for pat in ht_patterns.iter() {
        if let Ok(re) = Regex::new(pat) {
            for c in re.captures_iter(&cleaned) {
                let v = normalize_amount(&c[1]);
                if let Ok(n) = v.parse::<f64>() {
                    if n > best_ht && n < 10_000_000.0 {
                        best_ht = n;
                    }
                }
            }
        }
    }
    if best_ht > 0.0 {
        result.total_ht = format!("{:.2}", best_ht);
    }

    // ============ TVA global ============
    let tva_patterns = [
        r"(?i)total\s+tva\s*:?[^\d]{0,20}([\d\s]+[.,]\d{2})\s*€?",
        r"(?i)montant\s+tva[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
    ];
    let mut best_tva: f64 = 0.0;
    for pat in tva_patterns.iter() {
        if let Ok(re) = Regex::new(pat) {
            for c in re.captures_iter(&cleaned) {
                let v = normalize_amount(&c[1]);
                if let Ok(n) = v.parse::<f64>() {
                    if n > best_tva && n < 10_000_000.0 {
                        best_tva = n;
                    }
                }
            }
        }
    }
    if best_tva > 0.0 {
        result.total_tva = format!("{:.2}", best_tva);
    }

    // ============ Multi-taux TVA ============
    let vat_line_re = Regex::new(
        r"(?i)(?:montant\s+ht[^\d]{0,20}([\d\s]+[.,]\d{2})\s*€?[^\d]{0,20})?tva\s*\(\s*(\d{1,2}[.,]?\d{0,2})\s*%\s*\)[^\d]{0,10}([\d\s]+[.,]\d{2})\s*€?"
    ).unwrap();

    for c in vat_line_re.captures_iter(&cleaned) {
        let base = c.get(1).map(|m| normalize_amount(m.as_str())).unwrap_or_default();
        let rate = normalize_amount(&c[2]);
        let tva = normalize_amount(&c[3]);
        let ttc = if !base.is_empty() {
            let b: f64 = base.parse().unwrap_or(0.0);
            let t: f64 = tva.parse().unwrap_or(0.0);
            format!("{:.2}", b + t)
        } else {
            String::new()
        };
        if !rate.is_empty() && !tva.is_empty() {
            result.vat_lines.push(VatLine {
                rate,
                base_ht: base,
                amount_tva: tva,
                amount_ttc: ttc,
            });
        }
    }

    if !result.vat_lines.is_empty() {
        let mut sum_ht = 0.0;
        let mut sum_tva = 0.0;
        let mut sum_ttc = 0.0;
        for vl in result.vat_lines.iter() {
            sum_ht += vl.base_ht.parse::<f64>().unwrap_or(0.0);
            sum_tva += vl.amount_tva.parse::<f64>().unwrap_or(0.0);
            sum_ttc += vl.amount_ttc.parse::<f64>().unwrap_or(0.0);
        }
        if sum_ht > 0.0 {
            result.total_ht = format!("{:.2}", sum_ht);
        }
        if sum_tva > 0.0 {
            result.total_tva = format!("{:.2}", sum_tva);
        }
        if sum_ttc > 0.0 && (best_ttc - sum_ttc).abs() < 1.0 {
            // accord parfait, on garde best_ttc
        } else if sum_ttc > 0.0 {
            if best_ttc > sum_ttc {
                result.total_ttc = format!("{:.2}", best_ttc);
            } else {
                result.total_ttc = format!("{:.2}", sum_ttc);
            }
        }
    }

    if !result.vat_lines.is_empty() {
        let main = result.vat_lines.iter().max_by(|a, b| {
            let ta: f64 = a.amount_tva.parse().unwrap_or(0.0);
            let tb: f64 = b.amount_tva.parse().unwrap_or(0.0);
            ta.partial_cmp(&tb).unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Some(m) = main {
            result.tva_rate = m.rate.clone();
        }
    } else {
        let tva_rate_re = Regex::new(r"(?i)tva[^\d]{0,15}(\d{1,2})[.,]?(\d{0,2})\s*%").unwrap();
        if let Some(c) = tva_rate_re.captures(&cleaned) {
            let integer_part = &c[1];
            let decimal_part = if c[2].is_empty() { "00" } else { &c[2] };
            result.tva_rate = format!("{}.{}", integer_part, decimal_part);
        }
    }

    // ============ Fournisseur par URL ============
    let url_with_prefix_re = Regex::new(
        r"(?i)(?:https?://|www\.)([a-z0-9][a-z0-9\-]{1,30})\.[a-z]{2,4}"
    ).unwrap();
    let mut supplier_from_url = String::new();
    if let Some(c) = url_with_prefix_re.captures(&cleaned) {
        let name = c[1].to_string();
        if name.len() >= 2 {
            supplier_from_url = name.to_uppercase().replace('-', " ");
        }
    }

    if supplier_from_url.is_empty() {
        let url_bare_re = Regex::new(
            r"(?i)\b([a-z0-9][a-z0-9\-]{2,30})\.(?:fr|com|net|eu|be|ch|de|es|it)\b"
        ).unwrap();
        for c in url_bare_re.captures_iter(&cleaned) {
            let m = c.get(0).unwrap();
            let start = m.start();
            if start > 0 {
                let before = cleaned.as_bytes()[start - 1];
                if before == b'@' {
                    continue;
                }
            }
            let name = c[1].to_string();
            let lower = name.to_lowercase();
            if lower == "gmail" || lower == "hotmail" || lower == "yahoo"
                || lower == "orange" || lower == "wanadoo" || lower == "free"
                || lower == "laposte" || lower == "outlook"
            {
                continue;
            }
            if name.len() >= 2 {
                supplier_from_url = name.to_uppercase().replace('-', " ");
                break;
            }
        }
    }

    if !supplier_from_url.is_empty() {
        result.supplier_name = supplier_from_url;
    }

    if result.supplier_name.is_empty() {
        let lines: Vec<&str> = cleaned.lines().take(25).collect();
        let mut fallback_candidate = String::new();
        for line in lines.iter() {
            let t = line.trim();
            if t.len() < 3 || t.len() > 70 { continue; }
            if t.chars().all(|c| c.is_ascii_digit() || c == ' ' || c == '/' || c == '-') { continue; }
            let tl = t.to_lowercase();
            if tl.contains("facture") || tl.contains("invoice") || tl.contains("date")
                || tl.contains("client") || tl.contains("adresse") || tl.contains("tva")
                || tl.contains("accueil") || tl.contains("telephonique") || tl.contains("telephone")
                || tl.contains("reception") || tl.contains("rendez") || tl.contains("www.")
                || tl.contains("rcs ") || tl.contains("sas ") || tl.contains("sarl ")
                || tl.contains("carte cpi") || tl.contains("capital")
                || tl.contains("france metropolitaine") || tl.contains("votre facture")
                || tl.contains("detaillee") || tl.contains("montant")
            {
                continue;
            }
            let letters: Vec<char> = t.chars().filter(|c| c.is_alphabetic()).collect();
            if letters.is_empty() { continue; }
            let upper_count = letters.iter().filter(|c| c.is_uppercase()).count();
            let ratio = upper_count as f64 / letters.len() as f64;

            if ratio >= 0.7 {
                result.supplier_name = t.to_string();
                break;
            }
            if fallback_candidate.is_empty() {
                if let Some(first_char) = t.chars().next() {
                    if first_char.is_uppercase() {
                        fallback_candidate = t.to_string();
                    }
                }
            }
        }
        if result.supplier_name.is_empty() && !fallback_candidate.is_empty() {
            result.supplier_name = fallback_candidate;
        }
    }

    // ============ Numero de facture ============
    let num_re = Regex::new(r"(?i)(?:facture|invoice|n[°o]|num[eé]ro|r[eé]f\s*client)\s*[:\-]?\s*([A-Z0-9][A-Z0-9\-/]{3,30})").unwrap();
    if let Some(c) = num_re.captures(&cleaned) {
        let candidate = c[1].trim().to_string();
        if candidate.len() >= 4 && candidate.chars().any(|ch| ch.is_ascii_digit()) {
            result.invoice_number = candidate;
        }
    }

    // ============ Date facture ET date échéance ============
    // 1. "avant le XX mois YYYY" → date d'échéance
    let avant_le_re = Regex::new(
        r"(?i)avant\s+le\s+(\d{1,2})\s+(janvier|f[eé]vrier|mars|avril|mai|juin|juillet|ao[uû]t|septembre|octobre|novembre|d[eé]cembre)\s+(\d{4})"
    ).unwrap();
    if let Some(c) = avant_le_re.captures(&cleaned) {
        if let Some(dt) = parse_french_date(&c[1], &c[2], &c[3]) {
            result.due_date = dt;
        }
    }

    // 2. "FACTURE du XX mois YYYY" → date d'émission
    let facture_du_re = Regex::new(
        r"(?i)facture\s+du\s+(\d{1,2})\s+(janvier|f[eé]vrier|mars|avril|mai|juin|juillet|ao[uû]t|septembre|octobre|novembre|d[eé]cembre)\s+(\d{4})"
    ).unwrap();
    if let Some(c) = facture_du_re.captures(&cleaned) {
        if let Some(dt) = parse_french_date(&c[1], &c[2], &c[3]) {
            result.invoice_date = dt;
        }
    }

    // 3. Sinon date en toutes lettres "16 mai 2022" / "le 16 mai 2022"
    if result.invoice_date.is_empty() {
        let month_re = Regex::new(
            r"(?i)(?:le\s+)?(\d{1,2})\s+(janvier|f[eé]vrier|mars|avril|mai|juin|juillet|ao[uû]t|septembre|octobre|novembre|d[eé]cembre)\s+(\d{4})"
        ).unwrap();
        if let Some(c) = month_re.captures(&cleaned) {
            if let Some(dt) = parse_french_date(&c[1], &c[2], &c[3]) {
                result.invoice_date = dt;
            }
        }
    }

    // 4. Sinon date numerique classique (JJ/MM/AAAA ou JJ-MM-AAAA)
    if result.invoice_date.is_empty() {
        let date_re = Regex::new(r"(\d{1,2})[/\-.](\d{1,2})[/\-.](\d{4})").unwrap();
        for c in date_re.captures_iter(&cleaned) {
            let d = c[1].parse::<u32>().unwrap_or(1);
            let m = c[2].parse::<u32>().unwrap_or(1);
            let y = c[3].parse::<i32>().unwrap_or(2024);
            if m >= 1 && m <= 12 && d >= 1 && d <= 31 {
                result.invoice_date = format!("{:04}-{:02}-{:02}", y, m, d);
                break;
            }
        }
    }

    // 5. Fallback : date ISO AAAA-MM-JJ
    if result.invoice_date.is_empty() {
        let date_iso = Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap();
        if let Some(c) = date_iso.captures(&cleaned) {
            let y = c[1].parse::<i32>().unwrap_or(2024);
            let m = c[2].parse::<u32>().unwrap_or(1);
            let d = c[3].parse::<u32>().unwrap_or(1);
            if m >= 1 && m <= 12 && d >= 1 && d <= 31 {
                result.invoice_date = format!("{:04}-{:02}-{:02}", y, m, d);
            }
        }
    }

    // ============ IBAN ============
    let iban_re = Regex::new(r"\b([A-Z]{2}\d{2}[\sA-Z0-9]{10,30})\b").unwrap();
    if let Some(c) = iban_re.captures(&cleaned) {
        let candidate = c[1].replace(' ', "");
        if candidate.len() >= 15 {
            result.iban = candidate;
        }
    }

    // ============ Score ============
    let mut score = 0;
    if !result.supplier_name.is_empty() { score += 20; }
    if !result.total_ttc.is_empty() { score += 30; }
    if !result.total_tva.is_empty() { score += 15; }
    if !result.invoice_date.is_empty() { score += 15; }
    if !result.invoice_number.is_empty() { score += 10; }
    if !result.total_ht.is_empty() { score += 5; }
    if !result.vat_lines.is_empty() { score += 5; }
    result.confidence = score.min(100);

    // Fallback TTC -> HT/TVA
    if result.total_ht.is_empty() && !result.total_ttc.is_empty() && !result.tva_rate.is_empty() {
        if let (Ok(ttc), Ok(rate)) = (result.total_ttc.parse::<f64>(), result.tva_rate.parse::<f64>()) {
            let ht = ttc / (1.0 + rate / 100.0);
            let tva = ttc - ht;
            result.total_ht = format!("{:.2}", ht);
            result.total_tva = format!("{:.2}", tva);
        }
    }

    result.raw_text_preview = cleaned.chars().take(2000).collect();
    result
}

/// Parse une date en lettres ("16", "mai", "2022") -> "2022-05-16"
fn parse_french_date(day: &str, month_str: &str, year: &str) -> Option<String> {
    let d = day.parse::<u32>().ok()?;
    let y = year.parse::<i32>().ok()?;
    let month_key = month_str
        .to_lowercase()
        .replace('é', "e").replace('è', "e").replace('ê', "e")
        .replace('û', "u").replace('ù', "u").replace('ô', "o")
        .replace('à', "a").replace('â', "a").replace('î', "i")
        .replace('ï', "i").replace('ç', "c")
        .replace('ã', "a").replace('¢', "").replace('©', "e");
    let m = match month_key.as_str() {
        "janvier"   => 1,
        "fevrier"   => 2,
        "mars"      => 3,
        "avril"     => 4,
        "mai"       => 5,
        "juin"      => 6,
        "juillet"   => 7,
        "aout"      => 8,
        "septembre" => 9,
        "octobre"   => 10,
        "novembre"  => 11,
        "decembre"  => 12,
        _ => return None,
    };
    if d >= 1 && d <= 31 && m >= 1 && m <= 12 {
        Some(format!("{:04}-{:02}-{:02}", y, m, d))
    } else {
        None
    }
}

/// Normalise un montant : "1 234,56" -> "1234.56"
fn normalize_amount(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    if cleaned.is_empty() {
        return String::new();
    }
    let last_dot = cleaned.rfind('.');
    let last_comma = cleaned.rfind(',');
    let result = match (last_dot, last_comma) {
        (Some(d), Some(c)) => {
            if c > d {
                cleaned.replace(".", "").replace(',', ".")
            } else {
                cleaned.replace(',', "")
            }
        }
        (None, Some(_)) => cleaned.replace(',', "."),
        _ => cleaned,
    };
    result
}