use dioxus::prelude::*;
use super::models::*;
use super::forecast::ForecastWidget;
use super::tenants::TenantBars;
use super::tasks::TaskList;
use super::modal::TenantModal;
use super::vat::VatWidget;
use super::models::BankTx;

#[component]
pub fn DashboardWidgets() -> Element {
    let tenants = vec![
        Tenant {
            id: 1, name: "M. Dupont".into(), property: "Local A".into(),
            rent: 800.0, balance: 0.0, status: TenantStatus::Paid,
            payments: vec![
                Payment { date: "2026-07-05".into(), expected: 800.0, received: 800.0 },
                Payment { date: "2026-08-05".into(), expected: 800.0, received: 800.0 },
                Payment { date: "2026-09-05".into(), expected: 800.0, received: 800.0 },
            ],
        },
        Tenant {
            id: 2, name: "Mme Martin".into(), property: "Local B".into(),
            rent: 950.0, balance: -300.0, status: TenantStatus::Late,
            payments: vec![
                Payment { date: "2026-07-05".into(), expected: 950.0, received: 950.0 },
                Payment { date: "2026-08-05".into(), expected: 950.0, received: 950.0 },
                Payment { date: "2026-09-05".into(), expected: 950.0, received: 650.0 },
            ],
        },
        Tenant {
            id: 3, name: "SCI Immo".into(), property: "Local C".into(),
            rent: 1200.0, balance: -800.0, status: TenantStatus::Unpaid,
            payments: vec![
                Payment { date: "2026-07-05".into(), expected: 1200.0, received: 1200.0 },
                Payment { date: "2026-08-05".into(), expected: 1200.0, received: 700.0 },
                Payment { date: "2026-09-05".into(), expected: 1200.0, received: 900.0 },
            ],
        },
        Tenant {
            id: 4, name: "M. Petit".into(), property: "Local D".into(),
            rent: 700.0, balance: 0.0, status: TenantStatus::Paid,
            payments: vec![
                Payment { date: "2026-07-05".into(), expected: 700.0, received: 700.0 },
                Payment { date: "2026-08-05".into(), expected: 700.0, received: 700.0 },
                Payment { date: "2026-09-05".into(), expected: 700.0, received: 700.0 },
            ],
        },
        Tenant {
            id: 5, name: "Vacant".into(), property: "Local E".into(),
            rent: 0.0, balance: 0.0, status: TenantStatus::Vacant,
            payments: vec![],
        },
    ];

    let tasks = vec![
        Task { id: 1, label: "Paiement a effectuer : Taxe fonciere".into(), due_in_days: 3, category: TaskCategory::Payment },
        Task { id: 2, label: "Relance a effectuer : Mme Martin".into(), due_in_days: 12, category: TaskCategory::Relance },
        Task { id: 3, label: "Declaration TVA".into(), due_in_days: 2, category: TaskCategory::Tax },
        Task { id: 4, label: "Declaration annuelle (liasse 2072)".into(), due_in_days: 128, category: TaskCategory::Declaration },
    ];

    let bank_txs = vec![
        BankTx { id: 10, date: "2026-07-05".into(), label: "VIREMENT LOYER DUPONT".into(),  amount:  800.0 },
        BankTx { id: 11, date: "2026-08-05".into(), label: "VIREMENT LOYER MARTIN".into(),  amount:  950.0 },
        BankTx { id: 12, date: "2026-09-05".into(), label: "VIREMENT LOYER PETIT".into(),   amount:  700.0 },
        BankTx { id: 20, date: "2026-07-15".into(), label: "PRLV EDF".into(),                amount: -220.0 },
        BankTx { id: 21, date: "2026-08-15".into(), label: "PRLV EDF".into(),                amount: -240.0 },
        BankTx { id: 22, date: "2026-09-15".into(), label: "PRLV EAU".into(),                amount: -180.0 },
        BankTx { id: 30, date: "2026-07-01".into(), label: "ECHEANCE CREDIT IMMO".into(),    amount: -1200.0 },
        BankTx { id: 31, date: "2026-08-01".into(), label: "ECHEANCE CREDIT IMMO".into(),    amount: -1200.0 },
        BankTx { id: 32, date: "2026-09-01".into(), label: "ECHEANCE CREDIT IMMO".into(),    amount: -1200.0 },
    ];	

    let mut selected_tenant = use_signal(|| None::<Tenant>);
    let mut dragged_id = use_signal(|| None::<String>);
    let mut widgets = use_signal(|| vec![
        WidgetState { id: "forecast", title: "PREVISIONNEL", col_span: 8, pinned: false, order: 0 },
        WidgetState { id: "tenants", title: "LOCAUX & SOLDES", col_span: 4, pinned: false, order: 1 },
        WidgetState { id: "tasks", title: "TACHES ADMINISTRATIVES", col_span: 6, pinned: false, order: 2 },
        WidgetState { id: "vat", title: "TVA COLLECTÉE", col_span: 6, pinned: false, order: 3 },
    ]);

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
                                                w.iter().position(|x| x.id == id_for_drop)
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
                                "tenants" => rsx! { TenantBars { tenants: tenants.clone(), on_select: move |t| selected_tenant.set(Some(t)) } },
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