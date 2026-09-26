use dioxus::prelude::*;
use super::models::{Tenant, TenantStatus};

#[component]
pub fn TenantBars(
    tenants: Vec<Tenant>,
    on_select: EventHandler<Tenant>,
) -> Element {
    let mut items = use_signal(|| tenants.clone());
    let mut drag_idx = use_signal(|| None::<usize>);
    let mut drag_over_idx = use_signal(|| None::<usize>);
    let mut just_dragged = use_signal(|| false);

    rsx! {
        div {
            style: "display: flex; justify-content: space-around; align-items: flex-end; gap: 8px; padding: 4px 0;",
            for (idx, t) in items().into_iter().enumerate() {
                {
                    let is_vacant = t.status == TenantStatus::Vacant;
                    let max_abs = items().iter().map(|x| x.balance.abs()).fold(0.0, f64::max);
                    let max_abs = if max_abs == 0.0 { 1.0 } else { max_abs };
                    let ratio = (t.balance.abs() / max_abs).clamp(0.0, 1.0);

                    let (fill_pct, color, border_col) = if is_vacant {
                        (0.0, "#475569", "#334155")
                    } else if t.balance >= 0.0 {
                        (((1.0 - ratio) * 100.0).max(6.0), "#22c55e", "#334155")
                    } else if ratio < 0.5 {
                        (((1.0 - ratio) * 100.0).max(6.0), "#f59e0b", "#78350f")
                    } else {
                        (((1.0 - ratio) * 100.0).max(6.0), "#ef4444", "#7f1d1d")
                    };

                    let label = if is_vacant {
                        "Libre".to_string()
                    } else if t.balance == 0.0 {
                        "0 EUR".to_string()
                    } else {
                        format!("{:.0} EUR", t.balance)
                    };

                    let is_dragging = drag_idx() == Some(idx);
                    let is_drag_over = drag_over_idx() == Some(idx) && drag_idx() != Some(idx);
                    let opacity = if is_dragging { "0.35" } else { "1" };
                    let transform = if is_drag_over { "scale(1.06)" } else { "scale(1)" };
                    let cursor = if is_dragging { "grabbing" } else { "grab" };
                    let tenant_clone = t.clone();

                    rsx! {
                        div {
                            key: "{t.id}",
                            draggable: "true",
                            style: "display: flex; flex-direction: column; align-items: center; flex: 1; cursor: {cursor}; min-width: 0; opacity: {opacity}; transform: {transform}; transition: transform 0.15s, opacity 0.15s;",
                            ondragstart: move |_| {
                                drag_idx.set(Some(idx));
                                just_dragged.set(true);
                            },
                            ondragover: move |e| {
                                e.prevent_default();
                                drag_over_idx.set(Some(idx));
                            },
                            ondragleave: move |_| {
                                if drag_over_idx() == Some(idx) {
                                    drag_over_idx.set(None);
                                }
                            },
                            ondrop: move |_| {
                                if let Some(from) = drag_idx() {
                                    if from != idx {
                                        items.with_mut(|v| {
                                            let item = v.remove(from);
                                            v.insert(idx, item);
                                        });
                                    }
                                }
                                drag_idx.set(None);
                                drag_over_idx.set(None);
                            },
                            ondragend: move |_| {
                                drag_idx.set(None);
                                drag_over_idx.set(None);
                                // Laisse le flag actif le temps que le click éventuel soit ignoré
                                spawn(async move {
                                    gloo_timers::future::TimeoutFuture::new(50).await;
                                    just_dragged.set(false);
                                });
                            },
                            onclick: move |_| {
                                if !just_dragged() {
                                    on_select.call(tenant_clone.clone());
                                }
                            },
                            div {
                                style: "width: 100%; height: 80px; background: #0f172a; border: 1px solid {border_col}; border-radius: 4px; display: flex; align-items: flex-end; overflow: hidden;",
                                if fill_pct > 0.0 {
                                    div {
                                        style: "width: 100%; height: {fill_pct}%; background: {color}; transition: height 0.3s, background 0.3s;"
                                    }
                                }
                            }
                            span {
                                style: "font-size: 0.7rem; margin-top: 4px; color: #94a3b8; text-align: center; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 100%;",
                                "{t.name}"
                            }
                            span {
                                style: "font-size: 0.7rem; color: {color}; font-weight: 600;",
                                "{label}"
                            }
                        }
                    }
                }
            }
        }
    }
}