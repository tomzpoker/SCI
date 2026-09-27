use dioxus::prelude::*;
use uuid::Uuid;

use crate::domain::{TaskItem, TaskState};
use crate::server::{delete_task, set_task_state, update_task};
use super::adapters::task_category_from_code;
use super::models::{Task, TaskCategory};

fn category_label(cat: &TaskCategory) -> &'static str {
    match cat {
        TaskCategory::Payment => "Paiement",
        TaskCategory::Relance => "Relance",
        TaskCategory::Tax => "TVA",
        TaskCategory::Declaration => "Déclaration",
    }
}

/// Urgence : 5 paliers.
/// - En retard         : < 0 j    → rouge vif
/// - Imminent          : 0-3 j    → rouge pastel
/// - Cette semaine+    : 4-10 j   → orange pastel
/// - Ce mois           : 11-30 j  → ambre pastel
/// - Plus tard         : > 30 j   → gris ardoise
fn urgency_color(days: i32) -> &'static str {
    if days < 0 {
        "#ef4444"      // rouge vif     — en retard
    } else if days <= 3 {
        "#f87171"      // rouge pastel  — imminent
    } else if days <= 10 {
        "#fb923c"      // orange pastel — cette semaine+
    } else if days <= 30 {
        "#fbbf24"      // ambre pastel  — ce mois
    } else {
        "#64748b"      // gris ardoise  — plus tard
    }
}

fn urgency_text_style(days: i32, done: bool) -> String {
    if done {
        return "color: #475569;".to_string();
    }
    let c = urgency_color(days);
    format!("color: {c};")
}

fn urgency_label(days: i32) -> String {
    if days < 0 {
        format!("En retard de {} j", -days)
    } else if days == 0 {
        "Aujourd'hui".to_string()
    } else if days == 1 {
        "Demain".to_string()
    } else {
        format!("Dans {} j", days)
    }
}

// ============================================================
//  TaskCard — utilisé dans le widget Dashboard
// ============================================================

#[component]
pub fn TaskCard(task: Task, show_meta: bool) -> Element {
    let cat_label = category_label(&task.category);
    let _ = show_meta;

    let border = urgency_color(task.due_in_days);
    let text_style = urgency_text_style(task.due_in_days, false);
    let label_due = urgency_label(task.due_in_days);

    rsx! {
        div {
            style: "background: var(--bg-glass); border: 1px solid var(--border); border-left: 3px solid {border}; padding: 10px 14px; border-radius: 10px; display: flex; align-items: center; gap: 12px;",
            div { style: "flex: 1; min-width: 0;",
                div { style: "font-size: 0.87rem; color: #e2e8f0; font-weight: 500;", "{task.label}" }
                div { style: "font-size: 0.72rem; margin-top: 2px;",
                    span { style: "color: #64748b;", "{cat_label}" }
                    span { style: "color: #334155;", " · " }
                    span { style: "{text_style}", "{label_due}" }
                }
            }
        }
    }
}

#[component]
pub fn TaskList(tasks: Vec<Task>) -> Element {
    if tasks.is_empty() {
        return rsx! {
            div {
                style: "padding: 16px; text-align: center; color: #64748b; font-size: 0.82rem; background: var(--bg-input); border: 1px dashed var(--border-strong); border-radius: 8px;",
                "Aucune tâche en cours."
            }
        };
    }

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 8px;",
            for t in tasks {
                TaskCard { task: t, show_meta: false }
            }
        }
    }
}

// ============================================================
//  TaskManagerRow — page Tâches
// ============================================================

#[component]
pub fn TaskManagerRow(item: TaskItem, bump: Signal<u64>) -> Element {
    let id: Uuid = item.id;
    let display_title = item.title.clone();
    let display_priority = item.priority;
    let display_due_at = item.due_at;
    let is_done = matches!(item.state, TaskState::Done);
    let item_code = item.code.clone();

    let initial_title = item.title.clone();
    let initial_description = item.description.clone();
    let initial_due = item.due_at.date_naive().format("%Y-%m-%d").to_string();
    let initial_priority = item.priority.to_string();

    let title_for_edit = display_title.clone();
    let description_for_edit = item.description.clone();
    let due_for_edit = initial_due.clone();
    let priority_for_edit = initial_priority.clone();

    let now = chrono::Utc::now();
    let due_in_days = (display_due_at - now).num_days() as i32;
    let category = task_category_from_code(&item_code);
    let cat_label = category_label(&category);
    let border_color = urgency_color(due_in_days);
    let text_style = urgency_text_style(due_in_days, is_done);
    let label_due = urgency_label(due_in_days);

    let mut editing = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let mut edit_title = use_signal(move || initial_title);
    let mut edit_description = use_signal(move || initial_description);
    let mut edit_due = use_signal(move || initial_due);
    let mut edit_priority = use_signal(move || initial_priority);

    // ========== Mode édition ==========
    if editing() {
        return rsx! {
            div {
                style: "background: var(--bg-glass); border: 1px solid var(--border-strong); border-left: 3px solid {border_color}; padding: 12px 14px; border-radius: 10px; display: flex; flex-direction: column; gap: 8px;",
                div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.08em;", "Titre" }
                input {
                    r#type: "text",
                    value: "{edit_title}",
                    oninput: move |e| edit_title.set(e.value()),
                    style: "padding: 6px 10px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px; color: #e2e8f0; font-size: 0.85rem;",
                }
                div { style: "font-size: 0.7rem; color: #94a3b8;", "Description" }
                textarea {
                    value: "{edit_description}",
                    oninput: move |e| edit_description.set(e.value()),
                    rows: "2",
                    style: "padding: 6px 10px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px; color: #e2e8f0; font-size: 0.8rem; resize: vertical;",
                }
                div { style: "display: flex; gap: 8px; flex-wrap: wrap;",
                    div { style: "display: flex; flex-direction: column; gap: 4px; flex: 1; min-width: 140px;",
                        span { style: "font-size: 0.7rem; color: #94a3b8;", "Échéance" }
                        input {
                            r#type: "date",
                            value: "{edit_due}",
                            oninput: move |e| edit_due.set(e.value()),
                            style: "padding: 6px 10px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px; color: #e2e8f0; font-size: 0.8rem;",
                        }
                    }
                    div { style: "display: flex; flex-direction: column; gap: 4px; width: 120px;",
                        span { style: "font-size: 0.7rem; color: #94a3b8;", "Priorité (0-100)" }
                        input {
                            r#type: "number",
                            min: "0", max: "100",
                            value: "{edit_priority}",
                            oninput: move |e| edit_priority.set(e.value()),
                            style: "padding: 6px 10px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px; color: #e2e8f0; font-size: 0.8rem;",
                        }
                    }
                }
                if let Some(err) = error() {
                    div { style: "color: #f87171; font-size: 0.75rem;", "{err}" }
                }
                div { style: "display: flex; gap: 8px; justify-content: flex-end; margin-top: 4px;",
                    button {
                        disabled: busy(),
                        onclick: move |_| { editing.set(false); error.set(None); },
                        style: "padding: 6px 14px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.8rem;",
                        "Annuler"
                    }
                    button {
                        disabled: busy(),
                        onclick: move |_| {
                            let t = edit_title();
                            let d = edit_description();
                            let due_str = edit_due();
                            let prio_str = edit_priority();
                            busy.set(true);
                            error.set(None);
                            spawn(async move {
                                let due = match chrono::NaiveDate::parse_from_str(&due_str, "%Y-%m-%d") {
                                    Ok(d) => d,
                                    Err(_) => {
                                        error.set(Some("Date invalide".into()));
                                        busy.set(false);
                                        return;
                                    }
                                };
                                let prio: i32 = prio_str.parse().unwrap_or(50);
                                match update_task(id, t, d, due, prio).await {
                                    Ok(_) => {
                                        bump.with_mut(|v| *v += 1);
                                        editing.set(false);
                                        busy.set(false);
                                    }
                                    Err(e) => {
                                        error.set(Some(format!("{e}")));
                                        busy.set(false);
                                    }
                                }
                            });
                        },
                        style: "padding: 6px 14px; background: #7c3aed; border: none; color: white; border-radius: 6px; cursor: pointer; font-size: 0.8rem; font-weight: 600;",
                        if busy() { "..." } else { "Enregistrer" }
                    }
                }
            }
        };
    }

    // ========== Mode lecture ==========
    let id_toggle = id;
    let id_del = id;

    let title_style = if is_done {
        "font-size: 0.87rem; color: #64748b; font-weight: 500; text-decoration: line-through;"
    } else {
        "font-size: 0.87rem; color: #e2e8f0; font-weight: 500;"
    };

    rsx! {
        div {
            style: "background: var(--bg-glass); border: 1px solid var(--border); border-left: 3px solid {border_color}; padding: 10px 14px; border-radius: 10px; display: flex; align-items: center; gap: 12px;",

            // === Case à cocher ===
            button {
                title: if is_done { "Marquer comme non faite" } else { "Marquer comme faite" },
                disabled: busy(),
                onclick: move |_| {
                    busy.set(true);
                    let new_state = if is_done { "READY" } else { "DONE" };
                    spawn(async move {
                        let _ = set_task_state(id_toggle, new_state.into()).await;
                        bump.with_mut(|v| *v += 1);
                        busy.set(false);
                    });
                },
                style: "width: 22px; height: 22px; flex-shrink: 0; border-radius: 50%; border: 1.5px solid #475569; background: transparent; cursor: pointer; padding: 0; display: flex; align-items: center; justify-content: center; transition: all 0.15s;",
                if is_done {
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        width: "12", height: "12",
                        view_box: "0 0 24 24", fill: "none",
                        stroke: "#4ade80", stroke_width: "3",
                        stroke_linecap: "round", stroke_linejoin: "round",
                        polyline { points: "20 6 9 17 4 12" }
                    }
                }
            }

            // === Contenu principal ===
            div { style: "flex: 1; min-width: 0;",
                div {
                    title: "Priorité : {display_priority}/100",
                    style: "{title_style}",
                    "{display_title}"
                }
                div { style: "font-size: 0.72rem; margin-top: 2px;",
                    span { style: "color: #64748b;", "{cat_label}" }
                    span { style: "color: #334155;", " · " }
                    span { style: "{text_style}", "{label_due}" }
                }
            }

            // === Actions ===
            div { style: "display: flex; gap: 4px; flex-shrink: 0;",
                button {
                    title: "Modifier",
                    disabled: busy(),
                    onclick: move |_| {
                        edit_title.set(title_for_edit.clone());
                        edit_description.set(description_for_edit.clone());
                        edit_due.set(due_for_edit.clone());
                        edit_priority.set(priority_for_edit.clone());
                        error.set(None);
                        editing.set(true);
                    },
                    style: "padding: 4px 10px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 600;",
                    "Modifier"
                }
                button {
                    title: "Supprimer",
                    disabled: busy(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            let _ = delete_task(id_del).await;
                            bump.with_mut(|v| *v += 1);
                            busy.set(false);
                        });
                    },
                    style: "padding: 4px 10px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 600;",
                    "Supprimer"
                }
            }
        }
    }
}

pub fn task_item_to_task(item: &TaskItem) -> Task {
    let now = chrono::Utc::now();
    Task {
        id: item.id.as_u128() as usize,
        label: item.title.clone(),
        due_in_days: (item.due_at - now).num_days() as i32,
        category: task_category_from_code(&item.code),
    }
}