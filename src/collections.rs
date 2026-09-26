use chrono::{Duration, NaiveDate, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::entity_scope::current_legal_entity_id;
use crate::ui::{FormField, ModuleHeader};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArrearsCaseItem {
    pub id: Uuid,
    pub lease_id: Uuid,
    pub tenant_id: Uuid,
    pub invoice_id: Option<Uuid>,
    pub due_date: NaiveDate,
    pub expected_cents: i64,
    pub paid_cents: i64,
    pub outstanding_cents: i64,
    pub delay_days: i32,
    pub detection_type: String,
    pub reason: String,
    pub recurrence_count: i32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentPromiseItem {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub lease_id: Uuid,
    pub promised_date: NaiveDate,
    pub promised_amount_cents: i64,
    pub state: String,
    pub checked_paid_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectionActionItem {
    pub id: Uuid,
    pub arrears_case_id: Uuid,
    pub action_level: String,
    pub status: String,
    pub approval_required: bool,
    pub approved_by: Option<String>,
    pub approved_at: Option<chrono::DateTime<Utc>>,
    pub executed_at: Option<chrono::DateTime<Utc>>,
}

fn euro(cents:i64)->String{format!("{}.{:02} €",cents/100,cents.abs()%100)}
fn action_fr(code:&str)->&'static str{match code{"AMIABLE_REMINDER"=>"Rappel amiable","REMINDER"=>"Relance","FORMAL_NOTICE_PREPARATION"=>"Préparation mise en demeure","CASE_PREPARATION"=>"Préparation dossier","COURT_HUISSIER_PREPARATION"=>"Préparation commissaire de justice","FOLLOW_UP"=>"Suivi",_=>"Action"}}
fn promise_fr(code:&str)->String{match code{"HELD"=>"Tenue".into(),"PARTIALLY_HELD"=>"Partiellement tenue".into(),"BROKEN"=>"Non tenue".into(),"PENDING"=>"En attente".into(),"CANCELLED"=>"Annulée".into(),_=>code.to_owned()}}

#[server]
pub async fn detect_arrears()->Result<usize,ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=current_legal_entity_id(); use sqlx::Row; let rows=sqlx::query(r#"
        SELECT i.id invoice_id,i.lease_id,l.tenant_id,i.due_date,i.gross_cents,
               COALESCE((SELECT SUM(p.amount_cents) FROM payments p WHERE p.legal_entity_id=i.legal_entity_id AND p.invoice_id=i.id),0)::bigint paid_cents
        FROM invoices i JOIN leases l ON l.id=i.lease_id AND l.legal_entity_id=i.legal_entity_id
        WHERE i.legal_entity_id=$1 AND i.status NOT IN ('DRAFT','CANCELLED','CREDITED') AND i.due_date < CURRENT_DATE
          AND i.gross_cents > COALESCE((SELECT SUM(p.amount_cents) FROM payments p WHERE p.legal_entity_id=i.legal_entity_id AND p.invoice_id=i.id),0)
        ORDER BY i.due_date"#).bind(e).fetch_all(pool).await.map_err(ServerFnError::new)?; let mut count=0usize;
        for r in rows { let invoice:Uuid=r.get("invoice_id"); let lease:Uuid=r.get("lease_id"); let tenant:Uuid=r.get("tenant_id"); let due:NaiveDate=r.get("due_date"); let expected:i64=r.get("gross_cents"); let paid:i64=r.get("paid_cents"); let outstanding=expected.saturating_sub(paid); let delay=(Utc::now().date_naive()-due).num_days().max(0) as i32; let recurrence:i64=sqlx::query_scalar("SELECT COUNT(DISTINCT invoice_id)::bigint FROM arrears_cases WHERE legal_entity_id=$1 AND tenant_id=$2 AND status IN ('OPEN','PROMISED','PARTIAL','RESOLVED','CLOSED') AND created_at >= now()-interval '365 days'").bind(e).bind(tenant).fetch_one(pool).await.map_err(ServerFnError::new)?;
            let types=if paid==0{vec!["ABSENCE","RETARD"]}else{vec!["RETARD","PARTIAL","UNDERPAYMENT"]}; for typ in types { let reason=match typ{"ABSENCE"=>"Aucun paiement enregistré après l'échéance.","RETARD"=>if paid==0{"Aucun paiement enregistré et échéance dépassée."}else{"Échéance dépassée avec solde restant dû."},"PARTIAL"=>"Paiement partiel constaté après échéance.","UNDERPAYMENT"=>"Paiement inférieur au montant contractuellement/facturé attendu.",_=>""}; let key=typ.to_string(); let id:Uuid=sqlx::query_scalar("INSERT INTO arrears_cases(legal_entity_id,lease_id,tenant_id,invoice_id,due_date,expected_cents,paid_cents,outstanding_cents,delay_days,detection_type,reason,recurrence_count,status) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,'OPEN') ON CONFLICT(legal_entity_id,invoice_id,detection_type) DO UPDATE SET paid_cents=EXCLUDED.paid_cents,outstanding_cents=EXCLUDED.outstanding_cents,delay_days=EXCLUDED.delay_days,recurrence_count=EXCLUDED.recurrence_count,reason=EXCLUDED.reason,last_checked_at=now(),updated_at=now() RETURNING id").bind(e).bind(lease).bind(tenant).bind(invoice).bind(due).bind(expected).bind(paid).bind(outstanding).bind(delay).bind(&key).bind(reason).bind(recurrence).fetch_one(pool).await.map_err(ServerFnError::new)?; sqlx::query("INSERT INTO arrears_events(legal_entity_id,arrears_case_id,event_type,amount_cents,payload) VALUES($1,$2,'DETECTED',$3,$4)").bind(e).bind(id).bind(outstanding).bind(json!({"invoice_id":invoice,"detection_type":typ,"delay_days":delay,"recurrence_count":recurrence})).execute(pool).await.map_err(ServerFnError::new)?; count+=1; }
            if recurrence>=2 { let id:Uuid=sqlx::query_scalar("INSERT INTO arrears_cases(legal_entity_id,lease_id,tenant_id,invoice_id,due_date,expected_cents,paid_cents,outstanding_cents,delay_days,detection_type,reason,recurrence_count,status) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,'RECURRENCE',$10,$11,'OPEN') ON CONFLICT(legal_entity_id,invoice_id,detection_type) DO UPDATE SET recurrence_count=EXCLUDED.recurrence_count,last_checked_at=now(),updated_at=now() RETURNING id").bind(e).bind(lease).bind(tenant).bind(invoice).bind(due).bind(expected).bind(paid).bind(outstanding).bind(delay).bind(format!("Récidive détectée : {} dossiers sur 12 mois.",recurrence+1)).bind(recurrence+1).fetch_one(pool).await.map_err(ServerFnError::new)?; sqlx::query("INSERT INTO arrears_events(legal_entity_id,arrears_case_id,event_type,amount_cents,payload) VALUES($1,$2,'RECURRENCE_DETECTED',$3,$4)").bind(e).bind(id).bind(outstanding).bind(json!({"recurrence_count":recurrence+1})).execute(pool).await.map_err(ServerFnError::new)?; count+=1; }
        } Ok(count) }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("detect_arrears est exécutée côté serveur"))
}

#[server]
pub async fn list_arrears_cases()->Result<Vec<ArrearsCaseItem>,ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; use sqlx::Row; let rows=sqlx::query("SELECT id,lease_id,tenant_id,invoice_id,due_date,expected_cents,paid_cents,outstanding_cents,delay_days,detection_type,reason,recurrence_count,status FROM arrears_cases WHERE legal_entity_id=$1 ORDER BY due_date DESC,id DESC").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; Ok(rows.into_iter().map(|r|ArrearsCaseItem{id:r.get("id"),lease_id:r.get("lease_id"),tenant_id:r.get("tenant_id"),invoice_id:r.get("invoice_id"),due_date:r.get("due_date"),expected_cents:r.get("expected_cents"),paid_cents:r.get("paid_cents"),outstanding_cents:r.get("outstanding_cents"),delay_days:r.get("delay_days"),detection_type:r.get("detection_type"),reason:r.get("reason"),recurrence_count:r.get("recurrence_count"),status:r.get("status")}).collect()) }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("list_arrears_cases est exécutée côté serveur"))
}

#[server]
pub async fn create_payment_promise(arrears_case_id:Option<Uuid>,tenant_id:Uuid,lease_id:Uuid,promised_date:NaiveDate,promised_amount_cents:i64,response_note:String)->Result<Uuid,ServerFnError>{
    #[cfg(feature="server")]
    { if promised_amount_cents<=0{return Err(ServerFnError::new("Montant de promesse invalide"));} let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=current_legal_entity_id(); let id:Uuid=sqlx::query_scalar("INSERT INTO payment_promises(legal_entity_id,arrears_case_id,tenant_id,lease_id,promised_date,promised_amount_cents,response_note) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING id").bind(e).bind(arrears_case_id).bind(tenant_id).bind(lease_id).bind(promised_date).bind(promised_amount_cents).bind(response_note.trim()).fetch_one(pool).await.map_err(ServerFnError::new)?; if let Some(case_id)=arrears_case_id{sqlx::query("UPDATE arrears_cases SET status='PROMISED',updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(case_id).bind(e).execute(pool).await.map_err(ServerFnError::new)?; sqlx::query("INSERT INTO arrears_events(legal_entity_id,arrears_case_id,event_type,amount_cents,payload) VALUES($1,$2,'PROMISE_CREATED',$3,$4)").bind(e).bind(case_id).bind(promised_amount_cents).bind(json!({"promise_id":id,"promised_date":promised_date})).execute(pool).await.map_err(ServerFnError::new)?;} Ok(id) }
    #[cfg(not(feature="server"))] { let _=(arrears_case_id,tenant_id,lease_id,promised_date,promised_amount_cents,response_note); Err(ServerFnError::new("create_payment_promise est exécutée côté serveur")) }
}

#[server]
pub async fn verify_payment_promises()->Result<usize,ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=current_legal_entity_id(); use sqlx::Row; let rows=sqlx::query("SELECT id,tenant_id,lease_id,promised_date,promised_amount_cents,state FROM payment_promises WHERE legal_entity_id=$1 AND state='PENDING' AND promised_date <= CURRENT_DATE ORDER BY promised_date").bind(e).fetch_all(pool).await.map_err(ServerFnError::new)?; let mut n=0; for r in rows{let id:Uuid=r.get("id");let tenant:Uuid=r.get("tenant_id");let lease:Uuid=r.get("lease_id");let date:NaiveDate=r.get("promised_date");let promised:i64=r.get("promised_amount_cents");let paid:i64=sqlx::query_scalar("SELECT COALESCE(SUM(amount_cents),0)::bigint FROM payments WHERE legal_entity_id=$1 AND invoice_id IN (SELECT invoice_id FROM arrears_cases WHERE legal_entity_id=$1 AND tenant_id=$2 AND lease_id=$3) AND received_at::date >= $4").bind(e).bind(tenant).bind(lease).bind(date).fetch_one(pool).await.map_err(ServerFnError::new)?;let state=if paid>=promised{"HELD"}else if paid>0{"PARTIALLY_HELD"}else{"BROKEN"};sqlx::query("UPDATE payment_promises SET state=$2,checked_at=now(),checked_paid_cents=$3,updated_at=now() WHERE id=$1 AND legal_entity_id=$4").bind(id).bind(state).bind(paid).bind(e).execute(pool).await.map_err(ServerFnError::new)?;n+=1;} Ok(n) }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("verify_payment_promises est exécutée côté serveur"))
}

#[server]
pub async fn prepare_collection_action(arrears_case_id:Uuid,action_level:String)->Result<Uuid,ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=current_legal_entity_id(); let level=action_level.trim().to_uppercase(); if !matches!(level.as_str(),"AMIABLE_REMINDER"|"REMINDER"|"FORMAL_NOTICE_PREPARATION"|"CASE_PREPARATION"|"COURT_HUISSIER_PREPARATION"|"FOLLOW_UP"){return Err(ServerFnError::new("Niveau de relance invalide"));} let payload:serde_json::Value=sqlx::query_scalar("SELECT jsonb_build_object('case_id',id,'invoice_id',invoice_id,'due_date',due_date,'outstanding_cents',outstanding_cents,'detection_type',detection_type,'reason',reason) FROM arrears_cases WHERE id=$1 AND legal_entity_id=$2").bind(arrears_case_id).bind(e).fetch_one(pool).await.map_err(ServerFnError::new)?; let id:Uuid=sqlx::query_scalar("INSERT INTO collection_actions(legal_entity_id,arrears_case_id,action_level,status,content_snapshot,approval_required) VALUES($1,$2,$3,'VALIDATION_REQUIRED',$4,true) ON CONFLICT(legal_entity_id,arrears_case_id,action_level) DO UPDATE SET content_snapshot=EXCLUDED.content_snapshot,status='VALIDATION_REQUIRED',updated_at=now() RETURNING id").bind(e).bind(arrears_case_id).bind(level).bind(payload).fetch_one(pool).await.map_err(ServerFnError::new)?;Ok(id)}
    #[cfg(not(feature="server"))] { let _=(arrears_case_id,action_level); Err(ServerFnError::new("prepare_collection_action est exécutée côté serveur")) }
}

#[server]
pub async fn approve_collection_action(id:Uuid,approve:bool,reason:String)->Result<(),ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=current_legal_entity_id(); let state=if approve{"VALIDATED"}else{"CANCELLED"};let by=if approve{"MANAGER"}else{"MANAGER"};let r=sqlx::query("UPDATE collection_actions SET status=$3,approved_by=CASE WHEN $4 THEN $5 ELSE approved_by END,approved_at=CASE WHEN $4 THEN now() ELSE approved_at END,notes=$6,updated_at=now() WHERE id=$1 AND legal_entity_id=$2 AND status IN ('VALIDATION_REQUIRED','PREPARED')").bind(id).bind(e).bind(state).bind(approve).bind(by).bind(reason.trim()).execute(pool).await.map_err(ServerFnError::new)?;if r.rows_affected()==0{return Err(ServerFnError::new("Action introuvable ou déjà traitée"));}Ok(())}
    #[cfg(not(feature="server"))] { let _=(id,approve,reason); Err(ServerFnError::new("approve_collection_action est exécutée côté serveur")) }
}

#[server]
pub async fn execute_collection_action(id:Uuid)->Result<(),ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=current_legal_entity_id(); let legal:bool=sqlx::query_scalar("SELECT action_level IN ('FORMAL_NOTICE_PREPARATION','CASE_PREPARATION','COURT_HUISSIER_PREPARATION') FROM collection_actions WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(e).fetch_one(pool).await.map_err(ServerFnError::new)?; let r=sqlx::query("UPDATE collection_actions SET status='EXECUTED',executed_at=now(),updated_at=now() WHERE id=$1 AND legal_entity_id=$2 AND status='VALIDATED' AND ($3=false OR (approved_by IS NOT NULL AND approved_at IS NOT NULL))").bind(id).bind(e).bind(legal).execute(pool).await.map_err(ServerFnError::new)?;if r.rows_affected()==0{if legal{return Err(ServerFnError::new("Validation humaine obligatoire pour une action juridique"));}return Err(ServerFnError::new("Action non validée"));} Ok(()) }
    #[cfg(not(feature="server"))] { let _=id; Err(ServerFnError::new("execute_collection_action est exécutée côté serveur")) }
}

#[server]
pub async fn list_collection_actions(case_id:Option<Uuid>)->Result<Vec<CollectionActionItem>,ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; use sqlx::Row; let rows=sqlx::query("SELECT id,arrears_case_id,action_level,status,approval_required,approved_by,approved_at,executed_at FROM collection_actions WHERE legal_entity_id=$1 AND ($2::uuid IS NULL OR arrears_case_id=$2) ORDER BY created_at DESC").bind(current_legal_entity_id()).bind(case_id).fetch_all(pool).await.map_err(ServerFnError::new)?; Ok(rows.into_iter().map(|r|CollectionActionItem{id:r.get("id"),arrears_case_id:r.get("arrears_case_id"),action_level:r.get("action_level"),status:r.get("status"),approval_required:r.get("approval_required"),approved_by:r.get("approved_by"),approved_at:r.get("approved_at"),executed_at:r.get("executed_at")}).collect()) }
    #[cfg(not(feature="server"))] { let _=case_id; Err(ServerFnError::new("list_collection_actions est exécutée côté serveur")) }
}

#[component]
pub fn RecoveryPage(mut refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let cases = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_arrears_cases().await.unwrap_or_default() } });
    let actions = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_collection_actions(None).await.unwrap_or_default() } });
    let mut msg = use_signal(String::new);
    let mut promise_case = use_signal(String::new);
    let mut promise_date = use_signal(|| (Utc::now().date_naive() + Duration::days(7)).to_string());
    let mut promise_amount = use_signal(String::new);

    rsx! {
        ModuleHeader { title: "Impayés & recouvrement", kicker: "DÉTECTION • PROMESSES • RELANCES • VALIDATION", detail: "Les actions juridiques restent préparées et bloquées tant qu’une validation humaine explicite n’a pas été donnée." }
        section {
            class: "panel",
            button {
                class: "primary",
                onclick: move |_| async move {
                    match detect_arrears().await { Ok(n) => { msg.set(format!("{} détection(s) actualisée(s)", n)); bump += 1; }, Err(e) => msg.set(e.to_string()) }
                },
                "Détecter les impayés"
            }
            button {
                class: "secondary",
                onclick: move |_| async move {
                    match verify_payment_promises().await { Ok(n) => { msg.set(format!("{} promesse(s) vérifiée(s)", n)); bump += 1; }, Err(e) => msg.set(e.to_string()) }
                },
                "Vérifier les promesses"
            }
            span { class: "save-ok", "{msg}" }
        }
        section {
            class: "two-col",
            div {
                class: "panel",
                h3 { "Promesse de paiement" }
                label {
                    class: "field",
                    span { "Dossier" }
                    select {
                        value: promise_case(), onchange: move |e: FormEvent| promise_case.set(e.value()),
                        option { value: "", "Sélectionner" }
                        for c in cases.read().as_deref().unwrap_or(&[]).iter() {
                            option { value: c.id.to_string(), "{c.due_date} • reste {euro(c.outstanding_cents)} • {c.detection_type}" }
                        }
                    }
                }
                FormField { label: "Date promise", value: promise_date(), oninput: move |e: FormEvent| promise_date.set(e.value()) }
                FormField { label: "Montant €", value: promise_amount(), oninput: move |e: FormEvent| promise_amount.set(e.value()) }
                button {
                    class: "secondary",
                    onclick: move |_| async move {
                        let case_id = Uuid::parse_str(&promise_case());
                        let date = NaiveDate::parse_from_str(&promise_date(), "%Y-%m-%d");
                        let amount = promise_amount().replace(',', ".").parse::<f64>().ok().map(|x| (x * 100.0).round() as i64);
                        match (case_id, date, amount) {
                            (Ok(cid), Ok(date), Some(amount)) => {
                                let current = cases.read().as_deref().and_then(|v| v.iter().find(|x| x.id == cid)).cloned();
                                match current {
                                    Some(c) => match create_payment_promise(Some(cid), c.tenant_id, c.lease_id, date, amount, "Saisie manager".into()).await {
                                        Ok(_) => { msg.set("Promesse enregistrée".into()); bump += 1; }
                                        Err(e) => msg.set(e.to_string()),
                                    },
                                    None => msg.set("Dossier introuvable".into()),
                                }
                            }
                            _ => msg.set("Données de promesse invalides".into()),
                        }
                    },
                    "Enregistrer la promesse"
                }
            }
            div {
                class: "panel",
                h3 { "Dossiers détectés" }
                for c in cases.read().as_deref().unwrap_or(&[]).iter() {
                    div {
                        class: "data-row",
                        div {
                            div { class: "data-title", "{c.detection_type} • {c.status} • {c.due_date}" }
                            div { class: "small", "Attendu {euro(c.expected_cents)} • payé {euro(c.paid_cents)} • reste {euro(c.outstanding_cents)} • {c.delay_days} jour(s)" }
                            div { class: "small", "{c.reason}" }
                        }
                        div {
                            class: "row-actions",
                            button { class: "secondary", onclick: { let id = c.id; move |_| async move { match prepare_collection_action(id, "AMIABLE_REMINDER".into()).await { Ok(_) => bump += 1, Err(e) => msg.set(e.to_string()) } } }, "Préparer rappel" }
                            button { class: "secondary", onclick: { let id = c.id; move |_| async move { match prepare_collection_action(id, "FORMAL_NOTICE_PREPARATION".into()).await { Ok(_) => bump += 1, Err(e) => msg.set(e.to_string()) } } }, "Préparer mise en demeure" }
                        }
                    }
                }
            }
        }
        section {
            class: "panel",
            h3 { "Actions de recouvrement" }
            for a in actions.read().as_deref().unwrap_or(&[]).iter() {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{action_fr(&a.action_level)}" }
                        div { class: "small", "{a.status} • validation requise : {a.approval_required}" }
                    }
                    div {
                        class: "row-actions",
                        if a.status == "VALIDATION_REQUIRED" {
                            button { class: "secondary", onclick: { let id = a.id; move |_| async move { let _ = approve_collection_action(id, true, "Validation manager".into()).await; bump += 1 } }, "Valider" }
                        }
                        if a.status == "VALIDATED" {
                            button { class: "secondary", onclick: { let id = a.id; move |_| async move { let _ = execute_collection_action(id).await; bump += 1 } }, "Exécuter" }
                        }
                    }
                }
            }
        }
    }
}


#[cfg(test)]
mod tests { use super::*; #[test] fn action_labels(){assert_eq!(action_fr("FORMAL_NOTICE_PREPARATION"),"Préparation mise en demeure");assert_eq!(promise_fr("HELD"),"Tenue");} }
