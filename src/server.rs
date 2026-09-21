use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::domain::{CashForecastPoint, DashboardSnapshot, ModuleCounts, SciProfile, TaskItem, TaskState};

#[cfg(feature = "server")]
use chrono::Duration as ChronoDuration;
#[cfg(feature = "server")]
use sqlx::Row;
#[cfg(feature = "server")]
use uuid::Uuid;
#[cfg(feature = "server")]
use crate::infrastructure::db;

#[server]
pub async fn dashboard_snapshot() -> Result<DashboardSnapshot, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let sci_id = sci_id();
        let profile = fetch_profile(pool, sci_id).await.map_err(ServerFnError::new)?;
        let counts = fetch_counts(pool, sci_id).await.map_err(ServerFnError::new)?;

        let receivables_cents: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(i.gross_cents - COALESCE(p.paid, 0)), 0) FROM invoices i LEFT JOIN (SELECT invoice_id, SUM(amount_cents) paid FROM payments GROUP BY invoice_id) p ON p.invoice_id=i.id WHERE i.sci_id=$1 AND i.status IN ('ISSUED','PARTIAL','OVERDUE')"
        ).bind(sci_id).fetch_one(pool).await.map_err(ServerFnError::new)?;

        let cash_cents: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_cents),0) FROM bank_transactions WHERE sci_id=$1"
        ).bind(sci_id).fetch_one(pool).await.map_err(ServerFnError::new)?;

        let vat_to_prepare_cents: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(ROUND(p.amount_cents::numeric * i.vat_cents / NULLIF(i.gross_cents, 0))),0)::bigint FROM payments p JOIN invoices i ON i.id=p.invoice_id WHERE p.sci_id=$1 AND p.received_at >= date_trunc('month', now())"
        ).bind(sci_id).fetch_one(pool).await.map_err(ServerFnError::new)?;

        let tasks_due_30d: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE sci_id=$1 AND state NOT IN ('DONE','SKIPPED') AND due_at < now() + interval '30 days'"
        ).bind(sci_id).fetch_one(pool).await.map_err(ServerFnError::new)?;

        let overdue_tasks: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE sci_id=$1 AND state NOT IN ('DONE','SKIPPED') AND due_at < now()"
        ).bind(sci_id).fetch_one(pool).await.map_err(ServerFnError::new)?;

        let rows = sqlx::query(
            "SELECT id, title, due_at, state, priority, blocking FROM tasks WHERE sci_id=$1 AND state NOT IN ('DONE','SKIPPED') ORDER BY CASE state WHEN 'BLOCKED' THEN 0 WHEN 'READY' THEN 1 ELSE 2 END, priority DESC, due_at LIMIT 8"
        ).bind(sci_id).fetch_all(pool).await.map_err(ServerFnError::new)?;

        let next_actions = rows.into_iter().map(|r| TaskItem {
            id: r.get("id"),
            title: r.get("title"),
            due_at: r.get("due_at"),
            state: task_state(r.get::<String,_>("state")),
            priority: r.get("priority"),
            blocking: r.get("blocking"),
        }).collect::<Vec<_>>();

        let points = build_forecast(cash_cents, receivables_cents, vat_to_prepare_cents);
        let forecast_min_cash_cents = points.iter().map(|p| p.balance_cents).min().unwrap_or(cash_cents);
        let mut snapshot = DashboardSnapshot {
            sci_name: profile.legal_name,
            registered_office: profile.registered_office,
            tax_regime: profile.tax_regime,
            vat_basis: profile.vat_basis,
            cash_cents,
            receivables_cents,
            vat_to_prepare_cents,
            tasks_due_30d,
            overdue_tasks,
            forecast_min_cash_cents,
            risk_level: "NORMAL".into(),
            next_actions,
            forecast: points,
        };
        snapshot.risk_level = crate::application::AnticipationEngine::derive_risk(&snapshot);
        return Ok(snapshot);
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("dashboard_snapshot est exécutée côté serveur"))
}

#[server]
pub async fn module_counts() -> Result<ModuleCounts, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        return fetch_counts(pool, sci_id()).await.map_err(ServerFnError::new);
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("module_counts est exécutée côté serveur"))
}

#[server]
pub async fn get_sci_profile() -> Result<SciProfile, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        return fetch_profile(pool, sci_id()).await.map_err(ServerFnError::new);
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("get_sci_profile est exécutée côté serveur"))
}

#[server]
pub async fn update_sci_profile(profile: SciProfile) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        sqlx::query("UPDATE scis SET legal_name=$2, siren=NULLIF($3,''), siret=NULLIF($4,''), registered_office=NULLIF($5,''), iban=NULLIF($6,''), bic=NULLIF($7,''), updated_at=now() WHERE id=$1")
            .bind(sci_id()).bind(profile.legal_name.trim()).bind(profile.siren.trim()).bind(profile.siret.trim()).bind(profile.registered_office.trim()).bind(profile.iban.trim()).bind(profile.bic.trim())
            .execute(pool).await.map_err(ServerFnError::new)?;
        return Ok(());
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("update_sci_profile est exécutée côté serveur"))
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AutomationRunResult {
    pub created_tasks: usize,
    pub evaluated_rules: usize,
    pub ran_at: DateTime<Utc>,
}

#[server]
pub async fn run_anticipation_cycle() -> Result<AutomationRunResult, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        return automation_tick(pool).await.map_err(ServerFnError::new);
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("run_anticipation_cycle est exécutée côté serveur"))
}

#[cfg(feature = "server")]
pub async fn automation_tick(pool: &sqlx::PgPool) -> Result<AutomationRunResult, sqlx::Error> {
    let sci_id = sci_id();
    let now = Utc::now();
    let rules = sqlx::query("SELECT id, code, name, horizon_days FROM automation_rules WHERE sci_id=$1 AND enabled=true")
        .bind(sci_id).fetch_all(pool).await?;
    let mut created = 0usize;
    for rule in &rules {
        let rule_id: Uuid = rule.get("id");
        let code: String = rule.get("code");
        let title: String = rule.get("name");
        let horizon_days: i32 = rule.get("horizon_days");
        let due_date = (now + ChronoDuration::days((horizon_days.clamp(1, 365)) as i64)).date_naive();
        let due = due_date.and_hms_opt(9, 0, 0).unwrap().and_utc();
        let occurrence_key = due_date.to_string();
        let result = sqlx::query("INSERT INTO tasks(sci_id, automation_rule_id, code, title, due_at, state, priority, source, occurrence_key) VALUES ($1,$2,$3,$4,$5,'PLANNED',60,'AUTOMATION',$6) ON CONFLICT (sci_id, code, occurrence_key) DO NOTHING")
            .bind(sci_id).bind(rule_id).bind(&code).bind(&title).bind(due).bind(&occurrence_key).execute(pool).await?;
        created += result.rows_affected() as usize;
    }
    sqlx::query("INSERT INTO audit_events(sci_id, actor, action, payload) VALUES ($1,'SYSTEM','ANTICIPATION_CYCLE',$2)")
        .bind(sci_id).bind(serde_json::json!({"created_tasks": created, "evaluated_rules": rules.len()})).execute(pool).await?;
    Ok(AutomationRunResult { created_tasks: created, evaluated_rules: rules.len(), ran_at: now })
}

#[cfg(feature = "server")]
fn sci_id() -> Uuid { Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap() }

#[cfg(not(feature = "server"))]
#[cfg(feature = "server")]
fn sci_id() -> Uuid { Uuid::nil() }

#[cfg(feature = "server")]
async fn fetch_profile(pool: &sqlx::PgPool, sci_id: Uuid) -> Result<SciProfile, sqlx::Error> {
    sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<String>, String, String, String, Option<String>, Option<String>)>(
        "SELECT legal_name, siren, siret, registered_office, tax_regime, vat_status, vat_basis, iban, bic FROM scis WHERE id=$1"
    ).bind(sci_id).fetch_one(pool).await.map(|r| SciProfile {
        legal_name:r.0, siren:r.1.unwrap_or_default(), siret:r.2.unwrap_or_default(), registered_office:r.3.unwrap_or_default(), tax_regime:r.4, vat_status:r.5, vat_basis:r.6, iban:r.7.unwrap_or_default(), bic:r.8.unwrap_or_default()
    })
}

#[cfg(feature = "server")]
async fn fetch_counts(pool: &sqlx::PgPool, sci_id: Uuid) -> Result<ModuleCounts, sqlx::Error> {
    let properties = scalar_count(pool, "SELECT COUNT(*) FROM properties WHERE sci_id=$1", sci_id).await?;
    let units = scalar_count(pool, "SELECT COUNT(*) FROM units u JOIN properties p ON p.id=u.property_id WHERE p.sci_id=$1", sci_id).await?;
    let tenants = scalar_count(pool, "SELECT COUNT(*) FROM tenants WHERE sci_id=$1", sci_id).await?;
    let leases = scalar_count(pool, "SELECT COUNT(*) FROM leases l JOIN units u ON u.id=l.unit_id JOIN properties p ON p.id=u.property_id WHERE p.sci_id=$1", sci_id).await?;
    let invoices = scalar_count(pool, "SELECT COUNT(*) FROM invoices WHERE sci_id=$1", sci_id).await?;
    let payments = scalar_count(pool, "SELECT COUNT(*) FROM payments WHERE sci_id=$1", sci_id).await?;
    let bank_transactions = scalar_count(pool, "SELECT COUNT(*) FROM bank_transactions WHERE sci_id=$1", sci_id).await?;
    let unmatched_bank = scalar_count(pool, "SELECT COUNT(*) FROM bank_transactions WHERE sci_id=$1 AND reconciliation_status='UNMATCHED'", sci_id).await?;
    let documents = scalar_count(pool, "SELECT COUNT(*) FROM documents WHERE sci_id=$1", sci_id).await?;
    let automation_rules = scalar_count(pool, "SELECT COUNT(*) FROM automation_rules WHERE sci_id=$1", sci_id).await?;
    let enabled_automation_rules = scalar_count(pool, "SELECT COUNT(*) FROM automation_rules WHERE sci_id=$1 AND enabled", sci_id).await?;
    let tax_deadlines = scalar_count(pool, "SELECT COUNT(*) FROM tax_deadlines WHERE sci_id=$1 AND deadline_date >= CURRENT_DATE", sci_id).await?;
    Ok(ModuleCounts { properties, units, tenants, leases, invoices, payments, bank_transactions, unmatched_bank, documents, automation_rules, enabled_automation_rules, tax_deadlines })
}

#[cfg(feature = "server")]
async fn scalar_count(pool: &sqlx::PgPool, sql: &'static str, sci_id: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(sql).bind(sci_id).fetch_one(pool).await
}

fn task_state(value: String) -> TaskState {
    match value.as_str() { "READY" => TaskState::Ready, "RUNNING" => TaskState::Running, "BLOCKED" => TaskState::Blocked, "DONE" => TaskState::Done, "SKIPPED" => TaskState::Skipped, _ => TaskState::Planned }
}

#[cfg(feature = "server")]
fn build_forecast(cash_cents: i64, receivables_cents: i64, vat_cents: i64) -> Vec<CashForecastPoint> {
    let today = Utc::now().date_naive();
    let mut balance = cash_cents;
    (0..12).map(|index| {
        let inflow = if index % 3 == 0 { receivables_cents / 4 } else { 0 };
        let outflow = if index % 2 == 0 { 120_000 } else { 65_000 };
        let vat = if index == 2 { vat_cents } else { 0 };
        balance += inflow - outflow - vat;
        CashForecastPoint { date: today + chrono::Duration::days((index + 1) * 30), expected_inflows_cents: inflow, expected_outflows_cents: outflow, expected_vat_cents: vat, balance_cents: balance }
    }).collect()
}
