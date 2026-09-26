use dioxus::prelude::*;
use super::models::BankTx;
use super::forecast::SimpleDate;

const VAT_RATE: f64 = 0.20;
const MONTHS_FR_FULL: &[&str] = &[
    "janvier", "février", "mars", "avril", "mai", "juin",
    "juillet", "août", "septembre", "octobre", "novembre", "décembre",
];

#[derive(Clone, Copy, PartialEq)]
enum VatRange { Month, Quarter }

impl VatRange {
    fn label(self) -> &'static str {
        match self {
            VatRange::Month => "Mois",
            VatRange::Quarter => "Trimestre",
        }
    }
    fn n_months(self) -> i32 {
        match self {
            VatRange::Month => 1,
            VatRange::Quarter => 3,
        }
    }
}

#[derive(Clone)]
struct Receipt {
    date_label: String,
    label: String,
    ttc: f64,
    ht: f64,
    tva: f64,
}

fn period_label(start: SimpleDate, n_months: i32) -> String {
    if n_months == 1 {
        format!("{} {}", MONTHS_FR_FULL[(start.month - 1) as usize], start.year)
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

#[component]
pub fn VatWidget(bank_txs: Vec<BankTx>) -> Element {
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

    // Filtrer les encaissements (bank_txs > 0) dans la période
    let n = range().n_months();
    let start = cursor().start_of_month();
    let end = start.shift_months(n - 1);
    let end_last_day = SimpleDate {
        year: end.year,
        month: end.month,
        day: 28,
    };

    let mut receipts: Vec<Receipt> = bank_txs.iter()
        .filter_map(|tx| {
            if tx.amount <= 0.0 { return None; }
            // Parse date
            let parts: Vec<&str> = tx.date.split('-').collect();
            if parts.len() < 3 { return None; }
            let y: i32 = parts[0].parse().ok()?;
            let m: u32 = parts[1].parse().ok()?;
            let d: u32 = parts[2].parse().ok()?;
            let date = SimpleDate { year: y, month: m, day: d };

            // Dans la période ?
            if date.month_abs() < start.month_abs() || date.month_abs() > end.month_abs() {
                return None;
            }

            let ttc = tx.amount;
            let ht = ttc / (1.0 + VAT_RATE);
            let tva = ttc - ht;
            Some(Receipt {
                date_label: format!("{:02}/{:02}", d, m),
                label: tx.label.clone(),
                ttc,
                ht,
                tva,
            })
        })
        .collect();

    receipts.sort_by(|a, b| a.date_label.cmp(&b.date_label));

    let total_ttc: f64 = receipts.iter().map(|r| r.ttc).sum();
    let total_ht: f64 = receipts.iter().map(|r| r.ht).sum();
    let total_tva: f64 = receipts.iter().map(|r| r.tva).sum();

    let label = period_label(start, n);
    let is_current = start.month_abs() == real_today().month_abs()
        || (real_today().month_abs() >= start.month_abs()
            && real_today().month_abs() <= end_last_day.month_abs());

    let year_min = real_today().year - 5;
    let year_max = real_today().year + 5;

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 10px; min-width: 0; max-width: 100%;",

            // En-tête
            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 8px; flex-wrap: wrap;",
                div { style: "display: flex; align-items: center; gap: 6px;",
                    div { style: "display: inline-flex; background: #0f172a; border-radius: 6px; padding: 2px; border: 1px solid #334155;",
                        for r in [VatRange::Month, VatRange::Quarter] {
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
                            onclick: move |_| cursor.set(real_today().start_of_month()),
                            "Aujourd'hui"
                        }
                    }
                }
                if !receipts.is_empty() {
                    button {
                        draggable: "false",
                        style: "display: inline-flex; align-items: center; gap: 5px; background: #0f172a; border: 1px solid #334155; color: #94a3b8; padding: 4px 12px; border-radius: 6px; font-size: 0.72rem; cursor: pointer;",
                        onclick: move |_| show_details.set(true),
                        "Détails →"
                    }
                }
            }

            // Montant principal
            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; padding: 14px 16px; background: #0f172a; border-radius: 8px; border: 1px solid #1e293b;",
                div {
                    div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.06em;",
                        "TVA collectée • {label}"
                    }
                    div { style: "font-size: 1.6rem; font-weight: 700; color: #38bdf8; font-variant-numeric: tabular-nums; margin-top: 4px;",
                        {format!("{:.2} EUR", total_tva)}
                    }
                    div { style: "font-size: 0.7rem; color: #94a3b8; margin-top: 4px;",
                        "{receipts.len()} encaissement(s) • taux {VAT_RATE * 100.0:.0}%"
                    }
                }
                div { style: "display: flex; gap: 20px;",
                    div { style: "text-align: right;",
                        div { style: "font-size: 0.65rem; color: #64748b;", "Base HT" }
                        div { style: "font-size: 1rem; font-weight: 600; color: #cbd5e1; font-variant-numeric: tabular-nums;",
                            {format!("{:.2} EUR", total_ht)}
                        }
                    }
                    div { style: "text-align: right;",
                        div { style: "font-size: 0.65rem; color: #64748b;", "Total TTC" }
                        div { style: "font-size: 1rem; font-weight: 600; color: #cbd5e1; font-variant-numeric: tabular-nums;",
                            {format!("{:.2} EUR", total_ttc)}
                        }
                    }
                }
            }

            if receipts.is_empty() {
                div { style: "padding: 12px; text-align: center; color: #64748b; font-size: 0.8rem; background: #0f172a; border-radius: 6px; border: 1px dashed #334155;",
                    "Aucun encaissement sur la période."
                }
            }

            // Modale détails
            if show_details() {
                div {
                    class: "dash-modal-overlay",
                    onclick: move |_| show_details.set(false),
                    div {
                        class: "dash-modal",
                        onclick: move |e| e.stop_propagation(),
                        style: "max-width: 720px; max-height: 85vh; overflow-y: auto;",
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

                        // Récap déclaration
                        div { style: "display: flex; gap: 12px; padding: 12px; background: #0f172a; border-radius: 6px; margin-bottom: 16px;",
                            div { style: "flex: 1;",
                                div { style: "font-size: 0.65rem; color: #64748b;", "Base HT imposable" }
                                div { style: "font-size: 1rem; font-weight: 600; color: #f8fafc; font-variant-numeric: tabular-nums;",
                                    {format!("{:.2} EUR", total_ht)}
                                }
                            }
                            div { style: "flex: 1;",
                                div { style: "font-size: 0.65rem; color: #64748b;", "TVA brute collectée" }
                                div { style: "font-size: 1rem; font-weight: 600; color: #38bdf8; font-variant-numeric: tabular-nums;",
                                    {format!("{:.2} EUR", total_tva)}
                                }
                            }
                            div { style: "flex: 1;",
                                div { style: "font-size: 0.65rem; color: #64748b;", "TVA à payer" }
                                div { style: "font-size: 1rem; font-weight: 700; color: #22c55e; font-variant-numeric: tabular-nums;",
                                    {format!("{:.2} EUR", total_tva)}
                                }
                            }
                        }

                        // Détail
                        div { style: "margin-bottom: 16px;",
                            h4 { style: "color: #94a3b8; margin: 0 0 8px 0; font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.05em;",
                                "Encaissements de la période"
                            }
                            table {
                                style: "width: 100%; border-collapse: collapse; font-size: 0.78rem;",
                                thead {
                                    tr {
                                        th { style: "text-align: left; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "Date" }
                                        th { style: "text-align: left; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "Libellé" }
                                        th { style: "text-align: right; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "HT" }
                                        th { style: "text-align: right; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "TVA" }
                                        th { style: "text-align: right; padding: 6px 8px; border-bottom: 1px solid #334155; color: #94a3b8; font-weight: 500;", "TTC" }
                                    }
                                }
                                tbody {
                                    for r in receipts.iter() {
                                        tr {
                                            td { style: "padding: 6px 8px; color: #cbd5e1; white-space: nowrap;", "{r.date_label}" }
                                            td { style: "padding: 6px 8px; color: #f8fafc;", "{r.label}" }
                                            td { style: "padding: 6px 8px; text-align: right; color: #cbd5e1; font-variant-numeric: tabular-nums;",
                                                {format!("{:.2}", r.ht)}
                                            }
                                            td { style: "padding: 6px 8px; text-align: right; color: #38bdf8; font-weight: 500; font-variant-numeric: tabular-nums;",
                                                {format!("{:.2}", r.tva)}
                                            }
                                            td { style: "padding: 6px 8px; text-align: right; color: #f8fafc; font-variant-numeric: tabular-nums;",
                                                {format!("{:.2}", r.ttc)}
                                            }
                                        }
                                    }
                                }
                                tfoot {
                                    tr {
                                        style: "border-top: 2px solid #334155;",
                                        td { style: "padding: 8px; color: #94a3b8; font-weight: 600;", colspan: "2", "Total" }
                                        td { style: "padding: 8px; text-align: right; color: #f8fafc; font-weight: 700; font-variant-numeric: tabular-nums;",
                                            {format!("{:.2}", total_ht)}
                                        }
                                        td { style: "padding: 8px; text-align: right; color: #38bdf8; font-weight: 700; font-variant-numeric: tabular-nums;",
                                            {format!("{:.2}", total_tva)}
                                        }
                                        td { style: "padding: 8px; text-align: right; color: #f8fafc; font-weight: 700; font-variant-numeric: tabular-nums;",
                                            {format!("{:.2}", total_ttc)}
                                        }
                                    }
                                }
                            }
                        }

                        div { style: "padding: 10px 12px; background: #0f172a; border-left: 3px solid #38bdf8; border-radius: 4px; font-size: 0.75rem; color: #94a3b8;",
                            "💡 Ce brouillon est calculé automatiquement à partir des encaissements. Vérifiez les opérations avant de déclarer."
                        }

                        div { style: "display: flex; gap: 8px; margin-top: 16px;",
                            button {
                                style: "flex: 1; background: transparent; color: #94a3b8; border: 1px solid #334155; padding: 8px; border-radius: 6px; cursor: pointer; font-size: 0.85rem;",
                                onclick: move |_| show_details.set(false),
                                "Fermer"
                            }
                            button {
                                style: "flex: 2; background: #38bdf8; color: #0f172a; border: none; padding: 8px; border-radius: 6px; cursor: pointer; font-weight: 600; font-size: 0.85rem;",
                                onclick: move |_| {
                                    // À faire : exporter en PDF / envoyer à l'admin
                                    show_details.set(false);
                                },
                                "Préparer la déclaration"
                            }
                        }
                    }
                }
            }
        }
    }
}