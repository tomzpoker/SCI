use dioxus::prelude::*;
use super::forecast::SimpleDate;

const MONTHS_FR_FULL: &[&str] = &[
    "janvier", "février", "mars", "avril", "mai", "juin",
    "juillet", "août", "septembre", "octobre", "novembre", "décembre",
];

#[derive(Clone, Copy, PartialEq)]
enum VatRange { Month, Quarter, Year }

impl VatRange {
    fn label(self) -> &'static str {
        match self {
            VatRange::Month => "Mois",
            VatRange::Quarter => "Trimestre",
            VatRange::Year => "Année",
        }
    }
    fn n_months(self) -> i32 {
        match self {
            VatRange::Month => 1,
            VatRange::Quarter => 3,
            VatRange::Year => 12,
        }
    }
}

#[derive(Clone)]
struct Receipt {
    dolibarr_id: String,
    date_label: String,
    date_iso: String,
    label: String,
    ttc: f64,
    ht: f64,
    tva: f64,
    rate_bp: Option<i32>,
    is_credit: bool,
}

fn parse_tx_date(s: &str) -> Option<SimpleDate> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() < 3 { return None; }
    let y: i32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;
    Some(SimpleDate { year: y, month: m, day: d })
}

fn split_ttc(ttc: f64, vat_rate_bp: Option<i32>) -> (f64, f64, f64) {
    let rate = match vat_rate_bp {
        Some(bp) if bp > 0 => (bp as f64) / 10000.0,
        _ => 0.0,
    };
    if rate <= 0.0 {
        (ttc, 0.0, ttc)
    } else {
        let ht = ttc / (1.0 + rate);
        let tva = ttc - ht;
        (ht, tva, ttc)
    }
}

fn compute_receipts(
    lines: &[crate::dolibarr::server_fns::BankLineEnriched],
    start: SimpleDate,
    end: SimpleDate,
) -> Vec<Receipt> {
    const DEFAULT_CREDIT_RATE_BP: i32 = 2000; // 20% par défaut pour les loyers

    let mut receipts: Vec<Receipt> = lines.iter()
        .filter_map(|tx| {
            if tx.amount == 0.0 { return None; }
            let date = parse_tx_date(&tx.date)?;
            if date.month_abs() < start.month_abs() || date.month_abs() > end.month_abs() {
                return None;
            }

            let is_credit = tx.amount > 0.0;

            // Taux effectif : si explicite en BDD, on l'utilise.
            // Sinon, pour un crédit (loyer) on prend 20% par défaut.
            // Sinon, pour un débit on reste à 0 (aucun) tant que l'utilisateur n'a rien dit.
            let effective_rate_bp = match tx.vat_rate_bp {
                Some(bp) => Some(bp),
                None if is_credit => Some(DEFAULT_CREDIT_RATE_BP),
                None => None,
            };

            let (ht, tva, ttc) = split_ttc(tx.amount.abs(), effective_rate_bp);
            Some(Receipt {
                dolibarr_id: tx.dolibarr_id.clone(),
                date_label: format!("{:02}/{:02}", date.day, date.month),
                date_iso: format!("{:04}-{:02}-{:02}", date.year, date.month, date.day),
                label: tx.label.clone(),
                ttc: ttc * tx.amount.signum(),
                ht: ht * tx.amount.signum(),
                tva: tva * tx.amount.signum(),
                rate_bp: effective_rate_bp,
                is_credit,
            })
        })
        .collect();
    receipts.sort_by(|a, b| a.date_iso.cmp(&b.date_iso));
    receipts
}

fn period_label(start: SimpleDate, n_months: i32) -> String {
    if n_months == 1 {
        format!("{} {}", MONTHS_FR_FULL[(start.month - 1) as usize], start.year)
    } else if n_months == 12 {
        format!("Année {}", start.year)
    } else {
        let end = start.shift_months(n_months - 1);
        if start.month == end.month && start.year == end.year {
            format!("{} {}", MONTHS_FR_FULL[(start.month - 1) as usize], start.year)
        } else {
            format!(
                "{} – {} {}",
                MONTHS_FR_FULL[(start.month - 1) as usize],
                MONTHS_FR_FULL[(end.month - 1) as usize],
                end.year
            )
        }
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(';') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn rate_label(bp: Option<i32>) -> String {
    match bp {
        Some(b) if b > 0 => format!("{:.2}%", (b as f64) / 100.0),
        _ => "—".to_string(),
    }
}

fn build_csv(
    receipts: &[Receipt],
    total_ht_credit: f64,
    total_tva_credit: f64,
    total_ttc_credit: f64,
    total_ht_debit: f64,
    total_tva_debit: f64,
    total_ttc_debit: f64,
    net_vat: f64,
    label: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("Periode;{}\r\n", csv_escape(label)));
    out.push_str("Date;Libelle;Sens;Taux;Base HT (EUR);TVA (EUR);Total TTC (EUR)\r\n");
    for r in receipts {
        let sens = if r.is_credit { "CREDIT (collectee)" } else { "DEBIT (deductible)" };
        out.push_str(&format!(
            "{};{};{};{};{:.2};{:.2};{:.2}\r\n",
            r.date_iso,
            csv_escape(&r.label),
            sens,
            rate_label(r.rate_bp),
            r.ht.abs(),
            r.tva.abs(),
            r.ttc.abs(),
        ));
    }
    out.push_str(&format!(
        "TOTAL CREDITS;;;;{:.2};{:.2};{:.2}\r\n",
        total_ht_credit, total_tva_credit, total_ttc_credit
    ));
    out.push_str(&format!(
        "TOTAL DEBITS;;;;{:.2};{:.2};{:.2}\r\n",
        total_ht_debit, total_tva_debit, total_ttc_debit
    ));
    out.push_str(&format!("TVA NETTE A PAYER;;;;;{:.2};\r\n", net_vat));
    out
}

fn download_csv(filename: &str, content: &str) {
    let escaped = content
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\r', "\\r")
        .replace('\n', "\\n");
    let script = format!(
        "const blob = new Blob(['{}'], {{type: 'text/csv;charset=utf-8'}});\
         const url = URL.createObjectURL(blob);\
         const a = document.createElement('a');\
         a.href = url;\
         a.download = '{}';\
         a.click();\
         URL.revokeObjectURL(url);",
        escaped, filename
    );
    let _ = document::eval(&script);
}

#[component]
pub fn VatWidget() -> Element {
    let mut real_today = use_signal(|| SimpleDate::fallback());
    use_effect(move || { real_today.set(SimpleDate::today()); });

    let mut cursor = use_signal(|| SimpleDate::fallback());
    let mut cursor_init = use_signal(|| false);
    use_effect(move || {
        if !cursor_init() {
            cursor.set(real_today().start_of_month());
            cursor_init.set(true);
        }
    });

    let mut range = use_signal(|| VatRange::Month);
    let mut show_details = use_signal(|| false);
    let mut show_rate_editor = use_signal(|| None::<String>);   // dolibarr_id de la ligne en édition
    let mut rate_input = use_signal(|| "20".to_string());
    let mut bump = use_signal(|| 0u64);

    // === Resource : charge les lignes Dolibarr + taux locaux ===
    let lines_resource = use_resource(move || {
        let _ = bump();
        async move {
            crate::dolibarr::server_fns::dolibarr_get_all_bank_lines_enriched()
                .await
                .unwrap_or_default()
        }
    });

    let all_lines: Vec<crate::dolibarr::server_fns::BankLineEnriched> =
        (*lines_resource.read()).clone().unwrap_or_default();

    let n = range().n_months();
    let start = cursor().start_of_month();
    let end = start.shift_months(n - 1);

    let receipts = compute_receipts(&all_lines, start, end);

    let credits: Vec<&Receipt> = receipts.iter().filter(|r| r.is_credit).collect();
    let debits: Vec<&Receipt> = receipts.iter().filter(|r| !r.is_credit).collect();

    let total_ttc_credit: f64 = credits.iter().map(|r| r.ttc).sum();
    let total_ht_credit: f64 = credits.iter().map(|r| r.ht).sum();
    let total_tva_credit: f64 = credits.iter().map(|r| r.tva).sum();

    let total_ttc_debit: f64 = debits.iter().map(|r| r.ttc.abs()).sum();
    let total_ht_debit: f64 = debits.iter().map(|r| r.ht.abs()).sum();
    let total_tva_debit: f64 = debits.iter().map(|r| r.tva.abs()).sum();

    let net_vat = total_tva_credit - total_tva_debit;
    let (net_label, net_color) = if net_vat > 0.01 {
        ("TVA à payer", "#f87171")
    } else if net_vat < -0.01 {
        ("Crédit de TVA", "#4ade80")
    } else {
        ("Neutre", "#94a3b8")
    };

    let label = period_label(start, n);

    // === Bloc "À déclarer" (mois precedent) ===
    let current_month = real_today().start_of_month();
    let prev_month = current_month.shift_months(-1).start_of_month();
    let prev_receipts = compute_receipts(&all_lines, prev_month, prev_month);
    let prev_credits: Vec<&Receipt> = prev_receipts.iter().filter(|r| r.is_credit).collect();
    let prev_debits: Vec<&Receipt> = prev_receipts.iter().filter(|r| !r.is_credit).collect();
    let prev_tva_collected: f64 = prev_credits.iter().map(|r| r.tva).sum();
    let prev_tva_deductible: f64 = prev_debits.iter().map(|r| r.tva.abs()).sum();
    let prev_net: f64 = prev_tva_collected - prev_tva_deductible;
    let prev_ht_collected: f64 = prev_credits.iter().map(|r| r.ht).sum();
    let prev_label = period_label(prev_month, 1);

    // === Bloc "En cours" (mois courant) ===
    let cur_receipts = compute_receipts(&all_lines, current_month, current_month);
    let cur_credits: Vec<&Receipt> = cur_receipts.iter().filter(|r| r.is_credit).collect();
    let cur_debits: Vec<&Receipt> = cur_receipts.iter().filter(|r| !r.is_credit).collect();
    let cur_tva_collected: f64 = cur_credits.iter().map(|r| r.tva).sum();
    let cur_tva_deductible: f64 = cur_debits.iter().map(|r| r.tva.abs()).sum();
    let cur_net: f64 = cur_tva_collected - cur_tva_deductible;
    let cur_label = period_label(current_month, 1);

    let deadline_day = 15u32;
    let deadline_month = current_month.month;
    let deadline_year = current_month.year;
    let today_day = real_today().day as i32;
    let days_left = deadline_day as i32 - today_day;
    let is_urgent = days_left <= 5 && days_left >= 0;
    let is_overdue = days_left < 0;

    let is_current = start.month_abs() == current_month.month_abs();
    let year_min = real_today().year - 5;
    let year_max = real_today().year + 5;

    let receipts_for_export = receipts.clone();
    let label_for_export = label.clone();

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 12px; min-width: 0; max-width: 100%;",

            // === BANNIERE ===
            div {
                style: if is_overdue {
                    "display: flex; gap: 16px; flex-wrap: wrap; padding: 14px 16px; background: rgba(248,113,113,0.08); border: 1px solid rgba(248,113,113,0.3); border-radius: 10px;"
                } else if is_urgent {
                    "display: flex; gap: 16px; flex-wrap: wrap; padding: 14px 16px; background: rgba(251,191,36,0.08); border: 1px solid rgba(251,191,36,0.3); border-radius: 10px;"
                } else {
                    "display: flex; gap: 16px; flex-wrap: wrap; padding: 14px 16px; background: rgba(56,189,248,0.06); border: 1px solid rgba(56,189,248,0.25); border-radius: 10px;"
                },
                div { style: "flex: 1; min-width: 220px;",
                    div {
                        style: if is_overdue {
                            "font-size: 0.7rem; color: #f87171; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600;"
                        } else if is_urgent {
                            "font-size: 0.7rem; color: #fbbf24; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600;"
                        } else {
                            "font-size: 0.7rem; color: #38bdf8; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600;"
                        },
                        "📌 À déclarer • {prev_label}"
                    }
                    div {
                        style: format!(
                            "font-size: 1.5rem; font-weight: 700; font-variant-numeric: tabular-nums; margin-top: 4px; color: {};",
                            if prev_net >= 0.0 { "#f8fafc" } else { "#4ade80" }
                        ),
                        {format!("{:.2} EUR", prev_net.abs())}
                    }
                    div { style: "font-size: 0.72rem; color: #94a3b8; margin-top: 4px;",
                        if is_overdue {
                            {format!("⚠ Échéance dépassée (était le {:02}/{:02}/{:04})", deadline_day, deadline_month, deadline_year)}
                        } else if is_urgent {
                            {format!("⏰ Échéance {:02}/{:02}/{:04} • J-{}", deadline_day, deadline_month, deadline_year, days_left)}
                        } else {
                            {format!("Échéance {:02}/{:02}/{:04}", deadline_day, deadline_month, deadline_year)}
                        }
                    }
                    div { style: "font-size: 0.7rem; color: #64748b; margin-top: 4px;",
                        {format!("Collectée {} € − Déductible {} € • HT encaissé {} €",
                            format!("{:.2}", prev_tva_collected),
                            format!("{:.2}", prev_tva_deductible),
                            format!("{:.2}", prev_ht_collected))}
                    }
                }
                div { style: "flex: 1; min-width: 220px; padding-left: 16px; border-left: 1px solid rgba(148,163,184,0.15);",
                    div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600;",
                        "🕐 En cours • {cur_label}"
                    }
                    div {
                        style: format!(
                            "font-size: 1.2rem; font-weight: 600; font-variant-numeric: tabular-nums; margin-top: 4px; color: {};",
                            if cur_net >= 0.0 { "#cbd5e1" } else { "#4ade80" }
                        ),
                        {format!("{:.2} EUR", cur_net.abs())}
                    }
                    div { style: "font-size: 0.72rem; color: #64748b; margin-top: 4px;",
                        {format!("Collectée {} € − Déductible {} € • pour anticiper",
                            format!("{:.2}", cur_tva_collected),
                            format!("{:.2}", cur_tva_deductible))}
                    }
                }
            }

            // === SELECTEUR ===
            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 8px; flex-wrap: wrap;",
                div { style: "display: flex; align-items: center; gap: 6px;",
                    div { style: "display: inline-flex; background: #0f172a; border-radius: 6px; padding: 2px; border: 1px solid #334155;",
                        for r in [VatRange::Month, VatRange::Quarter, VatRange::Year] {
                            button {
                                draggable: "false",
                                style: if range() == r {
                                    "background: #38bdf8; color: #0f172a; border: none; padding: 4px 12px; border-radius: 4px; font-size: 0.75rem; cursor: pointer; font-weight: 600;"
                                } else {
                                    "background: transparent; color: #94a3b8; border: none; padding: 4px 12px; border-radius: 4px; font-size: 0.75rem; cursor: pointer;"
                                },
                                onclick: move |_| range.set(r),
                                "{r.label()}"
                            }
                        }
                    }
                    button {
                        draggable: "false",
                        style: "background: #0f172a; border: 1px solid #334155; color: #94a3b8; padding: 4px 10px; border-radius: 6px; cursor: pointer; font-size: 0.85rem; line-height: 1;",
                        onclick: move |_| {
                            let s = range().n_months();
                            cursor.set(cursor().shift_months(-s).start_of_month());
                        },
                        "◀"
                    }
                    select {
                        style: "background: #0f172a; border: 1px solid #334155; color: #f8fafc; padding: 5px 10px; border-radius: 6px; font-size: 0.82rem; font-family: inherit; cursor: pointer; min-width: 110px;",
                        value: "{cursor().month}",
                        onchange: move |e| {
                            if let Ok(m) = e.value().parse::<u32>() {
                                let mut c = cursor();
                                c.month = m;
                                c.day = 1;
                                cursor.set(c);
                            }
                        },
                        for m in 1..=12u32 {
                            option { value: "{m}", selected: cursor().month == m, "{MONTHS_FR_FULL[(m - 1) as usize]}" }
                        }
                    }
                    select {
                        style: "background: #0f172a; border: 1px solid #334155; color: #f8fafc; padding: 5px 10px; border-radius: 6px; font-size: 0.82rem; font-family: inherit; cursor: pointer; min-width: 72px;",
                        value: "{cursor().year}",
                        onchange: move |e| {
                            if let Ok(y) = e.value().parse::<i32>() {
                                let mut c = cursor();
                                c.year = y;
                                c.day = 1;
                                cursor.set(c);
                            }
                        },
                        for y in year_min..=year_max {
                            option { value: "{y}", selected: cursor().year == y, "{y}" }
                        }
                    }
                    button {
                        draggable: "false",
                        style: "background: #0f172a; border: 1px solid #334155; color: #94a3b8; padding: 4px 10px; border-radius: 6px; cursor: pointer; font-size: 0.85rem; line-height: 1;",
                        onclick: move |_| {
                            let s = range().n_months();
                            cursor.set(cursor().shift_months(s).start_of_month());
                        },
                        "▶"
                    }
                    if !is_current {
                        button {
                            draggable: "false",
                            style: "background: transparent; border: 1px solid #4c1d95; color: #a78bfa; padding: 4px 10px; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 500; margin-left: 4px;",
                            onclick: move |_| cursor.set(current_month),
                            "Aujourd'hui"
                        }
                    }
                }
                div { style: "display: flex; gap: 8px; align-items: center;",
                    button {
                        draggable: "false",
                        style: "display: inline-flex; align-items: center; gap: 5px; background: #0f172a; border: 1px solid #334155; color: #94a3b8; padding: 4px 10px; border-radius: 6px; font-size: 0.7rem; cursor: pointer;",
                        onclick: move |_| {
                            let b = bump();
                            bump.set(b + 1);
                        },
                        "⟳ Rafraîchir"
                    }
                    if !receipts.is_empty() {
                        button {
                            draggable: "false",
                            style: "display: inline-flex; align-items: center; gap: 5px; background: #0f172a; border: 1px solid #334155; color: #94a3b8; padding: 4px 12px; border-radius: 6px; font-size: 0.72rem; cursor: pointer;",
                            onclick: move |_| show_details.set(true),
                            "Détails →"
                        }
                        button {
                            draggable: "false",
                            style: "display: inline-flex; align-items: center; gap: 5px; background: transparent; border: 1px solid #22c55e; color: #22c55e; padding: 4px 12px; border-radius: 6px; font-size: 0.72rem; font-weight: 600; cursor: pointer;",
                            onclick: {
                                let receipts = receipts_for_export.clone();
                                let label = label_for_export.clone();
                                move |_| {
                                    let csv = build_csv(
                                        &receipts,
                                        total_ht_credit, total_tva_credit, total_ttc_credit,
                                        total_ht_debit, total_tva_debit, total_ttc_debit,
                                        net_vat, &label,
                                    );
                                    let safe: String = label.chars()
                                        .map(|c| if c.is_alphanumeric() { c } else { '_' })
                                        .collect();
                                    download_csv(&format!("tva_{}.csv", safe), &csv);
                                }
                            },
                            "⬇ CSV comptable"
                        }
                    }
                }
            }

            // === MONTANT PRINCIPAL ===
            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; padding: 14px 16px; background: #0f172a; border-radius: 8px; border: 1px solid #1e293b;",
                div {
                    div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.06em;",
                        "{net_label} • {label}"
                    }
                    div {
                        style: format!(
                            "font-size: 1.6rem; font-weight: 700; font-variant-numeric: tabular-nums; margin-top: 4px; color: {};",
                            net_color
                        ),
                        {format!("{:.2} EUR", net_vat.abs())}
                    }
                    div { style: "font-size: 0.7rem; color: #94a3b8; margin-top: 4px;",
                        "{credits.len()} encaissement(s) • {debits.len()} décaissement(s)"
                    }
                }
                div { style: "display: flex; gap: 20px; flex-wrap: wrap;",
                    div { style: "text-align: right;",
                        div { style: "font-size: 0.65rem; color: #64748b;", "TVA collectée" }
                        div { style: "font-size: 1rem; font-weight: 600; color: #38bdf8; font-variant-numeric: tabular-nums;",
                            {format!("{:.2} EUR", total_tva_credit)}
                        }
                        div { style: "font-size: 0.65rem; color: #64748b; margin-top: 2px;",
                            {format!("HT {} €", format!("{:.2}", total_ht_credit))}
                        }
                    }
                    div { style: "text-align: right;",
                        div { style: "font-size: 0.65rem; color: #64748b;", "TVA déductible" }
                        div { style: "font-size: 1rem; font-weight: 600; color: #fbbf24; font-variant-numeric: tabular-nums;",
                            {format!("{:.2} EUR", total_tva_debit)}
                        }
                        div { style: "font-size: 0.65rem; color: #64748b; margin-top: 2px;",
                            {format!("HT {} €", format!("{:.2}", total_ht_debit))}
                        }
                    }
                    div { style: "text-align: right;",
                        div { style: "font-size: 0.65rem; color: #64748b;", "TTC net" }
                        div { style: "font-size: 1rem; font-weight: 600; color: #cbd5e1; font-variant-numeric: tabular-nums;",
                            {format!("{:.2} EUR", total_ttc_credit - total_ttc_debit)}
                        }
                    }
                }
            }

            if receipts.is_empty() {
                div { style: "padding: 12px; text-align: center; color: #64748b; font-size: 0.8rem; background: #0f172a; border-radius: 6px; border: 1px dashed #334155;",
                    "Aucun mouvement sur la période. Importe un CSV depuis la page Finances."
                }
            }

            // === MODALE DETAILS ===
            if show_details() {
                div {
                    class: "dash-modal-overlay",
                    onclick: move |_| show_details.set(false),
                    div {
                        class: "dash-modal",
                        onclick: move |e| e.stop_propagation(),
                        style: "max-width: 900px; max-height: 85vh; overflow-y: auto;",
                        div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                            div {
                                div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em;", "Déclaration TVA sur encaissements" }
                                h2 { style: "margin: 4px 0 0 0; color: #38bdf8;", "Brouillon • {label}" }
                            }
                            button {
                                style: "background: transparent; border: none; color: #94a3b8; font-size: 1.5rem; cursor: pointer; line-height: 1;",
                                onclick: move |_| show_details.set(false),
                                "×"
                            }
                        }

                        div { style: "display: flex; gap: 12px; padding: 12px; background: #0f172a; border-radius: 6px; margin-bottom: 16px; flex-wrap: wrap;",
                            div { style: "flex: 1; min-width: 120px;",
                                div { style: "font-size: 0.65rem; color: #64748b;", "TVA collectée" }
                                div { style: "font-size: 1rem; font-weight: 600; color: #38bdf8; font-variant-numeric: tabular-nums;",
                                    {format!("{:.2} EUR", total_tva_credit)}
                                }
                            }
                            div { style: "flex: 1; min-width: 120px;",
                                div { style: "font-size: 0.65rem; color: #64748b;", "TVA déductible" }
                                div { style: "font-size: 1rem; font-weight: 600; color: #fbbf24; font-variant-numeric: tabular-nums;",
                                    {format!("{:.2} EUR", total_tva_debit)}
                                }
                            }
                            div { style: "flex: 1; min-width: 120px;",
                                div { style: "font-size: 0.65rem; color: #64748b;", "{net_label}" }
                                div { style: "font-size: 1rem; font-weight: 700; color: {net_color}; font-variant-numeric: tabular-nums;",
                                    {format!("{:.2} EUR", net_vat.abs())}
                                }
                            }
                        }

                        div { style: "margin-bottom: 16px;",
                            h4 { style: "color: #94a3b8; margin: 0 0 8px 0; font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.05em;",
                                "Mouvements de la période • clique sur le taux pour le modifier"
                            }
                            table {
                                style: "width: 100%; border-collapse: collapse; font-size: 0.78rem;",
                                thead {
                                    tr {
                                        th { style: "text-align: left; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "Date" }
                                        th { style: "text-align: left; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "Libellé" }
                                        th { style: "text-align: center; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "Sens" }
                                        th { style: "text-align: right; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "Taux TVA" }
                                        th { style: "text-align: right; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "HT" }
                                        th { style: "text-align: right; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "TVA" }
                                        th { style: "text-align: right; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "TTC" }
                                    }
                                }
                                tbody {
                                    for r in receipts.iter() {
                                        {
                                            let dolibarr_id = r.dolibarr_id.clone();
                                            let is_editing = show_rate_editor() == Some(dolibarr_id.clone());
                                            let current_rate_bp = r.rate_bp;
                                            rsx! {
                                                tr {
                                                    td { style: "padding: 6px 8px; color: #cbd5e1; white-space: nowrap;", "{r.date_label}" }
                                                    td { style: "padding: 6px 8px; color: #f8fafc;", "{r.label}" }
                                                    td {
                                                        style: if r.is_credit {
                                                            "padding: 6px 8px; text-align: center; color: #38bdf8; font-size: 0.7rem; font-weight: 600;"
                                                        } else {
                                                            "padding: 6px 8px; text-align: center; color: #fbbf24; font-size: 0.7rem; font-weight: 600;"
                                                        },
                                                        if r.is_credit { "CRÉDIT" } else { "DÉBIT" }
                                                    }
                                                    td { style: "padding: 6px 8px; text-align: right;",
                                                        if is_editing {
                                                            div { style: "display: inline-flex; align-items: center; gap: 4px;",
                                                                select {
                                                                    style: "background: #0f172a; border: 1px solid #334155; color: #f8fafc; padding: 3px 6px; border-radius: 4px; font-size: 0.72rem; cursor: pointer;",
                                                                    value: match current_rate_bp {
                                                                        Some(0) => "0".to_string(),
                                                                        Some(550) => "550".to_string(),
                                                                        Some(1000) => "1000".to_string(),
                                                                        Some(2000) => "2000".to_string(),
                                                                        Some(v) => v.to_string(),
                                                                        None => "".to_string(),
                                                                    },
                                                                    onchange: {
                                                                        let id = dolibarr_id.clone();
                                                                        move |e| {
                                                                            let val = e.value();
                                                                            let rate = if val.is_empty() { None } else { val.parse::<i32>().ok() };
                                                                            let id2 = id.clone();
                                                                            spawn(async move {
                                                                                let _ = crate::dolibarr::server_fns::dolibarr_set_bank_line_vat_rate(id2, rate).await;
                                                                                show_rate_editor.set(None);
                                                                                let b = bump();
                                                                                bump.set(b + 1);
                                                                            });
                                                                        }
                                                                    },
                                                                    option { value: "", "Aucun" }
                                                                    option { value: "0", "0% (exonéré)" }
                                                                    option { value: "550", "5.5%" }
                                                                    option { value: "1000", "10%" }
                                                                    option { value: "2000", "20%" }
                                                                }
                                                                button {
                                                                    draggable: "false",
                                                                    style: "background: transparent; border: none; color: #94a3b8; cursor: pointer; font-size: 0.8rem; padding: 0 4px;",
                                                                    onclick: move |_| show_rate_editor.set(None),
                                                                    "×"
                                                                }
                                                            }
                                                        } else {
                                                            button {
                                                                draggable: "false",
                                                                title: "Cliquer pour modifier le taux",
                                                                style: "background: transparent; border: 1px dashed #475569; color: #94a3b8; padding: 3px 8px; border-radius: 4px; font-size: 0.72rem; cursor: pointer; font-variant-numeric: tabular-nums;",
                                                                onclick: {
                                                                    let id = dolibarr_id.clone();
                                                                    move |_| {
                                                                        show_rate_editor.set(Some(id.clone()));
                                                                    }
                                                                },
                                                                {rate_label(r.rate_bp)}
                                                            }
                                                        }
                                                    }
                                                    td { style: "padding: 6px 8px; text-align: right; color: #cbd5e1; font-variant-numeric: tabular-nums;",
                                                        {format!("{:.2}", r.ht.abs())}
                                                    }
                                                    td {
                                                        style: if r.is_credit {
                                                            "padding: 6px 8px; text-align: right; color: #38bdf8; font-weight: 500; font-variant-numeric: tabular-nums;"
                                                        } else {
                                                            "padding: 6px 8px; text-align: right; color: #fbbf24; font-weight: 500; font-variant-numeric: tabular-nums;"
                                                        },
                                                        {format!("{:.2}", r.tva.abs())}
                                                    }
                                                    td { style: "padding: 6px 8px; text-align: right; color: #f8fafc; font-variant-numeric: tabular-nums;",
                                                        {format!("{:.2}", r.ttc.abs())}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                tfoot {
                                    tr {
                                        style: "border-top: 2px solid #334155;",
                                        td { style: "padding: 8px; color: #94a3b8; font-weight: 600;", colspan: "3", "Total crédits" }
                                        td { style: "padding: 8px;" }
                                        td { style: "padding: 8px; text-align: right; color: #f8fafc; font-weight: 700; font-variant-numeric: tabular-nums;",
                                            {format!("{:.2}", total_ht_credit)}
                                        }
                                        td { style: "padding: 8px; text-align: right; color: #38bdf8; font-weight: 700; font-variant-numeric: tabular-nums;",
                                            {format!("{:.2}", total_tva_credit)}
                                        }
                                        td { style: "padding: 8px; text-align: right; color: #f8fafc; font-weight: 700; font-variant-numeric: tabular-nums;",
                                            {format!("{:.2}", total_ttc_credit)}
                                        }
                                    }
                                    tr {
                                        style: "border-top: 1px solid #334155;",
                                        td { style: "padding: 8px; color: #94a3b8; font-weight: 600;", colspan: "3", "Total débits" }
                                        td { style: "padding: 8px;" }
                                        td { style: "padding: 8px; text-align: right; color: #f8fafc; font-weight: 700; font-variant-numeric: tabular-nums;",
                                            {format!("{:.2}", total_ht_debit)}
                                        }
                                        td { style: "padding: 8px; text-align: right; color: #fbbf24; font-weight: 700; font-variant-numeric: tabular-nums;",
                                            {format!("{:.2}", total_tva_debit)}
                                        }
                                        td { style: "padding: 8px; text-align: right; color: #f8fafc; font-weight: 700; font-variant-numeric: tabular-nums;",
                                            {format!("{:.2}", total_ttc_debit)}
                                        }
                                    }
                                    tr {
                                        style: "border-top: 2px solid #334155; background: rgba(56,189,248,0.05);",
                                        td { style: "padding: 10px 8px; color: {net_color}; font-weight: 700; text-transform: uppercase;", colspan: "5", "{net_label}" }
                                        td { style: "padding: 10px 8px; text-align: right; color: {net_color}; font-weight: 700; font-variant-numeric: tabular-nums;",
                                            {format!("{:.2}", net_vat.abs())}
                                        }
                                        td { style: "padding: 10px 8px;" }
                                    }
                                }
                            }
                        }

                        div { style: "padding: 10px 12px; background: #0f172a; border-left: 3px solid #38bdf8; border-radius: 4px; font-size: 0.75rem; color: #94a3b8;",
                            "💡 Régime TVA sur encaissements. Source : Dolibarr. Clique sur le taux d'une ligne pour le modifier (0% = exonéré, 5.5%, 10%, 20%). Le taux est mémorisé localement (table bank_line_metadata) pour ne pas polluer Dolibarr."
                        }

                        div { style: "display: flex; gap: 8px; margin-top: 16px;",
                            button {
                                style: "flex: 1; background: transparent; color: #94a3b8; border: 1px solid #334155; padding: 8px; border-radius: 6px; cursor: pointer; font-size: 0.85rem;",
                                onclick: move |_| show_details.set(false),
                                "Fermer"
                            }
                            button {
                                style: "flex: 2; background: #22c55e; color: #04121f; border: none; padding: 8px; border-radius: 6px; cursor: pointer; font-weight: 600; font-size: 0.85rem;",
                                onclick: {
                                    let receipts = receipts_for_export.clone();
                                    let label = label_for_export.clone();
                                    move |_| {
                                        let csv = build_csv(
                                            &receipts,
                                            total_ht_credit, total_tva_credit, total_ttc_credit,
                                            total_ht_debit, total_tva_debit, total_ttc_debit,
                                            net_vat, &label,
                                        );
                                        let safe: String = label.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
                                        download_csv(&format!("tva_{}.csv", safe), &csv);
                                        show_details.set(false);
                                    }
                                },
                                "Télécharger CSV comptable"
                            }
                        }
                    }
                }
            }
        }
    }
}