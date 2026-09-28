//! Extraction et parsing de factures fournisseur depuis un PDF.
//! Pur Rust : utilise pdf-extract + regex. Aucune dépendance externe.

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedInvoice {
    pub supplier_name: String,
    pub invoice_number: String,
    pub invoice_date: String,      // AAAA-MM-JJ
    pub due_date: String,          // AAAA-MM-JJ
    pub total_ht: String,          // "123.45"
    pub total_tva: String,         // "24.69"
    pub total_ttc: String,         // "148.14"
    pub tva_rate: String,          // "20.00"
    pub iban: String,
    pub raw_text_preview: String,  // 500 premiers caracteres pour debug
    pub confidence: i32,           // 0-100
}

/// Extrait le texte d'un PDF en bytes.
#[cfg(feature = "server")]
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

    let lower = cleaned.to_lowercase();

    // --- TTC : plusieurs familles de patterns (facture classique + appel de fonds)
    let ttc_patterns = [
        // Facture classique
        r"(?i)(?:total\s+ttc|net\s+[aà]\s+payer|montant\s+total|total\s+g[eé]n[eé]ral)[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
        // Appel de fonds (syndic, gestionnaire)
        r"(?i)montant\s+de\s+l['']?\s*appel\s+de\s+fonds[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
        // Totaux par lot (prend le plus grand des totaux de lot)
        r"(?i)total\s+du\s+lot[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
        // Fallback : "à payer" / "reste à payer"
        r"(?i)(?:reste\s+[aà]\s+payer|[aà]\s+payer)[^\d]{0,30}([\d\s]+[.,]\d{2})\s*€?",
        // Dernier recours : "montant X €" (un peu générique)
        r"(?i)\bmontant\b[^\d]{0,15}([\d\s]+[.,]\d{2})\s*€",
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
        if best_ttc > 0.0 {
            // Priorité : on prend le premier pattern qui matche, puis on continue
            // pour voir si un pattern plus fiable donne plus.
            if pat.contains("total ttc") || pat.contains("appel de fonds") {
                break;
            }
        }
    }
    if best_ttc > 0.0 {
        result.total_ttc = format!("{:.2}", best_ttc);
    }

    // --- HT (facture classique uniquement)
    let ht_re = Regex::new(r"(?i)(?:total\s+ht|base\s+ht|montant\s+ht|total\s+hors\s+taxes)[^\d]{0,20}([\d\s]+[.,]\d{2})\s*€?").unwrap();
    if let Some(c) = ht_re.captures(&cleaned) {
        result.total_ht = normalize_amount(&c[1]);
    }

    // --- TVA (montant)
    let tva_amount_re = Regex::new(r"(?i)(?:montant\s+tva|tva\s+[aà]\s+payer)[^\d]{0,20}([\d\s]+[.,]\d{2})\s*€?").unwrap();
    if let Some(c) = tva_amount_re.captures(&cleaned) {
        result.total_tva = normalize_amount(&c[1]);
    }

    // --- Taux TVA
    let tva_rate_re = Regex::new(r"(?i)(?:tva|taux)[^\d]{0,15}(\d{1,2})[.,]?(\d{0,2})\s*%").unwrap();
    if let Some(c) = tva_rate_re.captures(&cleaned) {
        let integer_part = &c[1];
        let decimal_part = if c[2].is_empty() { "00" } else { &c[2] };
        result.tva_rate = format!("{}.{}", integer_part, decimal_part);
    }

    // --- Numero de facture
    let num_re = Regex::new(r"(?i)(?:facture|invoice|n[°o]|num[eé]ro|r[eé]f)\s*[:\-]?\s*([A-Z0-9][A-Z0-9\-/]{3,30})").unwrap();
    if let Some(c) = num_re.captures(&cleaned) {
        let candidate = c[1].trim().to_string();
        if candidate.len() >= 4 && candidate.chars().any(|ch| ch.is_ascii_digit()) {
            result.invoice_number = candidate;
        }
    }

    // --- Date (AAAA-MM-JJ final)
    let date_re = Regex::new(r"(\d{2})[/\-.](\d{2})[/\-.](\d{2,4})").unwrap();
    let mut dates: Vec<String> = Vec::new();
    for c in date_re.captures_iter(&cleaned) {
        let d = &c[1];
        let m = &c[2];
        let y_raw = &c[3];
        let y = if y_raw.len() == 2 {
            format!("20{}", y_raw)
        } else {
            y_raw.to_string()
        };
        dates.push(format!("{}-{}-{}", y, m, d));
    }
    if let Some(first) = dates.first() {
        result.invoice_date = first.clone();
    }

    // --- IBAN
    let iban_re = Regex::new(r"\b([A-Z]{2}\d{2}[\sA-Z0-9]{10,30})\b").unwrap();
    if let Some(c) = iban_re.captures(&cleaned) {
        let candidate = c[1].replace(' ', "");
        if candidate.len() >= 15 {
            result.iban = candidate;
        }
    }

    // --- Fournisseur : on cherche dans les 20 premieres lignes
    // On privilegie les lignes en majuscules (nom de societe)
    let lines: Vec<&str> = cleaned.lines().take(20).collect();
    let mut fallback_candidate = String::new();
    for line in lines.iter() {
        let t = line.trim();
        if t.len() < 3 || t.len() > 70 { continue; }
        if t.chars().all(|c| c.is_ascii_digit() || c == ' ' || c == '/' || c == '-') { continue; }
        let tl = t.to_lowercase();
        // Exclusions : mots-clés administratifs ou titres
        if tl.contains("facture") || tl.contains("invoice") || tl.contains("date")
            || tl.contains("client") || tl.contains("adresse") || tl.contains("tva")
            || tl.contains("accueil") || tl.contains("telephonique") || tl.contains("telephone")
            || tl.contains("reception") || tl.contains("rendez") || tl.contains("www.")
            || tl.contains("rcs ") || tl.contains("sas ") || tl.contains("sarl ")
            || tl.contains("carte cpi") || tl.contains("capital")
            || tl.contains("france metropolitaine")
        {
            continue;
        }
        // Heuristique : ligne en majuscules (>= 70% de majuscules alphabétiques)
        let letters: Vec<char> = t.chars().filter(|c| c.is_alphabetic()).collect();
        if letters.is_empty() { continue; }
        let upper_count = letters.iter().filter(|c| c.is_uppercase()).count();
        let ratio = upper_count as f64 / letters.len() as f64;

        if ratio >= 0.7 && result.supplier_name.is_empty() {
            result.supplier_name = t.to_string();
            break;
        }
        // Sinon on garde un candidat en secours (premiere ligne avec majuscule initiale)
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

    // --- Score de confiance
    let mut score = 0;
    if !result.supplier_name.is_empty() { score += 25; }
    if !result.total_ttc.is_empty() { score += 30; }
    if !result.total_tva.is_empty() { score += 15; }
    if !result.invoice_date.is_empty() { score += 15; }
    if !result.invoice_number.is_empty() { score += 10; }
    if !result.total_ht.is_empty() { score += 5; }
    result.confidence = score;

    // Fallback TTC -> HT/TVA si on a un taux
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

/// Normalise un montant : "1 234,56" -> "1234.56"
fn normalize_amount(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    if cleaned.is_empty() {
        return String::new();
    }
    // Determine le separateur decimal (le dernier des deux)
    let last_dot = cleaned.rfind('.');
    let last_comma = cleaned.rfind(',');
    let result = match (last_dot, last_comma) {
        (Some(d), Some(c)) => {
            if c > d {
                // Virgule est le separateur decimal
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