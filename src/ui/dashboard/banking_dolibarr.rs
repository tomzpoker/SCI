use chrono::{DateTime, NaiveDate, Utc};
use dioxus::prelude::*;

use crate::dolibarr::models::{DolibarrBankAccount, DolibarrBankLine, DolibarrInvoice, DolibarrThirdParty};

// ============================================================
//  Parsing CSV
// ============================================================

#[derive(Clone, PartialEq)]
struct CsvLine {
    raw: String,
    date_ts: i64,
    amount: f64,
    label: String,
    ext_id: String,
}

#[derive(Clone, PartialEq)]
struct MatchProposal {
    csv: CsvLine,
    invoice_id: Option<String>,
    invoice_ref: Option<String>,
    third_party: Option<String>,
    score: i32,
    reason: String,
    enabled: bool,
}

fn parse_amount_fr(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() { return None; }
    let mut t = t.replace(" ", "").replace("EUR", "").replace("eur", "");
    t = t.replace("\u{00e2}\u{0082}\u{00ac}", "");
    if t.contains(',') && t.contains('.') {
        if t.rfind(',') > t.rfind('.') {
            t = t.replace(".", "").replace(',', ".");
        } else {
            t = t.replace(",", "");
        }
    } else {
        t = t.replace(',', ".");
    }
    if t.is_empty() { return None; }
    t.parse::<f64>().ok()
}

fn parse_date_fr(s: &str) -> Option<i64> {
    let t = s.trim();
    let naivedate = NaiveDate::parse_from_str(t, "%d/%m/%Y")
        .or_else(|_| NaiveDate::parse_from_str(t, "%Y-%m-%d"))
        .or_else(|_| NaiveDate::parse_from_str(t, "%d-%m-%Y"))
        .ok()?;
    let dt = naivedate.and_hms_opt(12, 0, 0)?;
    Some(dt.and_utc().timestamp())
}

fn split_csv_line(line: &str) -> Vec<String> {
    let sep = if line.contains(';') { ';' } else { ',' };
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            _ if c == sep && !in_quotes => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    parts.push(current.trim().to_string());
    parts
}

fn parse_csv(content: &str) -> Vec<CsvLine> {
    let mut result = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for raw_line in content.lines() {
        let trimmed = raw_line.trim_end_matches('\r');
        if in_quotes {
            current.push('\n');
            current.push_str(trimmed);
        } else {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            current.push_str(trimmed);
        }
        let mut quote_count = 0;
        let mut escaped = false;
        for c in trimmed.chars() {
            if escaped { escaped = false; continue; }
            if c == '\\' { escaped = true; continue; }
            if c == '"' { quote_count += 1; }
        }
        if quote_count % 2 == 1 {
            in_quotes = !in_quotes;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }

    let mut start_idx = 0;
    for (i, l) in lines.iter().enumerate() {
        let lower = l.trim_start().to_lowercase();
        if lower.starts_with("date") || lower.starts_with("\u{feff}date") {
            start_idx = i + 1;
            break;
        }
    }

    for line in lines.iter().skip(start_idx) {
        let trimmed = line.trim().trim_start_matches('\u{feff}');
        if trimmed.is_empty() { continue; }
        let lower = trimmed.to_lowercase();
        if lower.starts_with("solde") || lower.contains("titulaire") || lower.contains("porteur") {
            continue;
        }

        let parts = split_csv_line(trimmed);
        if parts.len() < 3 { continue; }

        let Some(date_ts) = parse_date_fr(&parts[0]) else { continue };

        let (label_idx, debit_idx, credit_idx) = if parts.len() >= 5 {
            (2, 3, 4)
        } else {
            (1, 2, 3)
        };

        let label = parts.get(label_idx).cloned().unwrap_or_default();
        let debit = parts.get(debit_idx).and_then(|s| parse_amount_fr(s));
        let credit = parts.get(credit_idx).and_then(|s| parse_amount_fr(s));

        let amount = match (debit, credit) {
            (Some(d), _) if d.abs() > 0.0 => -d.abs(),
            (_, Some(c)) if c.abs() > 0.0 => c.abs(),
            _ => continue,
        };

        let ext_id = if parts.len() > credit_idx + 1 {
            parts[credit_idx + 1].clone()
        } else {
            String::new()
        };

        result.push(CsvLine {
            raw: trimmed.to_string(),
            date_ts,
            amount,
            label,
            ext_id,
        });
    }

    result
}

// ============================================================
//  Matching
// ============================================================

fn parse_total(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}

fn invoice_is_open(inv: &DolibarrInvoice) -> bool {
    let statut: i32 = inv.statut;
    let paye: i32 = inv.paye.parse().unwrap_or(0);
    statut >= 1 && paye == 0
}

fn match_line(
    csv: &CsvLine,
    invoices: &[DolibarrInvoice],
    thirds: &[DolibarrThirdParty],
) -> MatchProposal {
    let mut best: Option<(i32, String, String, String, String)> = None;

    if csv.amount <= 0.0 {
        return MatchProposal {
            csv: csv.clone(),
            invoice_id: None,
            invoice_ref: None,
            third_party: None,
            score: 0,
            reason: "Debit (pas de matching)".to_string(),
            enabled: true,
        };
    }

    let label_lower = csv.label.to_lowercase();

    for inv in invoices.iter().filter(|i| invoice_is_open(i)) {
        let ttc = parse_total(&inv.total_ttc);
        let diff = (ttc - csv.amount).abs();
        let mut score = 0i32;
        let mut reasons: Vec<&'static str> = Vec::new();

        if diff < 0.01 {
            score += 40;
            reasons.push("montant exact");
        } else if diff < 1.0 {
            score += 25;
            reasons.push("montant proche");
        } else if ttc > 0.0 && diff < ttc * 0.05 {
            score += 10;
            reasons.push("montant approximatif");
        }

        let tp_name = thirds
            .iter()
            .find(|t| t.id == inv.socid)
            .map(|t| t.name.clone())
            .unwrap_or_default();
        let tp_lower = tp_name.to_lowercase();
        if !tp_lower.is_empty() && label_lower.contains(&tp_lower) {
            score += 40;
            reasons.push("nom tiers");
        } else if !tp_lower.is_empty() {
            let words: Vec<&str> = tp_lower.split_whitespace().filter(|w| w.len() >= 4).collect();
            if words.iter().any(|w| label_lower.contains(w)) {
                score += 20;
                reasons.push("nom partiel");
            }
        }

        let inv_ts = inv.date;
        if inv_ts > 0 && inv_ts < csv.date_ts {
            score += 20;
            reasons.push("date coherente");
        }

        let ref_lower = inv.r#ref.to_lowercase();
        if !ref_lower.is_empty() && label_lower.contains(&ref_lower) {
            score += 30;
            reasons.push("ref facture");
        }

        let reason_str = if reasons.is_empty() {
            "aucun signal".to_string()
        } else {
            reasons.join(" + ")
        };

        let candidate = (
            score,
            inv.id.clone(),
            inv.r#ref.clone(),
            tp_name.clone(),
            reason_str,
        );

        match &best {
            None => best = Some(candidate),
            Some((s, _, _, _, _)) if *s < candidate.0 => best = Some(candidate),
            _ => {}
        }
    }

    match best {
        Some((score, inv_id, inv_ref, tp_name, reason)) if score >= 40 => MatchProposal {
            csv: csv.clone(),
            invoice_id: Some(inv_id),
            invoice_ref: Some(inv_ref),
            third_party: Some(tp_name),
            score,
            reason,
            enabled: true,
        },
        _ => MatchProposal {
            csv: csv.clone(),
            invoice_id: None,
            invoice_ref: None,
            third_party: None,
            score: best.map(|b| b.0).unwrap_or(0),
            reason: "Aucun match fiable".to_string(),
            enabled: true,
        },
    }
}

// ============================================================
//  Helpers
// ============================================================

fn score_color(score: i32) -> &'static str {
    if score >= 80 { "#4ade80" }
    else if score >= 60 { "#fbbf24" }
    else if score >= 40 { "#fb923c" }
    else { "#64748b" }
}

fn format_signed_amount(v: f64) -> String {
    if v >= 0.0 { format!("+{:.2} EUR", v) } else { format!("{:.2} EUR", v) }
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

fn parse_amount_str(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}

fn compute_solde(lines: &[DolibarrBankLine]) -> f64 {
    lines.iter().map(|l| parse_amount_str(&l.amount)).sum()
}

fn sort_lines_desc(mut lines: Vec<DolibarrBankLine>) -> Vec<DolibarrBankLine> {
    lines.sort_by(|a, b| b.dateo.cmp(&a.dateo));
    lines
}

// Action de suppression en attente de confirmation
#[derive(Clone, PartialEq)]
enum PendingDelete {
    Single(String, String),  // id, label
    All(Vec<String>),        // ids de tous les non-rapproches
}

// ============================================================
//  Page
// ============================================================

#[component]
pub fn BankingDolibarrPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);

    let accounts = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { crate::dolibarr::server_fns::dolibarr_list_bank_accounts().await }
    });

    let invoices = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { crate::dolibarr::server_fns::dolibarr_list_invoices(500).await }
    });

    let thirds = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { crate::dolibarr::server_fns::dolibarr_list_third_parties(500).await }
    });

    let mut selected_account_id = use_signal(String::new);

    if selected_account_id().is_empty() {
        let accs = accounts.read();
        let default_id = match accs.as_ref() {
            Some(Ok(list)) => list.first().map(|a| a.id.clone()).unwrap_or_else(|| "1".to_string()),
            _ => "1".to_string(),
        };
        selected_account_id.set(default_id);
    }

    let lines = use_resource(move || {
        let acc = selected_account_id();
        let _ = bump();
        async move {
            if acc.is_empty() {
                Ok(Vec::<DolibarrBankLine>::new())
            } else {
                crate::dolibarr::server_fns::dolibarr_list_bank_lines(acc).await
            }
        }
    });

    let mut csv_input = use_signal(String::new);
    let mut proposals = use_signal(Vec::<MatchProposal>::new);
    let mut busy = use_signal(|| false);
    let mut msg = use_signal(String::new);
    let mut pushed = use_signal(|| 0usize);
    let mut file_name = use_signal(String::new);
    let mut show_import = use_signal(|| false);
    let mut pending_delete = use_signal(|| None::<PendingDelete>);
    let mut bulk_msg = use_signal(String::new);

    let solde_reel: f64 = match &*lines.read() {
        Some(Ok(list)) => compute_solde(list),
        _ => 0.0,
    };

    rsx! {
        section { class: "page-intro",
            div {
                div { class: "eyebrow", "ARGENT / DOLIBARR" }
                h2 { "Finances" }
                p { "Comptes, mouvements et rapprochement bancaire. Import CSV Credit Agricole integre." }
            }
        }

        // === Barre d'actions ===
        section { class: "panel",
            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap;",
                div { style: "display: flex; gap: 8px; flex-wrap: wrap;",
                    button {
                        style: if show_import() {
                            "padding: 5px 12px; background: #7c3aed; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;"
                        } else {
                            "padding: 5px 12px; background: transparent; color: #a78bfa; border: 1px solid #7c3aed; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;"
                        },
                        onclick: move |_| {
                            let cur = show_import();
                            show_import.set(!cur);
                        },
                        if show_import() { "Masquer l'import CSV" } else { "Importer un releve CSV" }
                    }
                    button {
                        style: "padding: 5px 12px; background: transparent; color: #a78bfa; border: 1px solid #7c3aed; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;",
                        onclick: {
                            let acc_id = selected_account_id();
                            move |_| {
                                let id = if acc_id.is_empty() { "1".to_string() } else { acc_id.clone() };
                                let url = format!("http://localhost:8081/compta/bank/bankentries_list.php?id={}", id);
                                let script = format!("window.open('{}', '_blank');", url);
                                let _ = document::eval(&script);
                            }
                        },
                        "Ouvrir dans Dolibarr"
                    }
                }
            }
        }

        // === Section Import CSV ===
        if show_import() {
            section { class: "panel",
                div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;",
                    "Import CSV bancaire"
                }

                div { style: "display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-bottom: 10px;",
                    label {
                        style: "display: inline-block; padding: 8px 16px; background: #7c3aed; color: white; border-radius: 6px; cursor: pointer; font-size: 0.8rem; font-weight: 600;",
                        "Choisir un fichier CSV"
                        input {
                            r#type: "file",
                            accept: ".csv,text/csv",
                            style: "display: none;",
                            onchange: move |evt: FormEvent| {
                                let files = evt.files();
                                if let Some(file) = files.first() {
                                    let name = file.name();
                                    file_name.set(name);
                                    let file = file.clone();
                                    spawn(async move {
                                        if let Ok(content) = file.read_string().await {
                                            csv_input.set(content);
                                        }
                                    });
                                }
                            }
                        }
                    }
                    if !file_name().is_empty() {
                        div { style: "font-size: 0.78rem; color: #4ade80;",
                            "Fichier : {file_name()}"
                        }
                    }
                    if !csv_input().is_empty() {
                        button {
                            style: "padding: 6px 12px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.72rem;",
                            onclick: move |_| {
                                csv_input.set(String::new());
                                file_name.set(String::new());
                                proposals.set(Vec::new());
                            },
                            "Vider"
                        }
                    }
                }

                div { style: "font-size: 0.72rem; color: #64748b; margin-bottom: 8px;",
                    "Ou colle directement le contenu CSV :"
                }
                textarea {
                    value: "{csv_input}",
                    oninput: move |e| csv_input.set(e.value()),
                    rows: "5",
                    placeholder: "Date operation;Libelle;Debit euros;Credit euros\n24/01/2026;VIR INST vers SDC LA BELLE;-2000,00;",
                    style: "width: 100%; padding: 10px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 8px; color: #e2e8f0; font-size: 0.8rem; font-family: monospace; resize: vertical;",
                }

                div { style: "display: flex; gap: 8px; align-items: center; margin-top: 12px; flex-wrap: wrap;",
                    button {
                        class: "primary",
                        disabled: busy() || csv_input().trim().is_empty(),
                        onclick: move |_| {
                            let raw = csv_input();
                            busy.set(true);
                            msg.set(String::new());
                            let invoices_vec: Vec<DolibarrInvoice> = invoices.read().as_ref().and_then(|r| r.as_ref().ok()).map(|v| v.clone()).unwrap_or_default();
                            let thirds_vec: Vec<DolibarrThirdParty> = thirds.read().as_ref().and_then(|r| r.as_ref().ok()).map(|v| v.clone()).unwrap_or_default();
                            let csv_lines = parse_csv(&raw);
                            if csv_lines.is_empty() {
                                msg.set("Aucune ligne valide detectee.".into());
                                busy.set(false);
                                return;
                            }
                            let mut result = Vec::new();
                            for line in csv_lines.iter() {
                                result.push(match_line(line, &invoices_vec, &thirds_vec));
                            }
                            proposals.set(result);
                            busy.set(false);
                        },
                        "Analyser le CSV"
                    }
                }

                if !msg().is_empty() {
                    div { style: "margin-top: 8px; color: #f87171; font-size: 0.8rem;", "{msg()}" }
                }
            }
        }

        // === Matching proposals ===
        if !proposals().is_empty() {
            section { class: "panel",
                div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;",
                    {format!("Verification du matching ({} lignes)", proposals().len())}
                }

                div { style: "display: flex; flex-direction: column; gap: 6px;",
                    for (idx, p) in proposals().iter().enumerate() {
                        {
                            let amount = p.csv.amount;
                            let amount_color = if amount > 0.0 { "#4ade80" } else { "#f87171" };
                            let score_col = score_color(p.score);
                            let enabled = p.enabled;
                            let date_str = format_date(p.csv.date_ts);
                            let label = p.csv.label.clone();
                            let invoice_ref = p.invoice_ref.clone().unwrap_or_else(|| "-".to_string());
                            let reason = p.reason.clone();
                            let score = p.score;
                            let id_key = format!("{}", idx);
                            let signed = format_signed_amount(amount);
                            rsx! {
                                div {
                                    key: "{id_key}",
                                    style: "background: var(--bg-glass); border: 1px solid var(--border); border-left: 3px solid {score_col}; padding: 8px 12px; border-radius: 8px; display: flex; align-items: center; gap: 10px;",
                                    input {
                                        r#type: "checkbox",
                                        checked: "{enabled}",
                                        onchange: move |e| {
                                            let checked = e.value() == "true";
                                            proposals.with_mut(|list| {
                                                if let Some(item) = list.get_mut(idx) {
                                                    item.enabled = checked;
                                                }
                                            });
                                        },
                                        style: "flex-shrink: 0;",
                                    }
                                    div { style: "flex: 1; min-width: 0;",
                                        div { style: "font-size: 0.82rem; color: #e2e8f0;", "{label}" }
                                        div { style: "font-size: 0.68rem; color: #94a3b8; margin-top: 2px;",
                                            span { "{date_str}" }
                                            span { style: "color: #334155;", " / " }
                                            span { "Match: {invoice_ref}" }
                                            span { style: "color: #334155;", " / " }
                                            span { "Score {score} / {reason}" }
                                        }
                                    }
                                    div { style: "font-size: 0.85rem; color: {amount_color}; font-weight: 600; flex-shrink: 0;",
                                        "{signed}"
                                    }
                                }
                            }
                        }
                    }
                }

                div { style: "display: flex; gap: 8px; margin-top: 14px; align-items: center;",
                    button {
                        class: "primary",
                        disabled: busy() || selected_account_id().is_empty(),
                        onclick: move |_| {
                            let acc_id = selected_account_id();
                            let list = proposals();
                            busy.set(true);
                            msg.set(String::new());
                            spawn(async move {
                                let mut ok_count = 0usize;
                                let mut err_count = 0usize;
                                let mut first_error: Option<String> = None;
                                for p in list.iter() {
                                    if !p.enabled { continue; }
                                    let line_type = if p.csv.amount >= 0.0 { "VIR" } else { "PRE" };
                                    let result = crate::dolibarr::server_fns::dolibarr_create_bank_line(
                                        acc_id.clone(),
                                        p.csv.date_ts,
                                        p.csv.amount,
                                        p.csv.label.clone(),
                                        line_type.to_string(),
                                        p.csv.ext_id.clone(),
                                    ).await;
                                    match result {
                                        Ok(_) => ok_count += 1,
                                        Err(e) => {
                                            err_count += 1;
                                            if first_error.is_none() {
                                                first_error = Some(format!("{}", e));
                                            }
                                        }
                                    }
                                }
                                pushed.set(ok_count);
                                if err_count > 0 {
                                    let err_detail = first_error.unwrap_or_else(|| "Erreur inconnue".to_string());
                                    msg.set(format!("{} OK, {} erreur(s). Detail : {}", ok_count, err_count, err_detail));
                                } else {
                                    msg.set(format!("{} ligne(s) poussee(s) dans Dolibarr.", ok_count));
                                    proposals.set(Vec::new());
                                    file_name.set(String::new());
                                    csv_input.set(String::new());
                                }
                                busy.set(false);
                                bump.with_mut(|v| *v += 1);
                            });
                        },
                        if busy() { "Envoi..." } else { "Pousser vers Dolibarr" }
                    }
                    button {
                        class: "secondary",
                        onclick: move |_| {
                            proposals.set(Vec::new());
                            pushed.set(0);
                            msg.set(String::new());
                        },
                        "Annuler"
                    }
                    if pushed() > 0 {
                        div { style: "font-size: 0.78rem; color: #4ade80; margin-left: auto;",
                            "{pushed()} ligne(s) OK"
                        }
                    }
                }
                if !msg().is_empty() {
                    div { style: "margin-top: 8px; color: #94a3b8; font-size: 0.8rem;", "{msg()}" }
                }
            }
        }

        // === Liste des comptes ===
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
                                    let is_selected = selected_account_id() == a.id;
                                    let color = if is_selected { "#7c3aed" } else { "#475569" };
                                    let bg = if is_selected { "rgba(124,58,237,0.10)" } else { "var(--bg-glass)" };
                                    let acc_id = a.id.clone();
                                    let label = if a.label.trim().is_empty() { a.r#ref.clone() } else { a.label.clone() };
                                    let banque = if a.bank.trim().is_empty() { "-".to_string() } else { a.bank.clone() };
                                    let solde_display = if is_selected { Some(solde_reel) } else { None };
                                    let iban_tail = if a.iban.len() > 6 { format!("...{}", &a.iban[a.iban.len()-4..]) } else { a.iban.clone() };
                                    rsx! {
                                        div {
                                            key: "{acc_id}",
                                            style: "background: {bg}; border: 1px solid var(--border); border-left: 3px solid {color}; padding: 10px 14px; border-radius: 10px; display: flex; align-items: center; gap: 12px; cursor: pointer;",
                                            onclick: move |_| {
                                                selected_account_id.set(acc_id.clone());
                                            },
                                            div { style: "flex: 1; min-width: 0;",
                                                div { style: "font-size: 0.88rem; color: #e2e8f0; font-weight: 500;", "{label}" }
                                                div { style: "font-size: 0.72rem; color: #94a3b8; margin-top: 2px;",
                                                    span { "{banque}" }
                                                    if !iban_tail.is_empty() {
                                                        span { style: "color: #334155;", " / " }
                                                        span { "IBAN {iban_tail}" }
                                                    }
                                                }
                                            }
                                            div { style: "text-align: right; flex-shrink: 0;",
                                                match solde_display {
                                                    Some(s) => rsx! {
                                                        div { style: "font-size: 0.92rem; color: #4ade80; font-weight: 600;",
                                                            {format_amount(s)}
                                                        }
                                                        div { style: "font-size: 0.68rem; color: #64748b;", "Solde" }
                                                    },
                                                    None => rsx! {
                                                        div { style: "font-size: 0.7rem; color: #64748b;",
                                                            "Cliquer pour voir"
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
            },
            Some(Err(e)) => rsx! {
                section { class: "panel",
                    div { style: "color: #f87171; font-size: 0.8rem;", {format!("Erreur comptes : {e}")} }
                }
            },
            _ => rsx! {}
        }

        // === Mouvements ===
        match &*lines.read() {
            Some(Ok(raw_list)) => {
                let sorted = sort_lines_desc(raw_list.clone());
                let non_rappro: Vec<DolibarrBankLine> = sorted.iter().filter(|l| l.rappro != 1).cloned().collect();
                let rappro: Vec<DolibarrBankLine> = sorted.iter().filter(|l| l.rappro == 1).cloned().collect();
                let non_rappro_count = non_rappro.len();
                let rappro_count = rappro.len();
                let non_rappro_ids: Vec<String> = non_rappro.iter().map(|l| l.id.clone()).collect();

                rsx! {
                    // --- Section non rapprochés ---
                    section { class: "panel",
                        div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px; flex-wrap: wrap; gap: 8px;",
                            div {
                                div { style: "font-size: 0.92rem; color: #e2e8f0; font-weight: 600;",
                                    if non_rappro_count > 0 {
                                        {format!("{} mouvement{} a traiter", non_rappro_count, if non_rappro_count > 1 { "s" } else { "" })}
                                    } else {
                                        "Aucun mouvement a traiter"
                                    }
                                }
                                if non_rappro_count > 0 {
                                    div { style: "font-size: 0.72rem; color: #94a3b8; margin-top: 3px;",
                                        "Ces flux ne sont pas encore rapproches. Tu peux les supprimer ou les rapprocher dans Dolibarr."
                                    }
                                }
                            }
                            if non_rappro_count > 0 {
                                div { style: "display: flex; gap: 8px;",
                                    button {
                                        style: "padding: 5px 12px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.72rem;",
                                        onclick: move |_| bump.with_mut(|v| *v += 1),
                                        "Rafraichir"
                                    }
                                    button {
                                        style: "padding: 5px 14px; background: transparent; border: 1px solid #dc2626; color: #f87171; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;",
                                        onclick: {
                                            let ids = non_rappro_ids.clone();
                                            move |_| {
                                                if !ids.is_empty() {
                                                    pending_delete.set(Some(PendingDelete::All(ids.clone())));
                                                }
                                            }
                                        },
                                        "Tout supprimer"
                                    }
                                }
                            }
                        }

                        if !bulk_msg().is_empty() {
                            div { style: "margin-bottom: 10px; padding: 8px 12px; background: rgba(74,222,128,0.08); border: 1px solid rgba(74,222,128,0.25); color: #4ade80; font-size: 0.78rem; border-radius: 6px;",
                                "{bulk_msg()}"
                            }
                        }

                        if non_rappro.is_empty() {
                            div { style: "padding: 20px 16px; text-align: center; color: #64748b; font-size: 0.8rem; background: var(--bg-input); border: 1px dashed var(--border-strong); border-radius: 8px;",
                                if rappro_count > 0 {
                                    {format!("Tout est a jour. {} mouvement{} deja rapproche{}.", rappro_count, if rappro_count > 1 { "s" } else { "" }, if rappro_count > 1 { "s" } else { "" })}
                                } else {
                                    "Aucun mouvement sur ce compte."
                                }
                            }
                        } else {
                            div { style: "display: flex; flex-direction: column; gap: 4px;",
                                for line in non_rappro.iter() {
                                    {
                                        let amount = parse_amount_str(&line.amount);
                                        let is_in = amount >= 0.0;
                                        let amount_color = if is_in { "#4ade80" } else { "#f87171" };
                                        let date_str = format_date(line.dateo);
                                        let label = if line.label.trim().is_empty() { "-".to_string() } else { line.label.clone() };
                                        let line_id = line.id.clone();
                                        let line_id_for_key = line_id.clone();
                                        let line_id_for_del = line_id.clone();
                                        let label_for_del = label.clone();
                                        let signed = format_signed_amount(amount);
                                        rsx! {
                                            div {
                                                key: "{line_id_for_key}",
                                                style: "background: var(--bg-glass); border: 1px solid var(--border); border-left: 3px solid #fbbf24; padding: 10px 14px; border-radius: 8px; display: flex; align-items: center; gap: 12px;",
                                                div { style: "flex: 1; min-width: 0;",
                                                    div { style: "font-size: 0.84rem; color: #e2e8f0;", "{label}" }
                                                    div { style: "font-size: 0.7rem; color: #94a3b8; margin-top: 2px;", "{date_str}" }
                                                }
                                                div { style: "font-size: 0.88rem; color: {amount_color}; font-weight: 600; flex-shrink: 0;",
                                                    "{signed}"
                                                }
                                                button {
                                                    title: "Supprimer ce mouvement",
                                                    disabled: busy(),
                                                    onclick: move |_| {
                                                        pending_delete.set(Some(PendingDelete::Single(line_id_for_del.clone(), label_for_del.clone())));
                                                    },
                                                    style: "padding: 5px 12px; background: transparent; border: 1px solid var(--border); color: #f87171; border-radius: 6px; cursor: pointer; font-size: 0.72rem; font-weight: 600; flex-shrink: 0;",
                                                    "Suppr"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // --- Section déjà rapprochés (collapsible) ---
                    if rappro_count > 0 {
                        section { class: "panel",
                            details {
                                summary {
                                    style: "cursor: pointer; color: #94a3b8; font-size: 0.8rem; padding: 4px 0; user-select: none;",
                                    {format!("{} mouvement{} deja rapproche{} (cliquer pour afficher)", rappro_count, if rappro_count > 1 { "s" } else { "" }, if rappro_count > 1 { "s" } else { "" })}
                                }
                                div { style: "display: flex; flex-direction: column; gap: 4px; margin-top: 12px; opacity: 0.7;",
                                    for line in rappro.iter().take(50) {
                                        {
                                            let amount = parse_amount_str(&line.amount);
                                            let is_in = amount >= 0.0;
                                            let amount_color = if is_in { "#4ade80" } else { "#f87171" };
                                            let date_str = format_date(line.dateo);
                                            let label = if line.label.trim().is_empty() { "-".to_string() } else { line.label.clone() };
                                            let line_id = line.id.clone();
                                            let signed = format_signed_amount(amount);
                                            rsx! {
                                                div {
                                                    key: "{line_id}",
                                                    style: "background: var(--bg-input); border: 1px solid var(--border); padding: 8px 12px; border-radius: 6px; display: flex; align-items: center; gap: 10px;",
                                                    div { style: "flex: 1; min-width: 0;",
                                                        div { style: "font-size: 0.8rem; color: #cbd5e1;", "{label}" }
                                                        div { style: "font-size: 0.68rem; color: #64748b; margin-top: 2px;", "{date_str}" }
                                                    }
                                                    div { style: "font-size: 0.82rem; color: {amount_color}; font-weight: 600; flex-shrink: 0;",
                                                        "{signed}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if rappro_count > 50 {
                                    div { style: "margin-top: 8px; color: #64748b; font-size: 0.72rem; text-align: center;",
                                        {format!("... et {} autres", rappro_count - 50)}
                                    }
                                }
                            }
                        }
                    }
                }
            },
            Some(Err(e)) => rsx! {
                section { class: "panel",
                    div { style: "color: #f87171; font-size: 0.8rem;", {format!("Erreur mouvements : {e}")} }
                }
            },
            None => rsx! {
                section { class: "panel",
                    div { style: "color: #94a3b8;", "Chargement des mouvements..." }
                }
            },
        }
            Some(Ok(raw_list)) => {
                let sorted = sort_lines_desc(raw_list.clone());
                let non_rappro: Vec<DolibarrBankLine> = sorted.iter().filter(|l| l.rappro != 1).cloned().collect();
                let rappro: Vec<DolibarrBankLine> = sorted.iter().filter(|l| l.rappro == 1).cloned().collect();
                let non_rappro_count = non_rappro.len();
                let rappro_count = rappro.len();
                let non_rappro_ids: Vec<String> = non_rappro.iter().map(|l| l.id.clone()).collect();

                rsx! {
                    // --- Section 1 : Non rapprochés (derniers imports) ---
                    section { class: "panel",
                        div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; flex-wrap: wrap; gap: 8px;",
                            div {
                                div { style: "font-size: 0.7rem; color: #fbbf24; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 700;",
                                    {format!("Derniers imports ({} non rapproches)", non_rappro_count)}
                                }
                                div { style: "font-size: 0.72rem; color: #64748b; margin-top: 2px;",
                                    "Ces mouvements peuvent etre supprimes tant qu'ils ne sont pas rapproches dans Dolibarr."
                                }
                            }
                            div { style: "display: flex; gap: 8px;",
                                button {
                                    style: "padding: 4px 10px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.7rem;",
                                    onclick: move |_| bump.with_mut(|v| *v += 1),
                                    "Rafraichir"
                                }
                                if non_rappro_count > 0 {
                                    button {
                                        style: "padding: 4px 12px; background: transparent; border: 1px solid #dc2626; color: #f87171; border-radius: 6px; cursor: pointer; font-size: 0.72rem; font-weight: 600;",
                                        onclick: {
                                            let ids = non_rappro_ids.clone();
                                            move |_| {
                                                if !ids.is_empty() {
                                                    pending_delete.set(Some(PendingDelete::All(ids.clone())));
                                                }
                                            }
                                        },
                                        {format!("Supprimer tout ({})", non_rappro_count)}
                                    }
                                }
                            }
                        }

                        if !bulk_msg().is_empty() {
                            div { style: "margin-bottom: 8px; color: #94a3b8; font-size: 0.78rem;", "{bulk_msg()}" }
                        }

                        if non_rappro.is_empty() {
                            div { style: "padding: 14px; text-align: center; color: #64748b; font-size: 0.8rem; background: var(--bg-input); border: 1px dashed var(--border-strong); border-radius: 8px;",
                                "Aucun mouvement non rapproche. Tous les flux sont rapproches ou le compte est vide."
                            }
                        } else {
                            div { style: "display: flex; flex-direction: column; gap: 4px;",
                                for line in non_rappro.iter() {
                                    {
                                        let amount = parse_amount_str(&line.amount);
                                        let is_in = amount >= 0.0;
                                        let amount_color = if is_in { "#4ade80" } else { "#f87171" };
                                        let date_str = format_date(line.dateo);
                                        let label = if line.label.trim().is_empty() { "-".to_string() } else { line.label.clone() };
                                        let line_id = line.id.clone();
                                        let line_id_for_key = line_id.clone();
                                        let line_id_for_del = line_id.clone();
                                        let label_for_del = label.clone();
                                        let signed = format_signed_amount(amount);
                                        rsx! {
                                            div {
                                                key: "{line_id_for_key}",
                                                style: "background: var(--bg-glass); border: 1px solid var(--border); border-left: 3px solid #fbbf24; padding: 8px 12px; border-radius: 8px; display: flex; align-items: center; gap: 10px;",
                                                div { style: "flex: 1; min-width: 0;",
                                                    div { style: "font-size: 0.82rem; color: #e2e8f0;", "{label}" }
                                                    div { style: "font-size: 0.68rem; color: #94a3b8; margin-top: 2px;",
                                                        span { "{date_str}" }
                                                        span { style: "color: #334155;", " / " }
                                                        span { style: "color: #fbbf24;", "Non rapproche" }
                                                    }
                                                }
                                                div { style: "font-size: 0.85rem; color: {amount_color}; font-weight: 600; flex-shrink: 0;",
                                                    "{signed}"
                                                }
                                                button {
                                                    title: "Supprimer ce mouvement",
                                                    disabled: busy(),
                                                    onclick: move |_| {
                                                        pending_delete.set(Some(PendingDelete::Single(line_id_for_del.clone(), label_for_del.clone())));
                                                    },
                                                    style: "padding: 4px 10px; background: transparent; border: 1px solid var(--border); color: #f87171; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 600; flex-shrink: 0;",
                                                    "Suppr"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // --- Section 2 : Déjà rapprochés (lecture seule) ---
                    if rappro_count > 0 {
                        section { class: "panel",
                            div { style: "font-size: 0.7rem; color: #4ade80; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 700; margin-bottom: 4px;",
                                {format!("Deja rapproches ({})", rappro_count)}
                            }
                            div { style: "font-size: 0.72rem; color: #64748b; margin-bottom: 12px;",
                                "Ces mouvements sont proteges et ne peuvent plus etre supprimes."
                            }
                            div { style: "display: flex; flex-direction: column; gap: 4px; opacity: 0.65;",
                                for line in rappro.iter().take(50) {
                                    {
                                        let amount = parse_amount_str(&line.amount);
                                        let is_in = amount >= 0.0;
                                        let amount_color = if is_in { "#4ade80" } else { "#f87171" };
                                        let date_str = format_date(line.dateo);
                                        let label = if line.label.trim().is_empty() { "-".to_string() } else { line.label.clone() };
                                        let line_id = line.id.clone();
                                        let signed = format_signed_amount(amount);
                                        rsx! {
                                            div {
                                                key: "{line_id}",
                                                style: "background: var(--bg-input); border: 1px solid var(--border); padding: 8px 12px; border-radius: 8px; display: flex; align-items: center; gap: 10px;",
                                                div { style: "flex: 1; min-width: 0;",
                                                    div { style: "font-size: 0.82rem; color: #cbd5e1;", "{label}" }
                                                    div { style: "font-size: 0.68rem; color: #64748b; margin-top: 2px;",
                                                        span { "{date_str}" }
                                                        span { style: "color: #334155;", " / " }
                                                        span { style: "color: #4ade80;", "Rapproche" }
                                                    }
                                                }
                                                div { style: "font-size: 0.85rem; color: {amount_color}; font-weight: 600; flex-shrink: 0;",
                                                    "{signed}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if rappro_count > 50 {
                                div { style: "margin-top: 8px; color: #64748b; font-size: 0.72rem; text-align: center;",
                                    {format!("... et {} autres", rappro_count - 50)}
                                }
                            }
                        }
                    }
                }
            },
            Some(Err(e)) => rsx! {
                section { class: "panel",
                    div { style: "color: #f87171; font-size: 0.8rem;", {format!("Erreur mouvements : {e}")} }
                }
            },
            None => rsx! {
                section { class: "panel",
                    div { style: "color: #94a3b8;", "Chargement des mouvements..." }
                }
            },
        }

        // === Modale de confirmation ===
        if let Some(action) = pending_delete() {
            div {
                class: "dash-modal-overlay",
                onclick: move |_| pending_delete.set(None),
                div {
                    class: "dash-modal",
                    style: "max-width: 500px;",
                    onclick: move |e| e.stop_propagation(),

                    div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em;",
                        "Confirmation de suppression"
                    }

                    match action.clone() {
                        PendingDelete::Single(_, label) => rsx! {
                            h2 { style: "margin: 6px 0 12px 0;", "Supprimer ce mouvement ?" }
                            div { style: "background: var(--bg-input); border: 1px solid var(--border); padding: 10px 14px; border-radius: 8px; font-size: 0.85rem; color: #e2e8f0; margin-bottom: 16px;",
                                "{label}"
                            }
                        },
                        PendingDelete::All(ids) => rsx! {
                            h2 { style: "margin: 6px 0 12px 0;", {format!("Supprimer les {} mouvements non rapproches ?", ids.len())} }
                            div { style: "background: var(--bg-input); border: 1px solid var(--border); padding: 10px 14px; border-radius: 8px; font-size: 0.85rem; color: #fbbf24; margin-bottom: 16px;",
                                "Tous les flux non rapproches de ce compte seront supprimes de Dolibarr."
                            }
                        },
                    }

                    div { style: "font-size: 0.75rem; color: #94a3b8; margin-bottom: 20px;",
                        "Cette action est irreversible. Les mouvements deja rapproches ne sont pas concernes."
                    }

                    div { style: "display: flex; gap: 8px; justify-content: flex-end;",
                        button {
                            class: "secondary",
                            onclick: move |_| pending_delete.set(None),
                            "Annuler"
                        }
                        button {
                            disabled: busy(),
                            onclick: move |_| {
                                let action_now = pending_delete();
                                let acc_id = selected_account_id();
                                pending_delete.set(None);
                                busy.set(true);
                                bulk_msg.set(String::new());
                                spawn(async move {
                                    let ids: Vec<String> = match action_now {
                                        Some(PendingDelete::Single(id, _)) => vec![id],
                                        Some(PendingDelete::All(list)) => list,
                                        None => Vec::new(),
                                    };

                                    if ids.is_empty() {
                                        busy.set(false);
                                        return;
                                    }

                                    match crate::dolibarr::server_fns::dolibarr_delete_bank_lines(acc_id, ids).await {
                                        Ok(n) => {
                                            bulk_msg.set(format!("{} mouvement(s) supprime(s).", n));
                                        }
                                        Err(e) => {
                                            bulk_msg.set(format!("Erreur : {}", e));
                                        }
                                    }
                                    busy.set(false);
                                    bump.with_mut(|v| *v += 1);
                                });
                            },
                            style: "padding: 8px 18px; background: #dc2626; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.82rem; font-weight: 600;",
                            if busy() { "Suppression..." } else { "Confirmer la suppression" }
                        }
                    }
                }
            }
        }
    }
}