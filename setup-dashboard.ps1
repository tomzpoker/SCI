# ============================================================
#  SCI Family Pilot - Integration Dashboard Modulaire
#  Dioxus 0.7 - Non destructif (sauvegarde ui.rs avant modif)
# ============================================================

$ErrorActionPreference = "Stop"
$root = "D:\SCI\DEV\SCI-family-rust"

if (-not (Test-Path $root)) {
    Write-Host "ERREUR : Le dossier $root n'existe pas." -ForegroundColor Red
    exit 1
}

Set-Location $root

# --- 1. Sauvegarde de ui.rs ---
$uiPath = "src\ui.rs"
$uiBak  = "src\ui.rs.bak"
if (Test-Path $uiPath) {
    Copy-Item $uiPath $uiBak -Force
    Write-Host "  [BACKUP] $uiPath -> $uiBak" -ForegroundColor Yellow
} else {
    Write-Host "ERREUR : $uiPath introuvable. Verifie ton arborescence." -ForegroundColor Red
    exit 1
}

# --- 2. Creation du dossier dashboard ---
$dashDir = "src\ui\dashboard"
if (-not (Test-Path $dashDir)) {
    New-Item -ItemType Directory -Path $dashDir -Force | Out-Null
    Write-Host "  [DIR]  $dashDir" -ForegroundColor Cyan
}

# --- 3. Fichier : assets\dashboard.css ---
$css = @'
/* === Dashboard Modulaire SCI === */
.dash-grid {
    display: grid;
    grid-template-columns: repeat(12, 1fr);
    gap: 16px;
    padding: 4px 0;
}

.dash-widget {
    background: #1e293b;
    border-radius: 12px;
    padding: 16px;
    border: 1px solid #334155;
    display: flex;
    flex-direction: column;
    transition: transform 0.2s, box-shadow 0.2s;
    cursor: grab;
    position: relative;
}

.dash-widget:active { cursor: grabbing; }
.dash-widget.dragging { opacity: 0.5; transform: scale(1.02); }
.dash-widget.pinned { cursor: default; border-color: #38bdf8; }

.dash-widget-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
    border-bottom: 1px solid #334155;
    padding-bottom: 8px;
}

.dash-widget-header h3 {
    margin: 0;
    font-size: 0.9rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #94a3b8;
}

.dash-pin-btn {
    background: none;
    border: none;
    color: #64748b;
    cursor: pointer;
    font-size: 1.1rem;
    padding: 2px 6px;
    border-radius: 4px;
    transition: color 0.2s, background 0.2s;
}

.dash-pin-btn:hover { background: #334155; }
.dash-pin-btn.pinned { color: #38bdf8; }

.dash-chart-container { flex: 1; position: relative; min-height: 160px; }
.dash-chart-container svg { width: 100%; height: 100%; overflow: visible; }

.dash-bar { cursor: pointer; transition: opacity 0.2s; }
.dash-bar:hover { opacity: 0.8; }

.dash-modal-overlay {
    position: fixed; top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex; justify-content: center; align-items: center;
    z-index: 1000;
}

.dash-modal {
    background: #1e293b; padding: 24px; border-radius: 12px;
    max-width: 520px; width: 90%; border: 1px solid #334155;
    box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5);
}

.dash-modal h2 { margin-top: 0; color: #38bdf8; }
.dash-modal-close {
    background: #ef4444; color: white; border: none;
    padding: 8px 16px; border-radius: 6px; cursor: pointer; margin-top: 16px;
}

.dash-widget-4 { grid-column: span 4; }
.dash-widget-6 { grid-column: span 6; }
.dash-widget-8 { grid-column: span 8; }
.dash-widget-12 { grid-column: span 12; }

@media (max-width: 900px) {
    .dash-widget-4, .dash-widget-6, .dash-widget-8 {
        grid-column: span 12;
    }
}
'@
Set-Content -Path "assets\dashboard.css" -Value $css -Encoding UTF8
Write-Host "  [FILE] assets\dashboard.css" -ForegroundColor Green

# ============================================================
#  FICHIERS DU MODULE DASHBOARD
# ============================================================

# --- models.rs ---
$content = @'
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tenant {
    pub id: usize,
    pub name: String,
    pub property: String,
    pub rent: f64,
    pub balance: f64,
    pub status: TenantStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TenantStatus { Paid, Late, Unpaid, Vacant }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: usize,
    pub label: String,
    pub due_in_days: i32,
    pub category: TaskCategory,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TaskCategory { Payment, Relance, Tax, Declaration }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ForecastPoint {
    pub label: String,
    pub planned: f64,
    pub actual: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub struct WidgetState {
    pub id: &'static str,
    pub title: &'static str,
    pub col_span: u8,
    pub pinned: bool,
    pub order: usize,
}
'@
Set-Content -Path "src\ui\dashboard\models.rs" -Value $content -Encoding UTF8
Write-Host "  [FILE] src\ui\dashboard\models.rs" -ForegroundColor Green

# --- modal.rs ---
$content = @'
use dioxus::prelude::*;
use super::models::Tenant;

#[component]
pub fn TenantModal(
    tenant: Signal<Option<Tenant>>,
    on_close: EventHandler<()>,
) -> Element {
    let Some(t) = tenant() else {
        return rsx! { div {} };
    };

    let status_color = match t.status {
        super::models::TenantStatus::Paid => "#22c55e",
        super::models::TenantStatus::Late => "#f59e0b",
        super::models::TenantStatus::Unpaid => "#ef4444",
        super::models::TenantStatus::Vacant => "#64748b",
    };

    rsx! {
        div {
            class: "dash-modal-overlay",
            onclick: move |_| on_close.call(()),
            div {
                class: "dash-modal",
                onclick: move |e| e.stop_propagation(),
                h2 { "{t.name}" }
                p { strong { "Local : " } "{t.property}" }
                p { strong { "Loyer : " } "{t.rent:.2} EUR" }
                p {
                    strong { "Solde : " }
                    span { style: "color: {status_color};", "{t.balance:.2} EUR" }
                }
                p { strong { "Statut : " } "{t.status:?}" }

                if t.balance < 0.0 {
                    div {
                        style: "background: #450a0a; padding: 12px; border-radius: 8px; margin-top: 12px; border-left: 4px solid #ef4444;",
                        h4 { style: "margin: 0 0 8px 0; color: #ef4444;", "Detail des impayes" }
                        ul { style: "margin: 0; padding-left: 20px;",
                            li { "Loyer Janvier : -{t.rent:.2} EUR" }
                            li { "Loyer Fevrier : -{t.rent:.2} EUR" }
                            li { "Charges : -150.00 EUR" }
                        }
                    }
                } else {
                    p { style: "color: #22c55e;", "Aucun impaye" }
                }

                button {
                    class: "dash-modal-close",
                    onclick: move |_| on_close.call(()),
                    "Fermer"
                }
            }
        }
    }
}
'@
Set-Content -Path "src\ui\dashboard\modal.rs" -Value $content -Encoding UTF8
Write-Host "  [FILE] src\ui\dashboard\modal.rs" -ForegroundColor Green

# --- forecast.rs ---
$content = @'
use dioxus::prelude::*;
use super::models::ForecastPoint;

#[component]
pub fn ForecastChart(data: Vec<ForecastPoint>) -> Element {
    let width = 400.0;
    let height = 200.0;
    let padding = 30.0;

    let max_val = data.iter().map(|d| d.planned.max(d.actual)).fold(0.0, f64::max);
    let max_val = if max_val == 0.0 { 100.0 } else { max_val * 1.2 };

    let points_planned: Vec<(f64, f64)> = data.iter().enumerate().map(|(i, d)| {
        let x = padding + (i as f64 / (data.len() - 1) as f64) * (width - 2.0 * padding);
        let y = height - padding - (d.planned / max_val) * (height - 2.0 * padding);
        (x, y)
    }).collect();

    let points_actual: Vec<(f64, f64)> = data.iter().enumerate().map(|(i, d)| {
        let x = padding + (i as f64 / (data.len() - 1) as f64) * (width - 2.0 * padding);
        let y = height - padding - (d.actual / max_val) * (height - 2.0 * padding);
        (x, y)
    }).collect();

    let path_planned = points_planned.iter().enumerate().map(|(i, (x, y))| {
        if i == 0 { format!("M {},{}", x, y) } else { format!(" L {},{}", x, y) }
    }).collect::<String>();

    let path_actual = points_actual.iter().enumerate().map(|(i, (x, y))| {
        if i == 0 { format!("M {},{}", x, y) } else { format!(" L {},{}", x, y) }
    }).collect::<String>();

    rsx! {
        div { class: "dash-chart-container",
            svg { view_box: "0 0 {width} {height}",
                line { x1: "{padding}", y1: "{height - padding}", x2: "{width - padding}", y2: "{height - padding}", stroke: "#334155", stroke_width: "1" }
                line { x1: "{padding}", y1: "{padding}", x2: "{padding}", y2: "{height - padding}", stroke: "#334155", stroke_width: "1" }

                path { d: "{path_planned}", fill: "none", stroke: "#38bdf8", stroke_width: "2", stroke_dasharray: "5,5" }
                path { d: "{path_actual}", fill: "none", stroke: "#22c55e", stroke_width: "2" }

                for (x, y) in points_planned.iter() {
                    circle { cx: "{x}", cy: "{y}", r: "3", fill: "#38bdf8" }
                }
                for (x, y) in points_actual.iter() {
                    circle { cx: "{x}", cy: "{y}", r: "3", fill: "#22c55e" }
                }

                for (i, d) in data.iter().enumerate() {
                    {
                        let x = padding + (i as f64 / (data.len() - 1) as f64) * (width - 2.0 * padding);
                        rsx! {
                            text { x: "{x}", y: "{height - 10.0}", fill: "#64748b", font_size: "10", text_anchor: "middle", "{d.label}" }
                        }
                    }
                }
            }
        }
    }
}
'@
Set-Content -Path "src\ui\dashboard\forecast.rs" -Value $content -Encoding UTF8
Write-Host "  [FILE] src\ui\dashboard\forecast.rs" -ForegroundColor Green

# --- tenants.rs ---
$content = @'
use dioxus::prelude::*;
use super::models::{Tenant, TenantStatus};

#[component]
pub fn TenantBars(
    tenants: Vec<Tenant>,
    on_select: EventHandler<Tenant>,
) -> Element {
    let max_abs_balance = tenants.iter().map(|t| t.balance.abs()).fold(0.0, f64::max);
    let max_abs_balance = if max_abs_balance == 0.0 { 100.0 } else { max_abs_balance };

    rsx! {
        div {
            style: "display: flex; justify-content: space-around; align-items: flex-end; height: 150px; gap: 10px; padding-top: 20px;",
            for t in tenants {
                {
                    let height_pct = (t.balance.abs() / max_abs_balance) * 100.0;
                    let color = match t.status {
                        TenantStatus::Paid => "#22c55e",
                        TenantStatus::Late => "#f59e0b",
                        TenantStatus::Unpaid => "#ef4444",
                        TenantStatus::Vacant => "#64748b",
                    };
                    let tenant_clone = t.clone();
                    rsx! {
                        div {
                            style: "display: flex; flex-direction: column; align-items: center; flex: 1; cursor: pointer;",
                            onclick: move |_| on_select.call(tenant_clone.clone()),
                            div {
                                style: "width: 100%; height: {height_pct * 1.2}px; background: {color}; border-radius: 4px 4px 0 0; transition: height 0.3s; min-height: 4px;"
                            }
                            span { style: "font-size: 0.7rem; margin-top: 5px; color: #94a3b8; text-align: center;", "{t.name}" }
                            span { style: "font-size: 0.7rem; color: #64748b;", "{t.balance:.0}EUR" }
                        }
                    }
                }
            }
        }
    }
}
'@
Set-Content -Path "src\ui\dashboard\tenants.rs" -Value $content -Encoding UTF8
Write-Host "  [FILE] src\ui\dashboard\tenants.rs" -ForegroundColor Green

# --- tasks.rs ---
$content = @'
use dioxus::prelude::*;
use super::models::{Task, TaskCategory};

#[component]
pub fn TaskList(tasks: Vec<Task>) -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 8px;",
            for t in tasks {
                {
                    let (color, bg) = match t.category {
                        TaskCategory::Payment => ("#ef4444", "#450a0a"),
                        TaskCategory::Relance => ("#f59e0b", "#451a03"),
                        TaskCategory::Tax => ("#38bdf8", "#082f49"),
                        TaskCategory::Declaration => ("#22c55e", "#052e16"),
                    };
                    rsx! {
                        div {
                            style: "background: {bg}; border-left: 4px solid {color}; padding: 10px; border-radius: 4px; display: flex; justify-content: space-between; align-items: center;",
                            span { style: "font-size: 0.85rem;", "{t.label}" }
                            span { style: "font-size: 0.75rem; color: #94a3b8;", "Dans {t.due_in_days} jours" }
                        }
                    }
                }
            }
        }
    }
}
'@
Set-Content -Path "src\ui\dashboard\tasks.rs" -Value $content -Encoding UTF8
Write-Host "  [FILE] src\ui\dashboard\tasks.rs" -ForegroundColor Green

# --- dashboard.rs (composant principal) ---
$content = @'
use dioxus::prelude::*;
use super::models::*;
use super::forecast::ForecastChart;
use super::tenants::TenantBars;
use super::tasks::TaskList;
use super::modal::TenantModal;

#[component]
pub fn DashboardWidgets() -> Element {
    let tenants = vec![
        Tenant { id: 1, name: "M. Dupont".into(), property: "Local A".into(), rent: 800.0, balance: 0.0, status: TenantStatus::Paid },
        Tenant { id: 2, name: "Mme Martin".into(), property: "Local B".into(), rent: 950.0, balance: -300.0, status: TenantStatus::Late },
        Tenant { id: 3, name: "SCI Immo".into(), property: "Local C".into(), rent: 1200.0, balance: -800.0, status: TenantStatus::Unpaid },
        Tenant { id: 4, name: "M. Petit".into(), property: "Local D".into(), rent: 700.0, balance: 0.0, status: TenantStatus::Paid },
        Tenant { id: 5, name: "Vacant".into(), property: "Local E".into(), rent: 0.0, balance: 0.0, status: TenantStatus::Vacant },
    ];

    let tasks = vec![
        Task { id: 1, label: "Paiement a effectuer : Taxe fonciere".into(), due_in_days: 3, category: TaskCategory::Payment },
        Task { id: 2, label: "Relance a effectuer : Mme Martin".into(), due_in_days: 12, category: TaskCategory::Relance },
        Task { id: 3, label: "Declaration TVA".into(), due_in_days: 2, category: TaskCategory::Tax },
        Task { id: 4, label: "Declaration annuelle (liasse 2072)".into(), due_in_days: 128, category: TaskCategory::Declaration },
    ];

    let forecast_data = vec![
        ForecastPoint { label: "1m".into(), planned: 1000.0, actual: 950.0 },
        ForecastPoint { label: "2m".into(), planned: 2000.0, actual: 1800.0 },
        ForecastPoint { label: "3m".into(), planned: 3000.0, actual: 3100.0 },
        ForecastPoint { label: "6m".into(), planned: 6000.0, actual: 5800.0 },
        ForecastPoint { label: "1a".into(), planned: 12000.0, actual: 11500.0 },
        ForecastPoint { label: "3a".into(), planned: 36000.0, actual: 0.0 },
    ];

    let mut selected_tenant = use_signal(|| None::<Tenant>);
    let mut dragged_id = use_signal(|| None::<String>);
    let mut widgets = use_signal(|| vec![
        WidgetState { id: "forecast", title: "PREVISIONNEL", col_span: 8, pinned: false, order: 0 },
        WidgetState { id: "tenants", title: "LOCAUX & SOLDES", col_span: 4, pinned: false, order: 1 },
        WidgetState { id: "tasks", title: "TACHES ADMINISTRATIVES", col_span: 6, pinned: false, order: 2 },
    ]);

    rsx! {
        section {
            class: "dash-grid",
            for widget in widgets() {
                {
                    let is_dragging = dragged_id() == Some(widget.id.to_string());
                    let is_pinned = widget.pinned;
                    let id_str = widget.id.to_string();
                    let id_for_drop = id_str.clone();
                    let id_for_pin = widget.id;

                    let class = format!(
                        "dash-widget dash-widget-{} {} {}",
                        widget.col_span,
                        if is_dragging { "dragging" } else { "" },
                        if is_pinned { "pinned" } else { "" }
                    );

                    rsx! {
                        div {
                            class: "{class}",
                            draggable: if is_pinned { "false" } else { "true" },
                            ondragstart: move |_| {
                                if !is_pinned {
                                    dragged_id.set(Some(id_str.clone()));
                                }
                            },
                            ondragover: move |e| { e.prevent_default(); },
                            ondrop: move |_| {
                                if let Some(source_id) = dragged_id() {
                                    if source_id != id_for_drop {
                                        widgets.with_mut(|w| {
                                            if let (Some(si), Some(ti)) = (
                                                w.iter().position(|x| x.id == source_id),
                                                w.iter().position(|x| x.id == id_for_drop)
                                            ) {
                                                w.swap(si, ti);
                                                for (i, wgt) in w.iter_mut().enumerate() {
                                                    wgt.order = i;
                                                }
                                            }
                                        });
                                    }
                                }
                                dragged_id.set(None);
                            },

                            div { class: "dash-widget-header",
                                h3 { "{widget.title}" }
                                button {
                                    class: if is_pinned { "dash-pin-btn pinned" } else { "dash-pin-btn" },
                                    onclick: move |_| {
                                        widgets.with_mut(|w| {
                                            if let Some(wgt) = w.iter_mut().find(|x| x.id == id_for_pin) {
                                                wgt.pinned = !wgt.pinned;
                                            }
                                        });
                                    },
                                    title: if is_pinned { "Desepingler" } else { "Epingler" },
                                    if is_pinned { "PIN" } else { "LOC" }
                                }
                            }

                            match widget.id {
                                "forecast" => rsx! { ForecastChart { data: forecast_data.clone() } },
                                "tenants" => rsx! { TenantBars { tenants: tenants.clone(), on_select: move |t| selected_tenant.set(Some(t)) } },
                                "tasks" => rsx! { TaskList { tasks: tasks.clone() } },
                                _ => rsx! { div { "Widget inconnu" } },
                            }
                        }
                    }
                }
            }

            TenantModal { tenant: selected_tenant, on_close: move |_| selected_tenant.set(None) }
        }
    }
}
'@
Set-Content -Path "src\ui\dashboard\dashboard.rs" -Value $content -Encoding UTF8
Write-Host "  [FILE] src\ui\dashboard\dashboard.rs" -ForegroundColor Green

# --- mod.rs ---
$content = @'
pub mod models;
pub mod forecast;
pub mod tenants;
pub mod tasks;
pub mod modal;
pub mod dashboard;

pub use dashboard::DashboardWidgets;
'@
Set-Content -Path "src\ui\dashboard\mod.rs" -Value $content -Encoding UTF8
Write-Host "  [FILE] src\ui\dashboard\mod.rs" -ForegroundColor Green

# ============================================================
#  4. MODIFICATION DE ui.rs
# ============================================================

$uiContent = Get-Content $uiPath -Raw -Encoding UTF8

# 4a. Ajouter "mod dashboard;" apres "use dioxus::prelude::*;"
if ($uiContent -notmatch "mod dashboard;") {
    $uiContent = $uiContent -replace "(use dioxus::prelude::\*;)", "`$1`r`nmod dashboard;"
    Write-Host "  [PATCH] ajout de 'mod dashboard;' dans ui.rs" -ForegroundColor Magenta
} else {
    Write-Host "  [SKIP] 'mod dashboard;' deja present dans ui.rs" -ForegroundColor DarkGray
}

# 4b. Ajouter la liaison CSS dashboard dans le composant App
$cssLink = 'document::Link{rel:"stylesheet",href:CSS},document::Link{rel:"stylesheet",href:asset!("/assets/dashboard.css")},'
if ($uiContent -notmatch 'dashboard\.css') {
    $uiContent = $uiContent -replace '(document::Link\{rel:"stylesheet",href:CSS\},)', $cssLink
    Write-Host "  [PATCH] ajout du lien CSS dashboard.css dans App" -ForegroundColor Magenta
} else {
    Write-Host "  [SKIP] lien CSS dashboard.css deja present" -ForegroundColor DarkGray
}

# 4c. Injecter DashboardWidgets dans la page Dashboard
$oldDashboard = 'Page::Dashboard=>rsx!\{Dashboard\{refresh,on_setup:move \|_\|page\.set\(Page::Setup\)\}\},'
$newDashboard = 'Page::Dashboard=>rsx!{Dashboard{refresh,on_setup:move |_|page.set(Page::Setup)}dashboard::DashboardWidgets{}},'
if ($uiContent -match $oldDashboard) {
    $uiContent = $uiContent -replace $oldDashboard, $newDashboard
    Write-Host "  [PATCH] injection de DashboardWidgets dans la page Dashboard" -ForegroundColor Magenta
} elseif ($uiContent -match 'dashboard::DashboardWidgets') {
    Write-Host "  [SKIP] DashboardWidgets deja injecte" -ForegroundColor DarkGray
} else {
    Write-Host "  [WARN] Impossible de trouver la ligne 'Page::Dashboard=>rsx!{Dashboard{...}}'. Verifie manuellement dans ui.rs." -ForegroundColor Yellow
}

# Ecriture du fichier modifie
Set-Content -Path $uiPath -Value $uiContent -Encoding UTF8
Write-Host "  [WRITE] $uiPath mis a jour" -ForegroundColor Green

Write-Host ""
Write-Host "===============================================" -ForegroundColor Yellow
Write-Host " Integration terminee !" -ForegroundColor Green
Write-Host "===============================================" -ForegroundColor Yellow
Write-Host ""
Write-Host "Fichiers crees :" -ForegroundColor Cyan
Write-Host "  - src\ui\dashboard\mod.rs" -ForegroundColor Gray
Write-Host "  - src\ui\dashboard\models.rs" -ForegroundColor Gray
Write-Host "  - src\ui\dashboard\forecast.rs" -ForegroundColor Gray
Write-Host "  - src\ui\dashboard\tenants.rs" -ForegroundColor Gray
Write-Host "  - src\ui\dashboard\tasks.rs" -ForegroundColor Gray
Write-Host "  - src\ui\dashboard\modal.rs" -ForegroundColor Gray
Write-Host "  - src\ui\dashboard\dashboard.rs" -ForegroundColor Gray
Write-Host "  - assets\dashboard.css" -ForegroundColor Gray
Write-Host ""
Write-Host "Fichier modifie :" -ForegroundColor Cyan
Write-Host "  - src\ui.rs (sauvegarde : src\ui.rs.bak)" -ForegroundColor Gray
Write-Host ""
Write-Host "Pour lancer :" -ForegroundColor Cyan
Write-Host "  dx serve" -ForegroundColor White
Write-Host ""
Write-Host "En cas de probleme, restaure ui.rs avec :" -ForegroundColor Yellow
Write-Host "  Copy-Item src\ui.rs.bak src\ui.rs -Force" -ForegroundColor Gray
Write-Host ""