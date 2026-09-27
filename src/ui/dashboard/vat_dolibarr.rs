use chrono::{Datelike, Local, TimeZone};
use dioxus::prelude::*;

use crate::dolibarr::models::{DolibarrInvoice, DolibarrPayment};

#[derive(Clone, Copy, PartialEq, Eq)]
enum VatPeriod {
    ToDeclare,
    CurrentMonth,
    Quarter,
    Year,
}

impl VatPeriod {
    fn label(&self) -> &'static str {
        match self {
            VatPeriod::ToDeclare => "À déclarer",
            VatPeriod::CurrentMonth => "Mois en cours",
            VatPeriod::Quarter => "Trimestre",
            VatPeriod::Year => "Année",
        }
    }

    fn range(&self) -> (i64, i64) {
        let now = Local::now();
        let (year, month) = (now.year(), now.month());

        match self {
            VatPeriod::ToDeclare => {
                let (py, pm) = if month == 1 { (year - 1, 12) } else { (year, month - 1) };
                let (ny, nm) = if pm == 12 { (py + 1, 1) } else { (py, pm + 1) };
                (
                    Local.with_ymd_and_hms(py, pm, 1, 0, 0, 0).unwrap().timestamp(),
                    Local.with_ymd_and_hms(ny, nm, 1, 0, 0, 0).unwrap().timestamp(),
                )
            }
            VatPeriod::CurrentMonth => {
                let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
                (
                    Local.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap().timestamp(),
                    Local.with_ymd_and_hms(ny, nm, 1, 0, 0, 0).unwrap().timestamp(),
                )
            }
            VatPeriod::Quarter => {
                let q_start = ((month - 1) / 3) * 3 + 1;
                let (ey, em) = if q_start + 3 > 12 {
                    (year + 1, q_start + 3 - 12)
                } else {
                    (year, q_start + 3)
                };
                (
                    Local.with_ymd_and_hms(year, q_start, 1, 0, 0, 0).unwrap().timestamp(),
                    Local.with_ymd_and_hms(ey, em, 1, 0, 0, 0).unwrap().timestamp(),
                )
            }
            VatPeriod::Year => (
                Local.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap().timestamp(),
                Local.with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0).unwrap().timestamp(),
            ),
        }
    }

    fn period_label(&self) -> String {
        let now = Local::now();
        let (year, month) = (now.year(), now.month());

        match self {
            VatPeriod::ToDeclare => {
                let (py, pm) = if month == 1 { (year - 1, 12) } else { (year, month - 1) };
                month_name_fr(pm, py)
            }
            VatPeriod::CurrentMonth => month_name_fr(month, year),
            VatPeriod::Quarter => {
                let q = (month - 1) / 3 + 1;
                format!("T{} {}", q, year)
            }
            VatPeriod::Year => format!("Année {}", year),
        }
    }

    fn hint(&self) -> String {
        let now = Local::now();
        match self {
            VatPeriod::ToDeclare => {
                let (_, m) = if now.month() == 1 { (now.year() - 1, 12) } else { (now.year(), now.month() - 1) };
                format!(
                    "TVA encaissée en {} — à déclarer en {}",
                    month_name_short_fr(m),
                    month_name_short_fr(now.month())
                )
            }
            VatPeriod::CurrentMonth => "Encaissements du mois en cours".to_string(),
            VatPeriod::Quarter => "Cumul trimestriel des encaissements".to_string(),
            VatPeriod::Year => "Cumul annuel des encaissements".to_string(),
        }
    }
}

fn month_name_fr(month: u32, year: i32) -> String {
    let names = [
        "Janvier", "Février", "Mars", "Avril", "Mai", "Juin",
        "Juillet", "Août", "Septembre", "Octobre", "Novembre", "Décembre",
    ];
    format!("{} {}", names[(month - 1) as usize], year)
}

fn month_name_short_fr(month: u32) -> &'static str {
    let names = [
        "janv.", "févr.", "mars", "avr.", "mai", "juin",
        "juil.", "août", "sept.", "oct.", "nov.", "déc.",
    ];
    names[(month - 1) as usize]
}

/// Calcule la TVA sur encaissements pour une période donnée.
///
/// Pour chaque facture validée, on récupère ses paiements, et on ne compte
/// que la TVA correspondant à la part effectivement encaissée dans la période.
///
/// Formule : pour un paiement P sur une facture avec taux de TVA T,
/// la part HT encaissée est P / (1 + T), et la TVA associée est (P / (1 + T)) * T.
fn compute_vat_on_cash(
    invoices: &[DolibarrInvoice],
    payments_by_invoice: &[(String, Vec<DolibarrPayment>)],
    start_ts: i64,
    end_ts: i64,
) -> (f64, usize) {
    let mut total_tva = 0.0;
    let mut count = 0;

    for (invoice_id, payments) in payments_by_invoice {
        let Some(invoice) = invoices.iter().find(|i| i.id == *invoice_id) else {
            continue;
        };

        let invoice_ht: f64 = invoice.total_ht.parse().unwrap_or(0.0);
        let invoice_tva: f64 = invoice.total_tva.parse().unwrap_or(0.0);

        if invoice_ht <= 0.0 {
            continue;
        }

        // Taux de TVA effectif de la facture (0.20 pour 20%)
        let tva_rate = invoice_tva / invoice_ht;

        let mut matched = false;
        for payment in payments {
            let amount: f64 = payment.amount.parse().unwrap_or(0.0);
            if amount <= 0.0 {
                continue;
            }
            if payment.datepaye >= start_ts && payment.datepaye < end_ts {
                // Part HT du paiement, puis TVA correspondante
                let amount_ht = amount / (1.0 + tva_rate);
                total_tva += amount_ht * tva_rate;
                matched = true;
            }
        }
        if matched {
            count += 1;
        }
    }

    (total_tva, count)
}

#[component]
pub fn VatWidgetDolibarr() -> Element {
    let mut period = use_signal(|| VatPeriod::ToDeclare);

    // Récupère les factures validées ET leurs paiements (appels en cascade)
    let invoices_with_payments = use_resource(|| async move {
        let invoices = crate::dolibarr::server_fns::dolibarr_list_invoices(200).await?;

        let mut result: Vec<(String, Vec<DolibarrPayment>)> = Vec::new();
        for inv in &invoices {
            if inv.statut >= 1 {
                let payments =
                    crate::dolibarr::server_fns::dolibarr_list_invoice_payments(inv.id.clone())
                        .await
                        .unwrap_or_default();
                result.push((inv.id.clone(), payments));
            }
        }

        Ok::<_, dioxus::prelude::ServerFnError>((invoices, result))
    });

    rsx! {
        div { class: "vat-widget",
            // --- Header dynamique ---
            div { style: "margin-bottom: 12px;",
                div {
                    style: "font-size: 11px; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.5px;",
                    "TVA sur encaissements"
                }
                div {
                    style: "font-size: 15px; color: #e2e8f0; font-weight: 600; margin-top: 2px;",
                    {period().period_label()}
                }
            }

            // --- Sélecteur de période ---
            div {
                style: "display: flex; gap: 6px; margin-bottom: 14px; flex-wrap: wrap;",
                for p in [VatPeriod::ToDeclare, VatPeriod::CurrentMonth, VatPeriod::Quarter, VatPeriod::Year] {
                    button {
                        style: if period() == p {
                            "padding: 4px 10px; background: #7c3aed; color: white; border: 1px solid #7c3aed; border-radius: 4px; cursor: pointer; font-size: 11px; font-weight: 600;"
                        } else {
                            "padding: 4px 10px; background: transparent; color: #a78bfa; border: 1px solid #6d28d9; border-radius: 4px; cursor: pointer; font-size: 11px;"
                        },
                        onclick: move |_| period.set(p),
                        "{p.label()}"
                    }
                }
            }

            // --- Contenu ---
            match &*invoices_with_payments.read() {
                Some(Ok((invoices, payments_by_invoice))) => {
                    let (start_ts, end_ts) = period().range();
                    let (total_tva, count) =
                        compute_vat_on_cash(invoices, payments_by_invoice, start_ts, end_ts);

                    rsx! {
                        div {
                            div {
                                style: "font-size: 32px; font-weight: 700; color: #f1f5f9; line-height: 1.1;",
                                {format!("{:.2} €", total_tva)}
                            }
                            div {
                                style: "font-size: 11px; color: #94a3b8; margin-top: 6px;",
                                {period().hint()}
                            }
                            div {
                                style: "font-size: 11px; color: #64748b; margin-top: 4px;",
                                {format!("{} facture(s) avec encaissement sur la période", count)}
                            }
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    div { style: "color: #ef4444; font-size: 12px;", {format!("Erreur : {e}")} }
                },
                None => rsx! {
                    div { style: "color: #94a3b8;", "Chargement des encaissements..." }
                },
            }
        }
    }
}