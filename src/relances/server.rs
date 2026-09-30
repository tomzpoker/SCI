use dioxus::prelude::*;
use crate::entity_scope::current_legal_entity_id;
use super::data::*;
use super::templates::{self, RelanceLevel, RelanceContext};

// ============================================================
//  IMPAYÉS DOLIBARR
// ============================================================

#[server]
pub async fn list_unpaid_tenants() -> Result<Vec<UnpaidTenantSummary>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::dolibarr::client::DolibarrClient;
        use crate::dolibarr::models::DolibarrThirdParty;
        use std::collections::HashMap;

        let client = DolibarrClient::from_env().map_err(ServerFnError::new)?;
        let invoices = client.list_invoices(500).await.map_err(ServerFnError::new)?;
        let thirds: Vec<DolibarrThirdParty> = client
            .list_third_parties(500)
            .await
            .map_err(ServerFnError::new)?;

        let third_index: HashMap<String, DolibarrThirdParty> = thirds
            .into_iter()
            .map(|t| (t.id.clone(), t))
            .collect();

        let now_ts = chrono::Utc::now().timestamp();

        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        use sqlx::Row;
        let relance_rows = sqlx::query(
            "SELECT dolibarr_invoice_id, MAX(level) AS max_level, MAX(sent_at) AS last_sent \
             FROM email_relances \
             WHERE legal_entity_id = $1 AND status = 'sent' \
             GROUP BY dolibarr_invoice_id",
        )
        .bind(entity)
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        let mut relance_by_invoice: HashMap<String, (i32, i64)> = HashMap::new();
        for r in relance_rows {
            let id: String = r.get("dolibarr_invoice_id");
            let lvl: i32 = r.get::<Option<i32>, _>("max_level").unwrap_or(0);
            let last: Option<chrono::DateTime<chrono::Utc>> = r.get("last_sent");
            let ts = last.map(|d| d.timestamp()).unwrap_or(0);
            relance_by_invoice.insert(id, (lvl, ts));
        }

        let mut unpaid: Vec<UnpaidInvoiceItem> = Vec::new();
        for inv in invoices {
            let statut = inv.statut;
            if !(statut == 1 || statut == 2) {
                continue;
            }
            let total_ttc: f64 = inv.total_ttc.parse().unwrap_or(0.0);
            let paid: f64 = inv.paye.parse().unwrap_or(0.0);
            let outstanding = total_ttc - paid;
            if outstanding < 0.5 {
                continue;
            }

            let due_ts = inv.date_lim_reglement;
            let days_overdue = if due_ts > 0 && now_ts > due_ts {
                ((now_ts - due_ts) / 86400) as i32
            } else {
                0
            };

            let third = third_index.get(&inv.socid);
            let client_name = third
                .map(|t| t.name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| inv.ref_client.clone());
            let client_email = third.map(|t| t.email.clone()).unwrap_or_default();

            let (relance_level_sent, last_relance_at) =
                relance_by_invoice.get(&inv.id).copied().unwrap_or((0, 0));

            unpaid.push(UnpaidInvoiceItem {
                dolibarr_invoice_id: inv.id.clone(),
                invoice_ref: inv.r#ref.clone(),
                client_id: inv.socid.clone(),
                client_name,
                client_email,
                issue_date_ts: inv.date,
                due_date_ts: due_ts,
                total_ttc,
                paid,
                outstanding,
                days_overdue,
                relance_level_sent,
                last_relance_at: if last_relance_at > 0 { Some(last_relance_at) } else { None },
            });
        }

        let mut by_client: HashMap<String, UnpaidTenantSummary> = HashMap::new();
        for inv in unpaid {
            let entry = by_client.entry(inv.client_id.clone()).or_insert_with(|| {
                UnpaidTenantSummary {
                    client_id: inv.client_id.clone(),
                    client_name: inv.client_name.clone(),
                    client_email: inv.client_email.clone(),
                    total_outstanding: 0.0,
                    max_days_overdue: 0,
                    invoice_count: 0,
                    highest_level_sent: 0,
                    invoices: Vec::new(),
                }
            });
            entry.total_outstanding += inv.outstanding;
            entry.max_days_overdue = entry.max_days_overdue.max(inv.days_overdue);
            entry.invoice_count += 1;
            entry.highest_level_sent = entry.highest_level_sent.max(inv.relance_level_sent);
            entry.invoices.push(inv);
        }

        let mut result: Vec<UnpaidTenantSummary> = by_client.into_values().collect();
        result.sort_by(|a, b| b.max_days_overdue.cmp(&a.max_days_overdue));

        Ok(result)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_unpaid_tenants est executee cote serveur"))
}

// ============================================================
//  VUE GLOBALE DES LOCAUX
// ============================================================

#[server]
pub async fn list_all_locals() -> Result<Vec<LocalBarItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use std::collections::{HashMap, HashSet};
        use sqlx::Row;

        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();

        let rows = sqlx::query(
            "SELECT
                u.id AS unit_id, u.code AS unit_code, u.label AS unit_label,
                p.name AS property_name,
                t.id AS tenant_id, t.legal_name AS tenant_name,
                COALESCE(t.contact_email, '') AS tenant_email,
                l.id AS lease_id
             FROM units u
             JOIN properties p ON p.id = u.property_id
             LEFT JOIN leases l ON l.unit_id = u.id AND l.active = true
             LEFT JOIN tenants t ON t.id = l.tenant_id
             WHERE p.legal_entity_id = $1 AND u.active = true
             ORDER BY p.name ASC, u.code ASC",
        )
        .bind(entity)
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        let unpaid = list_unpaid_tenants().await.unwrap_or_default();

        let mut by_email: HashMap<String, &UnpaidTenantSummary> = HashMap::new();
        let mut by_name: HashMap<String, &UnpaidTenantSummary> = HashMap::new();
        for u in unpaid.iter() {
            if !u.client_email.is_empty() {
                by_email.insert(u.client_email.to_lowercase(), u);
            }
            if !u.client_name.is_empty() {
                by_name.insert(u.client_name.to_lowercase(), u);
            }
        }

        let mut locals: Vec<LocalBarItem> = Vec::new();
        let mut matched_client_ids: HashSet<String> = HashSet::new();

        for r in rows {
            let unit_id: uuid::Uuid = r.get("unit_id");
            let unit_code: String = r.get("unit_code");
            let unit_label: String = r.get("unit_label");
            let property_name: String = r.get("property_name");
            let tenant_name: Option<String> = r.get("tenant_name");
            let tenant_email: Option<String> = r.get("tenant_email");

            let (status, dolibarr_client_id, total_outstanding, max_days, inv_count, level_sent, invoices) =
                match tenant_name.as_ref() {
                    None => (LocalStatus::Vacant, None, 0.0, 0, 0, 0, Vec::new()),
                    Some(_) => {
                        let matched = tenant_email
                            .as_ref()
                            .filter(|e| !e.is_empty())
                            .and_then(|e| by_email.get(&e.to_lowercase()).copied())
                            .or_else(|| tenant_name.as_ref().and_then(|n| by_name.get(&n.to_lowercase()).copied()));

                        match matched {
                            Some(summary) => {
                                matched_client_ids.insert(summary.client_id.clone());
                                let status = if summary.max_days_overdue >= 60 {
                                    LocalStatus::Critical
                                } else if summary.max_days_overdue > 0 {
                                    LocalStatus::Late
                                } else {
                                    LocalStatus::UpToDate
                                };
                                (
                                    status,
                                    Some(summary.client_id.clone()),
                                    summary.total_outstanding,
                                    summary.max_days_overdue,
                                    summary.invoice_count,
                                    summary.highest_level_sent,
                                    summary.invoices.clone(),
                                )
                            }
                            None => (LocalStatus::UpToDate, None, 0.0, 0, 0, 0, Vec::new()),
                        }
                    }
                };

            locals.push(LocalBarItem {
                unit_id: unit_id.to_string(),
                unit_code, unit_label, property_name,
                tenant_name, tenant_email,
                dolibarr_client_id,
                total_outstanding,
                max_days_overdue: max_days,
                invoice_count: inv_count,
                highest_level_sent: level_sent,
                status,
                invoices,
            });
        }

        // Impayés orphelins
        for summary in unpaid.iter() {
            if matched_client_ids.contains(&summary.client_id) {
                continue;
            }
            let status = if summary.max_days_overdue >= 60 {
                LocalStatus::Critical
            } else if summary.max_days_overdue > 0 {
                LocalStatus::Late
            } else {
                LocalStatus::UpToDate
            };
            locals.push(LocalBarItem {
                unit_id: format!("orphan-{}", summary.client_id),
                unit_code: "—".to_string(),
                unit_label: format!("Tiers : {}", summary.client_name),
                property_name: "Hors patrimoine".to_string(),
                tenant_name: Some(summary.client_name.clone()),
                tenant_email: Some(summary.client_email.clone()),
                dolibarr_client_id: Some(summary.client_id.clone()),
                total_outstanding: summary.total_outstanding,
                max_days_overdue: summary.max_days_overdue,
                invoice_count: summary.invoice_count,
                highest_level_sent: summary.highest_level_sent,
                status,
                invoices: summary.invoices.clone(),
            });
        }

        // Tri : Critical, Late, UpToDate, Vacant
        locals.sort_by(|a, b| {
            let order = |s: LocalStatus| match s {
                LocalStatus::Critical => 0,
                LocalStatus::Late => 1,
                LocalStatus::UpToDate => 2,
                LocalStatus::Vacant => 3,
            };
            order(a.status).cmp(&order(b.status)).then_with(|| {
                b.total_outstanding.partial_cmp(&a.total_outstanding).unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        Ok(locals)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_all_locals est executee cote serveur"))
}

// ============================================================
//  RELANCES — helper interne (preview + send)
// ============================================================

#[cfg(feature = "server")]
async fn build_relance_preview(
    invoice_id: &str,
    level_code: i32,
) -> Result<(RelancePreview, RelanceLevel, String, f64, f64, f64, String), String> {
    use crate::dolibarr::client::DolibarrClient;
    use crate::dolibarr::models::DolibarrThirdParty;

    let level = RelanceLevel::from_code(level_code)
        .ok_or_else(|| "Niveau de relance invalide".to_string())?;

    let client = DolibarrClient::from_env()?;
    let invoice = client.get_invoice(invoice_id).await?;
    let third: DolibarrThirdParty = client
        .get_third_party(&invoice.socid)
        .await
        .unwrap_or(DolibarrThirdParty {
            id: invoice.socid.clone(),
            name: invoice.ref_client.clone(),
            name_alias: String::new(),
            email: String::new(),
            phone: String::new(),
            address: String::new(),
            zip: String::new(),
            town: String::new(),
            client: String::new(),
            fournisseur: String::new(),
            code_client: String::new(),
            code_fournisseur: String::new(),
            siren: String::new(),
            siret: String::new(),
        });

    let recipient_email = third.email.clone();

    let pool = crate::infrastructure::db().await.map_err(|e| e.to_string())?;
    let entity = current_legal_entity_id();
    use sqlx::Row;
    let entity_row = sqlx::query(
        "SELECT legal_name, registered_office, COALESCE(siren,'') AS siren,
                COALESCE(MAX(a.iban) FILTER (WHERE a.active AND a.is_primary),'') AS iban,
                COALESCE(MAX(a.bic) FILTER (WHERE a.active AND a.is_primary),'') AS bic
         FROM legal_entities e
         LEFT JOIN legal_entity_bank_accounts a ON a.legal_entity_id = e.id
         WHERE e.id = $1
         GROUP BY e.id",
    )
    .bind(entity)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let sci_name: String = entity_row.get("legal_name");
    let sci_address: String = entity_row.get("registered_office");
    let sci_siren: String = entity_row.get("siren");
    let sci_iban: String = entity_row.get("iban");
    let sci_bic: String = entity_row.get("bic");

    let total_ttc: f64 = invoice.total_ttc.parse().unwrap_or(0.0);
    let paid: f64 = invoice.paye.parse().unwrap_or(0.0);
    let outstanding = (total_ttc - paid).max(0.0);
    let now_ts = chrono::Utc::now().timestamp();
    let days_overdue = if invoice.date_lim_reglement > 0 && now_ts > invoice.date_lim_reglement {
        ((now_ts - invoice.date_lim_reglement) / 86400) as i32
    } else { 0 };

    let penalty_rate = 0.0802;
    let ht_ratio = if total_ttc > 0.0 {
        invoice.total_ht.parse::<f64>().unwrap_or(0.0) / total_ttc
    } else { 1.0 };
    let outstanding_ht = outstanding * ht_ratio;
    let penalties = if days_overdue > 0 {
        (outstanding_ht * penalty_rate * days_overdue as f64 / 365.0).max(0.0)
    } else { 0.0 };
    let forfait = if days_overdue > 0 { 40.0 } else { 0.0 };
    let total_due = outstanding + penalties + forfait;

    let due_date_str = chrono::DateTime::<chrono::Utc>::from_timestamp(
        invoice.date_lim_reglement, 0,
    ).map(|d| d.format("%d/%m/%Y").to_string()).unwrap_or_default();

    let ctx = RelanceContext {
        sci_name: sci_name.clone(),
        sci_address: sci_address.clone(),
        sci_siren: sci_siren.clone(),
        sci_iban: sci_iban.clone(),
        sci_bic: sci_bic.clone(),
        client_name: third.name.clone(),
        invoice_ref: invoice.r#ref.clone(),
        invoice_due_date: due_date_str,
        invoice_amount: format!("{:.2}", total_ttc),
        outstanding_amount: format!("{:.2}", outstanding),
        days_overdue,
        penalties: format!("{:.2}", penalties),
        forfait: format!("{:.2}", forfait),
        total_due: format!("{:.2}", total_due),
        tribunal: "Toulouse".to_string(),
    };

    let subject = templates::subject_for(level, &ctx);
    let body = templates::body_for(level, &ctx);

    let preview = RelancePreview {
        level_code,
        level_label: level.label().to_string(),
        subject,
        body,
        recipient_email: recipient_email.clone(),
        days_overdue,
        outstanding,
        penalties,
        forfait,
        total_due,
    };

    Ok((preview, level, recipient_email, outstanding, penalties, forfait, third.name))
}

// ============================================================
//  PREVIEW RELANCE
// ============================================================

#[server]
pub async fn preview_invoice_relance(
    invoice_id: String,
    level_code: i32,
) -> Result<RelancePreview, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (preview, _, _, _, _, _, _) = build_relance_preview(&invoice_id, level_code)
            .await
            .map_err(ServerFnError::new)?;
        Ok(preview)
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = (invoice_id, level_code);
        Err(ServerFnError::new("preview_invoice_relance est executee cote serveur"))
    }
}

// ============================================================
//  ENVOI RELANCE
// ============================================================

#[server]
pub async fn send_invoice_relance(
    invoice_id: String,
    level_code: i32,
    recipient_email_override: Option<String>,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::dolibarr::client::DolibarrClient;

        let (preview, level, default_email, outstanding, penalties, forfait, client_name) =
            build_relance_preview(&invoice_id, level_code)
                .await
                .map_err(ServerFnError::new)?;

        let recipient = recipient_email_override
            .filter(|e| !e.trim().is_empty())
            .unwrap_or(default_email);

        if recipient.trim().is_empty() {
            return Err(ServerFnError::new("Aucun email client renseigné dans Dolibarr"));
        }

        let client = DolibarrClient::from_env().map_err(ServerFnError::new)?;

        let (sent_via, status, error_msg) = match client
            .send_invoice_email(&invoice_id, &recipient, &preview.subject, &preview.body)
            .await
        {
            Ok(_) => ("dolibarr".to_string(), "sent".to_string(), String::new()),
            Err(doli_err) => {
                match send_via_gmail_smtp(&recipient, &preview.subject, &preview.body).await {
                    Ok(_) => ("smtp".to_string(), "sent".to_string(), String::new()),
                    Err(smtp_err) => (
                        "smtp".to_string(),
                        "failed".to_string(),
                        format!("Dolibarr: {} | SMTP: {}", doli_err, smtp_err),
                    ),
                }
            }
        };

        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();

        // Récupère la ref de la facture pour le log
        let invoice_ref = client
            .get_invoice(&invoice_id)
            .await
            .map(|i| i.r#ref)
            .unwrap_or_default();

        sqlx::query(
            "INSERT INTO email_relances \
             (legal_entity_id, dolibarr_invoice_id, invoice_ref, client_name, client_email, \
              level, subject, body, sent_via, status, error_message, metadata) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULLIF($11,''), $12)",
        )
        .bind(entity)
        .bind(&invoice_id)
        .bind(&invoice_ref)
        .bind(&client_name)
        .bind(&recipient)
        .bind(level.code())
        .bind(&preview.subject)
        .bind(&preview.body)
        .bind(&sent_via)
        .bind(&status)
        .bind(&error_msg)
        .bind(serde_json::json!({
            "days_overdue": preview.days_overdue,
            "outstanding": outstanding,
            "penalties": penalties,
            "forfait": forfait,
        }))
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;

        if status == "failed" {
            return Err(ServerFnError::new(error_msg));
        }

        Ok(format!("{} envoyée à {} via {}", level.label(), recipient, sent_via))
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = (invoice_id, level_code, recipient_email_override);
        Err(ServerFnError::new("send_invoice_relance est executee cote serveur"))
    }
}

// ============================================================
//  HISTORIQUE PAR FACTURES
// ============================================================

#[server]
pub async fn list_relances_for_invoices(
    invoice_ids: Vec<String>,
) -> Result<Vec<RelanceHistoryItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        if invoice_ids.is_empty() {
            return Ok(Vec::new());
        }
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let rows = sqlx::query(
            "SELECT id, invoice_ref, client_name, level, subject, sent_at, \
                    sent_via, status, COALESCE(error_message,'') AS error_message \
             FROM email_relances \
             WHERE legal_entity_id = $1 AND dolibarr_invoice_id = ANY($2) \
             ORDER BY sent_at DESC LIMIT 100",
        )
        .bind(entity)
        .bind(&invoice_ids)
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let sent_at: chrono::DateTime<chrono::Utc> = r.get("sent_at");
                RelanceHistoryItem {
                    id: r.get::<uuid::Uuid, _>("id").to_string(),
                    invoice_ref: r.get("invoice_ref"),
                    client_name: r.get("client_name"),
                    level: r.get("level"),
                    subject: r.get("subject"),
                    sent_at_ts: sent_at.timestamp(),
                    sent_via: r.get("sent_via"),
                    status: r.get("status"),
                    error_message: r.get("error_message"),
                }
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = invoice_ids;
        Err(ServerFnError::new("list_relances_for_invoices est executee cote serveur"))
    }
}

// ============================================================
//  FALLBACK SMTP
// ============================================================

#[cfg(feature = "server")]
async fn send_via_gmail_smtp(to: &str, subject: &str, body: &str) -> Result<(), String> {
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{Message, SmtpTransport, Transport};

    let user = std::env::var("GMAIL_USER").map_err(|_| "GMAIL_USER manquant".to_string())?;
    let pass = std::env::var("GMAIL_APP_PASSWORD").map_err(|_| "GMAIL_APP_PASSWORD manquant".to_string())?;

    let email = Message::builder()
        .from(user.parse().map_err(|e| format!("From invalide : {}", e))?)
        .to(to.parse().map_err(|e| format!("To invalide : {}", e))?)
        .subject(subject)
        .body(body.to_string())
        .map_err(|e| format!("Erreur construction email : {}", e))?;

    let creds = Credentials::new(user, pass);
    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .map_err(|e| format!("Erreur relay SMTP : {}", e))?
        .credentials(creds)
        .build();

    mailer.send(&email).map_err(|e| format!("Erreur envoi SMTP : {}", e))?;
    Ok(())
}