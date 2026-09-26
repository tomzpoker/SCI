use crate::domain::*;
use crate::engines::{list_common_engines, CommonEnginesPage};
use crate::history::list_change_history;
use crate::entity_scope::{set_active_legal_entity, LEGACY_SCI_LEGAL_ENTITY_ID};
use crate::legal_entities::*;
use crate::sarl_activities::*;
use crate::rules::RulesPage;
use crate::documents::DocumentsWorkflowPage;
use crate::leases::LeasesPage;
use crate::billing::BillingManagementPage;
use crate::banking::BankManagementPage;
use crate::treasury::TreasuryPage;
use crate::fiscal::FiscalPage;
use crate::collections::RecoveryPage;
use crate::generation::GenerationPage;
use crate::einvoice::EInvoicePage;
use crate::assistant::control::AssistantPage;
use crate::ux::ZeroSaisiePage;
use crate::security::{auth_status,LoginPage,SecurityPage};
use crate::workflow::{WorkflowPage, ValidationInboxPage};
use crate::server::*;
use chrono::{Duration, NaiveDate, Utc};
use dioxus::prelude::*;
mod dashboard;
use std::collections::BTreeMap;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use uuid::Uuid;

const CSS: Asset = asset!("/assets/main.css");

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Dashboard,
    Setup,
    LegalEntities,
    SarlActivities,
    Rules,
    Engines,
    Workflows,
    ValidationInbox,
    Associates,
    Tenants,
    Patrimony,
    Rentals,
    Billing,
    EInvoice,
    Assistant,
    Bank,
    Treasury,
    Vat,
    Recovery,
    Generation,
    Calendar,
    Documents,
    Automations,
    Tasks,
    Audit,
    ZeroSaisie,
    Security,
}
impl Page {
    fn label(self) -> &'static str {
        match self {
            Page::Dashboard => "Accueil",
            Page::Setup => "Configuration",
            Page::LegalEntities => "Entités",
            Page::SarlActivities => "Activités SARL",
            Page::Rules => "Référentiels & règles",
            Page::Engines => "Moteurs communs",
            Page::Workflows => "Workflows",
            Page::ValidationInbox => "Validations",
            Page::Associates => "Associés",
            Page::Tenants => "Locataires",
            Page::Patrimony => "Biens",
            Page::Rentals => "Locations",
            Page::Billing => "Facturation",
            Page::EInvoice => "E-facturation",
            Page::Assistant => "Assistant IA",
            Page::Bank => "Argent",
            Page::Treasury => "Trésorerie",
            Page::Vat => "Fiscalité",
            Page::Recovery => "Impayés & recouvrement",
            Page::Generation => "Courriers / PDF",
            Page::Calendar => "Échéances",
            Page::Documents => "Documents",
            Page::Automations => "Automatisations",
            Page::Tasks => "Tâches",
            Page::Audit => "Audit",
            Page::ZeroSaisie => "À vérifier",
            Page::Security => "Sécurité / Recovery",
        }
    }
}

#[component]
pub fn App() -> Element {
    let mut auth_epoch=use_signal(||0u64);
    let auth=use_resource(move||{let _=auth_epoch();async move{auth_status().await}});
    rsx!{document::Link{rel:"stylesheet",href:CSS},document::Link{rel:"stylesheet",href:asset!("/assets/dashboard.css")},match auth.read().as_ref(){
        Some(Ok(status)) if status.authenticated=>rsx!{AuthenticatedShell{auth_status:status.clone(),on_logout:move |_|auth_epoch+=1}},
        Some(Ok(_))=>rsx!{LoginPage{on_success:move |_|auth_epoch+=1}},
        Some(Err(e))=>rsx!{div {class:"login-shell",div {class:"login-card",h1 {"SCI FAMILY"},p {"Initialisation sécurité impossible : {e}"}}}},
        None=>rsx!{div {class:"login-shell",div {class:"login-card",h1 {"SCI FAMILY"},p {"Initialisation sécurisée…"}}}},
    }}
}

#[component]
fn AuthenticatedShell(auth_status:crate::security::AuthStatusItem,on_logout:EventHandler<()>) -> Element {
    let mut page=use_signal(||if auth_status.must_change_password{Page::Security}else{Page::Dashboard});
    let mut refresh=use_signal(||0u64);
    let prefs=use_resource(move||{let _=refresh();async move{crate::ux::get_ux_preferences().await.ok()}});
    let pref=prefs.read().as_ref().and_then(|v|v.as_ref()).cloned();
    let theme=pref.as_ref().map(|p|p.theme_code.as_str()).unwrap_or("NORMAL");
    let fun=pref.as_ref().map(|p|p.fun_mode).unwrap_or(false);
    let shell_class=if fun{format!("app-shell theme-{} fun-mode",theme.to_lowercase())}else{format!("app-shell theme-{}",theme.to_lowercase())};
    let custom_style=pref.as_ref().and_then(|p|{let a=p.custom_theme.get("accent").and_then(|v|v.as_str()).unwrap_or("");let b=p.custom_theme.get("background").and_then(|v|v.as_str()).unwrap_or("");if a.is_empty()&&b.is_empty(){None}else{Some(format!("--custom-accent:{};--custom-background:{};",a,b))}}).unwrap_or_default();
    let role_label=auth_status.role.clone().unwrap_or_else(||"Rôle inconnu".into());
    rsx!{div {class:"{shell_class}",style:"{custom_style}",aside{class:"sidebar",div {class:"brand","SCI FAMILY"},div {class:"brand-sub","PILOTAGE ADMINISTRATIF AUTONOME"},div {class:"session-chip",div {class:"small","Session"},strong {"{auth_status.display_name.clone().unwrap_or_default()}"},span {class:"small","{role_label}"}},nav {
        NavItem{page,current:Page::Dashboard}
        NavItem{page,current:Page::ZeroSaisie}
        NavItem{page,current:Page::Bank}
        NavItem{page,current:Page::Patrimony}
        NavItem{page,current:Page::Tenants}
        NavItem{page,current:Page::Documents}
        NavItem{page,current:Page::Calendar}

        div {
            class: "nav-separator"
        }

        NavItem{page,current:Page::Setup}

        details {
            class: "expert-nav",

            summary {
                "🛠️ Outils avancés"
            }

            div {
                class: "expert-nav-list",

                NavItem{page,current:Page::Treasury}
                NavItem{page,current:Page::Associates}
                NavItem{page,current:Page::Rentals}
                NavItem{page,current:Page::Billing}
                NavItem{page,current:Page::EInvoice}
                NavItem{page,current:Page::Assistant}
                NavItem{page,current:Page::Vat}
                NavItem{page,current:Page::Recovery}
                NavItem{page,current:Page::Generation}
                NavItem{page,current:Page::Automations}
                NavItem{page,current:Page::Tasks}
                NavItem{page,current:Page::ValidationInbox}
                NavItem{page,current:Page::Workflows}
                NavItem{page,current:Page::Rules}
                NavItem{page,current:Page::Engines}
                NavItem{page,current:Page::LegalEntities}
                NavItem{page,current:Page::SarlActivities}
                NavItem{page,current:Page::Audit}
                NavItem{page,current:Page::Security}
            }
        }
    },div {class:"sidebar-footer","Données locales • règles versionnées • audit"},button {class:"secondary logout-button",onclick:move |_|{let cb=on_logout.clone();async move{let _=crate::security::logout().await;dioxus::fullstack::clear_request_headers();cb.call(())}},"Se déconnecter"}},main {class:"main",header {class:"topbar",div {div {class:"eyebrow","SCI FAMILY PILOT • {role_label}"},h1 {"{page().label()}" }},div {class:"top-actions",EntityScopeSelector{refresh,page,initial_entity_id:auth_status.legal_entity_id},button {class:"secondary",onclick:move |_|page.set(Page::Setup),"Configuration"},button {class:"primary",onclick:move |_|async move{let _=run_anticipation_cycle().await;refresh+=1},"Lancer l’anticipation"}}},match page(){
        Page::Dashboard=>rsx!{dashboard::DashboardWidgets{}},
        Page::ZeroSaisie=>rsx!{ZeroSaisiePage{refresh}}, Page::Security=>rsx!{SecurityPage{refresh,on_logged_out:move |_|on_logout.call(())}},
        Page::Setup=>rsx!{SetupPage{refresh}}, Page::LegalEntities=>rsx!{LegalEntitiesPage{refresh}}, Page::SarlActivities=>rsx!{SarlActivitiesPage{refresh}}, Page::Rules=>rsx!{RulesPage{refresh}}, Page::Engines=>rsx!{CommonEnginesPage{refresh}}, Page::Associates=>rsx!{AssociatesPage{refresh}},
        Page::Workflows=>rsx!{WorkflowPage{refresh}}, Page::ValidationInbox=>rsx!{ValidationInboxPage{refresh}}, Page::Tenants=>rsx!{TenantsPage{refresh}}, Page::Patrimony=>rsx!{PatrimonyPage{refresh}},
        Page::Rentals=>rsx!{LeasesPage{refresh}}, Page::Billing=>rsx!{BillingManagementPage{refresh}}, Page::EInvoice=>rsx!{EInvoicePage{refresh}}, Page::Assistant=>rsx!{AssistantPage{refresh}}, Page::Bank=>rsx!{BankManagementPage{refresh}}, Page::Treasury=>rsx!{TreasuryPage{refresh}},
        Page::Vat=>rsx!{FiscalPage{refresh}}, Page::Recovery=>rsx!{RecoveryPage{refresh}}, Page::Generation=>rsx!{GenerationPage{refresh}}, Page::Calendar=>rsx!{CalendarPage{refresh}}, Page::Documents=>rsx!{DocumentsWorkflowPage{refresh}}, Page::Automations=>rsx!{AutomationsPage{refresh}}, Page::Tasks=>rsx!{TasksPage{refresh}}, Page::Audit=>rsx!{AuditPage{refresh}},
    }}}}}


#[component]
fn EntityScopeSelector(mut refresh: Signal<u64>, mut page: Signal<Page>, initial_entity_id: Option<Uuid>) -> Element {
    let entities = use_resource(move || {
        let _ = refresh();
        async move { list_legal_entities().await.unwrap_or_default() }
    });
    let mut current = use_signal(move || initial_entity_id.unwrap_or(LEGACY_SCI_LEGAL_ENTITY_ID));
    let current_label = entities
        .read()
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .find(|e| e.id == current())
        .map(|e| format!("{} · {}", e.legal_name, e.legal_form_code))
        .unwrap_or_else(|| "Entité active".to_string());
    rsx! {
        label { class:"scope-selector",
            span { class:"small", "Périmètre" }
            select {
                value: current().to_string(),
                title: current_label.clone(),
                onchange: move |event: FormEvent| {
                    let value = event.value();
                    async move {
                        if let Ok(id) = Uuid::parse_str(&value) {
                            match set_active_legal_entity(id).await {
                                Ok(_) => {
                                    current.set(id);
                                    page.set(Page::Dashboard);
                                    refresh += 1;
                                }
                                Err(_) => {}
                            }
                        }
                    }
                },
                for entity in entities.read().as_deref().unwrap_or(&[]).iter().filter(|e| e.active) {
                    option {
                        value: entity.id.to_string(),
                        {format!("{} · {} / {}", entity.legal_name, entity.legal_form_code, entity.tax_regime)}
                    }
                }
            }
        }
    }
}

#[component]
fn NavItem(mut page: Signal<Page>, current: Page) -> Element {
    rsx! {button {class:if page()==current{"nav-item active"}else{"nav-item"},onclick:move |_|page.set(current),{current.label()}}}
}

#[component]
fn Dashboard(refresh: Signal<u64>, on_setup: EventHandler<MouseEvent>) -> Element {
    let data = use_resource(move || {
        let _ = refresh();
        async move {
            dashboard_snapshot().await.ok()
        }
    });

    let counts = use_resource(move || {
        let _ = refresh();
        async move {
            module_counts().await.ok()
        }
    });

    let status = use_resource(move || {
        let _ = refresh();
        async move {
            onboarding_status().await.ok()
        }
    });

    let real_history = use_resource(move || {
        let _ = refresh();
        async move {
            dashboard_real_cash_history(12).await.ok()
        }
    });

    let treasury = use_resource(move || {
        let _ = refresh();
        async move {
            crate::treasury::build_treasury_forecast(
                12,
                "BASE".to_string()
            )
            .await
            .ok()
        }
    });

    match (
        &*data.read(),
        &*counts.read(),
        &*status.read(),
        &*real_history.read(),
        &*treasury.read()
    ) {
        (
            Some(Some(d)),
            Some(Some(c)),
            Some(Some(s)),
            Some(Some(real)),
            Some(Some(forecast))
        ) => {
            let forecast_3m = forecast
                .points
                .get(2)
                .map(|p| p.balance_cents)
                .unwrap_or(forecast.ending_balance_cents);

            let forecast_min = forecast.minimum_balance_cents;

            rsx! {
                if !s.completed {
                    section {
                        class: "setup-banner",

                        div {
                            div {
                                class: "eyebrow",
                                "MISE EN ROUTE • {s.completion_pct}%"
                            }

                            h2 {
                                "Votre SCI passe du déclaratif au pilotage"
                            }

                            p {
                                "Configurez les référentiels une fois ; les baux, factures, encaissements, TVA, banque et échéances seront ensuite reliés."
                            }
                        }

                        button {
                            class: "primary",
                            onclick: on_setup,
                            "Continuer"
                        }
                    }
                }

                section {
                    class: "home-greeting",

                    div {
                        span {
                            class: "pill",
                            "SCI À L’IR"
                        }

                        span {
                            class: "pill muted",
                            "TVA sur encaissements"
                        }

                        h2 {
                            "{d.sci_name}"
                        }

                        p {
                            "Voici ce qui mérite ton attention aujourd’hui."
                        }
                    }

                    div {
                        class: "home-risk",

                        div {
                            class: "eyebrow",
                            "VIGILANCE"
                        }

                        div {
                            class: format!("risk risk-{}", d.risk_level.to_lowercase()),
                            "{d.risk_level}"
                        }

                        div {
                            class: "small",
                            "{d.overdue_tasks} tâche(s) en retard"
                        }
                    }
                }

                ZeroSaisieSummary {
                    refresh
                }

                section {
                    class: "metric-row home-metrics",

                    Metric {
                        label: "Argent réel",
                        value: euro(d.cash_cents),
                        tone: "positive"
                    }

                    Metric {
                        label: "Prévu à 3 mois",
                        value: euro(forecast_3m),
                        tone: "neutral"
                    }

                    Metric {
                        label: "Point bas prévu",
                        value: euro(forecast_min),
                        tone: "warning"
                    }

                    Metric {
                        label: "À traiter < 30 j",
                        value: d.tasks_due_30d.to_string(),
                        tone: "neutral"
                    }
                }

                section {
                    class: "module-grid home-modules",

                    ModuleCard {
                        title: "Argent",
                        value: euro(d.cash_cents),
                        label: "solde réel",
                        detail: format!(
                            "Prévu à 3 mois : {}",
                            euro(forecast_3m)
                        )
                    }

                    ModuleCard {
                        title: "Biens",
                        value: c.properties.to_string(),
                        label: "propriétés",
                        detail: format!(
                            "{} lots",
                            c.units
                        )
                    }

                    ModuleCard {
                        title: "Locataires",
                        value: c.tenants.to_string(),
                        label: "actifs",
                        detail: format!(
                            "{} baux",
                            c.leases
                        )
                    }

                    ModuleCard {
                        title: "Documents",
                        value: c.documents.to_string(),
                        label: "pièces",
                        detail: "Import, lecture et validation"
                    }

                    ModuleCard {
                        title: "Échéances",
                        value: c.tax_deadlines.to_string(),
                        label: "à venir",
                        detail: format!(
                            "{} tâche(s) ouvertes",
                            c.open_tasks
                        )
                    }

                    ModuleCard {
                        title: "TVA",
                        value: euro(c.vat_receipts_cents),
                        label: "encaissé ce mois",
                        detail: "Calculée depuis les encaissements"
                    }
                }

                TreasuryDualCurveChart {
                    real_history: real.clone(),
                    forecast: forecast.points.clone()
                }

                section {
                    class: "two-col home-bottom",

                    div {
                        class: "panel",

                        div {
                            class: "panel-head",

                            h3 {
                                "🔔 J’ai besoin de toi"
                            }

                            span {
                                class: "small",
                                "Priorisé"
                            }
                        }

                        if d.next_actions.is_empty() {
                            EmptyState {
                                title: "Rien à faire",
                                text: "Ta SCI est à jour pour le moment."
                            }
                        }

                        for t in d.next_actions.iter().take(6) {
                            TaskRow {
                                task: t.clone()
                            }
                        }
                    }

                    div {
                        class: "panel",

                        div {
                            class: "panel-head",

                            h3 {
                                "État de préparation"
                            }

                            span {
                                class: "small",
                                "Données fiables"
                            }
                        }

                        div {
                            class: "check-grid",

                            Check {
                                ok: s.profile_ready,
                                title: "SCI",
                                text: "Identité"
                            }

                            Check {
                                ok: s.associates_ready,
                                title: "Associés",
                                text: "Capital"
                            }

                            Check {
                                ok: s.property_ready,
                                title: "Biens",
                                text: "Patrimoine"
                            }

                            Check {
                                ok: s.tenant_ready,
                                title: "Locataires",
                                text: "Tiers"
                            }

                            Check {
                                ok: s.lease_ready,
                                title: "Baux",
                                text: "Occupation"
                            }

                            Check {
                                ok: s.finance_ready,
                                title: "Argent",
                                text: "Flux"
                            }

                            Check {
                                ok: s.automation_ready,
                                title: "Moteur",
                                text: "Automatisations"
                            }
                        }
                    }
                }
            }
        },

        _ => rsx! {
            Loading {}
        },
    }
}
#[server]
pub async fn dashboard_real_cash_history(months: i32) -> Result<Vec<(NaiveDate, i64)>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        let months = months.clamp(3, 24);
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = crate::entity_scope::current_legal_entity_id();
        let rows = sqlx::query(
            r#"
            WITH months AS (
                SELECT date_trunc(
                    'month', CURRENT_DATE + (g || ' month')::interval
                )::date AS month_start
                FROM generate_series(-($1::int - 1), 0) g
            )
            SELECT
                m.month_start,
                (
                    COALESCE((
                        SELECT SUM(opening_balance_cents)::bigint
                        FROM bank_account_profiles
                        WHERE legal_entity_id = $2 AND active
                    ), 0)
                    + COALESCE((
                        SELECT SUM(amount_cents)::bigint
                        FROM bank_transactions
                        WHERE legal_entity_id = $2
                          AND transaction_date < (m.month_start + interval '1 month')::date
                    ), 0)
                )::bigint AS balance_cents
            FROM months m
            ORDER BY m.month_start
            "#,
        )
        .bind(months)
        .bind(entity)
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| (r.get("month_start"), r.get("balance_cents")))
            .collect())
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = months;
        Err(ServerFnError::new(
            "dashboard_real_cash_history est exécutée côté serveur",
        ))
    }
}

#[derive(Clone)]
struct CurveScale {
    min: f64,
    max: f64,
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

fn curve_x(index: usize, count: usize, scale: &CurveScale) -> f64 {
    if count <= 1 {
        return scale.left;
    }
    scale.left + (index as f64 / (count - 1) as f64) * (scale.right - scale.left)
}

fn curve_y(value_cents: i64, scale: &CurveScale) -> f64 {
    let span = (scale.max - scale.min).max(1.0);
    scale.bottom - ((value_cents as f64 / 100.0 - scale.min) / span) * (scale.bottom - scale.top)
}

fn build_curve_path(
    dates: &[NaiveDate],
    values: &BTreeMap<NaiveDate, i64>,
    scale: &CurveScale,
) -> String {
    let mut path = String::new();
    let mut drawing = false;
    for (index, date) in dates.iter().enumerate() {
        let Some(value) = values.get(date) else {
            drawing = false;
            continue;
        };
        let x = curve_x(index, dates.len(), scale);
        let y = curve_y(*value, scale);
        if drawing {
            path.push_str(&format!(" L {:.1} {:.1}", x, y));
        } else {
            path.push_str(&format!("M {:.1} {:.1}", x, y));
            drawing = true;
        }
    }
    path
}

#[component]
fn TreasuryDualCurveChart(
    real_history: Vec<(NaiveDate, i64)>,
    forecast: Vec<crate::treasury::TreasuryForecastPointItem>,
) -> Element {
    let mut dates = Vec::<NaiveDate>::new();
    for (date, _) in &real_history {
        if !dates.contains(date) {
            dates.push(*date);
        }
    }
    for point in &forecast {
        if !dates.contains(&point.date) {
            dates.push(point.date);
        }
    }
    dates.sort_unstable();

    let mut real = BTreeMap::<NaiveDate, i64>::new();
    for (date, value) in real_history {
        real.insert(date, value);
    }
    let mut planned = BTreeMap::<NaiveDate, i64>::new();
    for point in forecast {
        planned.insert(point.date, point.balance_cents);
    }

    let mut all_values = Vec::<i64>::new();
    all_values.extend(real.values().copied());
    all_values.extend(planned.values().copied());
    let min_eur = all_values
        .iter()
        .map(|v| *v as f64 / 100.0)
        .fold(f64::INFINITY, f64::min);
    let max_eur = all_values
        .iter()
        .map(|v| *v as f64 / 100.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let pad = ((max_eur - min_eur).abs() * 0.12).max(300.0);
    let scale = CurveScale {
        min: min_eur - pad,
        max: max_eur + pad,
        left: 22.0,
        right: 738.0,
        top: 20.0,
        bottom: 235.0,
    };
    let real_path = build_curve_path(&dates, &real, &scale);
    let planned_path = build_curve_path(&dates, &planned, &scale);
    let y0 = curve_y(((scale.min + scale.max) / 2.0 * 100.0) as i64, &scale);

    rsx! {
        section { class: "panel dual-curve-panel",
            div { class: "panel-head dual-curve-head",
                div {
                    h3 { "💰 Trésorerie" }
                    div { class: "small", "Ce qui est arrivé • ce qui est prévu" }
                }
                div { class: "curve-legend",
                    span { class: "curve-legend-item real", span { class: "curve-dot" }, "Réel" }
                    span { class: "curve-legend-item planned", span { class: "curve-dot" }, "Prévu" }
                }
            }
            if dates.is_empty() {
                div { class: "empty-state", h3 { "Pas encore assez de données" }, p { "Le graphique apparaîtra dès que la banque et le prévisionnel contiendront des données." } }
            } else {
                div { class: "dual-curve-wrap",
                    svg { class: "dual-curve-svg", view_box: "0 0 760 280", preserve_aspect_ratio: "none",
                        line { x1: "22", y1: "45", x2: "738", y2: "45", class: "curve-grid-line" }
                        line { x1: "22", y1: "95", x2: "738", y2: "95", class: "curve-grid-line" }
                        line { x1: "22", y1: "145", x2: "738", y2: "145", class: "curve-grid-line" }
                        line { x1: "22", y1: "195", x2: "738", y2: "195", class: "curve-grid-line" }
                        line { x1: "22", y1: "{y0}", x2: "738", y2: "{y0}", class: "curve-zero-line" }
                        path { d: "{planned_path}", class: "curve-line planned" }
                        path { d: "{real_path}", class: "curve-line real" }
                        for (index, date) in dates.iter().enumerate().filter(|(i, _)| *i % 3 == 0 || *i + 1 == dates.len()) {
                            text { x: "{curve_x(index, dates.len(), &scale)}", y: "260", class: "curve-x-label", text_anchor: "middle", "{date.format(\"%b %Y\").to_string()}" }
                        }
                    }
                }
                div { class: "dual-curve-note",
                    span { "Réel" }
                    span { "→ mouvements bancaires constatés" }
                    span { "Prévu" }
                    span { "→ projection de trésorerie du moteur" }
                }
            }
        }
    }
}

#[component]
fn ZeroSaisieSummary(refresh:Signal<u64>)->Element{let snapshot=use_resource(move||{let _=refresh();async move{crate::ux::ux_dashboard_snapshot().await.ok()}});match snapshot.read().as_ref().and_then(|v|v.as_ref()){Some(s)=>rsx!{section {class:"zero-summary",div {div {class:"eyebrow","ZERO-SAISIE"},h3 {if s.validations>0||!s.anomalies.is_empty(){"Tout est presque prêt"}else{"Tout est prêt"}},p {if s.validations>0{"Il me manque simplement ta validation."}else if !s.anomalies.is_empty(){"Quelques points demandent ton attention."}else{"Aucune saisie inutile à effectuer maintenant."}}},div {class:"zero-summary-metrics",span {strong {"{s.validations}"}," validations"},span {strong {"{s.anomalies.len()}"}," anomalies"},span {strong {"{s.overall_quality}/100"}," qualité"}}}},_=>rsx!{}}}

#[component]
fn SetupPage(refresh: Signal<u64>) -> Element {
    let _ = refresh();
    let profile = use_resource(|| async move { get_sci_profile().await.ok() });
    let mut init = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut siren = use_signal(String::new);
    let mut siret = use_signal(String::new);
    let mut office = use_signal(String::new);
    let mut iban = use_signal(String::new);
    let mut bic = use_signal(String::new);
    let mut msg = use_signal(String::new);
    if !init() {
        if let Some(Some(p)) = &*profile.read() {
            name.set(p.legal_name.clone());
            siren.set(p.siren.clone());
            siret.set(p.siret.clone());
            office.set(p.registered_office.clone());
            iban.set(p.iban.clone());
            bic.set(p.bic.clone());
            init.set(true);
        }
    }
    rsx! {ModuleHeader{title:"Configuration de la SCI",kicker:"IDENTITÉ • FISCALITÉ • BANQUE",detail:"La configuration est versionnée et réutilisée par les autres modules."}
      section {class:"panel",div {class:"form-grid",FormField{label:"Dénomination sociale",value:name(),oninput:move|e:FormEvent|name.set(e.value())}FormField{label:"SIREN",value:siren(),oninput:move|e:FormEvent|siren.set(e.value())}FormField{label:"SIRET",value:siret(),oninput:move|e:FormEvent|siret.set(e.value())}FormField{label:"Siège social",value:office(),oninput:move|e:FormEvent|office.set(e.value())}FormField{label:"IBAN",value:iban(),oninput:move|e:FormEvent|iban.set(e.value())}FormField{label:"BIC",value:bic(),oninput:move|e:FormEvent|bic.set(e.value())}},div {class:"facts-row",InfoTile{label:"Régime",value:"IR"}InfoTile{label:"TVA",value:"Option locations"}InfoTile{label:"Exigibilité",value:"Encaissement"}InfoTile{label:"Devise",value:"EUR"}},div {class:"action-row",button {class:"primary",onclick:move |_|async move{let p=SciProfile{legal_name:name(),siren:siren(),siret:siret(),registered_office:office(),tax_regime:"IR".into(),vat_status:"OPTION_LOYERS".into(),vat_basis:"COLLECTION".into(),accounting_period_start:1,fiscal_year_end:12,iban:iban(),bic:bic()};match update_sci_profile(p).await{Ok(_)=>msg.set("Configuration enregistrée".into()),Err(e)=>msg.set(e.to_string())}},"Enregistrer"}span { class: "save-ok", "{msg}" }}}
    }
}


#[component]
fn LegalEntitiesPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let entities = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_legal_entities().await.unwrap_or_default() }
    });

    let mut editing_id = use_signal(|| None::<Uuid>);
    let mut name = use_signal(String::new);
    let mut form = use_signal(|| "SCI".to_string());
    let mut tax = use_signal(|| "IR".to_string());
    let mut vat_status = use_signal(|| "NONE".to_string());
    let mut vat_basis = use_signal(|| "COLLECTION".to_string());
    let mut siren = use_signal(String::new);
    let mut siret = use_signal(String::new);
    let mut office = use_signal(String::new);
    let mut start_month = use_signal(|| "1".to_string());
    let mut end_month = use_signal(|| "12".to_string());
    let mut currency = use_signal(|| "EUR".to_string());
    let mut iban = use_signal(String::new);
    let mut bic = use_signal(String::new);
    let mut msg = use_signal(String::new);

    let mut reset_form = move || {
        editing_id.set(None);
        name.set(String::new());
        form.set("SCI".into());
        tax.set("IR".into());
        vat_status.set("NONE".into());
        vat_basis.set("COLLECTION".into());
        siren.set(String::new());
        siret.set(String::new());
        office.set(String::new());
        start_month.set("1".into());
        end_month.set("12".into());
        currency.set("EUR".into());
        iban.set(String::new());
        bic.set(String::new());
        msg.set(String::new());
    };

    rsx! {
        ModuleHeader{
            title:"Entités juridiques",
            kicker:"MULTI-SOCIÉTÉS • IDENTITÉ • FISCALITÉ • BANQUE",
            detail:"Le registre permet de conserver plusieurs sociétés indépendantes. Le cloisonnement opérationnel des modules existants est traité dans US-0203."
        }
        section {class:"panel",
            div {class:"panel-head",
                h3 {if editing_id().is_some(){"Modifier l’entité"}else{"Ajouter une entité"}}
                if editing_id().is_some(){
                    button {class:"secondary",onclick:move |_|reset_form(),"Annuler"}
                }
            }
            div {class:"form-grid",
                FormField{label:"Dénomination sociale",value:name(),oninput:move|e:FormEvent|name.set(e.value())}
                label {class:"field",span {"Forme juridique"},select {value:form(),onchange:move|e:FormEvent|form.set(e.value()),option {value:"SCI","SCI"},option {value:"SARL","SARL"}}}
                label {class:"field",span {"Régime fiscal"},select {value:tax(),onchange:move|e:FormEvent|tax.set(e.value()),option {value:"IR","IR"},option {value:"IS","IS"}}}
                FormField{label:"SIREN",value:siren(),oninput:move|e:FormEvent|siren.set(e.value())}
                FormField{label:"SIRET",value:siret(),oninput:move|e:FormEvent|siret.set(e.value())}
                FormField{label:"Siège social",value:office(),oninput:move|e:FormEvent|office.set(e.value())}
                label {class:"field",span {"TVA"},select {value:vat_status(),onchange:move|e:FormEvent|vat_status.set(e.value()),option {value:"NONE","Non activée"},option {value:"OPTION_LOYERS","Option / loyers"},option {value:"SUBJECT","Assujettie"}}}
                label {class:"field",span {"Exigibilité TVA"},select {value:vat_basis(),onchange:move|e:FormEvent|vat_basis.set(e.value()),option {value:"COLLECTION","Encaissements"},option {value:"DEBIT","Débits"}}}
                FormField{label:"Début exercice (mois)",value:start_month(),oninput:move|e:FormEvent|start_month.set(e.value())}
                FormField{label:"Fin exercice (mois)",value:end_month(),oninput:move|e:FormEvent|end_month.set(e.value())}
                FormField{label:"Devise",value:currency(),oninput:move|e:FormEvent|currency.set(e.value())}
                FormField{label:"IBAN principal",value:iban(),oninput:move|e:FormEvent|iban.set(e.value())}
                FormField{label:"BIC principal",value:bic(),oninput:move|e:FormEvent|bic.set(e.value())}
            }
            div {class:"action-row",
                button {class:"primary",onclick:move |_|async move{
                    let draft=LegalEntityDraft{
                        legal_name:name(),legal_form_code:form(),tax_regime:tax(),vat_status:vat_status(),vat_basis:vat_basis(),
                        siren:siren(),siret:siret(),registered_office:office(),
                        accounting_period_start:start_month().parse::<u8>().unwrap_or(0),
                        fiscal_year_end:end_month().parse::<u8>().unwrap_or(0),
                        currency_code:currency(),primary_iban:iban(),primary_bic:bic()
                    };
                    let result=match editing_id(){
                        Some(id)=>update_legal_entity(id,draft).await.map(|_|"Entité mise à jour".to_string()),
                        None=>create_legal_entity(draft).await.map(|_|"Entité créée".to_string()).map(|m|m)
                    };
                    match result { Ok(text)=>{msg.set(text);bump+=1;}, Err(e)=>msg.set(e.to_string()) }
                },if editing_id().is_some(){"Enregistrer"}else{"Créer"}}
                span { class: "save-ok", "{msg}" }
            }
        }
        section {class:"panel",
            div {class:"panel-head",h3 {"Sociétés enregistrées"},span { class: "small", {format!("{} entité(s)", entities.read().as_deref().unwrap_or(&[]).len())} }}
            if entities.read().as_deref().unwrap_or(&[]).is_empty(){
                EmptyState{title:"Aucune entité",text:"Créez une SCI ou une SARL pour initialiser le registre juridique."}
            }
            for entity in entities.read().as_deref().unwrap_or(&[]).iter(){
                LegalEntityRow{
                    item:entity.clone(),
                    on_edit:move |item:LegalEntityItem|{
                        editing_id.set(Some(item.id)); name.set(item.legal_name); form.set(item.legal_form_code); tax.set(item.tax_regime);
                        vat_status.set(item.vat_status); vat_basis.set(item.vat_basis); siren.set(item.siren); siret.set(item.siret);
                        office.set(item.registered_office); start_month.set(item.accounting_period_start.to_string());
                        end_month.set(item.fiscal_year_end.to_string()); currency.set(item.currency_code);
                        iban.set(item.primary_iban); bic.set(item.primary_bic); msg.set(String::new());
                    },
                    on_toggle:move |item:LegalEntityItem|async move{
                        match set_legal_entity_active(item.id,!item.active).await{Ok(_)=>bump+=1,Err(e)=>msg.set(e.to_string())}
                    }
                }
            }
        }
    }
}

#[component]
fn LegalEntityRow(item: LegalEntityItem, on_edit: EventHandler<LegalEntityItem>, on_toggle: EventHandler<LegalEntityItem>) -> Element {
    let form_label=if item.legal_form_code=="SARL"{"SARL"}else{"SCI"};
    let status=if item.active{"ACTIVE"}else{"INACTIVE"};
    let iban_tail=if item.primary_iban.len()>4{format!("•••• {}",&item.primary_iban[item.primary_iban.len()-4..])}else{item.primary_iban.clone()};
    let item_for_edit = item.clone();
    let item_for_toggle = item.clone();
    rsx!{
        div {class:"list-row",
            div {class:"row-main",
                strong {"{item.legal_name}" }
                div { class: "small", {format!("{} • {} • {}", form_label, item.tax_regime, status)} }
                div { class: "small", {format!("SIREN {} • Compte(s) {}{}", if item.siren.is_empty() { "—".into() } else { item.siren.clone() }, item.bank_accounts_count, if iban_tail.is_empty() { String::new() } else { format!(" • {}", iban_tail) })} }
            }
            div {class:"row-actions",
                button {class:"secondary",onclick:move |_|on_edit.call(item_for_edit.clone()),"Modifier"}
                button {class:"secondary",onclick:move |_|on_toggle.call(item_for_toggle.clone()),if item.active{"Désactiver"}else{"Activer"}}
            }
        }
    }
}

#[component]
fn AssociatesPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let items = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_associates().await.unwrap_or_default() }
    });
    let mut name = use_signal(String::new);
    let mut pct = use_signal(|| "0".to_string());
    let mut cca = use_signal(|| "0".to_string());
    let mut msg = use_signal(String::new);
    rsx! {ModuleHeader{title:"Associés",kicker:"CAPITAL • COMPTES COURANTS",detail:"Le total des quote-parts actives est borné à 100 %."}
      section {class:"panel",div {class:"form-grid",FormField{label:"Nom",value:name(),oninput:move|e:FormEvent|name.set(e.value())}FormField{label:"Quote-part %",value:pct(),oninput:move|e:FormEvent|pct.set(e.value())}FormField{label:"Compte courant initial €",value:cca(),oninput:move|e:FormEvent|cca.set(e.value())}},div {class:"action-row",button {class:"primary",onclick:move |_|async move{match pct().replace(",",".").parse::<Decimal>(){Ok(v)=>match create_associate(name(),v,euros_to_cents(&cca())).await{Ok(_)=>{msg.set("Associé ajouté".into());bump+=1},Err(e)=>msg.set(e.to_string())},Err(_)=>msg.set("Quote-part invalide".into())}},"Ajouter"}span { class: "save-ok", "{msg}" }}}
      section {class:"panel",div {for a in items.read().as_deref().unwrap_or(&[]).iter(){AssociateRow{item:a.clone(),bump}}}}
    }
}

#[component]
fn TenantsPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let items = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_tenants().await.unwrap_or_default() }
    });
    let mut name = use_signal(String::new);
    let mut siret = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut phone = use_signal(String::new);
    let mut msg = use_signal(String::new);
    rsx! {ModuleHeader{title:"Locataires",kicker:"TIERS • CONTACTS • DOSSIERS",detail:"Les locataires alimentent les baux et les factures. Les suppressions sont bloquées dès qu’un bail existe."}
      section {class:"panel",div {class:"form-grid",FormField{label:"Raison sociale / nom",value:name(),oninput:move|e:FormEvent|name.set(e.value())}FormField{label:"SIRET",value:siret(),oninput:move|e:FormEvent|siret.set(e.value())}FormField{label:"Email",value:email(),oninput:move|e:FormEvent|email.set(e.value())}FormField{label:"Téléphone",value:phone(),oninput:move|e:FormEvent|phone.set(e.value())}},div {class:"action-row",button {class:"primary",onclick:move |_|async move{match create_tenant(name(),siret(),email(),phone()).await{Ok(_)=>{msg.set("Locataire créé".into());bump+=1},Err(e)=>msg.set(e.to_string())}},"Créer"}span { class: "save-ok", "{msg}" }}}
      section {class:"panel",div {for t in items.read().as_deref().unwrap_or(&[]).iter(){TenantRow{item:t.clone(),bump}}}}
    }
}

#[component]
fn PatrimonyPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let props = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_properties().await.unwrap_or_default() }
    });
    let units = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_units().await.unwrap_or_default() }
    });
    let mut name = use_signal(String::new);
    let mut address = use_signal(String::new);
    let mut acq = use_signal(String::new);
    let mut pid = use_signal(String::new);
    let mut code = use_signal(String::new);
    let mut label = use_signal(String::new);
    let mut rent = use_signal(String::new);
    let mut rate = use_signal(|| "20".to_string());
    let mut msg = use_signal(String::new);
    rsx! {ModuleHeader{title:"Patrimoine",kicker:"BIENS • LOTS • LOYERS",detail:"Le patrimoine est la source des baux, indexations et factures."}
      section {class:"two-col",div {class:"panel",h3 {"Nouveau bien"},div {class:"form-grid",FormField{label:"Nom",value:name(),oninput:move|e:FormEvent|name.set(e.value())}FormField{label:"Adresse",value:address(),oninput:move|e:FormEvent|address.set(e.value())}FormField{label:"Acquisition €",value:acq(),oninput:move|e:FormEvent|acq.set(e.value())}},button {class:"primary",onclick:move |_|async move{match create_property(name(),address(),None,Some(euros_to_cents(&acq()))).await{Ok(_)=>{msg.set("Bien créé".into());bump+=1},Err(e)=>msg.set(e.to_string())}},"Créer le bien"}},div {class:"panel",h3 {"Nouveau lot"},div {class:"form-grid",label {class:"field",span {"Bien"},select {value:pid(),onchange:move|e:FormEvent|pid.set(e.value()),option {value:"","Sélectionner"},for p in props.read().as_deref().unwrap_or(&[]).iter(){option { value: p.id.to_string(), "{p.name}" }}}}FormField{label:"Code",value:code(),oninput:move|e:FormEvent|code.set(e.value())}FormField{label:"Libellé",value:label(),oninput:move|e:FormEvent|label.set(e.value())}FormField{label:"Loyer €",value:rent(),oninput:move|e:FormEvent|rent.set(e.value())}FormField{label:"TVA %",value:rate(),oninput:move|e:FormEvent|rate.set(e.value())}},button {class:"primary",onclick:move |_|async move{match Uuid::parse_str(&pid()){Ok(id)=>{let bp=rate().replace(",",".").parse::<Decimal>().unwrap_or(Decimal::from(20u32));let bp=(bp*Decimal::from(100u32)).round_dp(0).to_i32().unwrap_or(2000);match create_unit(id,code(),label(),"COMMERCIAL".into(),None,euros_to_cents(&rent()),bp).await{Ok(_)=>{msg.set("Lot créé".into());bump+=1},Err(e)=>msg.set(e.to_string())}},Err(_)=>msg.set("Sélectionnez un bien".into())}},"Créer le lot"}}}
      section {class:"panel",div {class:"panel-head",h3 {"Biens"},span { class: "small", "{msg}" }},div {for p in props.read().as_deref().unwrap_or(&[]).iter(){PropertyRow{item:p.clone(),bump}}}}
      section {class:"panel",h3 {"Lots"},div {for u in units.read().as_deref().unwrap_or(&[]).iter(){UnitRow{item:u.clone(),bump}}}}
    }
}

#[component]
fn RentalsPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let leases = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_leases().await.unwrap_or_default() }
    });
    let units = use_resource(move || {
        let _ = refresh();
        async move { list_units().await.unwrap_or_default() }
    });
    let tenants = use_resource(move || {
        let _ = refresh();
        async move { list_tenants().await.unwrap_or_default() }
    });
    let mut unit = use_signal(String::new);
    let mut tenant = use_signal(String::new);
    let mut refx = use_signal(String::new);
    let mut start = use_signal(|| Utc::now().date_naive().to_string());
    let mut end = use_signal(String::new);
    let mut day = use_signal(|| "5".to_string());
    let mut msg = use_signal(String::new);
    rsx! {ModuleHeader{title:"Locations",kicker:"BAUX • PRÉAVIS • RÉVISIONS",detail:"Les baux actifs pilotent la préparation des loyers et les échéances administratives."}
      section {class:"panel",div {class:"form-grid",label {class:"field",span {"Lot"},select {value:unit(),onchange:move|e:FormEvent|unit.set(e.value()),option {value:"","Sélectionner"},for u in units.read().as_deref().unwrap_or(&[]).iter(){option { value: u.id.to_string(), {format!("{} • {}", u.property_name, u.label)} }}}}label {class:"field",span {"Locataire"},select {value:tenant(),onchange:move|e:FormEvent|tenant.set(e.value()),option {value:"","Sélectionner"},for t in tenants.read().as_deref().unwrap_or(&[]).iter(){option { value: t.id.to_string(), "{t.legal_name}" }}}}FormField{label:"Référence",value:refx(),oninput:move|e:FormEvent|refx.set(e.value())}FormField{label:"Début AAAA-MM-JJ",value:start(),oninput:move|e:FormEvent|start.set(e.value())}FormField{label:"Fin AAAA-MM-JJ",value:end(),oninput:move|e:FormEvent|end.set(e.value())}FormField{label:"Jour de paiement",value:day(),oninput:move|e:FormEvent|day.set(e.value())}},div {class:"action-row",button {class:"primary",onclick:move |_|async move{match(Uuid::parse_str(&unit()),Uuid::parse_str(&tenant()),NaiveDate::parse_from_str(&start(),"%Y-%m-%d")){(Ok(u),Ok(t),Ok(s))=>{let e=if end().trim().is_empty(){None}else{NaiveDate::parse_from_str(&end(),"%Y-%m-%d").ok()};match create_lease(u,t,refx(),s,e,3,day().parse().unwrap_or(5),None).await{Ok(_)=>{msg.set("Bail créé".into());bump+=1},Err(e)=>msg.set(e.to_string())}},_=>(msg.set("Lot, locataire ou date invalide".into()))}},"Créer le bail"}span { class: "save-ok", "{msg}" }}}
      section {class:"panel",div {for l in leases.read().as_deref().unwrap_or(&[]).iter(){LeaseRow{item:l.clone(),bump}}}}
    }
}

#[component]
fn BillingPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let inv = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_invoices().await.unwrap_or_default() }
    });
    let leases = use_resource(move || {
        let _ = refresh();
        async move { list_leases().await.unwrap_or_default() }
    });
    let mut lease = use_signal(String::new);
    let mut issue = use_signal(|| Utc::now().date_naive().to_string());
    let mut due = use_signal(|| (Utc::now().date_naive() + Duration::days(30)).to_string());
    let mut pay_invoice = use_signal(String::new);
    let mut pay_amount = use_signal(String::new);
    let mut pay_ref = use_signal(String::new);
    let mut msg = use_signal(String::new);
    rsx! {
        ModuleHeader{title:"Facturation",kicker:"FACTURES • ÉMISSIONS • ENCAISSEMENTS",detail:"Création des brouillons depuis les baux, émission, suivi des paiements et calcul de TVA à l’encaissement."}
        section {class:"two-col",
            div {class:"panel",
                h3 {"Préparer une facture"}
                div {class:"form-grid",
                    label {class:"field",
                        span {"Bail"}
                        select {value:lease(),onchange:move|e:FormEvent|lease.set(e.value()),
                            option {value:"","Sélectionner"}
                            for l in leases.read().as_deref().unwrap_or(&[]).iter(){
                                option { value: l.id.to_string(), {format!("{} • {}", l.reference, l.tenant_name)} }
                            }
                        }
                    }
                    FormField{label:"Émission",value:issue(),oninput:move|e:FormEvent|issue.set(e.value())}
                    FormField{label:"Échéance",value:due(),oninput:move|e:FormEvent|due.set(e.value())}
                }
                button {class:"primary",
                    onclick:move |_|async move{
                        match(Uuid::parse_str(&lease()),NaiveDate::parse_from_str(&issue(),"%Y-%m-%d"),NaiveDate::parse_from_str(&due(),"%Y-%m-%d")){
                            (Ok(l),Ok(i),Ok(d))=>match create_invoice_from_lease(l,i,d).await{
                                Ok(_)=>{msg.set("Brouillon créé".into());bump+=1},
                                Err(e)=>msg.set(e.to_string())
                            },
                            _=>msg.set("Données de facture invalides".into())
                        }
                    },
                    "Créer le brouillon"
                }
            }
            div {class:"panel",
                h3 {"Enregistrer un encaissement"}
                div {class:"form-grid",
                    label {class:"field",
                        span {"Facture"}
                        select {value:pay_invoice(),onchange:move|e:FormEvent|pay_invoice.set(e.value()),
                            option {value:"","Sélectionner"}
                            for i in inv.read().as_deref().unwrap_or(&[]).iter().filter(|x|x.status!="BROUILLON"&&x.paid_cents<x.gross_cents){
                                option { value: i.id.to_string(), {format!("{} • {}", i.invoice_number, i.tenant_name)} }
                            }
                        }
                    }
                    FormField{label:"Montant €",value:pay_amount(),oninput:move|e:FormEvent|pay_amount.set(e.value())}
                    FormField{label:"Référence",value:pay_ref(),oninput:move|e:FormEvent|pay_ref.set(e.value())}
                }
                button {class:"primary",
                    onclick:move |_|async move{
                        let iid=Uuid::parse_str(&pay_invoice()).ok();
                        match create_payment(iid,Utc::now(),euros_to_cents(&pay_amount()),pay_ref(),"BANK".into()).await{
                            Ok(_)=>{msg.set("Encaissement enregistré".into());bump+=1},
                            Err(e)=>msg.set(e.to_string())
                        }
                    },
                    "Enregistrer"
                }
            }
        }
        span { class: "save-ok", "{msg}" }
        section {class:"panel",
            div {
                for i in inv.read().as_deref().unwrap_or(&[]).iter(){
                    InvoiceRow{item:i.clone(),bump}
                }
            }
        }
        section {class:"panel",
            h3 {"Historique des paiements"}
            PaymentList{refresh:bump}
        }
    }
}

#[component]
fn BankPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let txs = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_bank_transactions().await.unwrap_or_default() }
    });
    let inv = use_resource(move || {
        let _ = refresh();
        async move { list_invoices().await.unwrap_or_default() }
    });
    let mut amount = use_signal(String::new);
    let mut label = use_signal(String::new);
    let mut cp = use_signal(String::new);
    let mut ext = use_signal(String::new);
    let mut selected = use_signal(String::new);
    let mut selected_inv = use_signal(String::new);
    let mut csv = use_signal(String::new);
    let mut msg = use_signal(String::new);
    rsx! {ModuleHeader{title:"Banque",kicker:"IMPORT CSV • RAPPROCHEMENT • CONTRÔLE",detail:"Import idempotent par identifiant externe, suivi des mouvements et rapprochement vers les factures."}
      section {class:"two-col",div {class:"panel",h3 {"Saisie d’un mouvement"},div {class:"form-grid",FormField{label:"Montant €",value:amount(),oninput:move|e:FormEvent|amount.set(e.value())}FormField{label:"Libellé",value:label(),oninput:move|e:FormEvent|label.set(e.value())}FormField{label:"Contrepartie",value:cp(),oninput:move|e:FormEvent|cp.set(e.value())}FormField{label:"Identifiant externe",value:ext(),oninput:move|e:FormEvent|ext.set(e.value())}},button {class:"primary",onclick:move |_|async move{match create_bank_transaction(Utc::now(),Some(Utc::now().date_naive()),euros_to_cents(&amount()),label(),cp(),ext()).await{Ok(_)=>{msg.set("Mouvement enregistré".into());bump+=1},Err(e)=>msg.set(e.to_string())}},"Enregistrer"}},div {class:"panel",h3 {"Import bancaire"},p {class:"small","Format : AAAA-MM-JJ;montant;libellé;identifiant"},textarea {value:csv(),oninput:move|e:FormEvent|csv.set(e.value()),placeholder:"2026-09-01;1200,50;VIREMENT LOYER;CA-001"},button {class:"primary",onclick:move |_|async move{match import_bank_csv(csv()).await{Ok(n)=>{msg.set(format!("{} mouvement(s) importé(s)",n));bump+=1},Err(e)=>msg.set(e.to_string())}},"Importer le CSV"}}}
      section {class:"panel",h3 {"Rapprochement"},div {class:"form-grid",label {class:"field",span {"Mouvement"},select {value:selected(),onchange:move|e:FormEvent|selected.set(e.value()),option {value:"","Sélectionner"},for t in txs.read().as_deref().unwrap_or(&[]).iter().filter(|x|x.reconciliation_status=="UNMATCHED"&&x.amount_cents>0){option { value: t.id.to_string(), {format!("{} • {}", euro(t.amount_cents), t.label)} }}}}label {class:"field",span {"Facture"},select {value:selected_inv(),onchange:move|e:FormEvent|selected_inv.set(e.value()),option {value:"","Sélectionner"},for i in inv.read().as_deref().unwrap_or(&[]).iter().filter(|x|x.status!="BROUILLON"&&x.paid_cents<x.gross_cents){option { value: i.id.to_string(), {format!("{} • {}", i.invoice_number, i.tenant_name)} }}}}},button {class:"primary",onclick:move |_|async move{match(Uuid::parse_str(&selected()),Uuid::parse_str(&selected_inv())){(Ok(b),Ok(i))=>match reconcile_bank_transaction(b,i).await{Ok(_)=>{msg.set("Rapprochement effectué".into());bump+=1},Err(e)=>msg.set(e.to_string())},_=>(msg.set("Sélection incomplète".into()))}},"Rapprocher"}span { class: "save-ok", "{msg}" }}
      section {class:"panel",div {for t in txs.read().as_deref().unwrap_or(&[]).iter(){BankRow{item:t.clone(),bump}}}}
    }
}

#[component]
fn VatPage(refresh: Signal<u64>) -> Element {
    let _ = refresh();
    let mut period = use_signal(|| Utc::now().date_naive().format("%Y-%m").to_string());
    let summary = use_resource(move || {
        let _ = refresh();
        let p = period();
        async move { vat_summary(p).await.ok() }
    });
    rsx! {
        ModuleHeader{title:"TVA",kicker:"COLLECTE • ENCAISSEMENT • PRÉPARATION",detail:"La TVA présentée ici est dérivée des encaissements rapprochés avec les factures."}
        section {class:"panel",FormField{label:"Période AAAA-MM",value:period(),oninput:move|e:FormEvent|period.set(e.value())}}
        match &*summary.read(){
            Some(Some(v))=>rsx!{
                section {class:"facts-row",
                    InfoTileOwned{label:"Encaissements",value:euro(v.receipts_gross_cents)}
                    InfoTileOwned{label:"Base taxable",value:euro(v.taxable_net_cents)}
                    InfoTileOwned{label:"TVA exigible",value:euro(v.vat_due_cents)}
                    InfoTileOwned{label:"Paiements",value:v.payments_count.to_string()}
                }
            },
            _=>rsx!{Loading{}}
        }
    }
}

#[component]
fn CalendarPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let items = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_deadlines().await.unwrap_or_default() }
    });
    let mut code = use_signal(String::new);
    let mut label = use_signal(String::new);
    let mut date = use_signal(|| (Utc::now().date_naive() + Duration::days(30)).to_string());
    let mut period = use_signal(String::new);
    let mut msg = use_signal(String::new);
    rsx! {ModuleHeader{title:"Calendrier",kicker:"ÉCHÉANCES • PRÉPARATION • RELANCES",detail:"Les échéances saisies deviennent des tâches bloquantes avant la date limite."}section {class:"panel",div {class:"form-grid",FormField{label:"Code",value:code(),oninput:move|e:FormEvent|code.set(e.value())}FormField{label:"Libellé",value:label(),oninput:move|e:FormEvent|label.set(e.value())}FormField{label:"Date AAAA-MM-JJ",value:date(),oninput:move|e:FormEvent|date.set(e.value())}FormField{label:"Période",value:period(),oninput:move|e:FormEvent|period.set(e.value())}},button {class:"primary",onclick:move |_|async move{match NaiveDate::parse_from_str(&date(),"%Y-%m-%d"){Ok(d)=>match create_deadline(code(),label(),d,period()).await{Ok(_)=>{msg.set("Échéance enregistrée".into());bump+=1},Err(e)=>msg.set(e.to_string())},Err(_)=>msg.set("Date invalide".into())}},"Ajouter"}span { class: "save-ok", "{msg}" }}section {class:"panel",div {for d in items.read().as_deref().unwrap_or(&[]).iter(){DeadlineRow{item:d.clone(),bump}}}}}
}

#[component]
fn DocumentsPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let items = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_documents().await.unwrap_or_default() }
    });
    let mut cat = use_signal(|| "ADMINISTRATIF".to_string());
    let mut title = use_signal(String::new);
    let mut file = use_signal(String::new);
    let mut key = use_signal(String::new);
    let mut expiry = use_signal(String::new);
    let mut msg = use_signal(String::new);
    rsx! {ModuleHeader{title:"Documents",kicker:"RÉFÉRENTIEL • EXPIRATIONS • TRAÇABILITÉ",detail:"Les pièces sont indexées dans le référentiel métier et peuvent être reliées aux contrôles."}section {class:"panel",div {class:"form-grid",FormField{label:"Catégorie",value:cat(),oninput:move|e:FormEvent|cat.set(e.value())}FormField{label:"Titre",value:title(),oninput:move|e:FormEvent|title.set(e.value())}FormField{label:"Nom de fichier",value:file(),oninput:move|e:FormEvent|file.set(e.value())}FormField{label:"Clé de stockage",value:key(),oninput:move|e:FormEvent|key.set(e.value())}FormField{label:"Expiration AAAA-MM-JJ",value:expiry(),oninput:move|e:FormEvent|expiry.set(e.value())}},button {class:"primary",onclick:move |_|async move{let d=if expiry().trim().is_empty(){None}else{NaiveDate::parse_from_str(&expiry(),"%Y-%m-%d").ok()};match register_document(cat(),title(),file(),key(),None,d).await{Ok(_)=>{msg.set("Document indexé".into());bump+=1},Err(e)=>msg.set(e.to_string())}},"Indexer"}span { class: "save-ok", "{msg}" }}section {class:"panel",div {for d in items.read().as_deref().unwrap_or(&[]).iter(){DocumentRow{item:d.clone(),bump}}}}}
}

#[component]
fn AutomationsPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let rules = use_resource(move || { let _=refresh(); let _=bump(); async move { list_automation_rules().await.unwrap_or_default() } });
    let policies = use_resource(move || { let _=refresh(); let _=bump(); async move { crate::workflow::list_automation_policies().await.unwrap_or_default() } });
    let mut result = use_signal(|| None::<AutomationRunResult>);
    let mut policy_activity = use_signal(String::new);
    let mut policy_object = use_signal(|| "AUTOMATION".to_owned());
    let mut policy_risk = use_signal(|| "MEDIUM".to_owned());
    let mut policy_level = use_signal(|| "2".to_owned());
    let mut policy_validation = use_signal(|| true);
    let mut policy_msg = use_signal(String::new);
    rsx! {
        ModuleHeader { title:"Automatisations", kicker:"RÈGLES • NIVEAUX 0–5 • IDEMPOTENCE", detail:"Les règles sont exécutées sans doublonner ; le niveau d’automatisation est configurable par entité, activité, type et risque." }
        section {
            class: "panel",
            div {
                class: "panel-head",
                h3 { "Cycle manuel" },
                button { class:"primary", onclick: move |_| async move { result.set(run_anticipation_cycle().await.ok()); bump+=1 }, "Exécuter maintenant" }
            }
        }
        if let Some(r)=result() {
            section { class:"run-result", h3 { "Cycle terminé" }, p { {format!("{} règles évaluées • {} tâches créées • {}", r.evaluated_rules, r.created_tasks, r.ran_at.format("%d/%m/%Y %H:%M"))} } }
        }
        section {
            class:"panel",
            h3 { "Politique d'automatisation" },
            div {
                class:"form-grid",
                FormField { label:"Activité (optionnelle)", value:policy_activity(), oninput:move|e:FormEvent|policy_activity.set(e.value()) },
                FormField { label:"Type", value:policy_object(), oninput:move|e:FormEvent|policy_object.set(e.value()) },
                FormField { label:"Risque", value:policy_risk(), oninput:move|e:FormEvent|policy_risk.set(e.value()) },
                FormField { label:"Niveau 0–5", value:policy_level(), oninput:move|e:FormEvent|policy_level.set(e.value()) },
            }
            label { class:"check-row", input { r#type:"checkbox", checked:policy_validation(), onchange:move|e:FormEvent|policy_validation.set(e.value()!="false") }, " Validation requise" }
            button { class:"primary", onclick:move |_| async move { let level=policy_level().parse::<i16>().unwrap_or(2); match crate::workflow::save_automation_policy(None,policy_activity(),policy_object(),policy_risk(),level,policy_validation(),true).await { Ok(_)=>{ policy_msg.set("Politique enregistrée".into()); bump+=1 }, Err(e)=>policy_msg.set(e.to_string()) } }, "Enregistrer" }
            span { class:"save-ok", "{policy_msg}" }
        }
        section {
            class:"panel",
            h3 { "Politiques actives" },
            div {
                class:"data-list",
                for p in policies.read().as_deref().unwrap_or(&[]).iter() {
                    div {
                        class:"data-row",
                        div {
                            div { class:"data-title", "{p.activity_code} · {p.object_type} · {p.risk_level}" },
                            div { class:"small", "niveau {p.automation_level} • validation {p.validation_required} • {p.enabled}" }
                        }
                    }
                }
            }
        }
        section {
            class:"automation-grid",
            for r in rules.read().as_deref().unwrap_or(&[]).iter() { AutomationRow { item:r.clone(), bump } }
        }
    }
}

#[component]
fn TasksPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let tasks = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_tasks().await.unwrap_or_default() }
    });
    rsx! {ModuleHeader{title:"Tâches",kicker:"WORKFLOW • ÉTATS • BLOQUANTS",detail:"Toute action administrative peut être suivie jusqu’à sa clôture."}section {class:"panel",div {for t in tasks.read().as_deref().unwrap_or(&[]).iter(){TaskManagerRow{item:t.clone(),bump}}}}}
}

#[component]
fn AuditPage(refresh: Signal<u64>) -> Element {
    let items = use_resource(move || { let _ = refresh(); async move { list_audit().await.unwrap_or_default() } });
    let changes = use_resource(move || { let _ = refresh(); async move { list_change_history(250).await.unwrap_or_default() } });
    rsx! {
        ModuleHeader{title:"Audit",kicker:"JOURNAL • JUSTIFICATION • HISTORIQUE",detail:"Le journal conserve les actions ; l’historique critique conserve la date d’effet, l’auteur et l’état avant/après."}
        section {class:"panel",div {class:"panel-head",h3 {"Actions enregistrées"},span { class: "small", {format!("{} événement(s)", items.read().as_deref().unwrap_or(&[]).len())} }},div {for a in items.read().as_deref().unwrap_or(&[]).iter(){AuditRow{item:a.clone()}}}}
        section {class:"panel",div {class:"panel-head",h3 {"Historique des changements critiques"},span { class: "small", {format!("{} changement(s)", changes.read().as_deref().unwrap_or(&[]).len())} }},div {for h in changes.read().as_deref().unwrap_or(&[]).iter(){ChangeHistoryRow{item:h.clone()}}}}
        section {class:"panel",div {class:"panel-head",h3 {"Moteurs communs"},span {class:"small","contrat partagé • périmètre d’entité actif"}},EngineCatalogPanel{refresh}}
    }
}

#[component]
fn ChangeHistoryRow(item: ChangeHistoryItem) -> Element {
    let before = if item.before_state.len() > 180 { format!("{}…", &item.before_state[..180]) } else { item.before_state.clone() };
    let after = if item.after_state.len() > 180 { format!("{}…", &item.after_state[..180]) } else { item.after_state.clone() };
    let reason = if item.reason.trim().is_empty() { "Motif : non renseigné".to_owned() } else { format!("Motif : {}", item.reason) };
    rsx! {
        div {
            class:"change-row",
            div {
                class:"change-head",
                strong { "{item.action}" },
                span { class:"small", {format!("{} • effet {} • {}", item.entity_type, item.effective_at.format("%d/%m/%Y %H:%M"), item.author)} }
            }
            div { class:"small", "{reason}" }
            div {
                class:"change-grid",
                div { strong { "Avant" }, pre { "{before}" } }
                div { strong { "Après" }, pre { "{after}" } }
            }
        }
    }
}

#[component]
fn EngineCatalogPanel(refresh: Signal<u64>) -> Element {
    let items = use_resource(move || {
        let _ = refresh();
        async move { list_common_engines().await.unwrap_or_default() }
    });
    rsx! {
        div {
            class: "engine-grid",
            for engine in items.read().as_deref().unwrap_or(&[]).iter().cloned() {
                div {
                    class: "engine-card",
                    div { class: "code", "{engine.code}" }
                    strong { "{engine.label}" }
                    div { class: "small", "{engine.status} • {engine.scope}" }
                }
            }
        }
    }
}

#[component]
fn PaymentList(refresh: Signal<u64>) -> Element {
    let items = use_resource(move || {
        let _ = refresh();
        async move { list_payments().await.unwrap_or_default() }
    });
    rsx! {
        div {
            for payment in items.read().as_deref().unwrap_or(&[]).iter().cloned() {
                PaymentRow { item: payment }
            }
        }
    }
}

#[component]
fn AssociateRow(item: AssociateItem, mut bump: Signal<u64>) -> Element {
    let id = item.id;
    rsx! {
        div {
            class: "data-row",
            div {
                div { class: "data-title", "{item.display_name}" }
                div { class: "small", {format!("Quote-part {} % • C/C {}", item.ownership_pct, euro(item.current_account_cents))} }
            }
            div {
                class: "row-actions",
                button {
                    class: "secondary",
                    onclick: move |_| async move { let _ = delete_associate(id).await; bump += 1; },
                    "Supprimer"
                }
            }
        }
    }
}

#[component]
fn TenantRow(item: TenantItem, mut bump: Signal<u64>) -> Element {
    let id = item.id;
    rsx! {
        div {
            class: "data-row",
            div {
                div { class: "data-title", "{item.legal_name}" }
                div { class: "small", {format!("{} • {}", item.contact_email, item.contact_phone)} }
            }
            div {
                class: "row-actions",
                button {
                    class: "secondary",
                    onclick: move |_| async move { let _ = delete_tenant(id).await; bump += 1; },
                    "Supprimer"
                }
            }
        }
    }
}

#[component]
fn PropertyRow(item: PropertyItem, mut bump: Signal<u64>) -> Element {
    let id = item.id;
    rsx! {
        div {
            class: "data-row",
            div {
                div { class: "data-title", "{item.name}" }
                div { class: "small", "{item.address}" }
            }
            div {
                class: "row-actions",
                span { class: "row-value", {format!("{} lots", item.units_count)} }
                button {
                    class: "secondary",
                    onclick: move |_| async move { let _ = delete_property(id).await; bump += 1; },
                    "Supprimer"
                }
            }
        }
    }
}

#[component]
fn UnitRow(item: UnitItem, mut bump: Signal<u64>) -> Element {
    let id = item.id;
    rsx! {
        div {
            class: "data-row",
            div {
                div { class: "data-title", {format!("{} • {}", item.property_name, item.label)} }
                div { class: "small", {format!("{} • TVA {} %", item.code, (item.vat_rate_bp as f64) / 100.0)} }
            }
            div {
                class: "row-actions",
                span { class: "row-value", "{euro(item.base_rent_cents)}" }
                button {
                    class: "secondary",
                    onclick: move |_| async move { let _ = delete_unit(id).await; bump += 1; },
                    "Supprimer"
                }
            }
        }
    }
}

#[component]
fn LeaseRow(item: LeaseItem, mut bump: Signal<u64>) -> Element {
    let id = item.id;
    rsx! {
        div {
            class: "data-row",
            div {
                div { class: "data-title", "{item.reference}" }
                div { class: "small", {format!("{} • {} • {}", item.property_name, item.unit_label, item.tenant_name)} }
                div { class: "small", {format!("{} → {}", item.start_date, item.end_date.map(|x| x.to_string()).unwrap_or_else(|| "ouvert".into()))} }
            }
            div {
                class: "row-actions",
                button {
                    class: "secondary",
                    onclick: move |_| async move { let _ = delete_lease(id).await; bump += 1; },
                    "Supprimer"
                }
            }
        }
    }
}

#[component]
fn InvoiceRow(item: InvoiceItem, mut bump: Signal<u64>) -> Element {
    let id = item.id;
    let draft = item.status == "BROUILLON";
    rsx! {
        div {
            class: "data-row",
            div {
                div { class: "data-title", "{item.invoice_number}" }
                div { class: "small", {format!("{} • {} • {}", item.tenant_name, item.issue_date, item.status)} }
                div { class: "small", {format!("{} payé sur {}", euro(item.paid_cents), euro(item.gross_cents))} }
            }
            div {
                class: "row-actions",
                if draft {
                    button { class: "secondary", onclick: move |_| async move { let _ = issue_invoice(id).await; bump += 1; }, "Émettre" }
                    button { class: "secondary", onclick: move |_| async move { let _ = delete_invoice(id).await; bump += 1; }, "Supprimer" }
                }
            }
        }
    }
}

#[component]
fn PaymentRow(item: PaymentItem) -> Element {
    rsx! {
        div {
            class: "data-row",
            div {
                div { class: "data-title", "{item.invoice_number}" }
                div { class: "small", {item.received_at.format("%d/%m/%Y %H:%M").to_string()} }
                div { class: "small", "{item.reference}" }
            }
            div { class: "row-value", "{euro(item.amount_cents)}" }
        }
    }
}

#[component]
fn BankRow(item: BankTransactionItem, mut bump: Signal<u64>) -> Element {
    let id = item.id;
    let unmatched = item.reconciliation_status == "UNMATCHED";
    rsx! {
        div {
            class: "data-row",
            div {
                div { class: "data-title", "{item.label}" }
                div { class: "small", "{item.counterparty}" }
                div { class: "small", "{item.reconciliation_status}" }
            }
            div {
                class: "row-actions",
                span { class: "row-value", "{euro(item.amount_cents)}" }
                if unmatched {
                    button { class: "secondary", onclick: move |_| async move { let _ = delete_bank_transaction(id).await; bump += 1; }, "Supprimer" }
                }
            }
        }
    }
}

#[component]
fn DeadlineRow(item: DeadlineItem, mut bump: Signal<u64>) -> Element {
    let id = item.id;
    rsx! {
        div {
            class: "data-row",
            div {
                div { class: "data-title", "{item.label}" }
                div { class: "small", {format!("{} • {}", item.deadline_date, item.period_label)} }
            }
            div {
                class: "row-actions",
                span { class: "status", "{item.status}" }
                if item.status != "DONE" {
                    button { class: "secondary", onclick: move |_| async move { let _ = set_deadline_status(id, "DONE".into()).await; bump += 1; }, "Terminer" }
                    button { class: "secondary", onclick: move |_| async move { let _ = delete_deadline(id).await; bump += 1; }, "Supprimer" }
                }
            }
        }
    }
}

#[component]
fn DocumentRow(item: DocumentItem, mut bump: Signal<u64>) -> Element {
    let id = item.id;
    let expiration = item.expires_at.map(|date| format!("Expiration {date}")).unwrap_or_default();
    rsx! {
        div {
            class: "data-row",
            div {
                div { class: "data-title", "{item.title}" }
                div { class: "small", {format!("{} • {}", item.category, item.file_name)} }
                div { class: "small", "{expiration}" }
            }
            div {
                class: "row-actions",
                button { class: "secondary", onclick: move |_| async move { let _ = delete_document(id).await; bump += 1; }, "Supprimer" }
            }
        }
    }
}

#[component]
fn AutomationRow(item: AutomationRuleItem, mut bump: Signal<u64>) -> Element {
    let id = item.id;
    let enabled = item.enabled;
    rsx! {
        div {
            class: "automation-tile",
            div { class: "code", "{item.code}" }
            h3 { "{item.name}" }
            p { "{item.description}" }
            div { class: "small", {format!("Horizon {} j • priorité {}", item.horizon_days, item.priority)} }
            button {
                class: "secondary",
                onclick: move |_| async move { let _ = set_automation_enabled(id, !enabled).await; bump += 1; },
                if enabled { "Désactiver" } else { "Activer" }
            }
        }
    }
}

#[component]
fn TaskManagerRow(item: TaskItem, mut bump: Signal<u64>) -> Element {
    let id = item.id;
    let v = task_state_value(&item.state).to_string();
    rsx! {
        div {
            class: "data-row",
            div {
                div { class: "data-title", "{item.title}" }
                div { class: "small", "{item.description}" }
                div { class: "small", {item.due_at.format("%d/%m/%Y %H:%M").to_string()} }
            }
            select {
                value: v,
                onchange: move |e: FormEvent| async move { let _ = set_task_state(id, e.value()).await; bump += 1; },
                option { value: "PLANNED", "Planifiée" }
                option { value: "READY", "Prête" }
                option { value: "RUNNING", "En cours" }
                option { value: "BLOCKED", "Bloquée" }
                option { value: "DONE", "Terminée" }
                option { value: "SKIPPED", "Ignorée" }
            }
        }
    }
}

#[component]
fn TaskRow(task: TaskItem) -> Element {
    rsx! {
        div {
            class: "task-row",
            div {
                div { class: "task-title", "{task.title}" }
                div { class: "small", {format!("{} • {}", task.code, task.due_at.format("%d/%m/%Y %H:%M"))} }
            }
            span { class: "status", "{task_state_label(&task.state)}" }
        }
    }
}

#[component]
fn AuditRow(item: AuditItem) -> Element {
    rsx! {
        div {
            class: "data-row",
            div {
                div { class: "data-title", "{item.action}" }
                div { class: "small", {format!("{} • {} • {}", item.occurred_at.format("%d/%m/%Y %H:%M"), item.actor, item.entity_type)} }
            }
            div { class: "small audit-payload", "{item.payload}" }
        }
    }
}

#[component]
pub(crate) fn ModuleHeader(title: &'static str, kicker: &'static str, detail: &'static str) -> Element {
    rsx! {
        section {
            class: "page-intro",
            div {
                div { class: "eyebrow", "{kicker}" }
                h2 { "{title}" }
                p { "{detail}" }
            }
        }
    }
}

#[component]
fn Metric(label: &'static str, value: String, tone: &'static str) -> Element {
    rsx! {
        div {
            class: "metric-card {tone}",
            div { class: "metric-label", "{label}" }
            div { class: "metric-value", "{value}" }
        }
    }
}

#[component]
fn ModuleCard(title: &'static str, value: String, label: &'static str, detail: String) -> Element {
    rsx! {
        div {
            class: "module-card",
            div { class: "eyebrow", "{title}" }
            div { class: "module-number", "{value}" }
            div { class: "small", "{label}" }
            p { "{detail}" }
        }
    }
}

#[component]
fn Check(ok: bool, title: &'static str, text: &'static str) -> Element {
    rsx! {
        div {
            class: if ok { "check ok" } else { "check" },
            span { class: "check-icon", if ok { "✓" } else { "·" } }
            div {
                strong { "{title}" }
                div { class: "small", "{text}" }
            }
        }
    }
}

#[component]
fn EmptyState(title: &'static str, text: &'static str) -> Element {
    rsx! { div { class: "empty-state", h3 { "{title}" }, p { "{text}" } } }
}

#[component]
pub(crate) fn FormField(label: &'static str, value: String, oninput: EventHandler<FormEvent>) -> Element {
    rsx! {
        label {
            class: "field",
            span { "{label}" }
            input { value: value, oninput: oninput }
        }
    }
}

#[component]
fn InfoTile(label: &'static str, value: &'static str) -> Element {
    rsx! {
        div { class: "info-tile", div { class: "small", "{label}" }, strong { "{value}" } }
    }
}

#[component]
pub(crate) fn InfoTileOwned(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "info-tile", div { class: "small", "{label}" }, strong { "{value}" } }
    }
}

#[component]
fn Loading() -> Element {
    rsx! {
        div {
            class: "loading-grid",
            div { class: "hero-card skeleton" }
            div {
                class: "metric-row",
                for _ in 0..4 {
                    div { class: "metric skeleton" }
                }
            }
        }
    }
}

pub(crate) fn euro(cents: i64) -> String {
    format!("{:.2} €", (cents as f64) / 100.0)
}
pub(crate) fn euros_to_cents(value: &str) -> i64 {
    let normalized = value.trim().replace(" ", "").replace(",", ".");
    normalized.parse::<f64>().unwrap_or(0.0).round() as i64 * 100
}
fn task_state_label(v: &TaskState) -> &'static str {
    match v {
        TaskState::Planned => "Planifiée",
        TaskState::Ready => "Prête",
        TaskState::Running => "En cours",
        TaskState::Blocked => "Bloquée",
        TaskState::Done => "Terminée",
        TaskState::Skipped => "Ignorée",
    }
}
fn task_state_value(v: &TaskState) -> &'static str {
    match v {
        TaskState::Planned => "PLANNED",
        TaskState::Ready => "READY",
        TaskState::Running => "RUNNING",
        TaskState::Blocked => "BLOCKED",
        TaskState::Done => "DONE",
        TaskState::Skipped => "SKIPPED",
    }
}



