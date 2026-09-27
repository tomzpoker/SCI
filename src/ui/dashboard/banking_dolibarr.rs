use chrono::{DateTime, Utc};
use dioxus::prelude::*;

use crate::dolibarr::models::{DolibarrBankAccount, DolibarrBankLine};

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

fn line_type_label(t: &str) -> &'static str {
    match t {
        "VIR" => "Virement",
        "CHQ" => "Cheque",
        "ESP" => "Especes",
        "CB" => "Carte",
        "PRE" => "Prelevement",
        "LIQ" => "Liquide",
        _ => "Autre",
    }
}

#[component]
pub fn BankingDolibarrPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let mut selected_account = use_signal(String::new);

    let accounts = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { crate::dolibarr::server_fns::dolibarr_list_bank_accounts().await }
    });

    // Initialise le compte selectionne des que la liste est chargee
    let accounts_for_init = accounts.read().clone();
    if selected_account().is_empty() {
        if let Some(Ok(list)) = accounts_for_init.as_ref() {
            if let Some(first) = list.first() {
                selected_account.set(first.id.clone());
            }
        }
    }

    let lines = use_resource(move || {
        let acc = selected_account();
        let _ = bump();
        async move {
            if acc.is_empty() {
                Ok(Vec::<DolibarrBankLine>::new())
            } else {
                crate::dolibarr::server_fns::dolibarr_list_bank_lines(acc).await
            }
        }
    });

    rsx! {
        section { class: "page-intro",
            div {
                div { class: "eyebrow", "ARGENT / DOLIBARR" }
                h2 { "Argent" }
                p { "Comptes et mouvements bancaires lus depuis Dolibarr. La saisie et le rapprochement se font dans Dolibarr." }
            }
        }

        section { class: "panel",
            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap;",
                div { style: "font-size: 0.82rem; color: #94a3b8;",
                    "Gestion bancaire complete dans Dolibarr"
                }
                div { style: "display: flex; gap: 8px;",
                    button {
                        style: "padding: 5px 12px; background: transparent; color: #a78bfa; border: 1px solid #7c3aed; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;",
                        onclick: move |_| {
                            let _ = document::eval("window.open('http://localhost:8081/compta/bank/list.php', '_blank');");
                        },
                        "Ouvrir les comptes"
                    }
                    button {
                        style: "padding: 5px 12px; background: #7c3aed; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;",
                        onclick: {
                            let acc_id = selected_account();
                            move |_| {
                                let id = if acc_id.is_empty() { "1".to_string() } else { acc_id.clone() };
                                let url = format!("http://localhost:8081/compta/bank/bankentries_list.php?id={}", id);
                                let script = format!("window.open('{}', '_blank');", url);
                                let _ = document::eval(&script);
                            }
                        },
                        "Saisir un mouvement"
                    }
                    button {
                        style: "padding: 5px 12px; background: transparent; color: #a78bfa; border: 1px solid #7c3aed; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;",
                        onclick: {
                            let acc_id = selected_account();
                            move |_| {
                                let id = if acc_id.is_empty() { "1".to_string() } else { acc_id.clone() };
                                let url = format!("http://localhost:8081/compta/bank/bankentries_list.php?id={}", id);
                                let script = format!("window.open('{}', '_blank');", url);
                                let _ = document::eval(&script);
                            }
                        },
                        "Rapprocher"
                    }
                }
            }
        }

        // --- Liste des comptes ---
        match &*accounts.read() {
            Some(Ok(accs)) if !accs.is_empty() => {
                rsx! {
                    section { class: "panel",
                        div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px;",
                            "Comptes bancaires"
                        }
                        div { style: "display: flex; flex-direction: column; gap: 6px;",
                            for a in accs.iter() {
                                {
                                    let is_selected = selected_account() == a.id;
                                    let color = if is_selected { "#7c3aed" } else { "#475569" };
                                    let bg = if is_selected { "rgba(124,58,237,0.10)" } else { "var(--bg-glass)" };
                                    let acc_id = a.id.clone();
                                    let label = if a.label.trim().is_empty() { a.r#ref.clone() } else { a.label.clone() };
                                    let banque = if a.bank.trim().is_empty() { "-".to_string() } else { a.bank.clone() };
                                    let solde = a.solde as f64 / 100.0;
                                    let iban_tail = if a.iban.len() > 6 { format!("...{}", &a.iban[a.iban.len()-4..]) } else { a.iban.clone() };
                                    rsx! {
                                        div {
                                            key: "{acc_id}",
                                            style: "background: {bg}; border: 1px solid var(--border); border-left: 3px solid {color}; padding: 10px 14px; border-radius: 10px; display: flex; align-items: center; gap: 12px; cursor: pointer;",
                                            onclick: move |_| {
                                                selected_account.set(acc_id.clone());
                                            },
                                            div { style: "flex: 1; min-width: 0;",
                                                div { style: "font-size: 0.88rem; color: #e2e8f0; font-weight: 500;",
                                                    "{label}"
                                                }
                                                div { style: "font-size: 0.72rem; color: #94a3b8; margin-top: 2px;",
                                                    span { "{banque}" }
                                                    span { style: "color: #334155;", " / " }
                                                    span { "IBAN {iban_tail}" }
                                                }
                                            }
                                            div { style: "text-align: right; flex-shrink: 0;",
                                                div { style: "font-size: 0.92rem; color: #4ade80; font-weight: 600;",
                                                    {format_amount(solde)}
                                                }
                                                div { style: "font-size: 0.68rem; color: #64748b;",
                                                    "Solde"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            _ => rsx! {}
        }

        // --- Liste des mouvements ---
        section { class: "panel",
            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;",
                span { class: "small", "Mouvements recents" }
                if !selected_account().is_empty() {
                    button {
                        style: "padding: 4px 10px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.7rem;",
                        onclick: move |_| bump.with_mut(|v| *v += 1),
                        "Rafraichir"
                    }
                }
            }

            match &*lines.read() {
                Some(Ok(list)) if list.is_empty() => rsx! {
                    div { class: "empty-state",
                        h3 { "Aucun mouvement" }
                        p { "Ce compte n'a pas encore de mouvements. Utilise le bouton 'Saisir un mouvement' pour en ajouter dans Dolibarr." }
                    }
                },
                Some(Ok(list)) => rsx! {
                    div { style: "display: flex; flex-direction: column; gap: 4px;",
                        for line in list.iter().take(100) {
                            {
                                let amount = parse_amount(&line.amount);
                                let is_in = amount >= 0.0;
                                let amount_color = if is_in { "#4ade80" } else { "#f87171" };
                                let amount_prefix = if is_in { "+" } else { "" };
                                let date_str = format_date(line.dateo);
                                let type_label = line_type_label(&line.r#type);
                                let rappro = line.rappro;
                                let is_rappro = rappro == 1;
                                let label = if line.label.trim().is_empty() { "-".to_string() } else { line.label.clone() };
                                let line_id = line.id.clone();
                                rsx! {
                                    div {
                                        key: "{line_id}",
                                        style: "background: var(--bg-glass); border: 1px solid var(--border); padding: 8px 12px; border-radius: 8px; display: flex; align-items: center; gap: 10px;",
                                        div { style: "flex: 1; min-width: 0;",
                                            div { style: "font-size: 0.82rem; color: #e2e8f0;",
                                                "{label}"
                                            }
                                            div { style: "font-size: 0.68rem; color: #94a3b8; margin-top: 2px;",
                                                span { "{date_str}" }
                                                span { style: "color: #334155;", " / " }
                                                span { "{type_label}" }
                                                if is_rappro {
                                                    span { style: "color: #334155;", " / " }
                                                    span { style: "color: #4ade80;", "Rapproche" }
                                                }
                                            }
                                        }
                                        div { style: "font-size: 0.88rem; color: {amount_color}; font-weight: 600; flex-shrink: 0;",
                                            "{amount_prefix}{format_amount(amount)}"
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