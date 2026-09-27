use chrono::{DateTime, NaiveDate, Utc};
use dioxus::prelude::*;

use crate::dolibarr::models::{DolibarrBankLine, DolibarrInvoice, DolibarrThirdParty};

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
    if t.is_empty() {
        return None;
    }
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
    if t.is_empty() {
        return None;
    }
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

fn score_color(score: i32) -> &'static str {
    if score >= 80 {
        "#4ade80"
    } else if score >= 60 {
        "#fbbf24"
    } else if score >= 40 {
        "#fb923c"
    } else {
        "#64748b"
    }
}

fn format_signed_amount(v: f64) -> String {
    if v >= 0.0 {
        format!("+{:.2} EUR", v)
    } else {
        format!("{:.2} EUR", v)
    }
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

fn format_amount(v: f64) -> String {
    format!("{:.2} EUR", v)
}

// ============================================================
//  Page
// ============================================================

#[component]
pub fn BankingImportPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);

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

    let accounts = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { crate::dolibarr::server_fns::dolibarr_list_bank_accounts().await }
    });

    let mut csv_input = use_signal(String::new);
    let mut proposals = use_signal(Vec::<MatchProposal>::new);
    let mut selected_account_id = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut msg = use_signal(String::new);
    let mut pushed = use_signal(|| 0usize);
    let mut file_name = use_signal(String::new);

    // Auto-selectionne le premier compte disponible
    if selected_account_id().is_empty() {
        let accs = accounts.read();
        let default_id = match accs.as_ref() {
            Some(Ok(list)) => list.first().map(|a| a.id.clone()).unwrap_or_else(|| "1".to_string()),
            _ => "1".to_string(),
        };
        selected_account_id.set(default_id);
    }

    // Ressource : derniers mouvements du compte selectionne
    let lines_resource = use_resource(move || {
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

    rsx! {
        section { class: "page-intro",
            div {
                div { class: "eyebrow", "ARGENT / IMPORT CSV" }
                h2 { "Import bancaire intelligent" }
                p { "Charge ton releve CSV (Credit Agricole, etc.). L'app propose les factures a rapprocher, tu valides, puis tout est pousse dans Dolibarr." }
            }
        }

        section { class: "panel",
            div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;",
                "Etape 1 : charge ton releve"
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
                        "Fichier charge : {file_name()}"
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
                "Ou colle directement le contenu CSV ci-dessous :"
            }
            textarea {
                value: "{csv_input}",
                oninput: move |e| csv_input.set(e.value()),
                rows: "6",
                placeholder: "Date operation;Libelle;Debit euros;Credit euros\n24/01/2026;VIR INST vers SDC LA BELLE;-2000,00;\n02/01/2026;LE GROUPE LA POSTE;;27457,94",
                style: "width: 100%; padding: 10px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 8px; color: #e2e8f0; font-size: 0.8rem; font-family: monospace; resize: vertical;",
            }

            div { style: "display: flex; gap: 8px; align-items: center; margin-top: 12px; flex-wrap: wrap;",
                div { style: "font-size: 0.72rem; color: #94a3b8;",
                    "Compte bancaire :"
                }
                select {
                    value: "{selected_account_id}",
                    onchange: move |e| selected_account_id.set(e.value()),
                    style: "padding: 6px 12px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px; color: #e2e8f0; font-size: 0.78rem;",
                    {
                        let accs_opt = accounts.read();
                        let list: Vec<crate::dolibarr::models::DolibarrBankAccount> = match accs_opt.as_ref() {
                            Some(Ok(v)) => v.clone(),
                            _ => Vec::new(),
                        };
                        if list.is_empty() {
                            rsx! {
                                option { value: "1", "Compte principal (#1)" }
                            }
                        } else {
                            rsx! {
                                for a in list.iter() {
                                    option {
                                        value: "{a.id}",
                                        if a.label.is_empty() { "{a.r#ref}" } else { "{a.label}" }
                                    }
                                }
                            }
                        }
                    }
                }
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
                            msg.set("Aucune ligne valide detectee. Verifie le format.".into());
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

        if !proposals().is_empty() {
            section { class: "panel",
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;",
                    div {
                        div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em;",
                            "Etape 2 : verifie le matching"
                        }
                        div { style: "font-size: 0.72rem; color: #64748b; margin-top: 2px;",
                            {format!("{} ligne(s) detectee(s)", proposals().len())}
                        }
                    }
                }

                div { style: "display: flex; flex-direction: column; gap: 6px;",
                    for (idx, p) in proposals().iter().enumerate() {
                        {
                            let amount = p.csv.amount;
                            let is_in = amount > 0.0;
                            let amount_color = if is_in { "#4ade80" } else if amount < 0.0 { "#f87171" } else { "#94a3b8" };
                            let score_col = score_color(p.score);
                            let enabled = p.enabled;
                            let date_str = format_date(p.csv.date_ts);
                            let label = p.csv.label.clone();
                            let invoice_ref = p.invoice_ref.clone().unwrap_or_else(|| "-".to_string());
                            let tp = p.third_party.clone().unwrap_or_else(|| "-".to_string());
                            let reason = p.reason.clone();
                            let score = p.score;
                            let id_key = format!("{}", idx);
                            let signed = format_signed_amount(amount);
                            rsx! {
                                div {
                                    key: "{id_key}",
                                    style: "background: var(--bg-glass); border: 1px solid var(--border); border-left: 3px solid {score_col}; padding: 10px 14px; border-radius: 10px; display: flex; align-items: center; gap: 12px;",
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
                                        div { style: "font-size: 0.82rem; color: #e2e8f0;",
                                            "{label}"
                                        }
                                        div { style: "font-size: 0.7rem; color: #94a3b8; margin-top: 2px;",
                                            span { "{date_str}" }
                                            span { style: "color: #334155;", " / " }
                                            span { "Match: {invoice_ref}" }
                                            span { style: "color: #334155;", " / " }
                                            span { "Tiers: {tp}" }
                                        }
                                        div { style: "font-size: 0.68rem; color: {score_col}; margin-top: 2px;",
                                            "Score {score} / {reason}"
                                        }
                                    }
                                    div { style: "font-size: 0.88rem; color: {amount_color}; font-weight: 600; flex-shrink: 0;",
                                        "{signed}"
                                    }
                                }
                            }
                        }
                    }
                }

                div { style: "display: flex; gap: 8px; margin-top: 16px; align-items: center;",
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
                                    msg.set(format!("{} ligne(s) poussee(s) dans Dolibarr. Verification ci-dessous.", ok_count));
                                }
                                busy.set(false);
                                // Recharge les mouvements du compte pour verification
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
                            "Dernier envoi : {pushed()} ligne(s) OK"
                        }
                    }
                }
                if !msg().is_empty() {
                    div { style: "margin-top: 8px; color: #94a3b8; font-size: 0.8rem;", "{msg()}" }
                }
            }
        }

        // ============================================================
        //  Verification : derniers mouvements du compte dans Dolibarr
        // ============================================================
        section { class: "panel",
            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;",
                div {
                    div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em;",
                        "Verification dans Dolibarr"
                    }
                    div { style: "font-size: 0.72rem; color: #64748b; margin-top: 2px;",
                        "Derniers mouvements du compte selectionne"
                    }
                }
                div { style: "display: flex; gap: 8px;",
                    button {
                        style: "padding: 5px 12px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.72rem; font-weight: 600;",
                        onclick: move |_| bump.with_mut(|v| *v += 1),
                        "Rafraichir"
                    }
                    button {
                        style: "padding: 5px 12px; background: transparent; color: #a78bfa; border: 1px solid #7c3aed; border-radius: 6px; cursor: pointer; font-size: 0.72rem; font-weight: 600;",
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

            match &*lines_resource.read() {
                Some(Ok(list)) if list.is_empty() => rsx! {
                    div { class: "empty-state",
                        h3 { "Aucun mouvement" }
                        p { "Ce compte n'a pas encore de mouvements dans Dolibarr." }
                    }
                },
                Some(Ok(list)) => {
                    let total = list.len();
                    rsx! {
                        div { style: "font-size: 0.72rem; color: #64748b; margin-bottom: 8px;",
                            {format!("{} mouvement(s) au total. Affichage des 15 derniers.", total)}
                        }
                        div { style: "display: flex; flex-direction: column; gap: 4px;",
                            for line in list.iter().take(15) {
                                {
                                    let amount = parse_amount_str(&line.amount);
                                    let is_in = amount >= 0.0;
                                    let amount_color = if is_in { "#4ade80" } else { "#f87171" };
                                    let date_str = format_date(line.dateo);
                                    let label = if line.label.trim().is_empty() { "-".to_string() } else { line.label.clone() };
                                    let is_rappro = line.rappro == 1;
                                    let line_id = line.id.clone();
                                    let signed = format_signed_amount(amount);
                                    rsx! {
                                        div {
                                            key: "{line_id}",
                                            style: "background: var(--bg-input); border: 1px solid var(--border); padding: 8px 12px; border-radius: 8px; display: flex; align-items: center; gap: 10px;",
                                            div { style: "flex: 1; min-width: 0;",
                                                div { style: "font-size: 0.82rem; color: #e2e8f0;",
                                                    "{label}"
                                                }
                                                div { style: "font-size: 0.68rem; color: #94a3b8; margin-top: 2px;",
                                                    span { "{date_str}" }
                                                    if is_rappro {
                                                        span { style: "color: #334155;", " / " }
                                                        span { style: "color: #4ade80;", "Rapproche" }
                                                    }
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
                    }
                },
                Some(Err(e)) => rsx! {
                    div { style: "color: #f87171; font-size: 0.8rem;", {format!("Erreur : {e}")} }
                },
                None => rsx! {
                    div { style: "color: #94a3b8;", "Chargement des mouvements..." }
                },
            }
        }
    }
}