use dioxus::prelude::*;

use crate::relances::LocalBarItem;
use crate::server::list_tasks;

use super::forecast::ForecastWidget;
use super::modal::LocalModal;
use super::models::{Task, TaskCategory, WidgetState};
use super::tasks::TaskList;
use super::tenants::LocalBars;
use super::vat::VatWidget;

#[component]
pub fn DashboardWidgets() -> Element {
    let mut refresh = use_signal(|| 0u64);

    // === Locaux + impayés depuis Dolibarr (vue complète : occupés, vides, à jour) ===
    let locals_resource = use_resource(move || {
        let _ = refresh();
        async move {
            crate::relances::list_all_locals()
                .await
                .unwrap_or_default()
        }
    });

    // === Tâches locales ===
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

    // === TEST DOLIBARR (debug, repliable) ===
    let dolibarr_test = use_resource(|| async move {
        crate::dolibarr::server_fns::dolibarr_list_third_parties(10).await
    });
    let dolibarr_invoices_test = use_resource(|| async move {
        crate::dolibarr::server_fns::dolibarr_list_invoices(10).await
    });

    let locals: Vec<LocalBarItem> =
        (*locals_resource.read()).clone().unwrap_or_default();
    let tasks: Vec<Task> = (*tasks_resource.read()).clone().unwrap_or_default();

    let mut selected_local = use_signal(|| None::<LocalBarItem>);
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
        // --- DEBUG DOLIBARR (repliable) ---
        details {
            class: "panel",
            style: "margin-bottom: 16px;",
            summary {
                style: "cursor: pointer; color: #94a3b8; padding: 8px; font-size: 12px;",
                "Debug Dolibarr (cliquer pour ouvrir)"
            }

            section {
                style: "margin-top: 12px;",
                h3 { "Test Dolibarr — Tiers" }
                match &*dolibarr_test.read() {
                    Some(Ok(list)) => rsx! {
                        div { {format!("{} tiers recuperes", list.len())} }
                        for tp in list.iter().take(5) {
                            div { {format!("- {} (id: {})", tp.name, tp.id)} }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        div { style: "color: red;", {format!("Erreur : {e}")} }
                    },
                    None => rsx! {
                        div { "Chargement..." }
                    },
                }
            }

            section {
                style: "margin-top: 12px;",
                h3 { "Test Dolibarr — Factures" }
                match &*dolibarr_invoices_test.read() {
                    Some(Ok(list)) => rsx! {
                        div { {format!("{} factures recuperees", list.len())} }
                        for inv in list.iter().take(5) {
                            div { {
                                format!(
                                    "- {} | {} EUR TTC | statut: {} | paye: {}",
                                    inv.r#ref,
                                    inv.total_ttc,
                                    inv.statut,
                                    inv.paye
                                )
                            } }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        div { style: "color: red;", {format!("Erreur : {e}")} }
                    },
                    None => rsx! {
                        div { "Chargement..." }
                    },
                }
            }
        }

        // --- DASHBOARD ---
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
                                    LocalBars {
                                        locals: locals.clone(),
                                        on_select: move |l: LocalBarItem| selected_local.set(Some(l)),
                                    }
                                },
                                "tasks" => rsx! { TaskList { tasks: tasks.clone() } },
                                "vat" => rsx! { VatWidget {} },
                                _ => rsx! { div { "Widget inconnu" } },
                            }
                        }
                    }
                }
            }

            // Modale détail local
            LocalModal {
                local: selected_local,
                on_close: move |_| selected_local.set(None),
            }
        }
    }
}

fn task_from_item(item: crate::domain::TaskItem) -> Task {
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