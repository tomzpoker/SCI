param(
    [switch]$Apply
)

$ErrorActionPreference = 'Stop'

# ============================================================================
# CONFIGURATION STRICTE
# ============================================================================

$Project = 'D:\SCI\DEV\SCI-family-rust'

if ((Get-Location).Path -ne $Project) {
    throw "Lance ce script depuis $Project uniquement. Aucun autre chemin n'est autorisé."
}

$UiPath      = Join-Path $Project 'src\ui.rs'
$CssPath     = Join-Path $Project 'assets\main.css'
$OverlayPath = Join-Path $PSScriptRoot '..\src\ui_cash_overlay.rs'

if (!(Test-Path -LiteralPath $UiPath)) {
    throw "Fichier UI introuvable : $UiPath"
}

if (!(Test-Path -LiteralPath $CssPath)) {
    throw "Fichier CSS introuvable : $CssPath"
}

if (!(Test-Path -LiteralPath $OverlayPath)) {
    throw "Overlay Rust introuvable : $OverlayPath"
}

# ============================================================================
# LECTURE UNIQUEMENT
# ============================================================================

$uiText      = Get-Content -LiteralPath $UiPath -Raw -Encoding UTF8
$cssText     = Get-Content -LiteralPath $CssPath -Raw -Encoding UTF8
$overlayText = Get-Content -LiteralPath $OverlayPath -Raw -Encoding UTF8

if ([string]::IsNullOrWhiteSpace($uiText)) {
    throw "src\ui.rs est vide."
}

if ([string]::IsNullOrWhiteSpace($cssText)) {
    throw "assets\main.css est vide."
}

if ([string]::IsNullOrWhiteSpace($overlayText)) {
    throw "src\ui_cash_overlay.rs est vide."
}

# ============================================================================
# PROTECTION CONTRE UNE MAUVAISE VERSION DE L'OVERLAY
# ============================================================================

$overlayRequired = @(
    'TreasuryDualCurveChart',
    'dashboard_real_cash_history',
    'TreasuryForecastPointItem'
)

foreach ($fragment in $overlayRequired) {
    if ($overlayText.IndexOf($fragment, [StringComparison]::Ordinal) -lt 0) {
        throw "Overlay incomplet : élément absent -> $fragment"
    }
}

# ============================================================================
# ANCRES DU UI.RS ACTUEL
# ============================================================================

$anchors = @(
    'fn AuthenticatedShell(',
    'fn Dashboard(',
    'fn ZeroSaisieSummary(',
    'NavItem{page,current:Page::Dashboard}',
    'Page::Security'
)

foreach ($anchor in $anchors) {
    if ($uiText.IndexOf($anchor, [StringComparison]::Ordinal) -lt 0) {
        throw "Anchor UI introuvable : $anchor"
    }
}

# ============================================================================
# IMPORT Btreemap
# ============================================================================

if ($uiText.IndexOf(
    'use std::collections::BTreeMap;',
    [StringComparison]::Ordinal
) -lt 0) {

    $oldImport = 'use dioxus::prelude::*;'
    $newImport = "use dioxus::prelude::*;`r`nuse std::collections::BTreeMap;"

    if ($uiText.IndexOf($oldImport, [StringComparison]::Ordinal) -lt 0) {
        throw "Import dioxus introuvable pour injection de BTreeMap."
    }

    $uiText = $uiText.Replace(
        $oldImport,
        $newImport
    )
}

# ============================================================================
# LABELS UTILISATEUR
# ============================================================================

$uiText = [regex]::Replace(
    $uiText,
    '(?m)Page::Dashboard\s*=>\s*"[^"]*",',
    'Page::Dashboard => "Accueil",',
    1
)

$uiText = [regex]::Replace(
    $uiText,
    '(?m)Page::ZeroSaisie\s*=>\s*"[^"]*",',
    'Page::ZeroSaisie => "À vérifier",',
    1
)

$uiText = [regex]::Replace(
    $uiText,
    '(?m)Page::Patrimony\s*=>\s*"[^"]*",',
    'Page::Patrimony => "Biens",',
    1
)

$uiText = [regex]::Replace(
    $uiText,
    '(?m)Page::Bank\s*=>\s*"[^"]*",',
    'Page::Bank => "Argent",',
    1
)

$uiText = [regex]::Replace(
    $uiText,
    '(?m)Page::Calendar\s*=>\s*"[^"]*",',
    'Page::Calendar => "Échéances",',
    1
)

# ============================================================================
# NAVIGATION SIMPLIFIÉE
# ============================================================================

$navPattern = '(?s)nav\s*\{.*?\},div\s*\{class:"sidebar-footer"'

$navReplacement = @'
nav {
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
    },div {class:"sidebar-footer"
'@

$updated = [regex]::Replace(
    $uiText,
    $navPattern,
    $navReplacement,
    1
)

if ($updated -eq $uiText) {
    throw "Impossible de remplacer la navigation."
}

$uiText = $updated

# ============================================================================
# REMPLACEMENT DU DASHBOARD
# ============================================================================

$dashPattern = '(?s)#\[component\]\s*fn Dashboard\(.*?\n\#\[component\]\s*fn ZeroSaisieSummary'

$newDashboard = @'
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
                            class: "risk {format!("risk-{}", d.risk_level.to_lowercase())}",
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

'@

$updated = [regex]::Replace(
    $uiText,
    $dashPattern,
    ($newDashboard + "#[component]`r`nfn ZeroSaisieSummary"),
    1
)

if ($updated -eq $uiText) {
    throw "Impossible de remplacer le Dashboard."
}

$uiText = $updated

# ============================================================================
# INJECTION OVERLAY
# ============================================================================

if ($uiText.IndexOf(
    'pub async fn dashboard_real_cash_history',
    [StringComparison]::Ordinal
) -lt 0) {

    $marker = "#[component]`r`nfn ZeroSaisieSummary"

    if ($uiText.IndexOf(
        $marker,
        [StringComparison]::Ordinal
    ) -lt 0) {
        throw "Point d'injection ZeroSaisieSummary introuvable."
    }

    $overlayBlock = ($overlayText.TrimEnd() -replace "(?m)^\s*use std::collections::BTreeMap;\s*\r?\n?", "")

    $uiText = $uiText.Replace(
        $marker,
        ($overlayBlock + "`r`n`r`n" + $marker)
    )
}

# ============================================================================
# VALIDATION FINALE EN MÉMOIRE
# ============================================================================
# AUCUN FICHIER N'EST ENCORE MODIFIÉ À CE STADE.
# ============================================================================

$requiredPatterns = @(
    'Page::Dashboard\s*=>\s*"Accueil"',
    'Page::ZeroSaisie\s*=>\s*"À vérifier"',
    'Page::Patrimony\s*=>\s*"Biens"',
    'Page::Bank\s*=>\s*"Argent"',
    'Page::Calendar\s*=>\s*"Échéances"',
    '🛠️ Outils avancés',
    'TreasuryDualCurveChart',
    'dashboard_real_cash_history\(12\)',
    'build_treasury_forecast'
)

foreach ($pattern in $requiredPatterns) {
    if ($uiText -notmatch $pattern) {
        throw "Vérification UI échouée : motif absent -> $pattern"
    }
}

# ============================================================================
# PROTECTION ANTI-DOUBLON CSS
# ============================================================================

$cssMarker = 'SCI FAMILY UI OVERLAY v0.2'

# ============================================================================
# MODE DRY RUN
# ============================================================================

if (!$Apply) {
    Write-Host ""
    Write-Host "============================================================" -ForegroundColor Cyan
    Write-Host "SCI FAMILY UI OVERLAY" -ForegroundColor Cyan
    Write-Host "============================================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "DRY RUN OK" -ForegroundColor Green
    Write-Host ""
    Write-Host "AUCUN fichier du projet n'a été modifié." -ForegroundColor Green
    Write-Host "Aucun fichier GitHub n'a été modifié." -ForegroundColor Green
    Write-Host ""
    Write-Host "Pour appliquer réellement les changements locaux :" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "powershell -ExecutionPolicy Bypass -File '.\patches\apply-ui-overlay.ps1' -Apply"
    Write-Host ""
    Write-Host "============================================================" -ForegroundColor Cyan
    exit 0
}

# ============================================================================
# APPLICATION RÉELLE
# ============================================================================
# À partir d'ici seulement, le working tree local est modifié.
# GitHub n'est jamais appelé par ce script.
# ============================================================================

$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'

$UiBackup  = "$UiPath.ui-overlay-backup-$stamp"
$CssBackup = "$CssPath.ui-overlay-backup-$stamp"

Copy-Item -LiteralPath $UiPath  -Destination $UiBackup  -Force
Copy-Item -LiteralPath $CssPath -Destination $CssBackup -Force

[System.IO.File]::WriteAllText(
    $UiPath,
    $uiText,
    [System.Text.UTF8Encoding]::new($false)
)

# ============================================================================
# CSS
# ============================================================================

$cssPatch = @'

/* SCI FAMILY UI OVERLAY v0.2 — simplified home + real/planned treasury curve */

.nav-separator {
    height: 1px;
    background: #202a3a;
    margin: 9px 4px;
}

.expert-nav {
    margin-top: 2px;
}

.expert-nav summary {
    list-style: none;
    cursor: pointer;
    padding: 10px 13px;
    color: #6f7c91;
    font-size: 11px;
    border-radius: 9px;
}

.expert-nav summary::-webkit-details-marker {
    display: none;
}

.expert-nav summary:hover {
    background: #111a28;
    color: #b8c4d8;
}

.expert-nav-list {
    display: grid;
    gap: 3px;
    padding: 3px 0 2px 8px;
}

.expert-nav-list .nav-item {
    font-size: 11px;
    padding: 8px 10px;
}

.home-greeting {
    margin-top: 28px;
    display: flex;
    justify-content: space-between;
    gap: 22px;
    padding: 25px 26px;
    border: 1px solid #26334b;
    border-radius: 20px;
    background: linear-gradient(
        135deg,
        rgba(17, 25, 38, .94),
        rgba(13, 18, 28, .90)
    );
    box-shadow: 0 18px 45px rgba(0, 0, 0, .14);
}

.home-greeting h2 {
    font-size: 27px;
    margin: 9px 0 5px;
}

.home-greeting p {
    margin: 0;
    color: #9aa7b9;
    font-size: 13px;
}

.home-risk {
    text-align: right;
    min-width: 180px;
}

.home-metrics {
    margin-top: 13px;
}

.home-modules {
    margin-top: 13px;
}

.home-bottom {
    margin-top: 13px;
}

.dual-curve-panel {
    margin-top: 13px;
    padding-bottom: 13px;
}

.dual-curve-head {
    margin-bottom: 4px;
}

.dual-curve-wrap {
    border: 1px solid #202a3a;
    border-radius: 14px;
    background: linear-gradient(180deg, #0f1620, #0c121b);
    padding: 8px 10px 5px;
    overflow: hidden;
}

.dual-curve-svg {
    display: block;
    width: 100%;
    height: 310px;
}

.curve-grid-line {
    stroke: #202a3a;
    stroke-width: 1;
}

.curve-zero-line {
    stroke: #33415a;
    stroke-width: 1;
    stroke-dasharray: 5 7;
}

.curve-line {
    fill: none;
    vector-effect: non-scaling-stroke;
    stroke-linecap: round;
    stroke-linejoin: round;
}

.curve-line.real {
    stroke: #7de0aa;
    stroke-width: 3;
}

.curve-line.planned {
    stroke: #82a8ff;
    stroke-width: 3;
    stroke-dasharray: 7 6;
}

.curve-x-label {
    fill: #6f7c91;
    font-size: 11px;
}

.curve-legend {
    display: flex;
    gap: 13px;
    align-items: center;
}

.curve-legend-item {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    font-size: 11px;
    color: #93a1b5;
}

.curve-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    display: inline-block;
}

.curve-legend-item.real .curve-dot {
    background: #7de0aa;
}

.curve-legend-item.planned .curve-dot {
    background: #82a8ff;
}

.dual-curve-note {
    display: flex;
    gap: 7px;
    flex-wrap: wrap;
    margin-top: 8px;
    color: #6f7c91;
    font-size: 10px;
}

.dual-curve-note span:nth-child(1),
.dual-curve-note span:nth-child(3) {
    font-weight: 800;
    color: #b8c3d5;
}

.theme-light .dual-curve-wrap {
    background: #f8fafc;
    border-color: #d5deea;
}

.theme-light .curve-grid-line {
    stroke: #d8e0ea;
}

.theme-light .curve-x-label {
    fill: #6a7789;
}

.theme-light .home-greeting {
    background: #fff;
    border-color: #d5deea;
}

.theme-high_contrast .dual-curve-wrap {
    background: #000;
    border-color: #fff;
}

.theme-high_contrast .curve-grid-line,
.theme-high_contrast .curve-zero-line {
    stroke: #fff;
}

.theme-high_contrast .curve-x-label {
    fill: #fff;
}

@media (max-width: 850px) {
    .home-greeting {
        flex-direction: column;
    }

    .home-risk {
        text-align: left;
    }

    .dual-curve-svg {
        height: 260px;
    }
}

@media (max-width: 650px) {
    .dual-curve-svg {
        height: 220px;
    }

    .curve-legend {
        gap: 8px;
    }

    .dual-curve-note {
        font-size: 9px;
    }
}
'@

if ($cssText.IndexOf(
    $cssMarker,
    [StringComparison]::Ordinal
) -lt 0) {

    [System.IO.File]::AppendAllText(
        $CssPath,
        $cssPatch,
        [System.Text.UTF8Encoding]::new($false)
    )

    Write-Host "CSS overlay ajoutée." -ForegroundColor Green
}
else {
    Write-Host "CSS overlay déjà présente : aucun doublon ajouté." -ForegroundColor Yellow
}

# ============================================================================
# FIN
# ============================================================================

Write-Host ""
Write-Host "============================================================" -ForegroundColor Green
Write-Host "UI OVERLAY APPLIQUÉE" -ForegroundColor Green
Write-Host "============================================================" -ForegroundColor Green
Write-Host ""
Write-Host "Fichiers locaux modifiés :" -ForegroundColor White
Write-Host "  $UiPath"
Write-Host "  $CssPath"
Write-Host ""
Write-Host "Backups créés :" -ForegroundColor White
Write-Host "  $UiBackup"
Write-Host "  $CssBackup"
Write-Host ""
Write-Host "GitHub : AUCUNE écriture effectuée." -ForegroundColor Green
Write-Host ""
Write-Host "Étape suivante :" -ForegroundColor Yellow
Write-Host "  cargo check --features web"
Write-Host ""
