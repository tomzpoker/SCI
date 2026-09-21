use dioxus::prelude::*;
use crate::domain::{DashboardSnapshot, ModuleCounts, SciProfile, TaskItem, TaskState};
use crate::server::{dashboard_snapshot, get_sci_profile, module_counts, run_anticipation_cycle, update_sci_profile};

const CSS: Asset = asset!("/assets/main.css");

#[derive(Clone, Copy, PartialEq)]
enum Page { Dashboard, Setup, Patrimony, Rentals, Billing, Bank, Vat, Calendar, Automations }

impl Page {
    fn label(self) -> &'static str { match self { Page::Dashboard=>"Vue d'ensemble", Page::Setup=>"Configuration SCI", Page::Patrimony=>"Patrimoine", Page::Rentals=>"Locations", Page::Billing=>"Facturation", Page::Bank=>"Banque", Page::Vat=>"TVA", Page::Calendar=>"Calendrier fiscal", Page::Automations=>"Automatisations" } }
}

#[component]
pub fn App() -> Element {
    let mut page = use_signal(|| Page::Dashboard);
    rsx! {
        document::Link { rel: "stylesheet", href: CSS }
        div { class: "app-shell",
            aside { class: "sidebar",
                div { class: "brand", "SCI FAMILY" }
                div { class: "brand-sub", "PILOTAGE AUTONOME" }
                nav {
                    NavItem { page, current: Page::Dashboard }
                    NavItem { page, current: Page::Setup }
                    NavItem { page, current: Page::Patrimony }
                    NavItem { page, current: Page::Rentals }
                    NavItem { page, current: Page::Billing }
                    NavItem { page, current: Page::Bank }
                    NavItem { page, current: Page::Vat }
                    NavItem { page, current: Page::Calendar }
                    NavItem { page, current: Page::Automations }
                }
                div { class: "sidebar-footer", "Mode autonome • données locales • audit" }
            }
            main { class: "main",
                header { class: "topbar",
                    div { div { class: "eyebrow", "SCI FAMILY PILOT" }, h1 { {page().label()} } }
                    div { class: "top-actions",
                        button { class: "secondary", onclick: move |_| page.set(Page::Setup), "Configurer la SCI" }
                        button { class: "primary", onclick: move |_| async move { let _ = run_anticipation_cycle().await; }, "Lancer l’anticipation" }
                    }
                }
                match page() {
                    Page::Dashboard => rsx! { Dashboard { on_setup: move |_| page.set(Page::Setup) } },
                    Page::Setup => rsx! { SetupPage {} },
                    Page::Patrimony => rsx! { ModulePage { title: "Patrimoine", kicker: "BIENS • LOTS • STRUCTURE", focus: "properties" } },
                    Page::Rentals => rsx! { ModulePage { title: "Locations", kicker: "BAUX • LOCATAIRES • INDEXATIONS", focus: "leases" } },
                    Page::Billing => rsx! { ModulePage { title: "Facturation", kicker: "LOYERS • QUITTANCES • ENCAISSEMENTS", focus: "invoices" } },
                    Page::Bank => rsx! { ModulePage { title: "Banque", kicker: "IMPORT • RAPPROCHEMENT • CONTRÔLE", focus: "bank" } },
                    Page::Vat => rsx! { ModulePage { title: "TVA", kicker: "ENCAISSEMENTS • EXIGIBILITÉ • DÉCLARATION", focus: "vat" } },
                    Page::Calendar => rsx! { CalendarPage {} },
                    Page::Automations => rsx! { AutomationPage {} },
                }
            }
        }
    }
}

#[component]
fn NavItem(page: Signal<Page>, current: Page) -> Element {
    rsx! { button { class: if page() == current { "nav-item active" } else { "nav-item" }, onclick: move |_| page.set(current), {current.label()} } }
}

#[component]
fn Dashboard(on_setup: EventHandler<MouseEvent>) -> Element {
    let snapshot = use_resource(|| async move { dashboard_snapshot().await.ok() });
    let counts = use_resource(|| async move { module_counts().await.ok() });
    match (&*snapshot.read(), &*counts.read()) {
        (Some(Some(data)), Some(Some(counts))) => rsx! { DashboardContent { data: data.clone(), counts: counts.clone(), on_setup } },
        _ => rsx! { div { class: "loading-grid", div { class: "hero-card skeleton" }, div { class: "metric-row", for _ in 0..4 { div { class: "metric skeleton" } } } } }
    }
}

#[component]
fn DashboardContent(data: DashboardSnapshot, counts: ModuleCounts, on_setup: EventHandler<MouseEvent>) -> Element {
    let needs_setup = data.sci_name.trim() == "SCI À CONFIGURER" || data.registered_office.trim().is_empty() || data.registered_office == "À configurer";
    rsx! {
        {if needs_setup { rsx! {
            section { class: "setup-banner",
                div { div { class: "eyebrow", "MISE EN ROUTE" }, h2 { "Votre SCI n’est pas encore configurée" }, p { "Renseignez les informations juridiques et bancaires une seule fois. Le moteur pourra ensuite préparer ses tâches sans ressaisie." } }
                button { class: "primary", onclick: on_setup, "Configurer maintenant" }
            }
        }} else { rsx! {} }}
        section { class: "welcome",
            div { span { class: "pill", "SCI À L’IR" }, span { class: "pill muted", "TVA sur encaissements" }, h2 { {data.sci_name} }, p { "Le cockpit centralise l’activité, détecte les échéances, prépare les travaux récurrents et expose ce qui nécessite réellement une intervention." } }
            div { class: "risk-block", div { class: "eyebrow", "VIGILANCE" }, div { class: "risk {risk_class(&data.risk_level)}", {data.risk_level.clone()} }, div { class: "small", "Calcul déterministe • traçable" } }
        }
        section { class: "metric-row",
            Metric { label: "Trésorerie", value: euro(data.cash_cents), tone: "positive" }
            Metric { label: "Créances ouvertes", value: euro(data.receivables_cents), tone: "neutral" }
            Metric { label: "TVA à préparer", value: euro(data.vat_to_prepare_cents), tone: "warning" }
            Metric { label: "À traiter < 30 j", value: data.tasks_due_30d.to_string(), tone: "neutral" }
        }
        section { class: "module-grid",
            ModuleCard { title: "Patrimoine", value: counts.properties.to_string(), label: "biens", detail: format!("{} lots actifs", counts.units) }
            ModuleCard { title: "Locations", value: counts.leases.to_string(), label: "baux", detail: format!("{} locataires", counts.tenants) }
            ModuleCard { title: "Facturation", value: counts.invoices.to_string(), label: "factures", detail: format!("{} encaissements", counts.payments) }
            ModuleCard { title: "Banque", value: counts.bank_transactions.to_string(), label: "mouvements", detail: format!("{} à rapprocher", counts.unmatched_bank) }
            ModuleCard { title: "Documents", value: counts.documents.to_string(), label: "pièces", detail: "archivage centralisé" }
            ModuleCard { title: "Automatisations", value: counts.enabled_automation_rules.to_string(), label: "règles actives", detail: format!("{} règles configurées", counts.automation_rules) }
        }
        section { class: "two-col",
            div { class: "panel", div { class: "panel-head", h3 { "Prochaines actions" }, span { class: "small", "Priorisées automatiquement" } },
                if data.next_actions.is_empty() { EmptyState { title: "Aucune action générée", text: "Lancez une anticipation après avoir configuré la SCI." } }
                for task in data.next_actions.iter() { TaskRow { task: task.clone() } }
            }
            div { class: "panel", div { class: "panel-head", h3 { "Trésorerie prévisionnelle" }, span { class: "small", "12 mois" } },
                div { class: "forecast-grid", for point in data.forecast.iter().take(6) { div { class: "forecast-card", div { class: "small", {point.date.format("%b %Y").to_string()} }, div { class: if point.balance_cents < 0 { "forecast-value negative" } else { "forecast-value" }, {euro(point.balance_cents)} }, div { class: "small", "solde projeté" } } } }
                div { class: "forecast-min", "Point bas projeté : ", {euro(data.forecast_min_cash_cents)} }
            }
        }
    }
}

#[component]
fn SetupPage() -> Element {
    let profile = use_resource(|| async move { get_sci_profile().await.ok() });
    let mut name = use_signal(String::new);
    let mut siren = use_signal(String::new);
    let mut siret = use_signal(String::new);
    let mut office = use_signal(String::new);
    let mut iban = use_signal(String::new);
    let mut bic = use_signal(String::new);
    let mut saved = use_signal(|| false);
    let mut initialized = use_signal(|| false);

    if !initialized() {
        if let Some(Some(p)) = &*profile.read() {
            name.set(p.legal_name.clone()); siren.set(p.siren.clone()); siret.set(p.siret.clone()); office.set(p.registered_office.clone()); iban.set(p.iban.clone()); bic.set(p.bic.clone()); initialized.set(true);
        }
    }

    rsx! {
        section { class: "page-intro", div { div { class: "eyebrow", "PARAMÈTRES MAÎTRES" }, h2 { "Configuration de la SCI" }, p { "Ces informations alimentent les documents, les contrôles et les tâches d’anticipation. Le régime fiscal de cette base est fixé à l’IR et la TVA au décaissement n’est pas utilisée : la logique est centrée sur les encaissements." } } }
        section { class: "form-grid",
            FormField { label: "Dénomination sociale", value: name(), oninput: move |e: FormEvent| name.set(e.value()) }
            FormField { label: "SIREN", value: siren(), oninput: move |e: FormEvent| siren.set(e.value()) }
            FormField { label: "SIRET", value: siret(), oninput: move |e: FormEvent| siret.set(e.value()) }
            FormField { label: "Siège social", value: office(), oninput: move |e: FormEvent| office.set(e.value()) }
            FormField { label: "IBAN", value: iban(), oninput: move |e: FormEvent| iban.set(e.value()) }
            FormField { label: "BIC", value: bic(), oninput: move |e: FormEvent| bic.set(e.value()) }
        }
        section { class: "facts-row",
            InfoTile { label: "Régime", value: "SCI à l’IR" }
            InfoTile { label: "TVA", value: "Sur encaissements" }
            InfoTile { label: "Automatisation", value: "Préparation + validation" }
            InfoTile { label: "Traçabilité", value: "Journal d’audit" }
        }
        div { class: "action-row", button { class: "primary", onclick: move |_| async move {
            let p = SciProfile { legal_name:name(), siren:siren(), siret:siret(), registered_office:office(), tax_regime:"IR".into(), vat_status:"OPTION_LOYERS".into(), vat_basis:"COLLECTION".into(), iban:iban(), bic:bic() };
            saved.set(update_sci_profile(p).await.is_ok());
        }, "Enregistrer la configuration" }, if saved() { span { class: "save-ok", "Configuration enregistrée" } } }
    }
}

#[component]
fn ModulePage(title: &'static str, kicker: &'static str, focus: &'static str) -> Element {
    let counts = use_resource(|| async move { module_counts().await.ok() });
    let snapshot = counts.read();
    let value = snapshot.as_ref().and_then(|x| x.as_ref()).map(|c| match focus { "properties"=>c.properties, "leases"=>c.leases, "invoices"=>c.invoices, "bank"=>c.bank_transactions, "vat"=>c.payments, _=>0 });
    rsx! { section { class: "page-intro", div { div { class: "eyebrow", {kicker} }, h2 { {title} }, p { "Ce module est prêt à recevoir les opérations de la SCI. Les indicateurs sont déjà branchés sur la base locale ; les prochaines actions seront ajoutées ici sans modifier le cœur métier." } } }
        section { class: "hero-module", div { class: "hero-module-number", {value.unwrap_or(0).to_string()} }, div { class: "hero-module-copy", h3 { "Éléments enregistrés" }, p { "Le nombre ci-dessus provient directement de PostgreSQL." } }, button { class: "secondary", "Ajouter" } }
        section { class: "empty-workspace", h3 { "Espace opérationnel" }, p { "Aucune donnée métier n’est encore saisie dans cette base neuve. C’est volontaire : on construit votre SCI depuis une configuration propre, puis les automatismes se déclencheront à partir de vos données réelles." }, div { class: "workspace-steps", Step { n:"01", t:"Configurer" }, Step { n:"02", t:"Importer / saisir" }, Step { n:"03", t:"Contrôler" }, Step { n:"04", t:"Automatiser" } } }
    }
}

#[component]
fn CalendarPage() -> Element {
    let counts = use_resource(|| async move { module_counts().await.ok() });
    rsx! { section { class: "page-intro", div { div { class: "eyebrow", "ANTICIPATION FISCALE" }, h2 { "Calendrier fiscal" }, p { "Les échéances fiscales seront centralisées ici avec leur source, leur période, leur statut et les tâches préparatoires associées." } } }
        section { class: "hero-module", div { class: "hero-module-number", {counts.read().as_ref().and_then(|x| x.as_ref()).map(|c| c.tax_deadlines).unwrap_or(0).to_string()} }, div { class: "hero-module-copy", h3 { "Échéances à venir" }, p { "Aucune échéance n’est encore personnalisée dans cette base neuve." } }, button { class: "secondary", "Ajouter une échéance" } }
    }
}

#[component]
fn AutomationPage() -> Element {
    let mut result = use_signal(|| None::<crate::server::AutomationRunResult>);
    let counts = use_resource(|| async move { module_counts().await.ok() });
    rsx! { section { class: "page-intro", div { div { class: "eyebrow", "MOTEUR DE RÈGLES" }, h2 { "Automatisations" }, p { "Chaque règle a une échéance, une priorité, un horizon et une trace d’audit. L’application prépare ; le gérant conserve la validation finale quand elle est nécessaire." } } button { class: "primary", onclick: move |_| async move { result.set(run_anticipation_cycle().await.ok()); }, "Exécuter maintenant" } }
        section { class: "automation-grid", AutomationTile { name:"Préparer les loyers", code:"RENT_INVOICE", days:"45 j" }, AutomationTile { name:"Rapprocher les encaissements", code:"PAYMENT_RECONCILIATION", days:"30 j" }, AutomationTile { name:"Préparer la TVA", code:"VAT_COLLECTION", days:"90 j" }, AutomationTile { name:"Préparer la clôture", code:"ANNUAL_CLOSE", days:"180 j" }, AutomationTile { name:"Anticiper les baux", code:"LEASE_REVIEW", days:"180 j" }, AutomationTile { name:"Surveiller les assurances", code:"INSURANCE_EXPIRY", days:"180 j" } }
        if let Some(r) = result() { section { class: "run-result", h3 { "Dernier cycle" }, p { {format!("{} règles évaluées • {} tâches créées • {}", r.evaluated_rules, r.created_tasks, r.ran_at.format("%d/%m/%Y %H:%M"))} } } }
        section { class: "panel", div { class: "panel-head", h3 { "État du moteur" }, span { class: "small", {format!("{} règles actives", counts.read().as_ref().and_then(|x| x.as_ref()).map(|c| c.enabled_automation_rules).unwrap_or(0))} } }, p { class: "small", "Le cycle automatique tourne côté serveur et reste idempotent : une même occurrence ne recrée pas une tâche existante." } }
    }
}

#[component] fn AutomationTile(name: &'static str, code: &'static str, days: &'static str) -> Element { rsx! { div { class: "automation-tile", div { class: "code", {code} }, h3 { {name} }, div { class: "small", "Horizon ", {days} }, div { class: "dotline", span {}, "Préparation automatique" } } } }
#[component] fn Metric(label: &'static str, value: String, tone: &'static str) -> Element { rsx! { div { class: "metric-card {tone}", div { class: "metric-label", {label} }, div { class: "metric-value", {value} } } } }
#[component] fn ModuleCard(title: &'static str, value: String, label: &'static str, detail: String) -> Element { rsx! { div { class: "module-card", div { class: "eyebrow", {title} }, div { class: "module-number", {value} }, div { class: "small", {label} }, p { {detail} } } } }
#[component] fn TaskRow(task: TaskItem) -> Element { rsx! { div { class: "task-row", div { class: "task-main", div { class: "task-title", {task.title} }, div { class: "small", "Échéance : ", {task.due_at.format("%d/%m/%Y %H:%M").to_string()} } }, div { class: state_class(&task.state), {state_label(&task.state)} } } } }
#[component] fn EmptyState(title: &'static str, text: &'static str) -> Element { rsx! { div { class: "empty-state", h3 { {title} }, p { {text} } } } }
#[component] fn FormField(label: &'static str, value: String, oninput: EventHandler<FormEvent>) -> Element { rsx! { label { class: "field", span { {label} }, input { value: value, oninput: oninput } } } }
#[component] fn InfoTile(label: &'static str, value: &'static str) -> Element { rsx! { div { class: "info-tile", div { class: "small", {label} }, strong { {value} } } } }
#[component] fn Step(n: &'static str, t: &'static str) -> Element { rsx! { div { class: "step", span { {n} }, strong { {t} } } } }

fn euro(cents: i64) -> String { let sign = if cents < 0 { "−" } else { "" }; let abs = cents.abs(); format!("{sign}{}, {:02} €", abs / 100, abs % 100).replace(", ", ",") }
fn risk_class(risk: &str) -> &'static str { match risk { "CRITICAL"=>"risk-critical", "WATCH"=>"risk-watch", _=>"risk-normal" } }
fn state_class(state: &TaskState) -> &'static str { match state { TaskState::Blocked=>"status danger", TaskState::Ready=>"status ready", TaskState::Running=>"status running", _=>"status planned" } }
fn state_label(state: &TaskState) -> &'static str { match state { TaskState::Blocked=>"Bloquée", TaskState::Ready=>"Prête", TaskState::Running=>"En cours", TaskState::Planned=>"Planifiée", TaskState::Done=>"Terminée", TaskState::Skipped=>"Ignorée" } }
