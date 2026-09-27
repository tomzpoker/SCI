use serde::{Deserialize, Serialize};

fn null_to_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

fn string_or_number<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr + Deserialize<'de>,
    T::Err: std::fmt::Display,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber<T> {
        String(String),
        Number(T),
    }

    match StringOrNumber::<T>::deserialize(d)? {
        StringOrNumber::String(s) => {
            if s.is_empty() {
                "0".parse::<T>().map_err(serde::de::Error::custom)
            } else {
                s.parse::<T>().map_err(serde::de::Error::custom)
            }
        }
        StringOrNumber::Number(n) => Ok(n),
    }
}

fn default_qty() -> f64 {
    1.0
}

// ============================================================
//  FACTURES
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DolibarrInvoice {
    pub id: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub r#ref: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub ref_client: String,

    #[serde(default)]
    pub date: i64,

    #[serde(default)]
    pub date_lim_reglement: i64,

    #[serde(default, deserialize_with = "null_to_default")]
    pub total_ht: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub total_tva: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub total_ttc: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub paye: String,

    #[serde(default, deserialize_with = "string_or_number")]
    pub statut: i32,

    #[serde(default, deserialize_with = "null_to_default")]
    pub socid: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub note_public: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub note_private: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DolibarrInvoiceLine {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub desc: String,

    #[serde(default = "default_qty")]
    pub qty: f64,

    #[serde(default)]
    pub subprice: f64,

    #[serde(default)]
    pub tva_tx: f64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_type: Option<i32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fk_product: Option<i64>,
}

impl DolibarrInvoiceLine {
    pub fn new(desc: impl Into<String>, qty: f64, subprice: f64, tva_tx: f64) -> Self {
        Self {
            desc: desc.into(),
            qty,
            subprice,
            tva_tx,
            product_type: Some(1),
            fk_product: None,
        }
    }
}

// ============================================================
//  TIERS
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DolibarrThirdParty {
    pub id: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub name: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub name_alias: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub email: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub phone: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub address: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub zip: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub town: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub client: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub fournisseur: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub code_client: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub code_fournisseur: String,

    #[serde(default, rename = "idprof1", deserialize_with = "null_to_default")]
    pub siren: String,

    #[serde(default, rename = "idprof2", deserialize_with = "null_to_default")]
    pub siret: String,
}

// ============================================================
//  PAIEMENTS
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DolibarrPayment {
    pub id: String,

    #[serde(default)]
    pub datepaye: i64,

    #[serde(default, deserialize_with = "null_to_default")]
    pub amount: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub fk_facture: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub num_paiement: String,
}

// ============================================================
//  BANQUE
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DolibarrBankAccount {
    pub id: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub r#ref: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub label: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub bank: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub currency_code: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub iban: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub bic: String,

    #[serde(default, deserialize_with = "string_or_number")]
    pub clos: i32,

    #[serde(default, deserialize_with = "string_or_number")]
    pub courant: i32,

    #[serde(default, deserialize_with = "string_or_number")]
    pub solde: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DolibarrBankLine {
    pub id: String,

    #[serde(default)]
    pub dateo: i64,

    #[serde(default)]
    pub datev: i64,

    #[serde(default, deserialize_with = "null_to_default")]
    pub amount: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub label: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub r#type: String,

    #[serde(default, deserialize_with = "null_to_default")]
    pub num_releve: String,

    #[serde(default, deserialize_with = "string_or_number")]
    pub rappro: i32,

    #[serde(default, deserialize_with = "string_or_number")]
    pub fk_bordereau: i32,
}

/// Ligne bancaire en cours de creation (payload POST /bankaccounts/{id}/lines).
/// Dolibarr attend `date` (obligatoire), `datev` (valeur), `amount`, `label`, `type`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DolibarrBankLineDraft {
    pub date: i64,
    pub datev: i64,
    pub amount: f64,
    pub label: String,
    #[serde(rename = "type")]
    pub line_type: String,
    #[serde(default)]
    pub num_releve: String,
}

// ============================================================
//  WRAPPERS
// ============================================================

#[derive(Debug, Clone, Deserialize)]
pub struct BankAccountsWrapper {
    #[serde(default)]
    pub value: Vec<DolibarrBankAccount>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BankLinesWrapper {
    #[serde(default)]
    pub value: Vec<DolibarrBankLine>,
}