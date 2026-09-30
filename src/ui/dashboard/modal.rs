use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use crate::relances::{LocalBarItem, LocalStatus, RelancePreview};

fn format_amount(v: f64) -> String {
    format!("{:.2} EUR", v)
}

fn format_date_ts(ts: i64) -> String {
    match DateTime::<Utc>::from_timestamp(ts, 0) {
        Some(dt) => dt.format("%d/%m/%Y").to_string(),
        None => "-".to_string(),
    }
}

/// Retourne le seuil minimum en jours pour chaque niveau.
fn level_threshold(level: i32) -> i32 {
    match level {
        1 => 7,
        2 => 30,
        3 => 60,
        _ => i32::MAX,
    }
}

fn level_label(level: i32) -> &'static str {
    match level {
        1 => "Amiable",
        2 => "Ferme",
        3 => "Mise en demeure",
        _ => "?",
    }
}

fn level_color(level: i32) -> &'static str {
    match level {
        1 => "#38bdf8",
        2 => "#fbbf24",
        3 => "#f87171",
        _ => "#94a3b8",
    }
}

fn level_bg(level: i32) -> &'static str {
    match level {
        1 => "rgba(56,189,248,0.10)",
        2 => "rgba(251,191,36,0.10)",
        3 => "rgba(248,113,113,0.10)",
        _ => "rgba(148,163,184,0.10)",
    }
}

/// Vérifie si un niveau peut être envoyé pour une facture donnée.
fn can_send(level: i32, days_overdue: i32, level_already_sent: i32) -> (bool, &'static str) {
    if level_already_sent >= level {
        return (false, "Déjà envoyée");
    }
    let threshold = level_threshold(level);
    if days_overdue < threshold {
        return (false, "Trop tôt");
    }
    (true, "")
}

#[component]
pub fn LocalModal(
    local: Signal<Option<LocalBarItem>>,
    on_close: EventHandler<()>,
) -> Element {
    let Some(l) = local() else {
        return rsx! { div {} };
    };

    let color = l.status.color();
    let status_label = l.status.label();
    let is_vacant = l.status == LocalStatus::Vacant;
    let is_uptodate = l.status == LocalStatus::UpToDate;
    let has_email = l.tenant_email.as_ref().map(|e| !e.is_empty()).unwrap_or(false);

    // === Preview state ===
    let mut preview_invoice_id = use_signal(|| None::<String>);
    let mut preview_level = use_signal(|| None::<i32>);
    let mut preview_data = use_signal(|| None::<RelancePreview>);
    let mut preview_loading = use_signal(|| false);
    let mut preview_error = use_signal(|| None::<String>);
    let mut sending = use_signal(|| false);
    let mut flash = use_signal(|| None::<(String, String)>);

    // === History state ===
    let mut history_bump = use_signal(|| 0u64);
    let history = use_resource(move || {
        let _ = history_bump();
        async move {
            let Some(l) = local() else { return Vec::new() };
            let ids: Vec<String> = l
                .invoices
                .iter()
                .map(|i| i.dolibarr_invoice_id.clone())
                .collect();
            if ids.is_empty() {
                return Vec::new();
            }
            crate::relances::list_relances_for_invoices(ids)
                .await
                .unwrap_or_default()
        }
    });
    let history_items = (*history.read()).clone().unwrap_or_default();

    // === Rendu ===
    rsx! {
        div {
            class: "dash-modal-overlay",
            onclick: move |_| on_close.call(()),
            div {
                class: "dash-modal",
                onclick: move |e| e.stop_propagation(),
                style: "max-width: 900px; max-height: 88vh; overflow-y: auto;",

                div {
                    style: "height: 4px; background: {color}; margin: -24px -24px 20px -24px; border-radius: 12px 12px 0 0;"
                }

                // Flash message
                if let Some((txt, kind)) = flash() {
                    div {
                        style: if kind == "error" {
                            "margin-bottom: 12px; padding: 10px 14px; background: rgba(248,113,113,0.10); border: 1px solid rgba(248,113,113,0.3); border-radius: 6px; color: #f87171; font-size: 0.8rem;"
                        } else {
                            "margin-bottom: 12px; padding: 10px 14px; background: rgba(74,222,128,0.10); border: 1px solid rgba(74,222,128,0.3); border-radius: 6px; color: #4ade80; font-size: 0.8rem;"
                        },
                        "{txt}"
                    }
                }

                // En-tête
                div { style: "display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 20px;",
                    div {
                        div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600;",
                            "{l.property_name} • {l.unit_code}"
                        }
                        h2 { style: "margin: 4px 0 8px 0; color: #f8fafc;", "{l.unit_label}" }
                        div { style: "display: inline-flex; align-items: center; gap: 6px; padding: 3px 10px; background: {color}20; border: 1px solid {color}66; border-radius: 12px;",
                            span { style: "width: 8px; height: 8px; border-radius: 50%; background: {color};" }
                            span { style: "font-size: 0.7rem; color: {color}; font-weight: 600;", "{status_label}" }
                        }
                    }
                    button {
                        style: "background: transparent; border: none; color: #94a3b8; font-size: 1.5rem; cursor: pointer; line-height: 1;",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }

                // Locataire
                div { style: "padding: 12px 14px; background: #0f172a; border-radius: 8px; margin-bottom: 16px;",
                    div { style: "font-size: 0.7rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em;", "Locataire" }
                    div { style: "font-size: 0.95rem; color: #e2e8f0; font-weight: 500; margin-top: 4px;",
                        {l.tenant_name.clone().unwrap_or_else(|| "Aucun locataire".to_string())}
                    }
                    div { style: "font-size: 0.75rem; color: #94a3b8; margin-top: 2px;",
                        {l.tenant_email.clone().filter(|e| !e.is_empty()).unwrap_or_else(|| "—".to_string())}
                    }
                    if !has_email && !is_vacant {
                        div { style: "margin-top: 6px; font-size: 0.7rem; color: #fbbf24;",
                            "⚠ Aucun email dans Dolibarr — impossible d'envoyer une relance"
                        }
                    }
                }

                if !is_vacant {
                    // Stats
                    div { style: "display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; margin-bottom: 20px;",
                        div { style: "padding: 12px; background: #0f172a; border-radius: 8px; border: 1px solid #1e293b;",
                            div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em;", "Total impayé" }
                            div { style: "font-size: 1.15rem; font-weight: 700; color: {color}; font-variant-numeric: tabular-nums; margin-top: 4px;",
                                {format_amount(l.total_outstanding)}
                            }
                        }
                        div { style: "padding: 12px; background: #0f172a; border-radius: 8px; border: 1px solid #1e293b;",
                            div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em;", "Factures" }
                            div { style: "font-size: 1.15rem; font-weight: 700; color: #e2e8f0; margin-top: 4px;",
                                "{l.invoice_count}"
                            }
                        }
                        div { style: "padding: 12px; background: #0f172a; border-radius: 8px; border: 1px solid #1e293b;",
                            div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em;", "Retard max" }
                            div { style: "font-size: 1.15rem; font-weight: 700; color: {color}; margin-top: 4px;",
                                if l.max_days_overdue > 0 { "J+{l.max_days_overdue}" } else { "—" }
                            }
                        }
                        div { style: "padding: 12px; background: #0f172a; border-radius: 8px; border: 1px solid #1e293b;",
                            div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em;", "Relances" }
                            div { style: "font-size: 0.85rem; font-weight: 600; color: #e2e8f0; margin-top: 6px;",
                                {match l.highest_level_sent {
                                    0 => "Aucune".to_string(),
                                    1 => "Amiable envoyée".to_string(),
                                    2 => "Ferme envoyée".to_string(),
                                    3 => "MED envoyée".to_string(),
                                    _ => "?".to_string(),
                                }}
                            }
                        }
                    }

                    // Factures avec actions
                    if !l.invoices.is_empty() {
                        div { style: "margin-bottom: 20px;",
                            h4 { style: "color: #94a3b8; margin: 0 0 10px 0; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em;",
                                "Factures impayées et relances"
                            }
                            div { style: "display: flex; flex-direction: column; gap: 8px;",
                                for inv in l.invoices.iter() {
                                    {
                                        let days = inv.days_overdue;
                                        let row_color = match days {
                                            d if d <= 0 => "#64748b",
                                            d if d <= 30 => "#f59e0b",
                                            d if d <= 60 => "#ef4444",
                                            _ => "#dc2626",
                                        };
                                        let inv_ref = inv.invoice_ref.clone();
                                        let issue = format_date_ts(inv.issue_date_ts);
                                        let due = format_date_ts(inv.due_date_ts);
                                        let total = format_amount(inv.total_ttc);
                                        let outstanding = format_amount(inv.outstanding);
                                        let key = inv.dolibarr_invoice_id.clone();
                                        let inv_id_for_buttons = inv.dolibarr_invoice_id.clone();
                                        let level_sent = inv.relance_level_sent;

                                        rsx! {
                                            div {
                                                key: "{key}",
                                                style: "background: var(--bg-glass); border: 1px solid var(--border); border-left: 3px solid {row_color}; padding: 12px 14px; border-radius: 8px;",
                                                div { style: "display: flex; align-items: center; gap: 12px; margin-bottom: 10px;",
                                                    div { style: "flex: 1; min-width: 0;",
                                                        div { style: "font-size: 0.85rem; color: #e2e8f0; font-weight: 500;", "Facture {inv_ref}" }
                                                        div { style: "font-size: 0.7rem; color: #94a3b8; margin-top: 2px;",
                                                            "Émise le {issue} • Échéance {due}"
                                                        }
                                                    }
                                                    div { style: "text-align: right; flex-shrink: 0;",
                                                        div { style: "font-size: 0.88rem; font-weight: 600; color: {row_color}; font-variant-numeric: tabular-nums;",
                                                            "{outstanding}"
                                                        }
                                                        div { style: "font-size: 0.68rem; color: #64748b;", "sur {total} • J+{days}" }
                                                    }
                                                }
                                                // Boutons de relance
                                                div { style: "display: flex; gap: 6px; flex-wrap: wrap;",
                                                    for level in [1, 2, 3] {
                                                        {
                                                            let (enabled, reason) = can_send(level, days, level_sent);
                                                            let enabled = enabled && has_email;
                                                            let lvl_color = level_color(level);
                                                            let lvl_bg = level_bg(level);
                                                            let lvl_label = level_label(level);
                                                            let inv_id = inv_id_for_buttons.clone();
                                                            let tooltip = if !has_email {
                                                                "Aucun email client"
                                                            } else { reason };
                                                            rsx! {
                                                                button {
                                                                    key: "{level}",
                                                                    disabled: !enabled,
                                                                    title: "{tooltip}",
                                                                    style: if enabled {
                                                                        format!(
                                                                            "padding: 5px 12px; background: {}; color: {}; border: 1px solid {}; border-radius: 6px; font-size: 0.72rem; font-weight: 600; cursor: pointer;",
                                                                            lvl_bg, lvl_color, lvl_color
                                                                        )
                                                                    } else {
                                                                        "padding: 5px 12px; background: rgba(148,163,184,0.05); color: #475569; border: 1px solid #334155; border-radius: 6px; font-size: 0.72rem; cursor: not-allowed;".to_string()
                                                                    },
                                                                    onclick: move |_| {
                                                                        let id = inv_id.clone();
                                                                        preview_invoice_id.set(Some(id.clone()));
                                                                        preview_level.set(Some(level));
                                                                        preview_data.set(None);
                                                                        preview_error.set(None);
                                                                        preview_loading.set(true);
                                                                        flash.set(None);
                                                                        spawn(async move {
                                                                            match crate::relances::preview_invoice_relance(id, level).await {
                                                                                Ok(p) => preview_data.set(Some(p)),
                                                                                Err(e) => preview_error.set(Some(e.to_string())),
                                                                            }
                                                                            preview_loading.set(false);
                                                                        });
                                                                    },
                                                                    "{lvl_label}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                    if level_sent > 0 {
                                                        div { style: "margin-left: auto; font-size: 0.68rem; color: #64748b; align-self: center;",
                                                            {format!("Dernière : {}", level_label(level_sent))}
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if is_uptodate {
                        div { style: "padding: 14px; text-align: center; color: #22c55e; font-size: 0.82rem; background: rgba(34,197,94,0.08); border: 1px solid rgba(34,197,94,0.25); border-radius: 8px; margin-bottom: 20px;",
                            "✓ Ce locataire est à jour de ses paiements."
                        }
                    }

                    // Historique
                    if !history_items.is_empty() {
                        div { style: "margin-bottom: 20px;",
                            h4 { style: "color: #94a3b8; margin: 0 0 10px 0; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em;",
                                "Historique des relances"
                            }
                            div { style: "display: flex; flex-direction: column; gap: 4px;",
                                for h in history_items.iter() {
                                    {
                                        let h_color = level_color(h.level);
                                        let h_label = level_label(h.level);
                                        let date_str = format_date_ts(h.sent_at_ts);
                                        let key = h.id.clone();
                                        let via = if h.sent_via == "dolibarr" { "Dolibarr" } else { "Gmail SMTP" };
                                        let is_failed = h.status == "failed";
                                        rsx! {
                                            div {
                                                key: "{key}",
                                                style: "display: flex; align-items: center; gap: 10px; padding: 6px 10px; background: var(--bg-input); border-radius: 6px; font-size: 0.72rem;",
                                                span {
                                                    style: "width: 6px; height: 6px; border-radius: 50%; background: {h_color}; flex-shrink: 0;"
                                                }
                                                span { style: "color: #e2e8f0; font-weight: 500;", "{h_label}" }
                                                span { style: "color: #64748b;", "•" }
                                                span { style: "color: #94a3b8;", "{h.invoice_ref}" }
                                                span { style: "color: #64748b;", "•" }
                                                span { style: "color: #94a3b8;", "{date_str}" }
                                                span { style: "color: #64748b; margin-left: auto;", "{via}" }
                                                if is_failed {
                                                    span { style: "color: #f87171;", "✗" }
                                                } else {
                                                    span { style: "color: #4ade80;", "✓" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Boutons pied
                div { style: "display: flex; gap: 8px; justify-content: flex-end;",
                    button {
                        class: "secondary",
                        onclick: move |_| on_close.call(()),
                        "Fermer"
                    }
                }

                // ============================================================
                //  PREVIEW MODAL (overlay nested)
                // ============================================================
                if let Some(_inv_id) = preview_invoice_id() {
                    div {
                        style: "position: fixed; inset: 0; background: rgba(0,0,0,0.7); display: flex; align-items: center; justify-content: center; z-index: 10000; padding: 20px;",
                        onclick: move |_| {
                            preview_invoice_id.set(None);
                            preview_level.set(None);
                            preview_data.set(None);
                            preview_error.set(None);
                        },
                        div {
                            style: "background: #1e293b; border: 1px solid #334155; border-radius: 12px; max-width: 780px; width: 100%; max-height: 85vh; overflow-y: auto; padding: 24px;",
                            onclick: move |e| e.stop_propagation(),

                            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                                div {
                                    div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em;", "Prévisualisation" }
                                    h3 { style: "margin: 4px 0 0 0; color: #f8fafc;",
                                        {preview_level().map(level_label).unwrap_or("—")}
                                    }
                                }
                                button {
                                    style: "background: transparent; border: none; color: #94a3b8; font-size: 1.5rem; cursor: pointer; line-height: 1;",
                                    onclick: move |_| {
                                        preview_invoice_id.set(None);
                                        preview_level.set(None);
                                        preview_data.set(None);
                                        preview_error.set(None);
                                    },
                                    "×"
                                }
                            }

                            if preview_loading() {
                                div { style: "padding: 20px; text-align: center; color: #94a3b8; font-size: 0.85rem;", "Génération de l'aperçu..." }
                            } else if let Some(err) = preview_error() {
                                div { style: "padding: 14px; background: rgba(248,113,113,0.10); border: 1px solid rgba(248,113,113,0.3); border-radius: 6px; color: #f87171; font-size: 0.8rem;",
                                    "Erreur : {err}"
                                }
                            } else if let Some(p) = preview_data() {
                                div {
                                    div { style: "padding: 10px 14px; background: #0f172a; border-radius: 6px; margin-bottom: 12px; font-size: 0.78rem;",
                                        div { style: "color: #64748b; font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em;", "Destinataire" }
                                        div { style: "color: #e2e8f0; margin-top: 2px;", "{p.recipient_email}" }
                                    }
                                    div { style: "padding: 10px 14px; background: #0f172a; border-radius: 6px; margin-bottom: 12px; font-size: 0.78rem;",
                                        div { style: "color: #64748b; font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em;", "Objet" }
                                        div { style: "color: #e2e8f0; margin-top: 2px; font-weight: 500;", "{p.subject}" }
                                    }
                                    div { style: "padding: 14px; background: #0f172a; border-radius: 6px; margin-bottom: 16px; font-size: 0.78rem;",
                                        div { style: "color: #64748b; font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;", "Corps" }
                                        pre {
                                            style: "margin: 0; white-space: pre-wrap; font-family: inherit; color: #cbd5e1; font-size: 0.78rem; line-height: 1.5;",
                                            "{p.body}"
                                        }
                                    }
                                    div { style: "display: flex; gap: 8px; justify-content: flex-end;",
                                        button {
                                            class: "secondary",
                                            disabled: sending(),
                                            onclick: move |_| {
                                                preview_invoice_id.set(None);
                                                preview_level.set(None);
                                                preview_data.set(None);
                                            },
                                            "Annuler"
                                        }
                                        button {
                                            style: "padding: 8px 18px; background: #7c3aed; color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: 600; font-size: 0.82rem;",
                                            disabled: sending(),
                                            onclick: move |_| {
                                                let Some(inv_id) = preview_invoice_id() else { return };
                                                let Some(level) = preview_level() else { return };
                                                let Some(p) = preview_data() else { return };
                                                let recipient = p.recipient_email.clone();
                                                sending.set(true);
                                                flash.set(None);
                                                spawn(async move {
                                                    match crate::relances::send_invoice_relance(
                                                        inv_id, level, Some(recipient)
                                                    ).await {
                                                        Ok(msg) => {
                                                            flash.set(Some((msg, "success".into())));
                                                            preview_invoice_id.set(None);
                                                            preview_level.set(None);
                                                            preview_data.set(None);
                                                            let b = history_bump();
                                                            history_bump.set(b + 1);
                                                        }
                                                        Err(e) => {
                                                            flash.set(Some((format!("Erreur envoi : {}", e), "error".into())));
                                                        }
                                                    }
                                                    sending.set(false);
                                                });
                                            },
                                            if sending() { "Envoi..." } else { "Envoyer" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}