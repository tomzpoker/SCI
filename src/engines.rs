use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::infrastructure::db;
#[cfg(feature = "server")]
use sqlx::PgPool;
#[cfg(feature = "server")]
use sqlx::Row;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommonEngine {
    Calcul,
    Event,
    Workflow,
    Bank,
    Vat,
    Document,
    Cash,
    Audit,
}

impl CommonEngine {
    pub fn code(self) -> &'static str {
        match self {
            Self::Calcul => "CALCUL",
            Self::Event => "EVENT",
            Self::Workflow => "WORKFLOW",
            Self::Bank => "BANK",
            Self::Vat => "VAT",
            Self::Document => "DOCUMENT",
            Self::Cash => "CASH",
            Self::Audit => "AUDIT",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Calcul => "Calcul",
            Self::Event => "Événements",
            Self::Workflow => "Workflow",
            Self::Bank => "Banque",
            Self::Vat => "TVA",
            Self::Document => "Documents",
            Self::Cash => "Trésorerie",
            Self::Audit => "Audit",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineDescriptor {
    pub code: String,
    pub label: String,
    pub scope: Uuid,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineContext {
    pub workspace_id: Uuid,
    pub legal_entity_id: Uuid,
    pub legal_form_code: String,
    pub tax_regime: String,
    pub vat_status: String,
    pub vat_basis: String,
    pub currency_code: String,
}

pub fn catalog(context: &EngineContext) -> Vec<EngineDescriptor> {
    [
        CommonEngine::Calcul,
        CommonEngine::Event,
        CommonEngine::Workflow,
        CommonEngine::Bank,
        CommonEngine::Vat,
        CommonEngine::Document,
        CommonEngine::Cash,
        CommonEngine::Audit,
    ]
    .into_iter()
    .map(|engine| EngineDescriptor {
        code: engine.code().into(),
        label: engine.label().into(),
        scope: context.legal_entity_id,
        status: "READY".into(),
    })
    .collect()
}

pub fn vat_from_gross_cents(gross_cents: i64, vat_rate_bp: i32) -> i64 {
    if gross_cents <= 0 || vat_rate_bp <= 0 {
        return 0;
    }
    ((gross_cents as i128 * vat_rate_bp as i128) / (10_000i128 + vat_rate_bp as i128)) as i64
}

pub fn sum_cash<I>(amounts: I) -> i64
where
    I: IntoIterator<Item = i64>,
{
    amounts.into_iter().fold(0i64, |acc, value| acc.saturating_add(value))
}

#[cfg(feature = "server")]
pub async fn load_engine_context(pool: &PgPool, id: Uuid) -> Result<EngineContext, sqlx::Error> {
    let row = sqlx::query(
        "SELECT workspace_id,id,legal_form_code,tax_regime,vat_status,vat_basis,currency_code FROM legal_entities WHERE id=$1 AND active=true",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(EngineContext {
        workspace_id: row.get("workspace_id"),
        legal_entity_id: row.get("id"),
        legal_form_code: row.get("legal_form_code"),
        tax_regime: row.get("tax_regime"),
        vat_status: row.get("vat_status"),
        vat_basis: row.get("vat_basis"),
        currency_code: row.get("currency_code"),
    })
}

#[server]
pub async fn list_common_engines() -> Result<Vec<EngineDescriptor>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let context = load_engine_context(pool, crate::entity_scope::current_legal_entity_id())
            .await
            .map_err(ServerFnError::new)?;
        Ok(catalog(&context))
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_common_engines est exécutée côté serveur"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_profile_neutral() {
        let sci = EngineContext { workspace_id:Uuid::nil(), legal_entity_id:Uuid::from_u128(1), legal_form_code:"SCI".into(), tax_regime:"IR".into(), vat_status:"OPTION_LOYERS".into(), vat_basis:"COLLECTION".into(), currency_code:"EUR".into() };
        let sarl = EngineContext { legal_entity_id:Uuid::from_u128(2), legal_form_code:"SARL".into(), tax_regime:"IS".into(), ..sci.clone() };
        let a = catalog(&sci);
        let b = catalog(&sarl);
        assert_eq!(a.len(), 8);
        assert_eq!(b.len(), 8);
        assert_eq!(a[0].code, b[0].code);
        assert_ne!(a[0].scope, b[0].scope);
    }

    #[test]
    fn vat_collection_formula_is_shared() {
        assert_eq!(vat_from_gross_cents(12000, 2000), 2000);
        assert_eq!(vat_from_gross_cents(10000, 0), 0);
    }

    #[test]
    fn cash_sum_is_saturating() {
        assert_eq!(sum_cash([100, -30, 5]), 75);
    }
}

#[component]
pub fn CommonEnginesPage(mut refresh: Signal<u64>) -> Element {
    let engines = use_resource(move || { let _ = refresh(); async move { list_common_engines().await.unwrap_or_default() } });
    rsx! {
        section { class:"page-intro", div { div { class:"eyebrow", "ARCHITECTURE • MOTEURS COMMUNS" }, h2 { "Moteurs communs" }, p { "Les mêmes moteurs servent les SCI et SARL ; le contexte d'entité est fourni explicitement." } } }
        section { class:"panel", h3 { "Moteurs disponibles" },
            div { class:"data-list",
                for item in engines.read().as_deref().unwrap_or(&[]).iter() {
                    div { class:"data-row",
                        div { div { class:"data-title", "{item.label}" }, div { class:"small", "{item.code} • portée entité {item.scope} • {item.status}" } }
                    }
                }
            }
        }
        section { class:"panel",
            h3 { "Trésorerie" },
            p { class:"small", "Les positions REAL, RECONCILED, THEORETICAL et FORECAST sont stockées séparément. Une prévision n'est jamais un encaissement." }
        }
        section { class:"panel",
            h3 { "Événements métier standard" },
            p { class:"small", "LeaseCreated · InvoiceIssued · PaymentDetected · StockReceived · StockSold · TaxDeadlineReached · autres événements communs." }
        }
    }
}
