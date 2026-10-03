//! Parser OCR pour avis de taxe foncière.

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedTaxNoticeBases {
    pub notice_reference: String,
    pub commune: String,
    pub departement: String,
    pub addresses: Vec<ParsedAddress>,
    pub confidence: i32,
    pub raw_text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedAddress {
    pub label: String,
    pub base_cents: i64,
    pub tax_cents: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedTaxNoticeFees {
    pub notice_reference: String,
    pub total_amount_cents: i64,
    pub cotisations_amount_cents: i64,
    pub management_fees_cents: i64,
    pub fiscal_year: Option<i32>,
    pub confidence: i32,
    pub raw_text: String,
}

pub fn parse_bases_sheet(raw_text: &str) -> ParsedTaxNoticeBases {
    let mut out = ParsedTaxNoticeBases::default();
    let cleaned = clean(raw_text);
    out.raw_text = cleaned.chars().take(4000).collect();

    let ref_re = Regex::new(r"(?i)N[°o]\s*de\s*facture\s*:?\s*([0-9][0-9\s]{6,30})").unwrap();
    if let Some(c) = ref_re.captures(&cleaned) {
        out.notice_reference = c[1].trim().to_string();
    }

    let com_re = Regex::new(r"(?i)Commune\s*:\s*([A-Za-zÀ-ÿ\- ]+)\s*\((\d+)\)\s*,?\s*([A-Za-zÀ-ÿ\- ]+)\s*\((\d+)\)").unwrap();
    if let Some(c) = com_re.captures(&cleaned) {
        out.commune = format!("{} ({})", c[1].trim(), c[2].trim());
        out.departement = format!("{} ({})", c[3].trim(), c[4].trim());
    }

    let addr_re = Regex::new(r"(?m)^\s*\|?\s*(\d{4})\s+([A-ZÀ-Ý][A-ZÀ-Ý0-9\s\.\-'/]{3,80}?)\s*\|?\s*$").unwrap();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut order = 0i32;

    for cap in addr_re.captures_iter(&cleaned) {
        let number = cap[1].trim();
        let street = cap[2].trim().trim_end_matches('|').trim();
        if street.len() < 3 { continue; }

        let label = format!("{} {}", number, street);
        let key = label.to_uppercase();
        if seen.contains(&key) { continue; }
        seen.insert(key);

        out.addresses.push(ParsedAddress {
            label,
            base_cents: 0,
            tax_cents: 0,
        });
        order += 1;
        if order >= 50 { break; }
    }

    let mut score: i32 = 0;
    if !out.notice_reference.is_empty() { score += 30; }
    if !out.commune.is_empty() { score += 20; }
    score += (out.addresses.len() as i32 * 15).min(50);
    out.confidence = score.min(100);
    out
}

/// Associe les totaux (colonne de droite OCR) aux adresses, par ordre vertical.
pub fn merge_addresses_with_totals(
    addresses_labels: &[String],
    right_column_text: &str,
) -> Vec<ParsedAddress> {
    let num_re = Regex::new(r"\b(\d{3,7})\b").unwrap();
    let mut numbers: Vec<i64> = Vec::new();
    for cap in num_re.captures_iter(right_column_text) {
        if let Ok(n) = cap[1].parse::<i64>() {
            if (100..=500_000).contains(&n) {
                numbers.push(n);
            }
        }
    }

    addresses_labels
        .iter()
        .enumerate()
        .map(|(i, label)| ParsedAddress {
            label: label.clone(),
            base_cents: 0,
            tax_cents: numbers.get(i).copied().unwrap_or(0) * 100,
        })
        .collect()
}

pub fn parse_fees_sheet(raw_text: &str, column_text: &str) -> ParsedTaxNoticeFees {
    let mut out = ParsedTaxNoticeFees::default();
    let cleaned = clean(raw_text);
    out.raw_text = cleaned.chars().take(4000).collect();

    // Année
    let year_re = Regex::new(r"(?i)taxes?\s+fonci[eè]res?\s+(\d{4})|cotisation\s+(\d{4})").unwrap();
    if let Some(c) = year_re.captures(&cleaned) {
        let y = c.get(1).or_else(|| c.get(2)).map(|m| m.as_str()).unwrap_or("");
        out.fiscal_year = y.parse::<i32>().ok();
    }

    // Référence
    let ref_re = Regex::new(r"(?i)N[°o]\s*de\s*facture\s*:?\s*([0-9][0-9\s]{6,30})").unwrap();
    if let Some(c) = ref_re.captures(&cleaned) {
        out.notice_reference = c[1].trim().to_string();
    }

    // ========================================================
    // Frais de gestion (cherché dans le texte complet)
    // Tesseract sort parfois : "396 \n Frais de gestion" (avant)
    // ou "Frais de gestion ... 396" (après)
    // ========================================================
    let fees_after_re = Regex::new(r"(?i)Frais\s+de\s+gestion[^\d]{0,120}(\d{2,6})").unwrap();
    let fees_before_re = Regex::new(r"(?m)^\s*(\d{2,6})\s*\n\s*Frais\s+de\s+gestion").unwrap();

    if let Some(c) = fees_after_re.captures(&cleaned) {
        out.management_fees_cents = to_cents(&c[1]);
    } else if let Some(c) = fees_before_re.captures(&cleaned) {
        out.management_fees_cents = to_cents(&c[1]);
    }

    // ========================================================
    // Montant total de l'impôt (extrait de la colonne droite ciblée)
    // Prend le PLUS GRAND nombre à 5-6 chiffres de la colonne (le montant total)
    // ========================================================
    let num_re = Regex::new(r"\b(\d{4,6})\b").unwrap();
    let mut candidates: Vec<i64> = Vec::new();
    for cap in num_re.captures_iter(column_text) {
        if let Ok(n) = cap[1].parse::<i64>() {
            // Plausible : entre 1 000 € et 100 000 €
            if (1_000..=100_000).contains(&n) {
                candidates.push(n);
            }
        }
    }
    // Le montant de l'impôt est le plus grand (total à payer)
    if let Some(max) = candidates.iter().max() {
        out.total_amount_cents = *max * 100;
    }

    // Cotisations = total − frais
    if out.total_amount_cents > 0 && out.management_fees_cents > 0 {
        out.cotisations_amount_cents = out.total_amount_cents - out.management_fees_cents;
    }

    let mut score: i32 = 0;
    if out.total_amount_cents > 0 { score += 50; }
    if out.management_fees_cents > 0 { score += 30; }
    if out.fiscal_year.is_some() { score += 20; }
    out.confidence = score.min(100);
    out
}

fn clean(text: &str) -> String {
    text.replace('\u{a0}', " ")
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_cents(s: &str) -> i64 {
    let n: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    n.parse::<i64>().unwrap_or(0) * 100
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge() {
        // Test avec des données fictives (adresses + montants non réels)
        let labels = vec![
            "0001 RUE DE L EXEMPLE".to_string(),
            "0002 AV DU TEST".to_string(),
            "0003 BD DE LA MAQUETTE".to_string(),
        ];
        let right = "Total des\nCotisations\n1200\n850\n450\n";
        let merged = merge_addresses_with_totals(&labels, right);
        assert_eq!(merged[0].tax_cents, 120_000);
        assert_eq!(merged[1].tax_cents, 85_000);
        assert_eq!(merged[2].tax_cents, 45_000);
    }

    #[test]
    fn test_parse_fees_sheet() {
        let text = "Taxes foncières 2025\n\
                    Montant de votre impôt 5000\n\
                    Frais de gestion de la fiscalité directe locale 200\n";
        let column_text = "5000\n200\n";
        let r = parse_fees_sheet(text, column_text);
        assert_eq!(r.fiscal_year, Some(2025));
        assert_eq!(r.total_amount_cents, 500_000);
        assert_eq!(r.management_fees_cents, 20_000);
    }
}