use dioxus::prelude::*;

use crate::dolibarr::models::DolibarrThirdParty;

fn display_name(t: &DolibarrThirdParty) -> String {
    if t.name.trim().is_empty() {
        format!("Tiers #{}", t.id)
    } else {
        t.name.clone()
    }
}

fn display_contact(t: &DolibarrThirdParty) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !t.email.trim().is_empty() {
        parts.push(t.email.clone());
    }
    if !t.phone.trim().is_empty() {
        parts.push(t.phone.clone());
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(" / ")
    }
}

fn display_address(t: &DolibarrThirdParty) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !t.address.trim().is_empty() { parts.push(t.address.clone()); }
    let cp_ville = format!("{} {}", t.zip.trim(), t.town.trim()).trim().to_string();
    if !cp_ville.is_empty() { parts.push(cp_ville); }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(", ")
    }
}

fn is_client(t: &DolibarrThirdParty) -> bool {
    let c = t.client.trim();
    !c.is_empty() && c != "0"
}

fn is_supplier(t: &DolibarrThirdParty) -> bool {
    let f = t.fournisseur.trim();
    !f.is_empty() && f != "0"
}

fn border_color(t: &DolibarrThirdParty) -> &'static str {
    if is_client(t) {
        "#3b82f6"
    } else if is_supplier(t) {
        "#f59e0b"
    } else {
        "#64748b"
    }
}

#[derive(Clone, PartialEq)]
struct ThirdPartyDraft {
    id: Option<String>,
    name: String,
    email: String,
    phone: String,
    address: String,
    zip: String,
    town: String,
}

impl ThirdPartyDraft {
    fn empty() -> Self {
        Self {
            id: None,
            name: String::new(),
            email: String::new(),
            phone: String::new(),
            address: String::new(),
            zip: String::new(),
            town: String::new(),
        }
    }

    fn from(t: &DolibarrThirdParty) -> Self {
        Self {
            id: Some(t.id.clone()),
            name: t.name.clone(),
            email: t.email.clone(),
            phone: t.phone.clone(),
            address: t.address.clone(),
            zip: t.zip.clone(),
            town: t.town.clone(),
        }
    }
}

#[component]
pub fn ThirdPartiesDolibarrPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);

    let thirds = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { crate::dolibarr::server_fns::dolibarr_list_third_parties(500).await }
    });

    let mut search = use_signal(String::new);
    let mut show_edit = use_signal(|| false);
    let mut draft = use_signal(ThirdPartyDraft::empty);
    let mut form_msg = use_signal(String::new);
    let mut busy = use_signal(|| false);

    let dl_url = "http://localhost:8081";

    rsx! {
        section { class: "page-intro",
            div {
                div { class: "eyebrow", "TIERS / DOLIBARR" }
                h2 { "Tiers" }
                p { "Tiers (clients et fournisseurs) lus et crees dans Dolibarr." }
            }
        }

        section { class: "panel",
            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap;",
                div { style: "flex: 1; min-width: 220px;",
                    input {
                        r#type: "text",
                        placeholder: "Rechercher un tiers...",
                        value: "{search}",
                        oninput: move |e| search.set(e.value()),
                        style: "width: 100%; padding: 7px 12px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px; color: #e2e8f0; font-size: 0.82rem;",
                    }
                }
                div { style: "display: flex; gap: 8px;",
                    button {
                        style: "padding: 5px 12px; background: #7c3aed; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;",
                        onclick: move |_| {
                            draft.set(ThirdPartyDraft::empty());
                            form_msg.set(String::new());
                            show_edit.set(true);
                        },
                        "+ Nouveau tiers"
                    }
                    button {
                        style: "padding: 5px 12px; background: transparent; color: #a78bfa; border: 1px solid #7c3aed; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;",
                        onclick: move |_| {
                            let _ = document::eval("window.open('http://localhost:8081/societe/list.php', '_blank');");
                        },
                        "Ouvrir Dolibarr"
                    }
                }
            }
        }

        match &*thirds.read() {
            Some(Ok(list)) => {
                let q = search().to_lowercase();
                let filtered: Vec<DolibarrThirdParty> = list
                    .iter()
                    .filter(|t| {
                        if q.is_empty() { return true; }
                        t.name.to_lowercase().contains(&q)
                            || t.email.to_lowercase().contains(&q)
                            || t.town.to_lowercase().contains(&q)
                            || t.code_client.to_lowercase().contains(&q)
                    })
                    .cloned()
                    .collect();

                let total = filtered.len();

                rsx! {
                    section { class: "panel",
                        div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;",
                            span { class: "small", {format!("{} tiers", total)} }
                        }

                        if filtered.is_empty() {
                            div { class: "empty-state",
                                h3 { "Aucun tiers" }
                                p { "Aucun tiers ne correspond a ta recherche." }
                            }
                        } else {
                            div { style: "display: flex; flex-direction: column; gap: 6px;",
                                for t in filtered.into_iter() {
                                    {
                                        let name = display_name(&t);
                                        let contact = display_contact(&t);
                                        let address = display_address(&t);
                                        let is_c = is_client(&t);
                                        let is_f = is_supplier(&t);
                                        let border = border_color(&t);
                                        let code_client = if t.code_client.trim().is_empty() { "-".to_string() } else { t.code_client.clone() };
                                        let url = format!("{}/societe/card.php?socid={}", dl_url, t.id);
                                        let url_for_click = url.clone();
                                        let id_key = t.id.clone();
                                        let draft_for_edit = ThirdPartyDraft::from(&t);
                                        rsx! {
                                            div {
                                                key: "{id_key}",
                                                style: "background: var(--bg-glass); border: 1px solid var(--border); border-left: 3px solid {border}; padding: 10px 14px; border-radius: 10px; display: flex; align-items: center; gap: 12px;",
                                                div { style: "flex: 1; min-width: 0;",
                                                    div { style: "font-size: 0.88rem; color: #e2e8f0; font-weight: 500;",
                                                        "{name}"
                                                    }
                                                    div { style: "font-size: 0.72rem; color: #94a3b8; margin-top: 2px;",
                                                        span { "{contact}" }
                                                    }
                                                    div { style: "font-size: 0.7rem; color: #64748b; margin-top: 2px;",
                                                        span { "{address}" }
                                                    }
                                                }
                                                div { style: "text-align: right; flex-shrink: 0;",
                                                    if is_c {
                                                        div { style: "font-size: 0.65rem; color: #3b82f6; font-weight: 600; text-transform: uppercase;", "Client" }
                                                    }
                                                    if is_f {
                                                        div { style: "font-size: 0.65rem; color: #f59e0b; font-weight: 600; text-transform: uppercase;", "Fournisseur" }
                                                    }
                                                    div { style: "font-size: 0.68rem; color: #64748b; margin-top: 2px;",
                                                        "Code {code_client}"
                                                    }
                                                }
                                                div { style: "display: flex; gap: 4px; flex-shrink: 0;",
                                                    button {
                                                        style: "padding: 5px 10px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 600;",
                                                        onclick: move |_| {
                                                            let u = url_for_click.clone();
                                                            let script = format!("window.open('{}', '_blank');", u);
                                                            let _ = document::eval(&script);
                                                        },
                                                        "Voir"
                                                    }
                                                    button {
                                                        style: "padding: 5px 10px; background: transparent; border: 1px solid var(--border); color: #a78bfa; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 600;",
                                                        onclick: move |_| {
                                                            draft.set(draft_for_edit.clone());
                                                            form_msg.set(String::new());
                                                            show_edit.set(true);
                                                        },
                                                        "Modifier"
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
            },
            Some(Err(e)) => rsx! {
                section { class: "panel",
                    div { style: "color: #f87171;", {format!("Erreur : {e}")} }
                }
            },
            None => rsx! {
                section { class: "panel",
                    div { style: "color: #94a3b8;", "Chargement des tiers..." }
                }
            },
        }

        if show_edit() {
            div {
                class: "dash-modal-overlay",
                onclick: move |_| show_edit.set(false),
                div {
                    class: "dash-modal",
                    style: "max-width: 560px;",
                    onclick: move |e| e.stop_propagation(),

                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                        div {
                            div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em;",
                                if draft().id.is_some() { "Modifier le tiers" } else { "Nouveau tiers" }
                            }
                            h2 { style: "margin: 4px 0 0 0;",
                                if draft().id.is_some() { "Modification dans Dolibarr" } else { "Creation dans Dolibarr" }
                            }
                        }
                        button {
                            style: "background: transparent; border: none; color: #94a3b8; font-size: 1.5rem; cursor: pointer; line-height: 1;",
                            onclick: move |_| show_edit.set(false),
                            "x"
                        }
                    }

                    if !form_msg().is_empty() {
                        div { style: "background: #450a0a; border: 1px solid #7f1d1d; color: #fecaca; padding: 8px 12px; border-radius: 4px; font-size: 0.8rem; margin-bottom: 12px;",
                            "{form_msg()}"
                        }
                    }

                    div { style: "display: flex; flex-direction: column; gap: 12px;",
                        label { class: "field",
                            span { "Nom / Raison sociale" }
                            input {
                                r#type: "text",
                                value: "{draft().name}",
                                oninput: move |e| draft.with_mut(|d| d.name = e.value()),
                            }
                        }
                        div { style: "display: flex; gap: 8px;",
                            div { style: "flex: 1;",
                                label { class: "field",
                                    span { "Email" }
                                    input {
                                        r#type: "text",
                                        value: "{draft().email}",
                                        oninput: move |e| draft.with_mut(|d| d.email = e.value()),
                                    }
                                }
                            }
                            div { style: "flex: 1;",
                                label { class: "field",
                                    span { "Telephone" }
                                    input {
                                        r#type: "text",
                                        value: "{draft().phone}",
                                        oninput: move |e| draft.with_mut(|d| d.phone = e.value()),
                                    }
                                }
                            }
                        }
                        label { class: "field",
                            span { "Adresse" }
                            input {
                                r#type: "text",
                                value: "{draft().address}",
                                oninput: move |e| draft.with_mut(|d| d.address = e.value()),
                            }
                        }
                        div { style: "display: flex; gap: 8px;",
                            div { style: "flex: 1;",
                                label { class: "field",
                                    span { "Code postal" }
                                    input {
                                        r#type: "text",
                                        value: "{draft().zip}",
                                        oninput: move |e| draft.with_mut(|d| d.zip = e.value()),
                                    }
                                }
                            }
                            div { style: "flex: 2;",
                                label { class: "field",
                                    span { "Ville" }
                                    input {
                                        r#type: "text",
                                        value: "{draft().town}",
                                        oninput: move |e| draft.with_mut(|d| d.town = e.value()),
                                    }
                                }
                            }
                        }
                    }

                    div { style: "display: flex; gap: 8px; margin-top: 20px;",
                        button {
                            class: "primary",
                            style: "flex: 1;",
                            disabled: busy(),
                            onclick: move |_| {
                                let d = draft();
                                let existing_id = d.id.clone();
                                busy.set(true);
                                form_msg.set(String::new());
                                spawn(async move {
                                    if d.name.trim().is_empty() {
                                        form_msg.set("Le nom est obligatoire".into());
                                        busy.set(false);
                                        return;
                                    }
                                    let result = match existing_id {
                                        Some(id) => {
                                            crate::dolibarr::server_fns::dolibarr_update_third_party(
                                                id, d.name, d.email, d.phone, d.address, d.zip, d.town
                                            ).await.map(|_| "Tiers modifie".to_string())
                                        }
                                        None => {
                                            crate::dolibarr::server_fns::dolibarr_create_third_party(
                                                d.name, d.email, d.phone, d.address, d.zip, d.town
                                            ).await.map(|_id| "Tiers cree".to_string())
                                        }
                                    };
                                    match result {
                                        Ok(_) => {
                                            show_edit.set(false);
                                            bump.with_mut(|v| *v += 1);
                                            busy.set(false);
                                        }
                                        Err(e) => {
                                            form_msg.set(format!("{}", e));
                                            busy.set(false);
                                        }
                                    }
                                });
                            },
                            if busy() { "Enregistrement..." } else { "Enregistrer" }
                        }
                        button {
                            class: "secondary",
                            onclick: move |_| show_edit.set(false),
                            "Annuler"
                        }
                    }
                }
            }
        }
    }
}