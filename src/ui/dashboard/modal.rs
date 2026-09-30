use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use crate::relances::{LocalBarItem, LocalStatus};

fn format_amount(v: f64) -> String {
    format!("{:.2} EUR", v)
}

fn format_date_ts(ts: i64) -> String {
    match DateTime::<Utc>::from_timestamp(ts, 0) {
        Some(dt) => dt.format("%d/%m/%Y").to_string(),
        None => "-".to_string(),
    }
}

#[component]
pub fn LocalModal(
    local: Signal<Option<LocalBarItem>>,
    on_close: EventHandler<()>,
) -> Element {
    let Some(l) = local() else {
        return rsx! { div {} };
    };

    let color = l.status.color();
    let status_label = l.status.label();
    let is_vacant = l.status == LocalStatus::Vacant;
    let is_uptodate = l.status == LocalStatus::UpToDate;

    let last_level_label = match l.highest_level_sent {
        0 => "Aucune relance envoyée",
        1 => "Relance amiable envoyée",
        2 => "Relance ferme envoyée",
        3 => "Mise en demeure envoyée",
        _ => "Inconnu",
    };

    let display_tenant = l
        .tenant_name
        .clone()
        .unwrap_or_else(|| "Aucun locataire en place".to_string());

    let display_email = l
        .tenant_email
        .clone()
        .filter(|e| !e.is_empty())
        .unwrap_or_else(|| "—".to_string());

    rsx! {
        div {
            class: "dash-modal-overlay",
            onclick: move |_| on_close.call(()),
            div {
                class: "dash-modal",
                onclick: move |e| e.stop_propagation(),
                style: "max-width: 820px; max-height: 85vh; overflow-y: auto;",

                // Bande colorée
                div {
                    style: "height: 4px; background: {color}; margin: -24px -24px 20px -24px; border-radius: 12px 12px 0 0;"
                }

                // En-tête
                div { style: "display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 20px;",
                    div {
                        div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600;",
                            "{l.property_name} • {l.unit_code}"
                        }
                        h2 { style: "margin: 4px 0 8px 0; color: #f8fafc;", "{l.unit_label}" }
                        div { style: "display: inline-flex; align-items: center; gap: 6px; padding: 3px 10px; background: {color}20; border: 1px solid {color}66; border-radius: 12px;",
                            span { style: "width: 8px; height: 8px; border-radius: 50%; background: {color};" }
                            span { style: "font-size: 0.7rem; color: {color}; font-weight: 600;", "{status_label}" }
                        }
                    }
                    button {
                        style: "background: transparent; border: none; color: #94a3b8; font-size: 1.5rem; cursor: pointer; line-height: 1;",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }

                // Locataire
                div { style: "padding: 12px 14px; background: #0f172a; border-radius: 8px; margin-bottom: 16px;",
                    div { style: "font-size: 0.7rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em;", "Locataire" }
                    div { style: "font-size: 0.95rem; color: #e2e8f0; font-weight: 500; margin-top: 4px;", "{display_tenant}" }
                    div { style: "font-size: 0.75rem; color: #94a3b8; margin-top: 2px;", "{display_email}" }
                }

                if !is_vacant {
                    // 4 cartes stats
                    div { style: "display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; margin-bottom: 20px;",
                        div { style: "padding: 12px; background: #0f172a; border-radius: 8px; border: 1px solid #1e293b;",
                            div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em;", "Total impayé" }
                            div { style: "font-size: 1.15rem; font-weight: 700; color: {color}; font-variant-numeric: tabular-nums; margin-top: 4px;",
                                {format_amount(l.total_outstanding)}
                            }
                        }
                        div { style: "padding: 12px; background: #0f172a; border-radius: 8px; border: 1px solid #1e293b;",
                            div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em;", "Factures impayées" }
                            div { style: "font-size: 1.15rem; font-weight: 700; color: #e2e8f0; margin-top: 4px;",
                                "{l.invoice_count}"
                            }
                        }
                        div { style: "padding: 12px; background: #0f172a; border-radius: 8px; border: 1px solid #1e293b;",
                            div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em;", "Retard max" }
                            div { style: "font-size: 1.15rem; font-weight: 700; color: {color}; margin-top: 4px;",
                                if l.max_days_overdue > 0 { "J+{l.max_days_overdue}" } else { "—" }
                            }
                        }
                        div { style: "padding: 12px; background: #0f172a; border-radius: 8px; border: 1px solid #1e293b;",
                            div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em;", "Dernière relance" }
                            div { style: "font-size: 0.72rem; font-weight: 600; color: #e2e8f0; margin-top: 6px;",
                                {last_level_label}
                            }
                        }
                    }

                    // Factures impayées (seulement si > 0)
                    if !l.invoices.is_empty() {
                        div { style: "margin-bottom: 20px;",
                            h4 { style: "color: #94a3b8; margin: 0 0 10px 0; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em;",
                                "Factures impayées"
                            }
                            div { style: "display: flex; flex-direction: column; gap: 6px;",
                                for inv in l.invoices.iter() {
                                    {
                                        let days = inv.days_overdue;
                                        let row_color = match days {
                                            d if d <= 0 => "#64748b",
                                            d if d <= 30 => "#f59e0b",
                                            d if d <= 60 => "#ef4444",
                                            _ => "#dc2626",
                                        };
                                        let inv_ref = inv.invoice_ref.clone();
                                        let issue = format_date_ts(inv.issue_date_ts);
                                        let due = format_date_ts(inv.due_date_ts);
                                        let total = format_amount(inv.total_ttc);
                                        let outstanding = format_amount(inv.outstanding);
                                        let key = inv.dolibarr_invoice_id.clone();

                                        rsx! {
                                            div {
                                                key: "{key}",
                                                style: "background: var(--bg-glass); border: 1px solid var(--border); border-left: 3px solid {row_color}; padding: 10px 14px; border-radius: 8px; display: flex; align-items: center; gap: 12px;",
                                                div { style: "flex: 1; min-width: 0;",
                                                    div { style: "font-size: 0.85rem; color: #e2e8f0; font-weight: 500;",
                                                        "Facture {inv_ref}"
                                                    }
                                                    div { style: "font-size: 0.7rem; color: #94a3b8; margin-top: 2px;",
                                                        "Émise le {issue} • Échéance {due}"
                                                    }
                                                }
                                                div { style: "text-align: right; flex-shrink: 0;",
                                                    div { style: "font-size: 0.88rem; font-weight: 600; color: {row_color}; font-variant-numeric: tabular-nums;",
                                                        "{outstanding}"
                                                    }
                                                    div { style: "font-size: 0.68rem; color: #64748b;",
                                                        "sur {total} • J+{days}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if is_uptodate {
                        div { style: "padding: 14px; text-align: center; color: #22c55e; font-size: 0.82rem; background: rgba(34,197,94,0.08); border: 1px solid rgba(34,197,94,0.25); border-radius: 8px; margin-bottom: 20px;",
                            "✓ Ce locataire est à jour de ses paiements."
                        }
                    }

                    // Info relances
                    div { style: "padding: 10px 12px; background: #0f172a; border-left: 3px solid {color}; border-radius: 4px; font-size: 0.75rem; color: #94a3b8; margin-bottom: 16px;",
                        if l.total_outstanding > 0.0 {
                            "💡 Les boutons de relance (amiable, ferme, mise en demeure) arrivent dans la prochaine étape du Sprint 11."
                        } else {
                            "💡 Aucune action nécessaire. Locataire à jour."
                        }
                    }
                }

                // Boutons
                div { style: "display: flex; gap: 8px; justify-content: flex-end;",
                    button {
                        class: "secondary",
                        onclick: move |_| on_close.call(()),
                        "Fermer"
                    }
                    if !is_vacant && l.total_outstanding > 0.0 {
                        button {
                            class: "primary",
                            disabled: l.tenant_email.as_ref().map(|e| e.is_empty()).unwrap_or(true),
                            title: if l.tenant_email.as_ref().map(|e| e.is_empty()).unwrap_or(true) {
                                "Aucun email renseigné pour ce locataire"
                            } else {
                                "Disponible après implémentation"
                            },
                            "Relancer (bientôt)"
                        }
                    }
                }
            }
        }
    }
}