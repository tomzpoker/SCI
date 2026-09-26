use dioxus::prelude::*;
use uuid::Uuid;

use crate::domain::{
    BankTransactionItem, InvoiceItem, LeaseItem, PaymentItem, TaskItem, TenantItem, UnitItem,
};
use crate::server::{
    list_bank_transactions, list_invoices, list_leases, list_payments, list_tasks, list_tenants,
    list_units,
};

use super::forecast::ForecastWidget;
use super::modal::TenantModal;
use super::models::{BankTx, Payment, Task, TaskCategory, Tenant, TenantStatus, WidgetState};
use super::tasks::TaskList;
use super::tenants::TenantBars;
use super::vat::VatWidget;

#[component]
pub fn DashboardWidgets() -> Element {
    // Signal de refresh : on l'incrémente quand une action externe doit recharger les données.
    let mut refresh = use_signal(|| 0u64);

    // === Chargement des données depuis la BDD ===

    let tenants_resource = use_resource(move || {
        let _ = refresh();
        async move {
            let tenants = list_tenants().await.unwrap_or_default();
            let leases = list_leases().await.unwrap_or_default();
            let units = list_units().await.unwrap_or_default();
            let invoices = list_invoices().await.unwrap_or_default();
            let payments = list_payments().await.unwrap_or_default();
            build_tenant_blocks(&tenants, &leases, &units, &invoices, &payments)
        }
    });

    let tasks_resource = use_resource(move || {
        let _ = refresh();
        async move {
            list_tasks()
                .await
                .unwrap_or_default()
                .into_iter()
                .map(task_from_item)
                .collect::<Vec<_>>()
        }
    });

    let bank_txs_resource = use_resource(move || {
        let _ = refresh();
        async move {
            list_bank_transactions()
                .await
                .unwrap_or_default()
                .into_iter()
                .map(bank_tx_from_item)
                .collect::<Vec<_>>()
        }
    });

    // Fallback vide tant que le chargement n'est pas terminé.
    let tenants: Vec<Tenant> = (*tenants_resource.read()).clone().unwrap_or_default();
    let tasks: Vec<Task> = (*tasks_resource.read()).clone().unwrap_or_default();
    let bank_txs: Vec<BankTx> = (*bank_txs_resource.read()).clone().unwrap_or_default();

    let mut selected_tenant = use_signal(|| None::<Tenant>);
    let mut dragged_id = use_signal(|| None::<String>);
    let mut widgets = use_signal(|| {
        vec![
            WidgetState { id: "forecast", title: "PREVISIONNEL", col_span: 8, pinned: false, order: 0 },
            WidgetState { id: "tenants", title: "LOCAUX & SOLDES", col_span: 4, pinned: false, order: 1 },
            WidgetState { id: "tasks", title: "TACHES ADMINISTRATIVES", col_span: 6, pinned: false, order: 2 },
            WidgetState { id: "vat", title: "TVA COLLECTEE", col_span: 6, pinned: false, order: 3 },
        ]
    });

    rsx! {
        section {
            class: "dash-grid",
            for widget in widgets() {
                {
                    let is_dragging = dragged_id() == Some(widget.id.to_string());
                    let is_pinned = widget.pinned;
                    let id_str = widget.id.to_string();
                    let id_for_drop = id_str.clone();
                    let id_for_pin = widget.id;

                    let class = format!(
                        "dash-widget dash-widget-{} {} {} {}",
                        widget.col_span,
                        if is_dragging { "dragging" } else { "" },
                        if is_pinned { "pinned" } else { "" },
                        if widget.id == "tenants" { "compact" } else { "" }
                    );

                    rsx! {
                        div {
                            class: "{class}",
                            draggable: if is_pinned { "false" } else { "true" },
                            ondragstart: move |_| {
                                if !is_pinned {
                                    dragged_id.set(Some(id_str.clone()));
                                }
                            },
                            ondragover: move |e| { e.prevent_default(); },
                            ondrop: move |_| {
                                if let Some(source_id) = dragged_id() {
                                    if source_id != id_for_drop {
                                        widgets.with_mut(|w| {
                                            if let (Some(si), Some(ti)) = (
                                                w.iter().position(|x| x.id == source_id),
                                                w.iter().position(|x| x.id == id_for_drop),
                                            ) {
                                                w.swap(si, ti);
                                                for (i, wgt) in w.iter_mut().enumerate() {
                                                    wgt.order = i;
                                                }
                                            }
                                        });
                                    }
                                }
                                dragged_id.set(None);
                            },

                            div { class: "dash-widget-header",
                                h3 { "{widget.title}" }
                                button {
                                    class: if is_pinned { "dash-pin-btn pinned" } else { "dash-pin-btn" },
                                    onclick: move |_| {
                                        widgets.with_mut(|w| {
                                            if let Some(wgt) = w.iter_mut().find(|x| x.id == id_for_pin) {
                                                wgt.pinned = !wgt.pinned;
                                            }
                                        });
                                    },
                                    title: if is_pinned { "Desepingler" } else { "Epingler" },
                                    if is_pinned { "PIN" } else { "LOC" }
                                }
                            }

                            match widget.id {
                                "forecast" => rsx! { ForecastWidget {} },
                                "tenants" => rsx! {
                                    TenantBars {
                                        tenants: tenants.clone(),
                                        on_select: move |t| selected_tenant.set(Some(t)),
                                    }
                                },
                                "tasks" => rsx! { TaskList { tasks: tasks.clone() } },
                                "vat" => rsx! { VatWidget { bank_txs: bank_txs.clone() } },
                                _ => rsx! { div { "Widget inconnu" } },
                            }
                        }
                    }
                }
            }

            TenantModal { tenant: selected_tenant, on_close: move |_| selected_tenant.set(None) }
        }
    }
}

// ============================================================
//  Adaptateurs : source de données → modèle UI
// ============================================================

fn task_from_item(item: TaskItem) -> Task {
    let now = chrono::Utc::now();
    let due_in_days = (item.due_at - now).num_days() as i32;
    let category = match item.code.as_str() {
        c if c.contains("VAT") || c.contains("TVA") => TaskCategory::Tax,
        c if c.contains("DEADLINE") || c.contains("2072") => TaskCategory::Declaration,
        c if c.contains("RELANCE") || c.contains("RECOVERY") => TaskCategory::Relance,
        _ => TaskCategory::Payment,
    };
    Task {
        id: item.id.as_u128() as usize,
        label: item.title,
        due_in_days,
        category,
    }
}

fn bank_tx_from_item(item: BankTransactionItem) -> BankTx {
    BankTx {
        id: item.id.as_u128() as usize,
        date: item.booked_at.format("%Y-%m-%d").to_string(),
        label: item.label,
        amount: item.amount_cents as f64 / 100.0,
    }
}

/// Construit les blocs visuels "Locaux & Soldes".
///
/// Un bloc par **unité** :
/// - Si un bail actif existe → bloc locataire avec solde réel et historique paiements
/// - Sinon → bloc "Vacant"
fn build_tenant_blocks(
    tenants: &[TenantItem],
    leases: &[LeaseItem],
    units: &[UnitItem],
    invoices: &[InvoiceItem],
    payments: &[PaymentItem],
) -> Vec<Tenant> {
    let mut blocks: Vec<Tenant> = Vec::new();

    for unit in units {
        // Bail actif sur cette unité
        let active_lease = leases.iter().find(|l| l.unit_id == unit.id && l.active);

        match active_lease {
            Some(lease) => {
                let Some(tenant) = tenants.iter().find(|t| t.id == lease.tenant_id) else {
                    continue;
                };

                // Factures liées à ce bail
                let tenant_invoices: Vec<&InvoiceItem> = invoices
                    .iter()
                    .filter(|i| i.lease_id == Some(lease.id))
                    .collect();

                // Solde = somme (reçu - attendu)
                let balance: f64 = tenant_invoices
                    .iter()
                    .map(|i| (i.paid_cents - i.gross_cents) as f64 / 100.0)
                    .sum();

                let tenant_invoice_ids: Vec<Uuid> =
                    tenant_invoices.iter().map(|i| i.id).collect();

                // Historique des paiements (jusqu'à 6 plus récents, triés par received_at DESC côté SQL)
                let tenant_payments: Vec<Payment> = payments
                    .iter()
                    .filter(|p| {
                        p.invoice_id
                            .map(|x| tenant_invoice_ids.contains(&x))
                            .unwrap_or(false)
                    })
                    .take(6)
                    .map(|p| {
                        let expected = p
                            .invoice_id
                            .and_then(|iid| invoices.iter().find(|i| i.id == iid))
                            .map(|i| i.gross_cents as f64 / 100.0)
                            .unwrap_or(0.0);
                        Payment {
                            date: p.received_at.format("%Y-%m-%d").to_string(),
                            expected,
                            received: p.amount_cents as f64 / 100.0,
                        }
                    })
                    .collect();

                let status = if !tenant.active {
                    TenantStatus::Vacant
                } else if balance >= -0.01 {
                    TenantStatus::Paid
                } else if balance > -300.0 {
                    TenantStatus::Late
                } else {
                    TenantStatus::Unpaid
                };

                blocks.push(Tenant {
                    id: unit.id.as_u128() as usize,
                    name: tenant.legal_name.clone(),
                    property: format!("{} • {}", unit.property_name, unit.label),
                    rent: unit.base_rent_cents as f64 / 100.0,
                    balance,
                    status,
                    payments: tenant_payments,
                });
            }
            None => {
                // Unité vacante
                blocks.push(Tenant {
                    id: unit.id.as_u128() as usize,
                    name: "Vacant".into(),
                    property: format!("{} • {}", unit.property_name, unit.label),
                    rent: unit.base_rent_cents as f64 / 100.0,
                    balance: 0.0,
                    status: TenantStatus::Vacant,
                    payments: vec![],
                });
            }
        }
    }

    blocks
}