use crate::entity_scope::current_legal_entity_id;
use crate::ui::{euro, FormField, InfoTileOwned, ModuleHeader};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use dioxus::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecurringFlowItem {
    pub id: Uuid,
    pub label_pattern: String,
    pub counterparty: String,
    pub direction: String,
    pub classification: String,
    pub classification_fr: String,
    pub average_amount_cents: i64,
    pub occurrence_count: i32,
    pub average_gap_days: Option<Decimal>,
    pub confidence_bp: i32,
    pub last_seen_date: Option<NaiveDate>,
    pub next_expected_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TreasuryForecastPointItem {
    pub date: NaiveDate,
    pub inflows_cents: i64,
    pub outflows_cents: i64,
    pub net_cents: i64,
    pub balance_cents: i64,
    pub qualification: String,
    pub qualification_fr: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TreasuryForecastResult {
    pub as_of_date: NaiveDate,
    pub horizon_months: i32,
    pub ending_balance_cents: i64,
    pub minimum_balance_cents: i64,
    pub points: Vec<TreasuryForecastPointItem>,
    pub snapshot_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TreasuryEventItem {
    pub id: Uuid,
    pub forecast_date: NaiveDate,
    pub label: String,
    pub amount_cents: i64,
    pub qualification: String,
    pub qualification_fr: String,
    pub state: String,
    pub state_fr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvestmentItem {
    pub id: Uuid,
    pub category: String,
    pub category_fr: String,
    pub label: String,
    pub planned_date: NaiveDate,
    pub amount_cents: i64,
    pub probability_bp: i32,
    pub vat_rate_bp: i32,
    pub vat_cents: i64,
    pub fiscal_impact_cents: i64,
    pub rental_impact_cents: i64,
    pub status: String,
    pub status_fr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvestmentScenarioItem {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: String,
    pub probability_bp: i32,
    pub total_amount_cents: i64,
    pub total_vat_cents: i64,
    pub total_fiscal_impact_cents: i64,
    pub total_rental_impact_cents: i64,
    pub cash_impact_cents: i64,
    pub first_date: Option<NaiveDate>,
}

fn month_add(date: NaiveDate, offset: i32) -> NaiveDate {
    let base = date.year() * 12 + date.month0() as i32 + offset;
    let year = base.div_euclid(12);
    let month0 = base.rem_euclid(12);
    NaiveDate::from_ymd_opt(year, month0 as u32 + 1, 1).unwrap()
}

fn classify_fr(code: &str) -> &'static str {
    match code { "FIXED"=>"Fixe", "VARIABLE"=>"Variable", "SEASONAL"=>"Saisonnier", "PUNCTUAL"=>"Ponctuel", "PROBABLY_RECURRING"=>"Probablement récurrent", _=>"Inconnu" }
}
fn qualification_fr(code: &str) -> &'static str { match code { "CERTAIN"=>"Certain", "PROBABLE"=>"Probable", "HYPOTHESIS"=>"Hypothèse", "SCENARIO"=>"Scénario", _=>"—" } }
fn state_fr(code: &str) -> &'static str { match code { "SCHEDULED"=>"Planifié", "PENDING"=>"En attente", "AWAITING_BANK_MATCH"=>"En attente de rapprochement bancaire", "MATCHED"=>"Rapproché", "COMPLETED"=>"Terminé", "CANCELLED"=>"Annulé", _=>"—" } }
fn category_fr(code:&str)->&'static str{match code{"TRAVAUX"=>"Travaux","EQUIPEMENT"=>"Équipement","RENOVATION"=>"Rénovation","COPROPRIETE"=>"Copropriété","SECURITE"=>"Sécurité","MISE_AUX_NORMES"=>"Mise aux normes","RENOUVELLEMENT"=>"Renouvellement",_=>"Autre"}}

fn normalize_key(raw: &str) -> String {
    raw.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() || c.is_whitespace() { c } else { ' ' }).collect::<String>().split_whitespace().take(8).collect::<Vec<&str>>().join(" ")
}

#[server]
pub async fn detect_recurring_bank_flows() -> Result<i32, ServerFnError> {
    #[cfg(feature="server")]
    {
        use sqlx::Row;
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;let entity=current_legal_entity_id();
        let rows=sqlx::query("SELECT transaction_date,amount_cents,label,COALESCE(counterparty,'') counterparty FROM bank_transactions WHERE legal_entity_id=$1 AND transaction_date >= CURRENT_DATE - INTERVAL '540 days' ORDER BY transaction_date").bind(entity).fetch_all(pool).await.map_err(ServerFnError::new)?;
        #[derive(Clone)]struct Obs{date:NaiveDate,amount:i64,label:String,cp:String}
        let mut groups=std::collections::HashMap::<String,Vec<Obs>>::new();
        for r in rows{let label:String=r.get("label");let cp:String=r.get("counterparty");let key=format!("{}|{}|{}",normalize_key(&label),normalize_key(&cp),if r.get::<i64,_>("amount_cents")<0{"OUT"}else{"IN"});groups.entry(key).or_default().push(Obs{date:r.get("transaction_date"),amount:r.get("amount_cents"),label,cp});}
        let mut count=0i32;
        for (key,mut obs) in groups{obs.sort_by_key(|x|x.date);if obs.is_empty(){continue;}let occ=obs.len() as i32;let gaps:Vec<i64>=obs.windows(2).map(|w|(w[1].date-w[0].date).num_days()).filter(|d|*d>0).collect();let avg_gap=if gaps.is_empty(){None}else{Some(Decimal::from(gaps.iter().sum::<i64>())/Decimal::from(gaps.len() as u32))};let avg=obs.iter().map(|x|x.amount.abs()).sum::<i64>()/occ.max(1) as i64;let min=obs.iter().map(|x|x.amount.abs()).min().unwrap_or(0);let max=obs.iter().map(|x|x.amount.abs()).max().unwrap_or(0);let variability=if avg==0{0.0}else{(max-min) as f64/avg as f64};let seasonal=if gaps.len()>=2 && gaps.iter().max().unwrap_or(&0)-gaps.iter().min().unwrap_or(&0)>45{"SEASONAL"}else if occ==1{"PUNCTUAL"}else if (Decimal::from(20u32)..Decimal::from(40u32)).contains(&avg_gap.unwrap_or(Decimal::ZERO)) || (Decimal::from(70u32)..Decimal::from(100u32)).contains(&avg_gap.unwrap_or(Decimal::ZERO)) || (Decimal::from(330u32)..Decimal::from(400u32)).contains(&avg_gap.unwrap_or(Decimal::ZERO)){if variability<=0.05{"FIXED"}else{"PROBABLY_RECURRING"}}else if occ>=2{"VARIABLE"}else{"UNKNOWN"};let conf=if occ>=5{9300}else if occ>=3{8200}else if occ>=2{6500}else{3000};let next=avg_gap.and_then(|g|g.round().to_i64()).map(|g|obs.last().unwrap().date+Duration::days(g));let first=&obs[0];sqlx::query("INSERT INTO treasury_recurring_patterns(legal_entity_id,pattern_key,label_pattern,counterparty,direction,classification,average_amount_cents,min_amount_cents,max_amount_cents,average_gap_days,confidence_bp,occurrence_count,seasonal_months,last_seen_date,next_expected_date,generated_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,now()) ON CONFLICT(legal_entity_id,pattern_key) DO UPDATE SET label_pattern=EXCLUDED.label_pattern,counterparty=EXCLUDED.counterparty,direction=EXCLUDED.direction,classification=EXCLUDED.classification,average_amount_cents=EXCLUDED.average_amount_cents,min_amount_cents=EXCLUDED.min_amount_cents,max_amount_cents=EXCLUDED.max_amount_cents,average_gap_days=EXCLUDED.average_gap_days,confidence_bp=EXCLUDED.confidence_bp,occurrence_count=EXCLUDED.occurrence_count,seasonal_months=EXCLUDED.seasonal_months,last_seen_date=EXCLUDED.last_seen_date,next_expected_date=EXCLUDED.next_expected_date,generated_at=now(),active=true")
            .bind(entity).bind(&key).bind(&first.label).bind(&first.cp).bind(if first.amount<0{"OUT"}else{"IN"}).bind(seasonal).bind(avg).bind(min).bind(max).bind(avg_gap).bind(conf).bind(occ).bind(json!([])).bind(obs.last().map(|x|x.date)).bind(next).execute(pool).await.map_err(ServerFnError::new)?;count+=1;}
        Ok(count)
    }
    #[cfg(not(feature="server"))]
    Err(ServerFnError::new("detect_recurring_bank_flows est exécutée côté serveur"))
}

#[server]
pub async fn list_recurring_bank_flows() -> Result<Vec<RecurringFlowItem>, ServerFnError> {
    #[cfg(feature="server")]
    {use sqlx::Row;let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;let rows=sqlx::query("SELECT id,label_pattern,counterparty,direction,classification,average_amount_cents,occurrence_count,average_gap_days,confidence_bp,last_seen_date,next_expected_date FROM treasury_recurring_patterns WHERE legal_entity_id=$1 AND active=true ORDER BY confidence_bp DESC,average_amount_cents DESC LIMIT 200").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;Ok(rows.into_iter().map(|r|{let c:String=r.get("classification");RecurringFlowItem{id:r.get("id"),label_pattern:r.get("label_pattern"),counterparty:r.get("counterparty"),direction:r.get("direction"),classification_fr:classify_fr(&c).into(),classification:c,average_amount_cents:r.get("average_amount_cents"),occurrence_count:r.get("occurrence_count"),average_gap_days:r.get("average_gap_days"),confidence_bp:r.get("confidence_bp"),last_seen_date:r.get("last_seen_date"),next_expected_date:r.get("next_expected_date")}}).collect())}
    #[cfg(not(feature="server"))] Err(ServerFnError::new("list_recurring_bank_flows est exécutée côté serveur"))
}

#[cfg(feature="server")]
async fn current_bank_balance(pool:&sqlx::PgPool,entity:Uuid)->Result<i64,sqlx::Error>{
    let with_balance:Option<i64>=sqlx::query_scalar("SELECT SUM(x.balance_cents)::bigint FROM (SELECT DISTINCT ON (COALESCE(bank_account_id,'00000000-0000-0000-0000-000000000000'::uuid)) balance_cents FROM bank_transactions WHERE legal_entity_id=$1 AND balance_cents IS NOT NULL ORDER BY COALESCE(bank_account_id,'00000000-0000-0000-0000-000000000000'::uuid),transaction_date DESC,booked_at DESC) x").bind(entity).fetch_optional(pool).await?;
    if let Some(v)=with_balance{return Ok(v);}let opening:i64=sqlx::query_scalar("SELECT COALESCE(SUM(opening_balance_cents),0)::bigint FROM bank_account_profiles WHERE legal_entity_id=$1 AND active").bind(entity).fetch_one(pool).await?;let moves:i64=sqlx::query_scalar("SELECT COALESCE(SUM(amount_cents),0)::bigint FROM bank_transactions WHERE legal_entity_id=$1").bind(entity).fetch_one(pool).await?;Ok(opening+moves)
}

#[server]
pub async fn build_treasury_forecast(horizon_months:i32,scenario_code:String)->Result<TreasuryForecastResult,ServerFnError>{
    #[cfg(feature="server")]
    {
        use sqlx::Row;
        let allowed=[1,2,3,6,9,12,24,36,60,120];if !allowed.contains(&horizon_months){return Err(ServerFnError::new("Horizon non autorisé"));}
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;let entity=current_legal_entity_id();let as_of=Utc::now().date_naive();let mut balance=current_bank_balance(pool,entity).await.map_err(ServerFnError::new)?;let recurring=list_recurring_bank_flows_inner(pool,entity).await.map_err(ServerFnError::new)?;let lease_rents=sqlx::query("SELECT COALESCE(SUM(CASE WHEN l.rent_amount_cents>0 THEN l.rent_amount_cents ELSE u.base_rent_cents END + l.charges_amount_cents),0)::bigint total FROM leases l JOIN units u ON u.id=l.unit_id JOIN properties p ON p.id=u.property_id WHERE p.legal_entity_id=$1 AND l.legal_entity_id=$1 AND l.active AND l.start_date<=CURRENT_DATE").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;let rent_base:i64=lease_rents.get("total");let investments=sqlx::query("SELECT planned_date,amount_cents,probability_bp,label FROM treasury_investments WHERE legal_entity_id=$1 AND status IN ('PLANNED','CONFIRMED') AND planned_date>=CURRENT_DATE AND planned_date < CURRENT_DATE + ($2::int || ' months')::interval").bind(entity).bind(horizon_months).fetch_all(pool).await.map_err(ServerFnError::new)?;
        let mut points=Vec::new();let mut min_balance=balance;for m in 1..=horizon_months{let date=month_add(as_of,m);let mut inflow=rent_base;let mut outflow=0i64;let mut labels=vec!["Loyers contractuels".to_owned()];let mut qualification="CERTAIN".to_owned();for r in &recurring{if r.direction=="IN" && r.classification!="PUNCTUAL" && r.average_gap_days.unwrap_or(Decimal::from(999u32)) <= Decimal::from(400u32) && !r.label_pattern.to_lowercase().contains("loyer"){inflow+=r.average_amount_cents;labels.push(format!("Récurrent : {}",r.label_pattern));qualification="PROBABLE".into();}else if r.direction=="OUT" && r.classification!="PUNCTUAL" && r.average_gap_days.unwrap_or(Decimal::from(999u32))<=Decimal::from(400u32){outflow+=r.average_amount_cents;labels.push(format!("Récurrent : {}",r.label_pattern));qualification="PROBABLE".into();}}
            for inv in &investments{let d:NaiveDate=inv.get("planned_date");if d.year()==date.year()&&d.month()==date.month(){let amount:i64=inv.get("amount_cents");let probability:i32=inv.get("probability_bp");outflow+=amount.saturating_mul(probability as i64)/10000;labels.push(format!("Investissement : {}",inv.get::<String,_>("label")));qualification="SCENARIO".into();}}
            let net=inflow-outflow;balance=balance.saturating_add(net);min_balance=min_balance.min(balance);points.push(TreasuryForecastPointItem{date,inflows_cents:inflow,outflows_cents:outflow,net_cents:net,balance_cents:balance,qualification:qualification.clone(),qualification_fr:qualification_fr(&qualification).into(),labels});}
        let ending=balance;let payload=json!({"points":points});let snapshot:Uuid=sqlx::query_scalar("INSERT INTO treasury_forecast_snapshots(legal_entity_id,scenario_code,horizon_months,as_of_date,min_balance_cents,max_balance_cents,ending_balance_cents,points) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(legal_entity_id,scenario_code,horizon_months,as_of_date) DO UPDATE SET generated_at=now(),min_balance_cents=EXCLUDED.min_balance_cents,max_balance_cents=EXCLUDED.max_balance_cents,ending_balance_cents=EXCLUDED.ending_balance_cents,points=EXCLUDED.points RETURNING id").bind(entity).bind(if scenario_code.trim().is_empty(){"BASE"}else{scenario_code.trim()}).bind(horizon_months).bind(as_of).bind(min_balance).bind(points.iter().map(|p|p.balance_cents).max().unwrap_or(ending)).bind(ending).bind(payload.get("points").cloned().unwrap_or(json!([]))).fetch_one(pool).await.map_err(ServerFnError::new)?;Ok(TreasuryForecastResult{as_of_date:as_of,horizon_months,ending_balance_cents:ending,minimum_balance_cents:min_balance,points,snapshot_id:snapshot})
    }
    #[cfg(not(feature="server"))]
    {let _=(horizon_months,scenario_code);Err(ServerFnError::new("build_treasury_forecast est exécutée côté serveur"))}
}

#[cfg(feature="server")]
async fn list_recurring_bank_flows_inner(pool:&sqlx::PgPool,entity:Uuid)->Result<Vec<RecurringFlowItem>,sqlx::Error>{use sqlx::Row;let rows=sqlx::query("SELECT id,label_pattern,counterparty,direction,classification,average_amount_cents,occurrence_count,average_gap_days,confidence_bp,last_seen_date,next_expected_date FROM treasury_recurring_patterns WHERE legal_entity_id=$1 AND active ORDER BY confidence_bp DESC LIMIT 200").bind(entity).fetch_all(pool).await?;Ok(rows.into_iter().map(|r|{let c:String=r.get("classification");RecurringFlowItem{id:r.get("id"),label_pattern:r.get("label_pattern"),counterparty:r.get("counterparty"),direction:r.get("direction"),classification_fr:classify_fr(&c).into(),classification:c,average_amount_cents:r.get("average_amount_cents"),occurrence_count:r.get("occurrence_count"),average_gap_days:r.get("average_gap_days"),confidence_bp:r.get("confidence_bp"),last_seen_date:r.get("last_seen_date"),next_expected_date:r.get("next_expected_date")}}).collect())}

#[server]
pub async fn materialize_treasury_forecast_event(forecast_date:NaiveDate,label:String,amount_cents:i64,qualification:String)->Result<Uuid,ServerFnError>{
    #[cfg(feature="server")]
    {let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;let entity=current_legal_entity_id();if !["CERTAIN","PROBABLE","HYPOTHESIS","SCENARIO"].contains(&qualification.as_str()){return Err(ServerFnError::new("Qualification invalide"));}let key=format!("FORECAST:{}:{}:{}",forecast_date,amount_cents,normalize_key(&label));let id:Uuid=sqlx::query_scalar("INSERT INTO treasury_forecast_events(legal_entity_id,forecast_date,label,amount_cents,qualification,state,source_type,materialization_key) VALUES($1,$2,$3,$4,$5,'SCHEDULED','FORECAST',$6) ON CONFLICT(legal_entity_id,materialization_key) DO UPDATE SET label=EXCLUDED.label,amount_cents=EXCLUDED.amount_cents,qualification=EXCLUDED.qualification,updated_at=now() RETURNING id").bind(entity).bind(forecast_date).bind(label.trim()).bind(amount_cents).bind(&qualification).bind(&key).fetch_one(pool).await.map_err(ServerFnError::new)?;Ok(id)}
    #[cfg(not(feature="server"))] {let _=(forecast_date,label,amount_cents,qualification);Err(ServerFnError::new("materialize_treasury_forecast_event est exécutée côté serveur"))}
}

#[server]
pub async fn create_treasury_hypothesis(forecast_date:NaiveDate,label:String,amount_cents:i64)->Result<Uuid,ServerFnError>{materialize_treasury_forecast_event(forecast_date,label,amount_cents,"HYPOTHESIS".to_owned()).await}

#[server]
pub async fn list_treasury_events()->Result<Vec<TreasuryEventItem>,ServerFnError>{
    #[cfg(feature="server")]
    {use sqlx::Row;let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;let rows=sqlx::query("SELECT id,forecast_date,label,amount_cents,qualification,state FROM treasury_forecast_events WHERE legal_entity_id=$1 ORDER BY forecast_date DESC,created_at DESC LIMIT 300").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;Ok(rows.into_iter().map(|r|{let q:String=r.get("qualification");let s:String=r.get("state");TreasuryEventItem{id:r.get("id"),forecast_date:r.get("forecast_date"),label:r.get("label"),amount_cents:r.get("amount_cents"),qualification_fr:qualification_fr(&q).into(),qualification:q,state_fr:state_fr(&s).into(),state:s}}).collect())}
    #[cfg(not(feature="server"))]Err(ServerFnError::new("list_treasury_events est exécutée côté serveur"))
}

#[server]
pub async fn set_treasury_event_state(id:Uuid,state:String)->Result<(),ServerFnError>{
    #[cfg(feature="server")]
    {let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;if !["SCHEDULED","PENDING","AWAITING_BANK_MATCH","MATCHED","COMPLETED","CANCELLED"].contains(&state.as_str()){return Err(ServerFnError::new("État prévisionnel invalide"));}sqlx::query("UPDATE treasury_forecast_events SET state=$3,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(current_legal_entity_id()).bind(state).execute(pool).await.map_err(ServerFnError::new)?;Ok(())}
    #[cfg(not(feature="server"))]{let _=(id,state);Err(ServerFnError::new("set_treasury_event_state est exécutée côté serveur"))}
}

#[server]
pub async fn create_treasury_investment(category:String,label:String,planned_date:NaiveDate,amount_cents:i64,probability_bp:i32,vat_rate_bp:i32,fiscal_impact_cents:i64,rental_impact_cents:i64)->Result<Uuid,ServerFnError>{
    #[cfg(feature="server")]
    {let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;if !["TRAVAUX","EQUIPEMENT","RENOVATION","COPROPRIETE","SECURITE","MISE_AUX_NORMES","RENOUVELLEMENT","AUTRE"].contains(&category.as_str())||label.trim().is_empty()||amount_cents<0||!(0..=10000).contains(&probability_bp)||!(0..=10000).contains(&vat_rate_bp){return Err(ServerFnError::new("Investissement invalide"));}let id:Uuid=sqlx::query_scalar("INSERT INTO treasury_investments(legal_entity_id,category,label,planned_date,amount_cents,probability_bp,vat_rate_bp,fiscal_impact_cents,rental_impact_cents,status) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,'PLANNED') RETURNING id").bind(current_legal_entity_id()).bind(category).bind(label.trim()).bind(planned_date).bind(amount_cents).bind(probability_bp).bind(vat_rate_bp).bind(fiscal_impact_cents).bind(rental_impact_cents).fetch_one(pool).await.map_err(ServerFnError::new)?;Ok(id)}
    #[cfg(not(feature="server"))]{let _=(category,label,planned_date,amount_cents,probability_bp,vat_rate_bp,fiscal_impact_cents,rental_impact_cents);Err(ServerFnError::new("create_treasury_investment est exécutée côté serveur"))}
}

#[server]
pub async fn list_treasury_investments()->Result<Vec<InvestmentItem>,ServerFnError>{
    #[cfg(feature="server")]
    {use sqlx::Row;let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;let rows=sqlx::query("SELECT id,category,label,planned_date,amount_cents,probability_bp,vat_rate_bp,ROUND(amount_cents::numeric*vat_rate_bp/10000)::bigint vat_cents,fiscal_impact_cents,rental_impact_cents,status FROM treasury_investments WHERE legal_entity_id=$1 ORDER BY planned_date LIMIT 200").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;Ok(rows.into_iter().map(|r|{let c:String=r.get("category");let s:String=r.get("status");InvestmentItem{id:r.get("id"),category_fr:category_fr(&c).into(),category:c,label:r.get("label"),planned_date:r.get("planned_date"),amount_cents:r.get("amount_cents"),probability_bp:r.get("probability_bp"),vat_rate_bp:r.get("vat_rate_bp"),vat_cents:r.get("vat_cents"),fiscal_impact_cents:r.get("fiscal_impact_cents"),rental_impact_cents:r.get("rental_impact_cents"),status_fr:match s.as_str(){"PLANNED"=>"Planifié","CONFIRMED"=>"Confirmé","CANCELLED"=>"Annulé",_=>"—"}.into(),status:s}}).collect())}
    #[cfg(not(feature="server"))]Err(ServerFnError::new("list_treasury_investments est exécutée côté serveur"))
}

#[server]
pub async fn create_investment_scenario(code:String,name:String,description:String,probability_bp:i32)->Result<Uuid,ServerFnError>{
    #[cfg(feature="server")]
    {let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;if code.trim().is_empty()||name.trim().is_empty()||!(0..=10000).contains(&probability_bp){return Err(ServerFnError::new("Scénario invalide"));}let entity=current_legal_entity_id();let id:Uuid=sqlx::query_scalar("INSERT INTO treasury_investment_scenarios(legal_entity_id,code,name,description,scenario_probability_bp) VALUES($1,$2,$3,$4,$5) ON CONFLICT(legal_entity_id,code) DO UPDATE SET name=EXCLUDED.name,description=EXCLUDED.description,scenario_probability_bp=EXCLUDED.scenario_probability_bp,updated_at=now() RETURNING id").bind(entity).bind(code.trim()).bind(name.trim()).bind(description.trim()).bind(probability_bp).fetch_one(pool).await.map_err(ServerFnError::new)?;sqlx::query("INSERT INTO treasury_investment_scenario_items(legal_entity_id,scenario_id,investment_id) SELECT $1,$2,id FROM treasury_investments WHERE legal_entity_id=$1 AND status IN ('PLANNED','CONFIRMED') ON CONFLICT(legal_entity_id,scenario_id,investment_id) DO NOTHING").bind(entity).bind(id).execute(pool).await.map_err(ServerFnError::new)?;Ok(id)}
    #[cfg(not(feature="server"))]{let _=(code,name,description,probability_bp);Err(ServerFnError::new("create_investment_scenario est exécutée côté serveur"))}
}

#[server]
pub async fn add_investment_to_scenario(scenario_id:Uuid,investment_id:Uuid,override_date:Option<NaiveDate>,override_amount_cents:Option<i64>,override_probability_bp:Option<i32>)->Result<Uuid,ServerFnError>{
    #[cfg(feature="server")]
    {let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;let entity=current_legal_entity_id();if override_amount_cents.is_some_and(|v|v<0)||override_probability_bp.is_some_and(|v|!(0..=10000).contains(&v)){return Err(ServerFnError::new("Paramètre scénario invalide"));}let id:Uuid=sqlx::query_scalar("INSERT INTO treasury_investment_scenario_items(legal_entity_id,scenario_id,investment_id,override_date,override_amount_cents,override_probability_bp) SELECT $1,$2,$3,$4,$5,$6 WHERE EXISTS(SELECT 1 FROM treasury_investment_scenarios WHERE id=$2 AND legal_entity_id=$1) AND EXISTS(SELECT 1 FROM treasury_investments WHERE id=$3 AND legal_entity_id=$1) ON CONFLICT(legal_entity_id,scenario_id,investment_id) DO UPDATE SET override_date=EXCLUDED.override_date,override_amount_cents=EXCLUDED.override_amount_cents,override_probability_bp=EXCLUDED.override_probability_bp RETURNING id").bind(entity).bind(scenario_id).bind(investment_id).bind(override_date).bind(override_amount_cents).bind(override_probability_bp).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Scénario ou investissement introuvable"))?;Ok(id)}
    #[cfg(not(feature="server"))]{let _=(scenario_id,investment_id,override_date,override_amount_cents,override_probability_bp);Err(ServerFnError::new("add_investment_to_scenario est exécutée côté serveur"))}
}

#[server]
pub async fn list_investment_scenario_summaries()->Result<Vec<InvestmentScenarioItem>,ServerFnError>{
    #[cfg(feature="server")]
    {use sqlx::Row;let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;let entity=current_legal_entity_id();let rows=sqlx::query("SELECT id FROM treasury_investment_scenarios WHERE legal_entity_id=$1 ORDER BY code").bind(entity).fetch_all(pool).await.map_err(ServerFnError::new)?;let mut out=Vec::new();for r in rows{out.push(summarize_investment_scenario_inner(pool,entity,r.get("id")).await.map_err(ServerFnError::new)?);}Ok(out)}
    #[cfg(not(feature="server"))]Err(ServerFnError::new("list_investment_scenario_summaries est exécutée côté serveur"))
}

#[cfg(feature="server")]
async fn summarize_investment_scenario_inner(pool:&sqlx::PgPool,entity:Uuid,scenario_id:Uuid)->Result<InvestmentScenarioItem,sqlx::Error>{
    use sqlx::Row;let header=sqlx::query("SELECT id,code,name,description,scenario_probability_bp FROM treasury_investment_scenarios WHERE id=$1 AND legal_entity_id=$2").bind(scenario_id).bind(entity).fetch_one(pool).await?;let rows=sqlx::query("SELECT COALESCE(si.override_amount_cents,i.amount_cents) amount,COALESCE(si.override_probability_bp,i.probability_bp) probability,COALESCE(si.override_date,i.planned_date) planned_date,i.vat_rate_bp,i.fiscal_impact_cents,i.rental_impact_cents FROM treasury_investment_scenario_items si JOIN treasury_investments i ON i.id=si.investment_id WHERE si.scenario_id=$1 AND si.legal_entity_id=$2 AND i.status<>'CANCELLED'").bind(scenario_id).bind(entity).fetch_all(pool).await?;let mut total=0i64;let mut vat=0i64;let mut fiscal=0i64;let mut rental=0i64;let mut first=None;for r in rows{let a:i64=r.get("amount");let p:i32=r.get("probability");total+=a.saturating_mul(p as i64)/10000;vat+=((Decimal::from(a)*Decimal::from(r.get::<i32,_>("vat_rate_bp")))/Decimal::from(10000u32)).round_dp(0).to_i64().unwrap_or(0).saturating_mul(p as i64)/10000;fiscal+=r.get::<i64,_>("fiscal_impact_cents").saturating_mul(p as i64)/10000;rental+=r.get::<i64,_>("rental_impact_cents").saturating_mul(p as i64)/10000;let d:NaiveDate=r.get("planned_date");if first.map(|x:NaiveDate|d<x).unwrap_or(true){first=Some(d);}}Ok(InvestmentScenarioItem{id:header.get("id"),code:header.get("code"),name:header.get("name"),description:header.get("description"),probability_bp:header.get("scenario_probability_bp"),total_amount_cents:total,total_vat_cents:vat,total_fiscal_impact_cents:fiscal,total_rental_impact_cents:rental,cash_impact_cents:-total,first_date:first})
}

#[server]
pub async fn summarize_investment_scenario(scenario_id:Uuid)->Result<InvestmentScenarioItem,ServerFnError>{
    #[cfg(feature="server")]
    {let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;summarize_investment_scenario_inner(pool,current_legal_entity_id(),scenario_id).await.map_err(ServerFnError::new)}
    #[cfg(not(feature="server"))]{let _=scenario_id;Err(ServerFnError::new("summarize_investment_scenario est exécutée côté serveur"))}
}


#[component]
pub fn TreasuryPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let mut horizon = use_signal(|| "12".to_owned());
    let mut scenario = use_signal(|| "BASE".to_owned());
    let forecast = use_resource(move || { let _ = refresh(); let _ = bump(); let h = horizon().parse::<i32>().unwrap_or(12); let s = scenario(); async move { build_treasury_forecast(h, s).await } });
    let recurring = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_recurring_bank_flows().await.unwrap_or_default() } });
    let events = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_treasury_events().await.unwrap_or_default() } });
    let scenarios = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_investment_scenario_summaries().await.unwrap_or_default() } });
    let investments = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_treasury_investments().await.unwrap_or_default() } });
    let mut category = use_signal(|| "TRAVAUX".to_owned());
    let mut inv_label = use_signal(String::new);
    let mut inv_date = use_signal(|| (Utc::now().date_naive() + Duration::days(90)).to_string());
    let mut inv_amount = use_signal(String::new);
    let mut inv_prob = use_signal(|| "5000".to_owned());
    let mut inv_vat = use_signal(|| "20".to_owned());
    let mut inv_fiscal = use_signal(|| "0".to_owned());
    let mut inv_rental = use_signal(|| "0".to_owned());
    let mut msg = use_signal(String::new);
    let mut hyp_date = use_signal(|| Utc::now().date_naive().to_string());
    let mut hyp_label = use_signal(String::new);
    let mut hyp_amount = use_signal(String::new);
    let mut scenario_code = use_signal(|| "TRAVAUX_BASE".to_owned());
    let mut scenario_name = use_signal(|| "Scénario travaux".to_owned());
    let mut scenario_prob = use_signal(|| "5000".to_owned());
    let forecast_value = forecast.read().clone();

    rsx! {
        ModuleHeader { title: "Trésorerie", kicker: "RÉCURRENCE • PRÉVISIONS • ÉVÉNEMENTS • INVESTISSEMENTS", detail: "Les prévisions restent des prévisions. La matérialisation d’un événement ne crée ni paiement ni encaissement bancaire." }
        section {
            class: "panel",
            h3 { "Prévisions multi-horizons" }
            div {
                class: "form-grid",
                label {
                    class: "field",
                    span { "Horizon" }
                    select {
                        value: horizon(),
                        onchange: move |e: FormEvent| horizon.set(e.value()),
                        option { value: "1", "1 mois" }
                        option { value: "2", "2 mois" }
                        option { value: "3", "3 mois" }
                        option { value: "6", "6 mois" }
                        option { value: "12", "1 an" }
                        option { value: "24", "2 ans" }
                        option { value: "36", "3 ans" }
                        option { value: "60", "5 ans" }
                        option { value: "120", "10 ans" }
                    }
                }
                FormField { label: "Code scénario", value: scenario(), oninput: move |e: FormEvent| scenario.set(e.value()) }
            }
            if let Some(Ok(f)) = forecast_value.as_ref() {
                div {
                    class: "facts-row",
                    InfoTileOwned { label: "Solde final", value: euro(f.ending_balance_cents) }
                    InfoTileOwned { label: "Point bas", value: euro(f.minimum_balance_cents) }
                    InfoTileOwned { label: "Mois", value: f.points.len().to_string() }
                }
                for p in f.points.clone().into_iter().take(24) {
                    div {
                        class: "data-row",
                        div {
                            div { class: "data-title", "{p.date} • {euro(p.balance_cents)}" }
                            div { class: "small", "Entrées {euro(p.inflows_cents)} • Sorties {euro(p.outflows_cents)} • {p.qualification_fr}" }
                        }
                        button {
                            class: "secondary",
                            onclick: move |_| {
                                let date = p.date;
                                let amount = p.net_cents;
                                async move {
                                    let _ = materialize_treasury_forecast_event(date, format!("Prévision {date}"), amount, "PROBABLE".into()).await;
                                    bump += 1;
                                }
                            },
                            "Matérialiser l’événement"
                        }
                    }
                }
            }
        }
        section {
            class: "panel",
            h3 { "Flux récurrents détectés" }
            button {
                class: "secondary",
                onclick: move |_| async move {
                    match detect_recurring_bank_flows().await {
                        Ok(n) => { msg.set(format!("{} groupe(s) analysé(s)", n)); bump += 1; }
                        Err(e) => msg.set(e.to_string()),
                    }
                },
                "Détecter / recalculer"
            }
            span { class: "save-ok", "{msg}" }
            for r in recurring.read().as_deref().unwrap_or(&[]).iter().take(30) {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{r.label_pattern} • {r.direction}" }
                        div { class: "small", "{r.classification_fr} • {r.occurrence_count} occurrence(s) • confiance {r.confidence_bp / 100}%" }
                        div { class: "small", {format!("Moyenne {} • prochaine {}", euro(r.average_amount_cents), r.next_expected_date.map(|d| d.to_string()).unwrap_or_else(|| "—".into()))} }
                    }
                }
            }
        }
        section {
            class: "panel",
            h3 { "Investissements" }
            div {
                class: "form-grid",
                label {
                    class: "field",
                    span { "Type" }
                    select {
                        value: category(), onchange: move |e: FormEvent| category.set(e.value()),
                        option { value: "TRAVAUX", "Travaux" }
                        option { value: "EQUIPEMENT", "Équipement" }
                        option { value: "RENOVATION", "Rénovation" }
                        option { value: "COPROPRIETE", "Copropriété" }
                        option { value: "SECURITE", "Sécurité" }
                        option { value: "MISE_AUX_NORMES", "Mise aux normes" }
                        option { value: "RENOUVELLEMENT", "Renouvellement" }
                        option { value: "AUTRE", "Autre" }
                    }
                }
                FormField { label: "Libellé", value: inv_label(), oninput: move |e: FormEvent| inv_label.set(e.value()) }
                FormField { label: "Date AAAA-MM-JJ", value: inv_date(), oninput: move |e: FormEvent| inv_date.set(e.value()) }
                FormField { label: "Montant €", value: inv_amount(), oninput: move |e: FormEvent| inv_amount.set(e.value()) }
                FormField { label: "Probabilité 0-10000", value: inv_prob(), oninput: move |e: FormEvent| inv_prob.set(e.value()) }
                FormField { label: "TVA %", value: inv_vat(), oninput: move |e: FormEvent| inv_vat.set(e.value()) }
                FormField { label: "Impact fiscal €", value: inv_fiscal(), oninput: move |e: FormEvent| inv_fiscal.set(e.value()) }
                FormField { label: "Impact locatif €", value: inv_rental(), oninput: move |e: FormEvent| inv_rental.set(e.value()) }
            }
            button {
                class: "primary",
                onclick: move |_| async move {
                    match NaiveDate::parse_from_str(&inv_date(), "%Y-%m-%d") {
                        Ok(date) => {
                            let amount = crate::ui::euros_to_cents(&inv_amount());
                            let probability = inv_prob().parse().unwrap_or(5000);
                            let vat = inv_vat().replace(',', ".").parse::<f64>().unwrap_or(0.0);
                            let vat_bp = (vat * 100.0).round() as i32;
                            match create_treasury_investment(category(), inv_label(), date, amount, probability, vat_bp, crate::ui::euros_to_cents(&inv_fiscal()), crate::ui::euros_to_cents(&inv_rental())).await {
                                Ok(_) => { msg.set("Investissement enregistré".into()); bump += 1; }
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                        Err(_) => msg.set("Date d'investissement invalide".into()),
                    }
                },
                "Ajouter"
            }
            for i in investments.read().as_deref().unwrap_or(&[]).iter() {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{i.label} • {i.category_fr}" }
                        div { class: "small", "{euro(i.amount_cents)} • probabilité {i.probability_bp / 100}% • TVA {i.vat_rate_bp / 100}%" }
                        div { class: "small", "Date {i.planned_date} • impact fiscal {euro(i.fiscal_impact_cents)} • impact locatif {euro(i.rental_impact_cents)}" }
                    }
                }
            }
        }
        section {
            class: "two-col",
            div {
                class: "panel",
                h3 { "Hypothèse manuelle" }
                div {
                    class: "form-grid",
                    FormField { label: "Date AAAA-MM-JJ", value: hyp_date(), oninput: move |e: FormEvent| hyp_date.set(e.value()) }
                    FormField { label: "Libellé", value: hyp_label(), oninput: move |e: FormEvent| hyp_label.set(e.value()) }
                    FormField { label: "Montant € (+ entrée / - sortie)", value: hyp_amount(), oninput: move |e: FormEvent| hyp_amount.set(e.value()) }
                }
                button {
                    class: "secondary",
                    onclick: move |_| async move {
                        match NaiveDate::parse_from_str(&hyp_date(), "%Y-%m-%d") {
                            Ok(date) => match create_treasury_hypothesis(date, hyp_label(), crate::ui::euros_to_cents(&hyp_amount())).await {
                                Ok(_) => { msg.set("Hypothèse ajoutée".into()); bump += 1; }
                                Err(e) => msg.set(e.to_string()),
                            },
                            Err(_) => msg.set("Date invalide".into()),
                        }
                    },
                    "Ajouter l’hypothèse"
                }
            }
            div {
                class: "panel",
                h3 { "Scénario d’investissement" }
                div {
                    class: "form-grid",
                    FormField { label: "Code", value: scenario_code(), oninput: move |e: FormEvent| scenario_code.set(e.value()) }
                    FormField { label: "Nom", value: scenario_name(), oninput: move |e: FormEvent| scenario_name.set(e.value()) }
                    FormField { label: "Probabilité 0-10000", value: scenario_prob(), oninput: move |e: FormEvent| scenario_prob.set(e.value()) }
                }
                button {
                    class: "secondary",
                    onclick: move |_| async move {
                        match create_investment_scenario(scenario_code(), scenario_name(), "Scénario calculé depuis les investissements actifs".into(), scenario_prob().parse().unwrap_or(5000)).await {
                            Ok(_) => { msg.set("Scénario enregistré".into()); bump += 1; }
                            Err(e) => msg.set(e.to_string()),
                        }
                    },
                    "Créer / actualiser le scénario"
                }
            }
        }
        section {
            class: "panel",
            h3 { "Synthèses des scénarios" }
            for s in scenarios.read().as_deref().unwrap_or(&[]).iter() {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{s.name} • {s.code}" }
                        div { class: "small", "Montant {euro(s.total_amount_cents)} • probabilité {s.probability_bp / 100}% • trésorerie {euro(s.cash_impact_cents)}" }
                        div { class: "small", {format!("TVA {} • fiscal {} • locatif {} • première date {}", euro(s.total_vat_cents), euro(s.total_fiscal_impact_cents), euro(s.total_rental_impact_cents), s.first_date.map(|d| d.to_string()).unwrap_or_else(|| "—".into()))} }
                    }
                }
            }
        }
        section {
            class: "panel",
            h3 { "Événements prévisionnels matérialisés" }
            p { class: "small", "SCHEDULED ne vaut pas PAID : le rapprochement bancaire reste séparé." }
            for e in events.read().as_deref().unwrap_or(&[]).iter() {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{e.forecast_date} • {e.label}" }
                        div { class: "small", "{euro(e.amount_cents)} • {e.qualification_fr} • {e.state_fr}" }
                    }
                    div {
                        class: "row-actions",
                        button { class: "secondary", onclick: { let id = e.id; move |_| async move { let _ = set_treasury_event_state(id, "AWAITING_BANK_MATCH".into()).await; bump += 1 } }, "Attendre la banque" }
                        button { class: "secondary", onclick: { let id = e.id; move |_| async move { let _ = set_treasury_event_state(id, "COMPLETED".into()).await; bump += 1 } }, "Terminer" }
                    }
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_add_crosses_year() {
        let d = NaiveDate::from_ymd_opt(2026, 12, 1).unwrap();
        assert_eq!(month_add(d, 1), NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());
        assert_eq!(month_add(d, 13), NaiveDate::from_ymd_opt(2028, 1, 1).unwrap());
    }

    #[test]
    fn labels_are_translated() {
        assert_eq!(classify_fr("FIXED"), "Fixe");
        assert_eq!(qualification_fr("HYPOTHESIS"), "Hypothèse");
        assert_eq!(state_fr("AWAITING_BANK_MATCH"), "En attente de rapprochement bancaire");
        assert_eq!(category_fr("MISE_AUX_NORMES"), "Mise aux normes");
    }
}
