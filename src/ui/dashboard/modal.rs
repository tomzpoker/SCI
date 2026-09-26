use dioxus::prelude::*;
use super::models::{Tenant, TenantStatus};

#[component]
pub fn TenantModal(
    tenant: Signal<Option<Tenant>>,
    on_close: EventHandler<()>,
) -> Element {
    let Some(t) = tenant() else {
        return rsx! { div {} };
    };

    let is_vacant = t.status == TenantStatus::Vacant;
    let status_color = match t.status {
        TenantStatus::Paid => "#22c55e",
        TenantStatus::Late => "#f59e0b",
        TenantStatus::Unpaid => "#ef4444",
        TenantStatus::Vacant => "#64748b",
    };

    rsx! {
        div {
            class: "dash-modal-overlay",
            onclick: move |_| on_close.call(()),
            div {
                class: "dash-modal",
                onclick: move |e| e.stop_propagation(),
                style: "max-height: 85vh; overflow-y: auto;",

                h2 { "{t.name}" }
                p { strong { "Local : " } "{t.property}" }

                if is_vacant {
                    div {
                        style: "background: #1e293b; padding: 12px; border-radius: 8px; margin-top: 12px; border-left: 4px solid #64748b;",
                        p { style: "margin: 0; color: #94a3b8;", "Local vacant — pas de locataire en place." }
                    }
                } else {
                    p { strong { "Loyer mensuel : " } {format!("{:.2} EUR", t.rent)} }
                    p {
                        strong { "Solde : " }
                        span { style: "color: {status_color}; font-weight: 600;", {format!("{:.2} EUR", t.balance)} }
                    }
                    p { strong { "Statut : " } {format!("{:?}", t.status)} }

                    if t.balance < 0.0 {
                        div {
                            style: "background: #450a0a; padding: 12px; border-radius: 8px; margin-top: 12px; border-left: 4px solid #ef4444;",
                            h4 { style: "margin: 0 0 8px 0; color: #ef4444;", "Impayés" }
                            p { style: "margin: 0;",
                                "Total dû : "
                                strong { {format!("{:.2} EUR", t.balance.abs())} }
                            }
                        }
                    } else {
                        p { style: "color: #22c55e;", "Aucun impayé" }
                    }
                }

                // --- Historique des paiements ---
                if !t.payments.is_empty() {
                    div {
                        style: "margin-top: 16px;",
                        h4 { style: "margin: 0 0 8px 0; color: #38bdf8;", "Historique des paiements" }
                        table {
                            style: "width: 100%; border-collapse: collapse; font-size: 0.85rem;",
                            thead {
                                tr {
                                    th { style: "text-align: left; padding: 6px 4px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "Date" }
                                    th { style: "text-align: right; padding: 6px 4px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "Attendu" }
                                    th { style: "text-align: right; padding: 6px 4px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "Reçu" }
                                    th { style: "text-align: right; padding: 6px 4px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "Écart" }
                                }
                            }
                            tbody {
                                for p in t.payments.iter() {
                                    {
                                        let ecart = p.received - p.expected;
                                        let color = if ecart >= 0.0 { "#22c55e" }
                                                    else if ecart > -0.5 * p.expected { "#f59e0b" }
                                                    else { "#ef4444" };
                                        let label = if ecart >= -0.01 { "OK".to_string() }
                                                    else { format!("{:.2}", ecart) };
                                        rsx! {
                                            tr {
                                                td { style: "padding: 6px 4px;", "{p.date}" }
                                                td { style: "padding: 6px 4px; text-align: right;", {format!("{:.2}", p.expected)} }
                                                td { style: "padding: 6px 4px; text-align: right;", {format!("{:.2}", p.received)} }
                                                td {
                                                    style: "padding: 6px 4px; text-align: right; color: {color}; font-weight: 600;",
                                                    "{label}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div {
                            style: "margin-top: 10px; padding: 8px 10px; background: #0f172a; border-radius: 6px; font-size: 0.8rem; color: #94a3b8;",
                            span { "Total reçu : " }
                            strong {
                                style: "color: #f8fafc;",
                                {
                                    let total: f64 = t.payments.iter().map(|p| p.received).sum();
                                    format!("{:.2} EUR", total)
                                }
                            }
                        }
                    }
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