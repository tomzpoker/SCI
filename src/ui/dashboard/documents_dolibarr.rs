use chrono::{DateTime, Utc};
use dioxus::prelude::*;

use crate::dolibarr::models::{DolibarrDocument, DolibarrInvoice, DolibarrThirdParty};
use crate::server::{list_properties, list_units};

#[derive(Clone, Copy, PartialEq)]
enum DocTarget {
    Tiers,
    FactureClient,
    FactureFournisseur,
    Patrimoine,
}

impl DocTarget {
    fn label(&self) -> &'static str {
        match self {
            DocTarget::Tiers => "Tiers",
            DocTarget::FactureClient => "Facture client",
            DocTarget::FactureFournisseur => "Facture fournisseur",
            DocTarget::Patrimoine => "Patrimoine",
        }
    }

    fn modulepart(&self) -> &'static str {
        match self {
            DocTarget::Tiers => "societe",
            DocTarget::FactureClient => "facture",
            DocTarget::FactureFournisseur => "facture_fournisseur",
            DocTarget::Patrimoine => "societe",
        }
    }

    fn help(&self) -> &'static str {
        match self {
            DocTarget::Tiers => "Bail signe, etat des lieux, piece locataire, contrat fournisseur.",
            DocTarget::FactureClient => "PDF de ta facture client emise (retour signe).",
            DocTarget::FactureFournisseur => "Facture recue : EDF, assurance, taxe fonciere.",
            DocTarget::Patrimoine => "Plans, photos, DPE, actes notaries, diagnostics, surfaces.",
        }
    }
}

fn format_size(bytes: i64) -> String {
    if bytes < 1024 { format!("{} o", bytes) }
    else if bytes < 1024 * 1024 { format!("{:.1} Ko", bytes as f64 / 1024.0) }
    else { format!("{:.1} Mo", bytes as f64 / (1024.0 * 1024.0)) }
}

fn format_date(ts: i64) -> String {
    match DateTime::<Utc>::from_timestamp(ts, 0) {
        Some(dt) => dt.format("%d/%m/%Y %H:%M").to_string(),
        None => "-".to_string(),
    }
}

fn file_icon(mime: &str, filename: &str) -> &'static str {
    let lower = filename.to_lowercase();
    if mime.contains("pdf") || lower.ends_with(".pdf") { "PDF" }
    else if mime.contains("image") || lower.ends_with(".png") || lower.ends_with(".jpg") { "IMG" }
    else if lower.ends_with(".doc") || lower.ends_with(".odt") { "DOC" }
    else if lower.ends_with(".xls") || lower.ends_with(".ods") { "XLS" }
    else { "FILE" }
}

#[component]
pub fn DocumentsDolibarrPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);

    let thirds = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { crate::dolibarr::server_fns::dolibarr_list_third_parties(500).await }
    });

    let invoices = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { crate::dolibarr::server_fns::dolibarr_list_invoices(500).await }
    });

    let properties = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_properties().await.unwrap_or_default() }
    });

    let units = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_units().await.unwrap_or_default() }
    });

    let mut target = use_signal(|| DocTarget::Tiers);
    let mut selected_id = use_signal(String::new);
    let mut selected_ref = use_signal(String::new);
    let mut selected_bien = use_signal(String::new);
    let mut selected_lot = use_signal(String::new);
    let mut docs = use_signal(Vec::<DolibarrDocument>::new);
    let mut busy = use_signal(|| false);

    // Message unifie : (texte, type)
    let mut flash = use_signal(|| None::<(String, String)>);

    let mut file_name = use_signal(String::new);
    let mut file_bytes = use_signal(Vec::<u8>::new);

    let dl_url = "http://localhost:8081";

    // Helper interne pour setter un flash de succes ou erreur
    let mut set_flash_ok = move |txt: String| flash.set(Some((txt, "success".to_string())));
    let mut set_flash_err = move |txt: String| flash.set(Some((txt, "error".to_string())));

    rsx! {
        section { class: "page-intro",
            div {
                div { class: "eyebrow", "DOCUMENTS / GED DOLIBARR" }
                h2 { "Documents" }
                p { "Depose et classe tes documents dans la GED Dolibarr." }
            }
        }

        // Bandeau d'aide
        section { class: "panel",
            style: "background: rgba(124,58,237,0.06); border-color: rgba(124,58,237,0.25);",
            div { style: "font-size: 0.78rem; color: #cbd5e1; line-height: 1.6;",
                div { style: "margin-bottom: 4px;",
                    span { style: "color: #a78bfa; font-weight: 600;", "Quel onglet choisir ?" }
                }
                div { style: "margin-bottom: 3px;",
                    span { style: "color: #94a3b8;", "Facture recue (EDF, assurance, taxe) ? " }
                    span { style: "color: #e2e8f0;", "-> Facture fournisseur" }
                }
                div { style: "margin-bottom: 3px;",
                    span { style: "color: #94a3b8;", "Bail, etat des lieux, piece locataire ? " }
                    span { style: "color: #e2e8f0;", "-> Tiers" }
                }
                div { style: "margin-bottom: 3px;",
                    span { style: "color: #94a3b8;", "Plans, photos, DPE, acte du batiment ? " }
                    span { style: "color: #e2e8f0;", "-> Patrimoine" }
                }
                div {
                    span { style: "color: #94a3b8;", "PDF de ta facture client emise ? " }
                    span { style: "color: #e2e8f0;", "-> Facture client" }
                }
            }
        }

        // Flash unique en haut
        if let Some((txt, kind)) = flash() {
            section { class: "panel",
                style: if kind == "error" {
                    "background: rgba(248,113,113,0.08); border-color: rgba(248,113,113,0.25);"
                } else {
                    "background: rgba(74,222,128,0.08); border-color: rgba(74,222,128,0.25);"
                },
                div { style: if kind == "error" {
                        "color: #f87171; font-size: 0.82rem;"
                    } else {
                        "color: #4ade80; font-size: 0.82rem;"
                    },
                    "{txt}"
                }
                div { style: "margin-top: 6px;",
                    button {
                        style: "padding: 3px 10px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 4px; cursor: pointer; font-size: 0.7rem;",
                        onclick: move |_| flash.set(None),
                        "Fermer"
                    }
                }
            }
        }

        // Onglets
        section { class: "panel",
            div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px;",
                "Etape 1 : a quoi rattacher le document ?"
            }
            div { style: "display: flex; gap: 8px; margin-bottom: 12px; flex-wrap: wrap;",
                for t in [DocTarget::Tiers, DocTarget::FactureClient, DocTarget::FactureFournisseur, DocTarget::Patrimoine] {
                    button {
                        style: if target() == t {
                            "padding: 6px 14px; background: #7c3aed; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.78rem; font-weight: 600;"
                        } else {
                            "padding: 6px 14px; background: transparent; color: #94a3b8; border: 1px solid var(--border); border-radius: 6px; cursor: pointer; font-size: 0.78rem;"
                        },
                        onclick: move |_| {
                            target.set(t);
                            selected_id.set(String::new());
                            selected_ref.set(String::new());
                            selected_bien.set(String::new());
                            selected_lot.set(String::new());
                            docs.set(Vec::new());
                            flash.set(None);
                        },
                        "{t.label()}"
                    }
                }
            }

            div { style: "font-size: 0.72rem; color: #64748b; margin-bottom: 12px; font-style: italic;",
                "{target().help()}"
            }

            // === Tiers ===
            if target() == DocTarget::Tiers {
                label { class: "field",
                    span { "Selectionne un tiers (client ou fournisseur)" }
                    select {
                        value: "{selected_id}",
                        onchange: move |e| {
                            let val = e.value();
                            selected_id.set(val.clone());
                            let list: Vec<DolibarrThirdParty> = thirds.read().as_ref().and_then(|r| r.as_ref().ok()).cloned().unwrap_or_default();
                            if let Some(t) = list.iter().find(|t| t.id == val) {
                                selected_ref.set(t.name.clone());
                            }
                            docs.set(Vec::new());
                        },
                        option { value: "", "-- Choisir --" }
                        for t in thirds.read().as_ref().and_then(|r| r.as_ref().ok()).cloned().unwrap_or_default().iter() {
                            option { value: "{t.id}", "{t.name}" }
                        }
                    }
                }
            }

            // === Facture client ===
            if target() == DocTarget::FactureClient {
                label { class: "field",
                    span { "Selectionne une facture client" }
                    select {
                        value: "{selected_id}",
                        onchange: move |e| {
                            let val = e.value();
                            selected_id.set(val.clone());
                            let list: Vec<DolibarrInvoice> = invoices.read().as_ref().and_then(|r| r.as_ref().ok()).cloned().unwrap_or_default();
                            if let Some(i) = list.iter().find(|i| i.id == val) {
                                selected_ref.set(i.r#ref.clone());
                            }
                            docs.set(Vec::new());
                        },
                        option { value: "", "-- Choisir --" }
                        for i in invoices.read().as_ref().and_then(|r| r.as_ref().ok()).cloned().unwrap_or_default().iter() {
                            option { value: "{i.id}", "{i.r#ref}" }
                        }
                    }
                }
            }

            // === Facture fournisseur ===
            if target() == DocTarget::FactureFournisseur {
                div { style: "padding: 14px; text-align: center; color: #64748b; font-size: 0.8rem; background: var(--bg-input); border: 1px dashed var(--border-strong); border-radius: 8px;",
                    "Module Fournisseurs non encore connecte. A configurer dans un prochain sprint."
                }
            }

            // === Patrimoine ===
            if target() == DocTarget::Patrimoine {
                div { style: "display: flex; flex-direction: column; gap: 12px;",
                    label { class: "field",
                        span { "Selectionne un bien" }
                        select {
                            value: "{selected_bien}",
                            onchange: move |e| {
                                selected_bien.set(e.value());
                                selected_lot.set(String::new());
                                docs.set(Vec::new());
                            },
                            option { value: "", "-- Choisir un bien --" }
                            for p in properties.read().as_deref().unwrap_or(&[]).iter() {
                                option { value: "{p.name}", "{p.name}" }
                            }
                        }
                    }

                    if !selected_bien().is_empty() {
                        label { class: "field",
                            span { "Lot (optionnel)" }
                            select {
                                value: "{selected_lot}",
                                onchange: move |e| selected_lot.set(e.value()),
                                option { value: "", "-- Tout le bien (aucun lot specifique) --" }
                                for u in units.read().as_deref().unwrap_or(&[]).iter().filter(|u| u.property_name == selected_bien()) {
                                    option { value: "{u.code}", {format!("{} - {}", u.code, u.label)} }
                                }
                            }
                        }
                    }

                    if !selected_bien().is_empty() {
                        div { style: "padding: 8px 12px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px; font-size: 0.72rem; color: #94a3b8; font-family: monospace;",
                            "Range dans : /var/www/documents/societe/Patrimoine_SCI/",
                            "{selected_bien().replace(' ', \"_\")}",
                            if !selected_lot().is_empty() {
                                {format!("/{}", selected_lot())}
                            }
                        }
                    }

                    if !selected_bien().is_empty() {
                        div { style: "display: flex; gap: 8px; flex-wrap: wrap;",
                            button {
                                style: "padding: 6px 12px; background: transparent; color: #a78bfa; border: 1px solid #7c3aed; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;",
                                disabled: busy(),
                                onclick: {
                                    let bname = selected_bien();
                                    move |_| {
                                        let b = bname.clone();
                                        busy.set(true);
                                        spawn(async move {
                                            match crate::dolibarr::server_fns::dolibarr_list_patrimoine_documents(b).await {
                                                Ok(list) => {
                                                    let n = list.len();
                                                    docs.set(list);
                                                    if n == 0 {
                                                        set_flash_err("Aucun document trouve pour ce bien.".to_string());
                                                    } else {
                                                        set_flash_ok(format!("{} document(s) trouve(s).", n));
                                                    }
                                                }
                                                Err(e) => {
                                                    docs.set(Vec::new());
                                                    set_flash_err(format!("Erreur : {}", e));
                                                }
                                            }
                                            busy.set(false);
                                        });
                                    }
                                },
                                if busy() { "Chargement..." } else { "Voir les documents existants" }
                            }
                            button {
                                style: "padding: 6px 12px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.75rem;",
                                onclick: move |_| {
                                    let script = "window.open('http://localhost:8081/societe/list.php', '_blank');";
                                    let _ = document::eval(script);
                                },
                                "Ouvrir Dolibarr (Tiers 'Patrimoine SCI')"
                            }
                        }
                    }
                }
            }

            // Bouton "Voir documents" pour les autres onglets
            if !selected_id().is_empty() && target() != DocTarget::Patrimoine {
                div { style: "display: flex; gap: 8px; margin-top: 12px; flex-wrap: wrap;",
                    button {
                        style: "padding: 6px 12px; background: transparent; color: #a78bfa; border: 1px solid #7c3aed; border-radius: 6px; cursor: pointer; font-size: 0.75rem; font-weight: 600;",
                        onclick: {
                            let t = target();
                            let id = selected_id();
                            move |_| {
                                let mp = t.modulepart();
                                let iid = id.clone();
                                busy.set(true);
                                spawn(async move {
                                    match crate::dolibarr::server_fns::dolibarr_list_documents(mp.to_string(), iid).await {
                                        Ok(list) => {
                                            let n = list.len();
                                            docs.set(list);
                                            if n == 0 {
                                                set_flash_err("Aucun document trouve.".to_string());
                                            } else {
                                                set_flash_ok(format!("{} document(s) trouve(s).", n));
                                            }
                                        }
                                        Err(e) => {
                                            docs.set(Vec::new());
                                            set_flash_err(format!("Erreur : {}", e));
                                        }
                                    }
                                    busy.set(false);
                                });
                            }
                        },
                        if busy() { "Chargement..." } else { "Voir les documents existants" }
                    }
                    {
                        let t = target();
                        let id = selected_id();
                        let subpath = match t {
                            DocTarget::Tiers | DocTarget::Patrimoine => "societe",
                            DocTarget::FactureClient => "compta/facture",
                            DocTarget::FactureFournisseur => "fournisseur/facture",
                        };
                        let url = format!("{}/{}/card.php?id={}", dl_url, subpath, id);
                        rsx! {
                            button {
                                style: "padding: 6px 12px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.75rem;",
                                onclick: move |_| {
                                    let script = format!("window.open('{}', '_blank');", url);
                                    let _ = document::eval(&script);
                                },
                                "Ouvrir dans Dolibarr"
                            }
                        }
                    }
                }
            }
        }

        // Etape 2 : upload
        {
            let can_upload = (target() == DocTarget::Patrimoine && !selected_bien().is_empty())
                || (target() != DocTarget::Patrimoine && !selected_id().is_empty());
            if can_upload {
                rsx! {
                    section { class: "panel",
                        div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px;",
                            "Etape 2 : depose un document"
                        }

                        div { style: "display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-bottom: 10px;",
                            label {
                                style: "display: inline-block; padding: 8px 16px; background: #7c3aed; color: white; border-radius: 6px; cursor: pointer; font-size: 0.8rem; font-weight: 600;",
                                "Choisir un fichier"
                                input {
                                    r#type: "file",
                                    accept: ".pdf,.png,.jpg,.jpeg,.doc,.docx,.odt,.xls,.xlsx,.ods",
                                    style: "display: none;",
                                    onchange: move |evt: FormEvent| {
                                        let files = evt.files();
                                        if let Some(file) = files.first() {
                                            let name = file.name();
                                            file_name.set(name);
                                            let file = file.clone();
                                            spawn(async move {
                                                match file.read_bytes().await {
                                                    Ok(bytes) => {
                                                        file_bytes.set(bytes.to_vec());
                                                    }
                                                    Err(_) => {
                                                        set_flash_err("Erreur lecture fichier".into());
                                                    }
                                                }
                                            });
                                        }
                                    }
                                }
                            }
                            if !file_name().is_empty() {
                                div { style: "font-size: 0.78rem; color: #4ade80;",
                                    "{file_name()} ({format_size(file_bytes().len() as i64)})"
                                }
                            }
                        }

                        if !file_bytes().is_empty() {
                            div { style: "display: flex; gap: 8px;",
                                button {
                                    class: "primary",
                                    disabled: busy(),
                                    onclick: {
                                        let t = target();
                                        let eid = selected_id();
                                        let eref = selected_ref();
                                        let bien = selected_bien();
                                        let lot = selected_lot();
                                        move |_| {
                                            let mp = t.modulepart().to_string();
                                            let iid = eid.clone();
                                            let iref = eref.clone();
                                            let b = bien.clone();
                                            let l = lot.clone();
                                            let fname = file_name();
                                            let bytes = file_bytes();
                                            busy.set(true);
                                            flash.set(None);
                                            spawn(async move {
                                                let result = if t == DocTarget::Patrimoine {
                                                    crate::dolibarr::server_fns::dolibarr_upload_patrimoine_document(
                                                        b, l, fname.clone(), bytes
                                                    ).await
                                                } else {
                                                    crate::dolibarr::server_fns::dolibarr_upload_document_sql(
                                                        mp, iid, iref, fname.clone(), bytes
                                                    ).await
                                                };
                                                match result {
                                                    Ok(final_name) => {
                                                        let msg = if final_name == fname {
                                                            format!("Fichier '{}' depose avec succes dans la GED.", final_name)
                                                        } else {
                                                            format!("Fichier depose sous le nom '{}' (renomme car un fichier du meme nom existait deja).", final_name)
                                                        };
                                                        set_flash_ok(msg);
                                                        file_name.set(String::new());
                                                        file_bytes.set(Vec::new());
                                                    }
                                                    Err(e) => {
                                                        set_flash_err(format!("Erreur upload : {}", e));
                                                    }
                                                }
                                                busy.set(false);
                                            });
                                        }
                                    },
                                    if busy() { "Upload..." } else { "Deposer dans la GED" }
                                }
                                button {
                                    class: "secondary",
                                    onclick: move |_| {
                                        file_name.set(String::new());
                                        file_bytes.set(Vec::new());
                                    },
                                    "Annuler"
                                }
                            }
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }

        // Liste des documents
        if !docs().is_empty() {
            section { class: "panel",
                div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px;",
                    {format!("{} document(s) dans la GED", docs().len())}
                }
                div { style: "display: flex; flex-direction: column; gap: 6px;",
                    for (idx, d) in docs().iter().enumerate() {
                        {
                            let icon = file_icon(&d.mime, &d.filename);
                            let date = format_date(d.date);
                            let fname = d.filename.clone();
                            let relpath = d.relativename.clone();
                            let key = format!("{}-{}", idx, fname);
                            let fname_del = fname.clone();
                            let rel_del = relpath.clone();
                            let mp_del = d.modulepart.clone();
                            // URL pour ouvrir le document dans Dolibarr
                            let url = format!("{}/document.php?modulepart={}&file={}", dl_url, mp_del, relpath);
                            rsx! {
                                div {
                                    key: "{key}",
                                    style: "background: var(--bg-glass); border: 1px solid var(--border); padding: 10px 14px; border-radius: 8px; display: flex; align-items: center; gap: 12px;",
                                    span {
                                        style: "width: 40px; height: 40px; flex-shrink: 0; border-radius: 8px; display: flex; align-items: center; justify-content: center; background: rgba(148,163,184,0.08); color: #94a3b8; font-size: 0.7rem; font-weight: 700;",
                                        "{icon}"
                                    }
                                    div { style: "flex: 1; min-width: 0;",
                                        div { style: "font-size: 0.84rem; color: #e2e8f0; font-weight: 500;", "{fname}" }
                                        div { style: "font-size: 0.7rem; color: #94a3b8; margin-top: 2px;",
                                            span { "{date}" }
                                            span { style: "color: #334155;", " / " }
                                            span { style: "font-family: monospace;", "{relpath}" }
                                        }
                                    }
                                    div { style: "display: flex; gap: 6px; flex-shrink: 0;",
                                        button {
                                            style: "padding: 5px 10px; background: transparent; border: 1px solid var(--border); color: #a78bfa; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 600;",
                                            onclick: move |_| {
                                                let u = url.clone();
                                                let script = format!("window.open('{}', '_blank');", u);
                                                let _ = document::eval(&script);
                                            },
                                            "Ouvrir"
                                        }
                                        button {
                                            style: "padding: 5px 10px; background: transparent; border: 1px solid var(--border); color: #f87171; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 600;",
                                            disabled: busy(),
                                            onclick: move |_| {
                                                let f = fname_del.clone();
                                                let m = mp_del.clone();
                                                let r = rel_del.clone();
                                                busy.set(true);
                                                flash.set(None);
                                                spawn(async move {
                                                    match crate::dolibarr::server_fns::dolibarr_delete_document(m, r).await {
                                                        Ok(_) => {
                                                            set_flash_ok(format!("Document '{}' supprime.", f));
                                                            docs.with_mut(|list| {
                                                                list.retain(|x| x.filename != f);
                                                            });
                                                        }
                                                        Err(e) => {
                                                            set_flash_err(format!("Erreur suppression : {}", e));
                                                        }
                                                    }
                                                    busy.set(false);
                                                });
                                            },
                                            "Suppr"
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