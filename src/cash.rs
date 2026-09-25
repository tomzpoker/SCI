use chrono::NaiveDate;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CashPositionType { Real, Reconciled, Theoretical, Forecast }
impl CashPositionType { pub fn code(self)->&'static str{match self{Self::Real=>"REAL",Self::Reconciled=>"RECONCILED",Self::Theoretical=>"THEORETICAL",Self::Forecast=>"FORECAST"}} }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CashPositionItem { pub id:Uuid,pub as_of_date:NaiveDate,pub position_type:String,pub amount_cents:i64,pub source_run_id:Option<Uuid> }

pub fn separate_cash_positions(real:i64,reconciled:i64,theoretical:i64,forecast:i64)->[(CashPositionType,i64);4]{[(CashPositionType::Real,real),(CashPositionType::Reconciled,reconciled),(CashPositionType::Theoretical,theoretical),(CashPositionType::Forecast,forecast)]}

#[server]
pub async fn list_cash_positions(as_of_date:NaiveDate)->Result<Vec<CashPositionItem>,ServerFnError>{
    #[cfg(feature="server")]
    {let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;let rows=sqlx::query("SELECT id,as_of_date,position_type,amount_cents,source_run_id FROM cash_position_snapshots WHERE legal_entity_id=$1 AND as_of_date=$2 ORDER BY position_type").bind(crate::entity_scope::current_legal_entity_id()).bind(as_of_date).fetch_all(pool).await.map_err(ServerFnError::new)?;Ok(rows.into_iter().map(|r|CashPositionItem{id:r.get("id"),as_of_date:r.get("as_of_date"),position_type:r.get("position_type"),amount_cents:r.get("amount_cents"),source_run_id:r.get("source_run_id")}).collect())}
    #[cfg(not(feature="server"))]
    {let _=as_of_date;Err(ServerFnError::new("list_cash_positions est exécutée côté serveur"))}
}

#[cfg(feature="server")]
use sqlx::Row;

#[cfg(test)]mod tests{use super::*;#[test]fn positions_stay_distinct(){let x=separate_cash_positions(1,2,3,4);assert_eq!(x[0].0,CashPositionType::Real);assert_eq!(x[3].1,4);}}
