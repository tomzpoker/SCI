use dioxus::prelude::*;
use crate::relances::{LocalBarItem, LocalStatus};

/// Calcule le "niveau de vie" de la barre selon le retard.
/// - À jour ou pas d'impayé : 100% (barre pleine verte)
/// - Plus le retard augmente, plus la barre baisse et vire au rouge
/// - Vide : barre quasi-nulle grise
fn compute_health(status: LocalStatus, max_days_overdue: i32) -> (f64, &'static str, &'static str) {
    match status {
        LocalStatus::Vacant => (8.0, "#475569", "#334155"),
        LocalStatus::UpToDate => (100.0, "#22c55e", "#14532d"),
        LocalStatus::Late | LocalStatus::Critical => {
            // Décroissance linéaire : 100% à J+0, 8% à J+90+
            let days = max_days_overdue.max(0) as f64;
            let pct = (100.0 - (days / 90.0) * 92.0).clamp(8.0, 100.0);
            // Couleur par palier
            let (color, border) = if pct >= 75.0 {
                ("#84cc16", "#365314") // vert-jaune (juste en retard)
            } else if pct >= 50.0 {
                ("#f59e0b", "#78350f") // orange
            } else if pct >= 25.0 {
                ("#ef4444", "#7f1d1d") // rouge
            } else {
                ("#dc2626", "#7f1d1d") // rouge foncé (critique)
            };
            (pct, color, border)
        }
    }
}

#[component]
pub fn LocalBars(
    locals: Vec<LocalBarItem>,
    on_select: EventHandler<LocalBarItem>,
) -> Element {
    let mut hide_vacant = use_signal(|| false);
    let mut hide_uptodate = use_signal(|| false);

    // Filtre
    let filtered: Vec<LocalBarItem> = locals
        .iter()
        .filter(|l| {
            if hide_vacant() && l.status == LocalStatus::Vacant {
                return false;
            }
            if hide_uptodate() && l.status == LocalStatus::UpToDate {
                return false;
            }
            true
        })
        .cloned()
        .collect();

    let total_locals = locals.len();
    let vacant_count = locals.iter().filter(|l| l.status == LocalStatus::Vacant).count();
    let uptodate_count = locals.iter().filter(|l| l.status == LocalStatus::UpToDate).count();
    let late_count = locals.iter().filter(|l| l.status == LocalStatus::Late).count();
    let critical_count = locals.iter().filter(|l| l.status == LocalStatus::Critical).count();

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 10px;",

            // === Filtres + légende ===
            div { style: "display: flex; flex-wrap: wrap; gap: 10px; align-items: center; font-size: 0.7rem; color: #94a3b8;",
                label { style: "display: inline-flex; align-items: center; gap: 5px; cursor: pointer; user-select: none;",
                    input {
                        r#type: "checkbox",
                        checked: hide_vacant(),
                        onchange: move |e| hide_vacant.set(e.value() == "true"),
                    }
                    "Masquer les vides ({vacant_count})"
                }
                label { style: "display: inline-flex; align-items: center; gap: 5px; cursor: pointer; user-select: none;",
                    input {
                        r#type: "checkbox",
                        checked: hide_uptodate(),
                        onchange: move |e| hide_uptodate.set(e.value() == "true"),
                    }
                    "Masquer les à jour ({uptodate_count})"
                }
                div { style: "margin-left: auto; display: flex; gap: 10px;",
                    span { style: "display: inline-flex; align-items: center; gap: 4px;",
                        span { style: "width: 8px; height: 8px; border-radius: 50%; background: #ef4444;" }
                        "Critique {critical_count}"
                    }
                    span { style: "display: inline-flex; align-items: center; gap: 4px;",
                        span { style: "width: 8px; height: 8px; border-radius: 50%; background: #f59e0b;" }
                        "Retard {late_count}"
                    }
                }
            }

            // === Barres ===
            if filtered.is_empty() {
                div {
                    style: "padding: 16px; text-align: center; color: #64748b; font-size: 0.8rem; background: var(--bg-input); border: 1px dashed var(--border-strong); border-radius: 8px;",
                    if total_locals == 0 {
                        "Aucun local actif. Crée-en dans Patrimoine."
                    } else {
                        "Tous les locaux sont masqués par les filtres."
                    }
                }
            } else {
                div {
                    style: "display: flex; justify-content: flex-start; align-items: flex-end; gap: 6px; padding: 4px 0; flex-wrap: wrap;",
                    for local in filtered.iter() {
                        {
                            let (health_pct, color, border_col) =
                                compute_health(local.status, local.max_days_overdue);

                            let label = if local.total_outstanding > 0.0 {
                                format!("{:.0} EUR", local.total_outstanding)
                            } else {
                                local.status.label().to_string()
                            };

                            let display_name = local
                                .tenant_name
                                .clone()
                                .unwrap_or_else(|| "Libre".to_string());

                            let local_clone = local.clone();
                            let is_vacant = local.status == LocalStatus::Vacant;
                            let is_uptodate = local.status == LocalStatus::UpToDate;

                            rsx! {
                                div {
                                    key: "{local.unit_id}",
                                    style: "display: flex; flex-direction: column; align-items: center; cursor: pointer; min-width: 60px; max-width: 90px; flex: 1;",
                                    onclick: move |_| on_select.call(local_clone.clone()),

                                    // Barre
                                    div {
                                        style: "width: 100%; height: 80px; background: #0f172a; border: 1px solid {border_col}; border-radius: 4px; display: flex; align-items: flex-end; overflow: hidden; position: relative;",
                                        div {
                                            style: "width: 100%; height: {health_pct}%; background: {color}; transition: height 0.4s, background 0.4s;"
                                        }
                                    }

                                    // Nom
                                    span {
                                        style: "font-size: 0.68rem; margin-top: 4px; color: #94a3b8; text-align: center; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 100%;",
                                        "{display_name}"
                                    }

                                    // Montant ou statut
                                    span {
                                        style: "font-size: 0.68rem; color: {color}; font-weight: 600;",
                                        "{label}"
                                    }

                                    // Retard
                                    if !is_vacant && local.max_days_overdue > 0 {
                                        span {
                                            style: "font-size: 0.62rem; color: #64748b;",
                                            "J+{local.max_days_overdue}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}