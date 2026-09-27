use chrono::{DateTime, Utc};
use dioxus::prelude::*;

use crate::dolibarr::models::{DolibarrInvoice, DolibarrInvoiceLine, DolibarrPayment, DolibarrThirdParty};

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

#[derive(Clone, PartialEq)]
struct LineDraft {
    desc: String,
    qty: String,
    subprice: String,
    tva_tx: String,
}

impl LineDraft {
    fn empty() -> Self {
        Self {
            desc: String::new(),
            qty: "1".to_string(),
            subprice: "0".to_string(),
            tva_tx: "20".to_string(),
        }
    }

    fn to_dolibarr_line(&self) -> Option<DolibarrInvoiceLine> {
        let qty: f64 = self.qty.replace(',', ".").parse().ok()?;
        let subprice: f64 = self.subprice.replace(',', ".").parse().ok()?;
        let tva: f64 = self.tva_tx.replace(',', ".").parse().ok()?;
        Some(DolibarrInvoiceLine::new(&self.desc, qty, subprice, tva))
    }
}

const PAGE_SIZE: usize = 20;

#[component]
pub fn InvoicesDolibarrPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);

    let invoices = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { crate::dolibarr::server_fns::dolibarr_list_invoices(200).await }
    });

    let thirds = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { crate::dolibarr::server_fns::dolibarr_list_third_parties(200).await }
    });

    let mut filter = use_signal(|| InvoiceFilter::All);
    let mut current_page = use_signal(|| 0usize);

    let mut show_new = use_signal(|| false);
    let mut new_socid = use_signal(String::new);
    let mut new_date = use_signal(|| Utc::now().date_naive().format("%Y-%m-%d").to_string());
    let mut new_lines = use_signal(|| vec![LineDraft::empty()]);
    let mut form_msg = use_signal(String::new);
    let mut busy = use_signal(|| false);

    let mut pay_for_invoice = use_signal(|| None::<DolibarrInvoice>);
    let mut pay_amount = use_signal(String::new);
    let mut pay_date = use_signal(|| Utc::now().date_naive().format("%Y-%m-%d").to_string());
    let mut pay_mode = use_signal(|| 3i32);
    let mut pay_msg = use_signal(String::new);

    let mut history_for_invoice = use_signal(|| None::<String>);

    let payments_resource = use_resource(move || {
        let inv_id = history_for_invoice();
        async move {
            match inv_id {
                Some(id) => crate::dolibarr::server_fns::dolibarr_list_invoice_payments(id).await,
                None => Ok(Vec::<DolibarrPayment>::new()),
            }
        }
    });

    let dl_url = "http://localhost:8081";

    rsx! {
        section { class: "page-intro",
            div {
                div { class: "eyebrow", "FACTURATION / DOLIBARR" }
                h2 { "Factures" }
                p { "Creation, consultation et encaissement des factures Dolibarr." }
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
                div { style: "display: flex; gap: 8px;",
                    button {
                        style: "padding: 5px 12px; background: #7c3aed; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;",
                        onclick: move |_| {
                            new_socid.set(String::new());
                            new_date.set(Utc::now().date_naive().format("%Y-%m-%d").to_string());
                            new_lines.set(vec![LineDraft::empty()]);
                            form_msg.set(String::new());
                            show_new.set(true);
                        },
                        "+ Nouvelle facture"
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
        }

        match (&*invoices.read(), &*thirds.read()) {
            (Some(Ok(inv_list)), Some(Ok(th_list))) => {
                let filtered: Vec<DolibarrInvoice> = inv_list
                    .iter()
                    .filter(|inv| filter().matches(inv))
                    .cloned()
                    .collect();

                let total = filtered.len();
                let total_pages = (total + PAGE_SIZE - 1) / PAGE_SIZE;
                let page_idx = current_page().min(total_pages.saturating_sub(1));
                let start = page_idx * PAGE_SIZE;
                let end = (start + PAGE_SIZE).min(total);
                let page_items: Vec<DolibarrInvoice> = filtered[start..end].to_vec();

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
                                for inv in page_items.into_iter() {
                                    {
                                        let (status_txt, status_color) = status_label(&inv);
                                        let client_name = third_party_name(&inv.socid, th_list);
                                        let ttc = parse_amount(&inv.total_ttc);
                                        let tva = parse_amount(&inv.total_tva);
                                        let ht = parse_amount(&inv.total_ht);
                                        let date_str = format_date(inv.date);
                                        let inv_url = format!("{}/compta/facture/card.php?id={}", dl_url, inv.id);
                                        let inv_url_for_click = inv_url.clone();
                                        let id_for_key = inv.id.clone();
                                        let ref_str = inv.r#ref.clone();
                                        let statut_i: i32 = inv.statut;
                                        let paye_i: i32 = inv.paye.parse().unwrap_or(0);
                                        let can_pay = statut_i >= 1 && paye_i == 0;
                                        let inv_clone_for_pay = inv.clone();
                                        let inv_id_for_history = inv.id.clone();
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
                                                div { style: "display: flex; gap: 4px; flex-shrink: 0;",
                                                    button {
                                                        style: "padding: 5px 10px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 600;",
                                                        onclick: move |_| {
                                                            let u = inv_url_for_click.clone();
                                                            let script = format!("window.open('{}', '_blank');", u);
                                                            let _ = document::eval(&script);
                                                        },
                                                        "Voir"
                                                    }
                                                    button {
                                                        style: "padding: 5px 10px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 600;",
                                                        onclick: {
                                                            let iid = inv_id_for_history.clone();
                                                            move |_| history_for_invoice.set(Some(iid.clone()))
                                                        },
                                                        "Paiements"
                                                    }
                                                    if can_pay {
                                                        button {
                                                            style: "padding: 5px 10px; background: transparent; border: 1px solid #4ade80; color: #4ade80; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 600;",
                                                            onclick: move |_| {
                                                                let inv_c = inv_clone_for_pay.clone();
                                                                let reste = parse_amount(&inv_c.total_ttc);
                                                                pay_amount.set(format!("{:.2}", reste));
                                                                pay_date.set(Utc::now().date_naive().format("%Y-%m-%d").to_string());
                                                                pay_mode.set(3);
                                                                pay_msg.set(String::new());
                                                                pay_for_invoice.set(Some(inv_c));
                                                            },
                                                            "Encaisser"
                                                        }
                                                    }
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

        if show_new() {
            div {
                class: "dash-modal-overlay",
                onclick: move |_| show_new.set(false),
                div {
                    class: "dash-modal",
                    style: "max-width: 720px; max-height: 85vh; overflow-y: auto;",
                    onclick: move |e| e.stop_propagation(),

                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                        div {
                            div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em;", "Nouvelle facture" }
                            h2 { style: "margin: 4px 0 0 0;", "Creer un brouillon dans Dolibarr" }
                        }
                        button {
                            style: "background: transparent; border: none; color: #94a3b8; font-size: 1.5rem; cursor: pointer; line-height: 1;",
                            onclick: move |_| show_new.set(false),
                            "x"
                        }
                    }

                    if !form_msg().is_empty() {
                        div { style: "background: #450a0a; border: 1px solid #7f1d1d; color: #fecaca; padding: 8px 12px; border-radius: 4px; font-size: 0.8rem; margin-bottom: 12px;",
                            "{form_msg()}"
                        }
                    }

                    div { style: "display: flex; flex-direction: column; gap: 12px;",
                        label { class: "field",
                            span { "Client" }
                            select {
                                value: "{new_socid}",
                                onchange: move |e| new_socid.set(e.value()),
                                option { value: "", "Selectionner un tiers..." }
                                for t in thirds.read().as_ref().and_then(|r| r.as_ref().ok()).map(|v| v.as_slice()).unwrap_or(&[]).iter() {
                                    option {
                                        value: "{t.id}",
                                        "{t.name}"
                                    }
                                }
                            }
                        }

                        label { class: "field",
                            span { "Date de facture (AAAA-MM-JJ)" }
                            input {
                                r#type: "text",
                                value: "{new_date}",
                                oninput: move |e| new_date.set(e.value()),
                            }
                        }

                        div { style: "margin-top: 8px;",
                            div { style: "font-size: 0.75rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 6px;",
                                "Lignes"
                            }
                            for (idx, line) in new_lines().iter().enumerate() {
                                div {
                                    key: "{idx}",
                                    style: "background: var(--bg-input); border: 1px solid var(--border); border-radius: 8px; padding: 10px; margin-bottom: 8px;",
                                    div { style: "display: flex; gap: 8px; align-items: center; margin-bottom: 6px;",
                                        input {
                                            r#type: "text",
                                            placeholder: "Description",
                                            value: "{line.desc}",
                                            oninput: move |e| {
                                                let v = e.value();
                                                new_lines.with_mut(|lines| {
                                                    if let Some(l) = lines.get_mut(idx) { l.desc = v; }
                                                });
                                            },
                                            style: "flex: 1; padding: 6px 10px; background: var(--bg-glass); border: 1px solid var(--border); border-radius: 6px; color: #e2e8f0; font-size: 0.82rem;",
                                        }
                                        button {
                                            style: "padding: 4px 8px; background: transparent; border: 1px solid var(--border); color: #f87171; border-radius: 6px; cursor: pointer; font-size: 0.7rem;",
                                            onclick: move |_| {
                                                new_lines.with_mut(|lines| {
                                                    if lines.len() > 1 { lines.remove(idx); }
                                                });
                                            },
                                            "Suppr"
                                        }
                                    }
                                    div { style: "display: flex; gap: 8px;",
                                        div { style: "flex: 1;",
                                            div { style: "font-size: 0.65rem; color: #64748b; margin-bottom: 2px;", "Qte" }
                                            input {
                                                r#type: "text",
                                                value: "{line.qty}",
                                                oninput: move |e| {
                                                    let v = e.value();
                                                    new_lines.with_mut(|lines| {
                                                        if let Some(l) = lines.get_mut(idx) { l.qty = v; }
                                                    });
                                                },
                                                style: "width: 100%; padding: 5px 8px; background: var(--bg-glass); border: 1px solid var(--border); border-radius: 6px; color: #e2e8f0; font-size: 0.78rem;",
                                            }
                                        }
                                        div { style: "flex: 2;",
                                            div { style: "font-size: 0.65rem; color: #64748b; margin-bottom: 2px;", "Prix HT" }
                                            input {
                                                r#type: "text",
                                                value: "{line.subprice}",
                                                oninput: move |e| {
                                                    let v = e.value();
                                                    new_lines.with_mut(|lines| {
                                                        if let Some(l) = lines.get_mut(idx) { l.subprice = v; }
                                                    });
                                                },
                                                style: "width: 100%; padding: 5px 8px; background: var(--bg-glass); border: 1px solid var(--border); border-radius: 6px; color: #e2e8f0; font-size: 0.78rem;",
                                            }
                                        }
                                        div { style: "flex: 1;",
                                            div { style: "font-size: 0.65rem; color: #64748b; margin-bottom: 2px;", "TVA %" }
                                            input {
                                                r#type: "text",
                                                value: "{line.tva_tx}",
                                                oninput: move |e| {
                                                    let v = e.value();
                                                    new_lines.with_mut(|lines| {
                                                        if let Some(l) = lines.get_mut(idx) { l.tva_tx = v; }
                                                    });
                                                },
                                                style: "width: 100%; padding: 5px 8px; background: var(--bg-glass); border: 1px solid var(--border); border-radius: 6px; color: #e2e8f0; font-size: 0.78rem;",
                                            }
                                        }
                                    }
                                }
                            }
                            button {
                                style: "padding: 5px 12px; background: transparent; border: 1px dashed var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.72rem; width: 100%;",
                                onclick: move |_| {
                                    new_lines.with_mut(|lines| lines.push(LineDraft::empty()));
                                },
                                "+ Ajouter une ligne"
                            }
                        }
                    }

                    div { style: "display: flex; gap: 8px; margin-top: 20px;",
                        button {
                            class: "primary",
                            style: "flex: 1;",
                            disabled: busy(),
                            onclick: move |_| {
                                let socid = new_socid();
                                let date_str = new_date();
                                let lines_draft = new_lines();
                                busy.set(true);
                                form_msg.set(String::new());
                                spawn(async move {
                                    if socid.is_empty() {
                                        form_msg.set("Selectionne un client".into());
                                        busy.set(false);
                                        return;
                                    }
                                    let date_naive = match chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                                        Ok(d) => d,
                                        Err(_) => {
                                            form_msg.set("Date invalide (AAAA-MM-JJ)".into());
                                            busy.set(false);
                                            return;
                                        }
                                    };
                                    let ts = date_naive.and_hms_opt(12, 0, 0).unwrap().and_utc().timestamp();
                                    let mut payload: Vec<DolibarrInvoiceLine> = Vec::new();
                                    for l in lines_draft.iter() {
                                        if l.desc.trim().is_empty() { continue; }
                                        match l.to_dolibarr_line() {
                                            Some(dl) => payload.push(dl),
                                            None => {
                                                form_msg.set(format!("Ligne invalide : {}", l.desc));
                                                busy.set(false);
                                                return;
                                            }
                                        }
                                    }
                                    if payload.is_empty() {
                                        form_msg.set("Au moins une ligne avec description".into());
                                        busy.set(false);
                                        return;
                                    }
                                    match crate::dolibarr::server_fns::dolibarr_create_invoice(
                                        socid, ts, payload
                                    ).await {
                                        Ok(_id) => {
                                            show_new.set(false);
                                            bump.with_mut(|v| *v += 1);
                                            busy.set(false);
                                        }
                                        Err(e) => {
                                            form_msg.set(format!("{}", e));
                                            busy.set(false);
                                        }
                                    }
                                });
                            },
                            if busy() { "Creation..." } else { "Creer le brouillon" }
                        }
                        button {
                            class: "secondary",
                            onclick: move |_| show_new.set(false),
                            "Annuler"
                        }
                    }
                }
            }
        }

        if let Some(inv) = pay_for_invoice() {
            div {
                class: "dash-modal-overlay",
                onclick: move |_| pay_for_invoice.set(None),
                div {
                    class: "dash-modal",
                    style: "max-width: 480px;",
                    onclick: move |e| e.stop_propagation(),

                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                        div {
                            div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em;", "Encaissement" }
                            h2 { style: "margin: 4px 0 0 0;", "{inv.r#ref}" }
                        }
                        button {
                            style: "background: transparent; border: none; color: #94a3b8; font-size: 1.5rem; cursor: pointer; line-height: 1;",
                            onclick: move |_| pay_for_invoice.set(None),
                            "x"
                        }
                    }

                    div { style: "font-size: 0.75rem; color: #94a3b8; margin-bottom: 16px;",
                        {format!("Total facture : {}", format_amount(parse_amount(&inv.total_ttc)))}
                    }

                    if !pay_msg().is_empty() {
                        div { style: "background: #450a0a; border: 1px solid #7f1d1d; color: #fecaca; padding: 8px 12px; border-radius: 4px; font-size: 0.8rem; margin-bottom: 12px;",
                            "{pay_msg()}"
                        }
                    }

                    div { style: "display: flex; flex-direction: column; gap: 12px;",
                        label { class: "field",
                            span { "Montant a encaisser (EUR)" }
                            input {
                                r#type: "text",
                                value: "{pay_amount}",
                                oninput: move |e| pay_amount.set(e.value()),
                            }
                        }
                        label { class: "field",
                            span { "Date (AAAA-MM-JJ)" }
                            input {
                                r#type: "text",
                                value: "{pay_date}",
                                oninput: move |e| pay_date.set(e.value()),
                            }
                        }
                        label { class: "field",
                            span { "Mode de paiement" }
                            select {
                                value: "{pay_mode}",
                                onchange: move |e| {
                                    if let Ok(v) = e.value().parse::<i32>() {
                                        pay_mode.set(v);
                                    }
                                },
                                option { value: "3", "Virement" }
                                option { value: "2", "Cheque" }
                                option { value: "1", "Especes" }
                                option { value: "4", "Carte bancaire" }
                                option { value: "5", "Prelevement" }
                            }
                        }
                    }

                    div { style: "display: flex; gap: 8px; margin-top: 20px;",
                        button {
                            class: "primary",
                            style: "flex: 1;",
                            disabled: busy(),
                            onclick: {
                                let inv_id = inv.id.clone();
                                move |_| {
                                    let iid = inv_id.clone();
                                    let amount_str = pay_amount();
                                    let date_str = pay_date();
                                    let mode = pay_mode();
                                    busy.set(true);
                                    pay_msg.set(String::new());
                                    spawn(async move {
                                        let amount: f64 = match amount_str.replace(',', ".").parse() {
                                            Ok(v) => v,
                                            Err(_) => {
                                                pay_msg.set("Montant invalide".into());
                                                busy.set(false);
                                                return;
                                            }
                                        };
                                        if amount <= 0.0 {
                                            pay_msg.set("Le montant doit etre > 0".into());
                                            busy.set(false);
                                            return;
                                        }
                                        let d = match chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                                            Ok(d) => d,
                                            Err(_) => {
                                                pay_msg.set("Date invalide".into());
                                                busy.set(false);
                                                return;
                                            }
                                        };
                                        let ts = d.and_hms_opt(12, 0, 0).unwrap().and_utc().timestamp();
                                        // account_id = 1 (compte bancaire principal Dolibarr).
                                        // A adapter si l'ID du compte bancaire est different.
                                        match crate::dolibarr::server_fns::dolibarr_create_payment(
                                            iid, ts, amount, mode, 1
                                        ).await {
                                            Ok(_) => {
                                                pay_for_invoice.set(None);
                                                bump.with_mut(|v| *v += 1);
                                                busy.set(false);
                                            }
                                            Err(e) => {
                                                pay_msg.set(format!("{}", e));
                                                busy.set(false);
                                            }
                                        }
                                    });
                                }
                            },
                            if busy() { "Encaissement..." } else { "Enregistrer l'encaissement" }
                        }
                        button {
                            class: "secondary",
                            onclick: move |_| pay_for_invoice.set(None),
                            "Annuler"
                        }
                    }
                }
            }
        }

        if history_for_invoice().is_some() {
            div {
                class: "dash-modal-overlay",
                onclick: move |_| history_for_invoice.set(None),
                div {
                    class: "dash-modal",
                    style: "max-width: 560px;",
                    onclick: move |e| e.stop_propagation(),

                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                        div {
                            div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em;", "Historique" }
                            h2 { style: "margin: 4px 0 0 0;", "Paiements de la facture" }
                        }
                        button {
                            style: "background: transparent; border: none; color: #94a3b8; font-size: 1.5rem; cursor: pointer; line-height: 1;",
                            onclick: move |_| history_for_invoice.set(None),
                            "x"
                        }
                    }

                    match &*payments_resource.read() {
                        Some(Ok(list)) if list.is_empty() => rsx! {
                            div { class: "empty-state",
                                h3 { "Aucun paiement" }
                                p { "Cette facture n'a pas encore ete encaissee." }
                            }
                        },
                        Some(Ok(list)) => rsx! {
                            div { style: "display: flex; flex-direction: column; gap: 8px;",
                                for p in list.iter() {
                                    {
                                        let amount = parse_amount(&p.amount);
                                        let date_str = format_date(p.datepaye);
                                        let ref_str = if p.num_paiement.is_empty() { format!("ID {}", p.id) } else { p.num_paiement.clone() };
                                        let id_key = p.id.clone();
                                        rsx! {
                                            div {
                                                key: "{id_key}",
                                                style: "background: var(--bg-input); border: 1px solid var(--border); border-radius: 8px; padding: 10px 14px; display: flex; justify-content: space-between; align-items: center;",
                                                div {
                                                    div { style: "font-size: 0.85rem; color: #e2e8f0; font-weight: 500;",
                                                        "Paiement"
                                                    }
                                                    div { style: "font-size: 0.72rem; color: #94a3b8; margin-top: 2px;",
                                                        "{date_str} / Ref {ref_str}"
                                                    }
                                                }
                                                div { style: "font-size: 0.9rem; color: #4ade80; font-weight: 600;",
                                                    {format_amount(amount)}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Some(Err(e)) => rsx! {
                            div { style: "color: #f87171;", {format!("Erreur : {e}")} }
                        },
                        None => rsx! {
                            div { style: "color: #94a3b8;", "Chargement..." }
                        },
                    }
                }
            }
        }
    }
}