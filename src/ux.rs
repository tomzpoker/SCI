use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UxAnomalyItem { pub level:String, pub title:String, pub detail:String, pub source:String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataQualityItem { pub domain_code:String, pub label_fr:String, pub score:i32, pub detail:Vec<String> }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UxDashboardSnapshot {
    pub automatic_actions:i64, pub validations:i64, pub anomalies:Vec<UxAnomalyItem>, pub open_tasks:i64,
    pub pending_payments:i64, pub pending_documents:i64, pub overall_quality:i32, pub quality:Vec<DataQualityItem>
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UxPreferencesItem { pub theme_code:String, pub fun_mode:bool, pub explanation_level:i32, pub custom_theme:Value }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OnboardingStepItem { pub step_no:i32, pub code:String, pub label_fr:String, pub status:String, pub notes:String }

#[server]
pub async fn ux_dashboard_snapshot() -> Result<UxDashboardSnapshot, ServerFnError> {
    #[cfg(feature="server")]
    {
        use sqlx::Row;
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity=crate::entity_scope::current_legal_entity_id();
        let sci_id:Option<Uuid>=sqlx::query_scalar("SELECT id FROM scis WHERE legal_entity_id=$1 LIMIT 1").bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?;
        let automatic_actions:i64=sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE ($1::uuid IS NULL OR sci_id=$1) AND state IN ('READY','RUNNING')").bind(sci_id).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let validations:i64=sqlx::query_scalar("SELECT COUNT(*) FROM validation_requests WHERE legal_entity_id=$1 AND status='PENDING'").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let open_tasks:i64=sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE ($1::uuid IS NULL OR sci_id=$1) AND state NOT IN ('DONE','CANCELLED')").bind(sci_id).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let pending_payments:i64=sqlx::query_scalar("SELECT COUNT(*) FROM invoices WHERE legal_entity_id=$1 AND status IN ('ISSUED','PAID_PARTIAL','OVERDUE')").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let pending_documents:i64=sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE ($1::uuid IS NULL OR sci_id=$1) AND (COALESCE(ocr_status,'') NOT IN ('COMPLETED','SKIPPED') OR COALESCE(status,'') IN ('PENDING_REVIEW','BLOCKED'))").bind(sci_id).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let mut anomalies=Vec::new();
        let overdue:i64=sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE ($1::uuid IS NULL OR sci_id=$1) AND due_at < now() AND state NOT IN ('DONE','CANCELLED')").bind(sci_id).fetch_one(pool).await.map_err(ServerFnError::new)?;
        if overdue>0 { anomalies.push(UxAnomalyItem{level:"BLOCKING".into(),title:"Échéances dépassées".into(),detail:format!("{overdue} tâche(s) dépassée(s)."),source:"tasks".into()}); }
        let bank_anomalies:i64=sqlx::query_scalar("SELECT COUNT(*) FROM bank_transactions WHERE legal_entity_id=$1 AND COALESCE(reconciliation_status,'')='ANOMALY'").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new).unwrap_or(0);
        if bank_anomalies>0 { anomalies.push(UxAnomalyItem{level:"VERIFY".into(),title:"Anomalies bancaires".into(),detail:format!("{bank_anomalies} mouvement(s) Ã  analyser."),source:"bank_transactions".into()}); }
        let blocked_docs:i64=sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE ($1::uuid IS NULL OR sci_id=$1) AND COALESCE(status,'')='BLOCKED'").bind(sci_id).fetch_one(pool).await.map_err(ServerFnError::new).unwrap_or(0);
        if blocked_docs>0 { anomalies.push(UxAnomalyItem{level:"WARNING".into(),title:"Documents bloqués".into(),detail:format!("{blocked_docs} document(s) bloqué(s)."),source:"documents".into()}); }
        let quality=calculate_quality(pool,entity).await?;
        let overall_quality=if quality.is_empty(){0}else{quality.iter().map(|q|q.score).sum::<i32>()/(quality.len() as i32)};
        Ok(UxDashboardSnapshot{automatic_actions,validations,anomalies,open_tasks,pending_payments,pending_documents,overall_quality,quality})
    }
    #[cfg(not(feature="server"))] { Err(ServerFnError::new("ux_dashboard_snapshot est exécutée côté serveur")) }
}

#[cfg(feature="server")]
async fn calculate_quality(pool:&sqlx::PgPool, entity:Uuid)->Result<Vec<DataQualityItem>,ServerFnError>{
    let mut out=Vec::new();
    let sci_contacts:i64=sqlx::query_scalar("SELECT COUNT(*) FROM legal_entities WHERE id=$1 AND btrim(legal_name)<>''").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
    out.push(DataQualityItem{domain_code:"SCI".into(),label_fr:"SCI".into(),score:if sci_contacts>0{100}else{0},detail:vec![if sci_contacts>0{"Identité juridique renseignée.".into()}else{"Identité juridique manquante.".into()}]});
    let baux_total:i64=sqlx::query_scalar("SELECT COUNT(*) FROM leases WHERE legal_entity_id=$1 AND active=true").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
    let baux_ok:i64=sqlx::query_scalar("SELECT COUNT(*) FROM leases l WHERE l.legal_entity_id=$1 AND l.active=true AND l.unit_id IS NOT NULL AND l.tenant_id IS NOT NULL").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
    out.push(DataQualityItem{domain_code:"BAUX".into(),label_fr:"Baux".into(),score:if baux_total==0{100}else{((baux_ok*100)/baux_total) as i32},detail:vec![format!("{baux_ok}/{baux_total} bail(s) actif(s) relié(s) à un lot et un locataire.")]});
    let tenants:i64=sqlx::query_scalar("SELECT COUNT(*) FROM tenants t JOIN scis s ON s.id=t.sci_id WHERE s.legal_entity_id=$1").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
    let tenants_ok:i64=sqlx::query_scalar("SELECT COUNT(*) FROM tenants t JOIN scis s ON s.id=t.sci_id WHERE s.legal_entity_id=$1 AND (COALESCE(email,'')<>'' OR COALESCE(phone,'')<>'')").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new).unwrap_or(0);
    out.push(DataQualityItem{domain_code:"LOCATAIRES".into(),label_fr:"Locataires".into(),score:if tenants==0{100}else{((tenants_ok*100)/tenants) as i32},detail:vec![format!("{tenants_ok}/{tenants} locataire(s) avec contact.")]});
    let accounts:i64=sqlx::query_scalar("SELECT COUNT(*) FROM legal_entity_bank_accounts WHERE legal_entity_id=$1 AND active=true").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new).unwrap_or(0);
    let bank_tx:i64=sqlx::query_scalar("SELECT COUNT(*) FROM bank_transactions WHERE legal_entity_id=$1").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new).unwrap_or(0);
    let bank_bad:i64=sqlx::query_scalar("SELECT COUNT(*) FROM bank_transactions WHERE legal_entity_id=$1 AND reconciliation_status IN ('ANOMALY','UNMATCHED')").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new).unwrap_or(0);
    let bank_score=if accounts==0{0}else{100-(((bank_bad*100)/(bank_tx.max(1))) as i32).min(100)};
    out.push(DataQualityItem{domain_code:"BANQUE".into(),label_fr:"Banque".into(),score:bank_score,detail:vec![format!("{accounts} compte(s) actif(s), {bank_bad} mouvement(s) non rapproché(s)/anomalie sur {bank_tx}.")]});
    let regime_ok:i64=sqlx::query_scalar("SELECT COUNT(*) FROM legal_entities WHERE id=$1 AND COALESCE(tax_regime,'')<>''").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new).unwrap_or(0);
    out.push(DataQualityItem{domain_code:"FISCALITE".into(),label_fr:"Fiscalité".into(),score:if regime_ok>0{100}else{40},detail:vec![if regime_ok>0{"Régime fiscal renseigné.".into()}else{"Régime fiscal à qualifier.".into()}]});
    let docs:i64=sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE ($1::uuid IS NULL OR sci_id=$1)").bind(sqlx::query_scalar::<_,Option<Uuid>>("SELECT id FROM scis WHERE legal_entity_id=$1 LIMIT 1").bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?).fetch_one(pool).await.map_err(ServerFnError::new).unwrap_or(0);
    let doc_bad:i64=sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE ($1::uuid IS NULL OR sci_id=$1) AND COALESCE(status,'') IN ('BLOCKED','PENDING_REVIEW')").bind(sqlx::query_scalar::<_,Option<Uuid>>("SELECT id FROM scis WHERE legal_entity_id=$1 LIMIT 1").bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?).fetch_one(pool).await.map_err(ServerFnError::new).unwrap_or(0);
    let doc_score=if docs==0{100}else{100-(((doc_bad*100)/docs) as i32).min(100)};
    out.push(DataQualityItem{domain_code:"DOCUMENTS".into(),label_fr:"Documents".into(),score:doc_score,detail:vec![format!("{doc_bad}/{docs} document(s) nécessitent une intervention.")]});
    Ok(out)
}

#[server]
pub async fn get_ux_preferences()->Result<UxPreferencesItem,ServerFnError>{
    #[cfg(feature="server")] { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=crate::entity_scope::current_legal_entity_id(); if let Some(row)=sqlx::query("SELECT theme_code,fun_mode,explanation_level::integer AS explanation_level,custom_theme FROM ux_preferences WHERE legal_entity_id=$1").bind(e).fetch_optional(pool).await.map_err(ServerFnError::new)? { use sqlx::Row; return Ok(UxPreferencesItem{theme_code:row.get("theme_code"),fun_mode:row.get("fun_mode"),explanation_level:row.get("explanation_level"),custom_theme:row.get("custom_theme")}); } return Ok(UxPreferencesItem{theme_code:"NORMAL".into(),fun_mode:false,explanation_level:1,custom_theme:Value::Object(Default::default())}); }
    #[cfg(not(feature="server"))] { Err(ServerFnError::new("get_ux_preferences est exécutée côté serveur")) }
}

#[server]
pub async fn set_ux_preferences(theme_code:String,fun_mode:bool,explanation_level:i32,custom_theme:Value)->Result<(),ServerFnError>{
    #[cfg(feature="server")] { let theme=theme_code.trim().to_uppercase(); if !matches!(theme.as_str(),"NORMAL"|"FUN"|"DARK"|"LIGHT"|"HIGH_CONTRAST"|"CUSTOM"){return Err(ServerFnError::new("Thème invalide"));} let level=explanation_level.clamp(1,4); let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=crate::entity_scope::current_legal_entity_id(); sqlx::query("INSERT INTO ux_preferences(legal_entity_id,theme_code,fun_mode,explanation_level,custom_theme,updated_at) VALUES($1,$2,$3,$4,$5,now()) ON CONFLICT(legal_entity_id) DO UPDATE SET theme_code=EXCLUDED.theme_code,fun_mode=EXCLUDED.fun_mode,explanation_level=EXCLUDED.explanation_level,custom_theme=EXCLUDED.custom_theme,updated_at=now()").bind(e).bind(theme).bind(fun_mode).bind(level).bind(custom_theme).execute(pool).await.map_err(ServerFnError::new)?; Ok(()) }
    #[cfg(not(feature="server"))] { let _=(theme_code,fun_mode,explanation_level,custom_theme); Err(ServerFnError::new("set_ux_preferences est exécutée côté serveur")) }
}

#[server]
pub async fn list_onboarding_steps()->Result<Vec<OnboardingStepItem>,ServerFnError>{
    #[cfg(feature="server")] { use sqlx::Row; let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=crate::entity_scope::current_legal_entity_id(); let rows=sqlx::query("SELECT step_no::integer AS step_no,code,label_fr,status,notes FROM onboarding_steps WHERE legal_entity_id=$1 ORDER BY step_no").bind(e).fetch_all(pool).await.map_err(ServerFnError::new)?; return Ok(rows.into_iter().map(|r|OnboardingStepItem{step_no:r.get("step_no"),code:r.get("code"),label_fr:r.get("label_fr"),status:r.get("status"),notes:r.get("notes")}).collect()); }
    #[cfg(not(feature="server"))] { Err(ServerFnError::new("list_onboarding_steps est exécutée côté serveur")) }
}

#[server]
pub async fn set_onboarding_step(step_no:i32,status:String,notes:String)->Result<(),ServerFnError>{
    #[cfg(feature="server")] { if !(1..=14).contains(&step_no){return Err(ServerFnError::new("Étape invalide"));} let state=status.trim().to_uppercase(); if !matches!(state.as_str(),"TODO"|"IN_PROGRESS"|"DONE"|"SKIPPED"){return Err(ServerFnError::new("Statut invalide"));} let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=crate::entity_scope::current_legal_entity_id(); sqlx::query("UPDATE onboarding_steps SET status=$3,notes=$4,updated_at=now() WHERE legal_entity_id=$1 AND step_no=$2").bind(e).bind(step_no).bind(state).bind(notes).execute(pool).await.map_err(ServerFnError::new)?; Ok(()) }
    #[cfg(not(feature="server"))] { let _=(step_no,status,notes); Err(ServerFnError::new("set_onboarding_step est exécutée côté serveur")) }
}

#[component]
pub fn ZeroSaisiePage(mut refresh: Signal<u64>) -> Element {
    let snapshot = use_resource(move || {
        let _ = refresh();
        async move { ux_dashboard_snapshot().await.ok() }
    });
    let prefs = use_resource(move || {
        let _ = refresh();
        async move { get_ux_preferences().await.ok() }
    });
    let onboarding = use_resource(move || {
        let _ = refresh();
        async move { list_onboarding_steps().await.unwrap_or_default() }
    });

    let snapshot_value = snapshot.read().as_ref().and_then(|value| value.as_ref()).cloned();
    let prefs_value = prefs.read().as_ref().and_then(|value| value.as_ref()).cloned();

    rsx! {
        div {
            class: "page-stack",
            div {
                class: "zero-hero",
                div { class: "zero-mascot", "✨" }
                div {
                    h2 { "Tout est prêt" }
                    p { "Le système prépare, contrôle et signale les actions utiles sans te demander de ressaisir les informations déjà connues." }
                }
            }
            if let Some(s) = snapshot_value {
                section {
                    class: "zero-summary",
                    div {
                        div { class: "eyebrow", "OÙ EN EST MA SCI ?" }
                        h3 {
                            if s.validations > 0 {
                                "Il me manque simplement ta validation."
                            } else if !s.anomalies.is_empty() {
                                "Quelques points demandent ton attention."
                            } else {
                                "Tout est prêt."
                            }
                        }
p { "{s.open_tasks} tâche(s) ouverte(s) · {s.pending_payments} paiement(s) en attente · {s.pending_documents} document(s) à traiter." }                    }
                    div {
                        class: "zero-summary-metrics",
                        span { strong { "{s.validations}" }, " validations" }
                        span { strong { "{s.anomalies.len()}" }, " anomalies" }
                        span { strong { "{s.overall_quality}/100" }, " qualité" }
                    }
                }
                section {
                    class: "panel",
                    h3 { "Centre d'anomalies" }
                    if s.anomalies.is_empty() {
                        p { "Tout va bien." }
                    } else {
                        for a in s.anomalies {
                            div {
                                class: "anomaly-row",
                                div { strong { "{a.level}" } }
                                div {
                                    strong { "{a.title}" }
                                    p { "{a.detail}" }
                                    span { class: "small", "Source : {a.source}" }
                                }
                            }
                        }
                    }
                }
                section {
                    class: "panel",
                    h3 { "Qualité des données" }
                    for q in s.quality {
                        div {
                            class: "quality-row",
                            div {
                                strong { "{q.label_fr}" }
                                div { class: "progress-track", div { class: "progress-fill", style: "width:{q.score}%" } }
                            }
                            strong { "{q.score}/100" }
                            div {
                                class: "small",
                                for d in q.detail { div { "{d}" } }
                            }
                        }
                    }
                }
            }
            section {
                class: "panel",
                h3 { "Explications" }
                p { "Niveau 1 : simple · niveau 2 : détail · niveau 3 : technique · niveau 4 : fiscal/juridique." }
                if let Some(p) = prefs_value {
                    p { "Niveau actuel : {p.explanation_level} · Thème : {p.theme_code}" }
                }
            }
            section {
                class: "panel",
                h3 { "Interface" }
                div {
                    class: "theme-grid",
                    button {
                        class: "secondary theme-button",
                        onclick: move |_| async move {
                            let _ = set_ux_preferences("NORMAL".into(), false, 1, Value::Object(Default::default())).await;
                            refresh += 1;
                        },
                        "Normal"
                    }
                    button {
                        class: "secondary theme-button",
                        onclick: move |_| async move {
                            let _ = set_ux_preferences("FUN".into(), true, 1, Value::Object(Default::default())).await;
                            refresh += 1;
                        },
                        "Fun"
                    }
                }
            }
            section {
                class: "panel",
                h3 { "Onboarding" }
                div {
                    class: "data-list",
                    for step in onboarding.read().as_deref().unwrap_or(&[]).iter() {
                        div {
                            class: "data-row",
                            div {
                                div { class: "data-title", "{step.step_no}. {step.label_fr}" }
                                div { class: "small", "{step.status} â€¢ {step.notes}" }
                            }
                        }
                    }
                }
            }
        }
    }
}



