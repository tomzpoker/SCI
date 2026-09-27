use chrono::{DateTime, Utc};
use dioxus::prelude::*;

use crate::dolibarr::models::{DolibarrInvoice, DolibarrThirdParty};

#[derive(Clone, Copy, PartialEq)]
enum InvoiceFilter {
    All,
    Draft,
    ToCollect,
    Paid,
}

impl InvoiceFilter {
    fn label(&self) -> &'static str {
        match self {
            InvoiceFilter::All => "Toutes",
            InvoiceFilter::Draft => "Brouillons",
            InvoiceFilter::ToCollect => "A encaisser",
            InvoiceFilter::Paid => "Payees",
        }
    }

    fn matches(&self, inv: &DolibarrInvoice) -> bool {
        let statut: i32 = inv.statut;
        let paye: i32 = inv.paye.parse().unwrap_or(0);
        match self {
            InvoiceFilter::All => true,
            InvoiceFilter::Draft => statut == 0,
            InvoiceFilter::ToCollect => statut >= 1 && paye == 0,
            InvoiceFilter::Paid => paye == 1,
        }
    }
}

fn parse_amount(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}

fn format_amount(v: f64) -> String {
    format!("{:.2} EUR", v)
}

fn format_date(ts: i64) -> String {
    match DateTime::<Utc>::from_timestamp(ts, 0) {
        Some(dt) => dt.format("%d/%m/%Y").to_string(),
        None => "-".to_string(),
    }
}

fn third_party_name(socid: &str, thirds: &[DolibarrThirdParty]) -> String {
    thirds
        .iter()
        .find(|t| t.id == socid)
        .map(|t| t.name.clone())
        .unwrap_or_else(|| format!("Tiers #{}", socid))
}

fn status_label(inv: &DolibarrInvoice) -> (&'static str, &'static str) {
    let statut: i32 = inv.statut;
    let paye: i32 = inv.paye.parse().unwrap_or(0);
    if statut == 0 {
        ("Brouillon", "#94a3b8")
    } else if paye == 1 {
        ("Payee", "#4ade80")
    } else if statut == 1 || statut == 2 {
        ("A encaisser", "#fbbf24")
    } else if statut == 3 {
        ("Annulee", "#64748b")
    } else {
        ("-", "#64748b")
    }
}

const PAGE_SIZE: usize = 20;

#[component]
pub fn InvoicesDolibarrPage(refresh: Signal<u64>) -> Element {
    let invoices = use_resource(move || {
        let _ = refresh();
        async move { crate::dolibarr::server_fns::dolibarr_list_invoices(200).await }
    });

    let thirds = use_resource(move || {
        let _ = refresh();
        async move { crate::dolibarr::server_fns::dolibarr_list_third_parties(200).await }
    });

    let mut filter = use_signal(|| InvoiceFilter::All);
    let mut current_page = use_signal(|| 0usize);

    let dl_url = "http://localhost:8081";

    rsx! {
        section { class: "page-intro",
            div {
                div { class: "eyebrow", "FACTURATION / DOLIBARR" }
                h2 { "Factures" }
                p { "Vue en lecture des factures Dolibarr. La creation et l'encaissement seront ajoutes au Sprint 4." }
            }
        }

        section { class: "panel",
            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap;",
                div { style: "display: flex; gap: 6px; flex-wrap: wrap;",
                    for f in [InvoiceFilter::All, InvoiceFilter::Draft, InvoiceFilter::ToCollect, InvoiceFilter::Paid] {
                        button {
                            style: if filter() == f {
                                "padding: 5px 12px; background: #7c3aed; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;"
                            } else {
                                "padding: 5px 12px; background: transparent; color: #94a3b8; border: 1px solid var(--border); border-radius: 6px; cursor: pointer; font-size: 0.75rem;"
                            },
                            onclick: move |_| {
                                filter.set(f);
                                current_page.set(0);
                            },
                            "{f.label()}"
                        }
                    }
                }
                button {
                    style: "padding: 5px 12px; background: transparent; color: #a78bfa; border: 1px solid #7c3aed; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;",
                    onclick: move |_| {
                        let _ = document::eval("window.open('http://localhost:8081/compta/facture/list.php', '_blank');");
                    },
                    "Ouvrir Dolibarr"
                }
            }
        }

        match (&*invoices.read(), &*thirds.read()) {
            (Some(Ok(inv_list)), Some(Ok(th_list))) => {
                let filtered: Vec<&DolibarrInvoice> = inv_list
                    .iter()
                    .filter(|inv| filter().matches(inv))
                    .collect();

                let total = filtered.len();
                let total_pages = (total + PAGE_SIZE - 1) / PAGE_SIZE;
                let page_idx = current_page().min(total_pages.saturating_sub(1));
                let start = page_idx * PAGE_SIZE;
                let end = (start + PAGE_SIZE).min(total);
                let page_items: Vec<&DolibarrInvoice> = filtered[start..end].to_vec();

                rsx! {
                    section { class: "panel",
                        div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;",
                            span { class: "small", {format!("{} facture(s)", total)} }
                            if total_pages > 1 {
                                span { class: "small", {format!("Page {} / {}", page_idx + 1, total_pages)} }
                            }
                        }

                        if page_items.is_empty() {
                            div { class: "empty-state",
                                h3 { "Aucune facture" }
                                p { "Aucune facture ne correspond a ce filtre." }
                            }
                        } else {
                            div { style: "display: flex; flex-direction: column; gap: 6px;",
                                for inv in page_items.iter() {
                                    {
                                        let (status_txt, status_color) = status_label(inv);
                                        let client_name = third_party_name(&inv.socid, th_list);
                                        let ttc = parse_amount(&inv.total_ttc);
                                        let tva = parse_amount(&inv.total_tva);
                                        let ht = parse_amount(&inv.total_ht);
                                        let date_str = format_date(inv.date);
                                        let inv_url = format!("{}/compta/facture/card.php?id={}", dl_url, inv.id);
                                        let inv_url_for_click = inv_url.clone();
                                        let id_for_key = inv.id.clone();
                                        let ref_str = inv.r#ref.clone();
                                        rsx! {
                                            div {
                                                key: "{id_for_key}",
                                                style: "background: var(--bg-glass); border: 1px solid var(--border); border-left: 3px solid {status_color}; padding: 10px 14px; border-radius: 10px; display: flex; align-items: center; gap: 12px;",
                                                div { style: "flex: 1; min-width: 0;",
                                                    div { style: "font-size: 0.88rem; color: #e2e8f0; font-weight: 500;",
                                                        "{ref_str} - {client_name}"
                                                    }
                                                    div { style: "font-size: 0.72rem; color: #94a3b8; margin-top: 2px;",
                                                        span { "{date_str}" }
                                                        span { style: "color: #334155;", " / " }
                                                        span { style: "color: {status_color}; font-weight: 600;", "{status_txt}" }
                                                    }
                                                }
                                                div { style: "text-align: right; flex-shrink: 0;",
                                                    div { style: "font-size: 0.85rem; color: #e2e8f0; font-weight: 600;",
                                                        {format_amount(ttc)}
                                                    }
                                                    div { style: "font-size: 0.7rem; color: #64748b;",
                                                        {format!("HT {} / TVA {}", format_amount(ht), format_amount(tva))}
                                                    }
                                                }
                                                button {
                                                    style: "padding: 5px 10px; background: transparent; border: 1px solid var(--border); color: #a78bfa; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 600; flex-shrink: 0;",
                                                    onclick: move |_| {
                                                        let u = inv_url_for_click.clone();
                                                        let script = format!("window.open('{}', '_blank');", u);
                                                        let _ = document::eval(&script);
                                                    },
                                                    "Voir"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if total_pages > 1 {
                            div { style: "display: flex; justify-content: center; gap: 8px; margin-top: 14px;",
                                button {
                                    disabled: page_idx == 0,
                                    onclick: move |_| current_page.set(page_idx.saturating_sub(1)),
                                    style: "padding: 6px 14px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.75rem;",
                                    "< Precedent"
                                }
                                button {
                                    disabled: page_idx + 1 >= total_pages,
                                    onclick: move |_| current_page.set(page_idx + 1),
                                    style: "padding: 6px 14px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.75rem;",
                                    "Suivant >"
                                }
                            }
                        }
                    }
                }
            },
            (Some(Err(e)), _) => rsx! {
                section { class: "panel",
                    div { style: "color: #f87171;", {format!("Erreur factures : {e}")} }
                }
            },
            (_, Some(Err(e))) => rsx! {
                section { class: "panel",
                    div { style: "color: #f87171;", {format!("Erreur tiers : {e}")} }
                }
            },
            _ => rsx! {
                section { class: "panel",
                    div { style: "color: #94a3b8;", "Chargement des factures..." }
                }
            },
        }
    }
}