use dioxus::prelude::*;
use super::models::{Task, TaskCategory};

#[component]
pub fn TaskList(tasks: Vec<Task>) -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 8px;",
            for t in tasks {
                {
                    let (color, bg) = match t.category {
                        TaskCategory::Payment => ("#ef4444", "#450a0a"),
                        TaskCategory::Relance => ("#f59e0b", "#451a03"),
                        TaskCategory::Tax => ("#38bdf8", "#082f49"),
                        TaskCategory::Declaration => ("#22c55e", "#052e16"),
                    };
                    rsx! {
                        div {
                            style: "background: {bg}; border-left: 4px solid {color}; padding: 10px; border-radius: 4px; display: flex; justify-content: space-between; align-items: center;",
                            span { style: "font-size: 0.85rem;", "{t.label}" }
                            span { style: "font-size: 0.75rem; color: #94a3b8;", "Dans {t.due_in_days} jours" }
                        }
                    }
                }
            }
        }
    }
}
