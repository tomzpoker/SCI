use chrono::{DateTime, Utc};
use dioxus::prelude::*;

use crate::dolibarr::invoice_parser::ParsedInvoice;
use crate::dolibarr::models::DolibarrDocument;
use crate::email::EmailFacture;
use crate::server::{list_properties, list_units};

#[derive(Clone, Copy, PartialEq)]
enum DocMode {
    ScanInvoice,
    UploadDocument,
}

impl DocMode {
    fn label(&self) -> &'static str {
        match self {
            DocMode::ScanInvoice => "Scanner une facture fournisseur",
            DocMode::UploadDocument => "Déposer un document",
        }
    }

    fn help(&self) -> &'static str {
        match self {
            DocMode::ScanInvoice => "Dépose un PDF de facture reçue (EDF, assurance, prestataire). L'app lit la facture et pré-remplit les champs. Tu valides, puis elle est créée dans Dolibarr.",
            DocMode::UploadDocument => "Dépose un bail, un état des lieux, un DPE, une pièce d'identité. Tu choisis à quoi ça se rattache.",
        }
    }
}

#[derive(Clone, PartialEq)]
struct CreatedInvoiceInfo {
    id: String,
    reference: String,
    supplier: String,
    number: String,
    ttc: String,
    bien: String,
    lot: String,
    pdf_filename: String,
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

fn file_icon(filename: &str) -> &'static str {
    let lower = filename.to_lowercase();
    if lower.ends_with(".pdf") { "PDF" }
    else if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") { "IMG" }
    else if lower.ends_with(".doc") || lower.ends_with(".docx") || lower.ends_with(".odt") { "DOC" }
    else if lower.ends_with(".xls") || lower.ends_with(".xlsx") || lower.ends_with(".ods") { "XLS" }
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

    let mut mode = use_signal(|| DocMode::ScanInvoice);
    let mut busy = use_signal(|| false);
    let mut flash = use_signal(|| None::<(String, String)>);
    let mut file_name = use_signal(String::new);
    let mut file_bytes = use_signal(Vec::<u8>::new);
    let mut url_input = use_signal(String::new);

    // Scan invoice
    let mut parsed = use_signal(|| None::<ParsedInvoice>);
    let mut scan_supplier = use_signal(String::new);
    let mut scan_number = use_signal(String::new);
    let mut scan_date = use_signal(String::new);
    let mut scan_due_date = use_signal(String::new);
    let mut scan_ht = use_signal(String::new);
    let mut scan_tva = use_signal(String::new);
    let mut scan_ttc = use_signal(String::new);
    let mut scan_rate = use_signal(String::new);
    let mut scan_bien = use_signal(String::new);
    let mut scan_lot = use_signal(String::new);

    let mut created_info = use_signal(|| None::<CreatedInvoiceInfo>);

    // Emails
    let mut email_scan_busy = use_signal(|| false);
    let mut email_factures = use_signal(Vec::<EmailFacture>::new);
    let mut email_status = use_signal(String::new);
    let mut email_importing_uid = use_signal(|| None::<u32>);

    // Upload document
    let mut doc_bien = use_signal(String::new);
    let mut doc_lot = use_signal(String::new);
    let mut doc_tenant = use_signal(String::new);
    let mut doc_sci = use_signal(|| true);
    let mut doc_reason = use_signal(String::new);
    let mut docs = use_signal(Vec::<DolibarrDocument>::new);

    let dl_url = "http://localhost:8081";

    rsx! {
        section { class: "page-intro",
            div {
                div { class: "eyebrow", "DOCUMENTS / GED" }
                h2 { "Documents" }
                p { "Deux actions : scanner une facture fournisseur, ou déposer un document classique." }
            }
        }

        if let Some((txt, kind)) = flash() {
            section { class: "panel",
                style: if kind == "error" {
                    "background: rgba(248,113,113,0.08); border-color: rgba(248,113,113,0.25);"
                } else {
                    "background: rgba(74,222,128,0.08); border-color: rgba(74,222,128,0.25);"
                },
                div { style: if kind == "error" { "color: #f87171; font-size: 0.82rem;" } else { "color: #4ade80; font-size: 0.82rem;" },
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

        // ============================================================
        //  FACTURES RECUES PAR EMAIL
        // ============================================================
        section { class: "panel",
            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; flex-wrap: wrap; gap: 10px;",
                div {
                    div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600;",
                        "📧 Factures reçues par email"
                    }
                    div { style: "font-size: 0.72rem; color: #64748b; margin-top: 4px;",
                        "Scanne la boîte mail et importe les factures PDF reçues."
                    }
                }
                button {
                    class: "primary",
                    disabled: email_scan_busy(),
                    onclick: move |_| {
                        email_scan_busy.set(true);
                        email_status.set("Scan en cours...".to_string());
                        spawn(async move {
                            match crate::dolibarr::server_fns::dolibarr_scan_email_factures().await {
                                Ok(list) => {
                                    let n = list.len();
                                    email_factures.set(list);
                                    if n == 0 {
                                        email_status.set("Aucune nouvelle facture trouvée dans la boîte.".to_string());
                                    } else {
                                        email_status.set(format!("{} facture(s) à importer.", n));
                                    }
                                }
                                Err(e) => {
                                    email_status.set(format!("Erreur : {}", e));
                                }
                            }
                            email_scan_busy.set(false);
                        });
                    },
                    if email_scan_busy() { "Scan..." } else { "Scanner les emails" }
                }
            }

            if !email_status().is_empty() {
                div {
                    style: if email_status().starts_with("Erreur") {
                        "font-size: 0.8rem; color: #f87171; padding: 6px 0;"
                    } else if email_factures().is_empty() {
                        "font-size: 0.8rem; color: #94a3b8; padding: 6px 0;"
                    } else {
                        "font-size: 0.8rem; color: #4ade80; padding: 6px 0; font-weight: 500;"
                    },
                    "{email_status()}"
                }
            }

            if !email_factures().is_empty() {
                div { style: "display: flex; flex-direction: column; gap: 6px; margin-top: 8px;",
                    for f in email_factures().iter() {
                        {
                            let uid = f.uid;
                            let from = f.from.clone();
                            let subject = f.subject.clone();
                            let date = f.date.clone();
                            let is_importing = email_importing_uid() == Some(uid);
                            rsx! {
                                div {
                                    key: "{uid}",
                                    style: "background: var(--bg-glass); border: 1px solid var(--border); padding: 10px 14px; border-radius: 8px; display: flex; align-items: center; gap: 12px;",
                                    span {
                                        style: "width: 40px; height: 40px; flex-shrink: 0; border-radius: 8px; display: flex; align-items: center; justify-content: center; background: rgba(124,58,237,0.15); color: #a78bfa; font-size: 0.75rem; font-weight: 700;",
                                        "PDF"
                                    }
                                    div { style: "flex: 1; min-width: 0;",
                                        div { style: "font-size: 0.84rem; color: #e2e8f0; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                            if subject.is_empty() { "(sans sujet)" } else { "{subject}" }
                                        }
                                        div { style: "font-size: 0.7rem; color: #94a3b8; margin-top: 2px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                            "{from}"
                                        }
                                        div { style: "font-size: 0.68rem; color: #64748b; margin-top: 2px;",
                                            "{date}"
                                        }
                                    }
                                    button {
                                        class: "primary",
                                        disabled: busy() || is_importing,
                                        onclick: move |_| {
                                            email_importing_uid.set(Some(uid));
                                            busy.set(true);
                                            flash.set(None);
                                            spawn(async move {
                                                match crate::dolibarr::server_fns::dolibarr_fetch_email_attachment(uid).await {
                                                    Ok((name, bytes)) => {
                                                        file_name.set(name.clone());
                                                        file_bytes.set(bytes.clone());
                                                        match crate::dolibarr::server_fns::dolibarr_parse_invoice_pdf(name, bytes).await {
                                                            Ok(p) => {
                                                                scan_supplier.set(p.supplier_name.clone());
                                                                scan_number.set(p.invoice_number.clone());
                                                                scan_date.set(p.invoice_date.clone());
                                                                scan_due_date.set(p.due_date.clone());
                                                                scan_ht.set(p.total_ht.clone());
                                                                scan_tva.set(p.total_tva.clone());
                                                                scan_ttc.set(p.total_ttc.clone());
                                                                scan_rate.set(p.tva_rate.clone());
                                                                parsed.set(Some(p));
                                                                mode.set(DocMode::ScanInvoice);
                                                                flash.set(Some(("Email importé. Vérifie les champs ci-dessous.".into(), "success".into())));
                                                            }
                                                            Err(e) => {
                                                                flash.set(Some((format!("Erreur analyse PDF : {}", e), "error".into())));
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        flash.set(Some((format!("Erreur import email : {}", e), "error".into())));
                                                    }
                                                }
                                                busy.set(false);
                                                email_importing_uid.set(None);
                                            });
                                        },
                                        if is_importing { "Import..." } else { "Importer" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // === Choix du mode ===
        section { class: "panel",
            div { style: "display: flex; gap: 10px; flex-wrap: wrap;",
                for m in [DocMode::ScanInvoice, DocMode::UploadDocument] {
                    button {
                        style: if mode() == m {
                            "flex: 1; min-width: 240px; padding: 14px 18px; background: #7c3aed; color: white; border: none; border-radius: 10px; cursor: pointer; text-align: left; font-size: 0.9rem; font-weight: 600;"
                        } else {
                            "flex: 1; min-width: 240px; padding: 14px 18px; background: transparent; color: #94a3b8; border: 1px solid var(--border); border-radius: 10px; cursor: pointer; text-align: left; font-size: 0.9rem;"
                        },
                        onclick: move |_| {
                            mode.set(m);
                            parsed.set(None);
                            file_name.set(String::new());
                            file_bytes.set(Vec::new());
                            url_input.set(String::new());
                            docs.set(Vec::new());
                            flash.set(None);
                        },
                        div { "{m.label()}" }
                    }
                }
            }
            div { style: "margin-top: 12px; font-size: 0.75rem; color: #64748b; font-style: italic;",
                "{mode().help()}"
            }
        }

        // ============================================================
        //  MODE 1 : SCAN FACTURE FOURNISSEUR
        // ============================================================
        if mode() == DocMode::ScanInvoice {
            section { class: "panel",
                div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px;",
                    "Etape 1 : choisis le PDF de la facture"
                }

                div { style: "display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-bottom: 10px;",
                    label {
                        style: "display: inline-block; padding: 8px 16px; background: #7c3aed; color: white; border-radius: 6px; cursor: pointer; font-size: 0.8rem; font-weight: 600;",
                        if parsed().is_some() { "Choisir un autre fichier" } else { "Choisir un fichier PDF" }
                        input {
                            r#type: "file",
                            accept: ".pdf,application/pdf",
                            style: "display: none;",
                            onchange: move |evt: FormEvent| {
                                let files = evt.files();
                                if let Some(file) = files.first() {
                                    let name = file.name();
                                    file_name.set(name.clone());
                                    parsed.set(None);
                                    let file = file.clone();
                                    busy.set(true);
                                    spawn(async move {
                                        match file.read_bytes().await {
                                            Ok(bytes) => {
                                                let bytes_vec = bytes.to_vec();
                                                file_bytes.set(bytes_vec.clone());
                                                match crate::dolibarr::server_fns::dolibarr_parse_invoice_pdf(name, bytes_vec).await {
                                                    Ok(p) => {
                                                        scan_supplier.set(p.supplier_name.clone());
                                                        scan_number.set(p.invoice_number.clone());
                                                        scan_date.set(p.invoice_date.clone());
                                                        scan_due_date.set(p.due_date.clone());
                                                        scan_ht.set(p.total_ht.clone());
                                                        scan_tva.set(p.total_tva.clone());
                                                        scan_ttc.set(p.total_ttc.clone());
                                                        scan_rate.set(p.tva_rate.clone());
                                                        parsed.set(Some(p));
                                                        flash.set(Some(("Analyse terminee. Verifie les champs.".into(), "success".into())));
                                                    }
                                                    Err(e) => {
                                                        flash.set(Some((format!("Erreur analyse : {}", e), "error".into())));
                                                    }
                                                }
                                                busy.set(false);
                                            }
                                            Err(_) => {
                                                flash.set(Some(("Erreur lecture fichier".into(), "error".into())));
                                                busy.set(false);
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

                div { style: "display: flex; gap: 8px; align-items: center; flex-wrap: wrap; padding-top: 10px; border-top: 1px solid var(--border);",
                    div { style: "font-size: 0.72rem; color: #64748b;",
                        "Ou depuis une URL :"
                    }
                    input {
                        r#type: "text",
                        placeholder: "https://exemple.com/facture.pdf",
                        value: "{url_input}",
                        oninput: move |e| url_input.set(e.value()),
                        style: "flex: 1; min-width: 200px; padding: 6px 12px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px; color: #e2e8f0; font-size: 0.78rem;",
                    }
                    button {
                        disabled: busy() || url_input().trim().is_empty(),
                        onclick: move |_| {
                            let url = url_input();
                            busy.set(true);
                            parsed.set(None);
                            flash.set(None);
                            spawn(async move {
                                match crate::dolibarr::server_fns::dolibarr_download_from_url(url.clone()).await {
                                    Ok((name, bytes)) => {
                                        file_name.set(name.clone());
                                        file_bytes.set(bytes.clone());
                                        match crate::dolibarr::server_fns::dolibarr_parse_invoice_pdf(name, bytes).await {
                                            Ok(p) => {
                                                scan_supplier.set(p.supplier_name.clone());
                                                scan_number.set(p.invoice_number.clone());
                                                scan_date.set(p.invoice_date.clone());
                                                scan_due_date.set(p.due_date.clone());
                                                scan_ht.set(p.total_ht.clone());
                                                scan_tva.set(p.total_tva.clone());
                                                scan_ttc.set(p.total_ttc.clone());
                                                scan_rate.set(p.tva_rate.clone());
                                                parsed.set(Some(p));
                                                flash.set(Some(("Fichier telecharge et analyse.".into(), "success".into())));
                                            }
                                            Err(e) => {
                                                flash.set(Some((format!("Fichier telecharge mais erreur analyse : {}", e), "error".into())));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        flash.set(Some((format!("Erreur URL : {}", e), "error".into())));
                                    }
                                }
                                busy.set(false);
                            });
                        },
                        style: "padding: 6px 14px; background: transparent; color: #a78bfa; border: 1px solid #7c3aed; border-radius: 6px; cursor: pointer; font-size: 0.78rem; font-weight: 600;",
                        "Charger depuis l'URL"
                    }
                }

                if busy() {
                    div { style: "margin-top: 10px; font-size: 0.78rem; color: #a78bfa;", "Chargement en cours..." }
                }
            }

            if let Some(p) = parsed() {
                section { class: "panel",
                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px;",
                        div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em;",
                            "Etape 2 : verifie et corrige les champs"
                        }
                        div {
                            style: if p.confidence >= 70 {
                                "font-size: 0.75rem; color: #4ade80; font-weight: 600;"
                            } else if p.confidence >= 40 {
                                "font-size: 0.75rem; color: #fbbf24; font-weight: 600;"
                            } else {
                                "font-size: 0.75rem; color: #f87171; font-weight: 600;"
                            },
                            {format!("Confiance : {}%", p.confidence)}
                        }
                    }

                    div { style: "display: flex; flex-direction: column; gap: 10px;",
                        label { class: "field",
                            span { "Fournisseur" }
                            input {
                                r#type: "text",
                                value: "{scan_supplier}",
                                oninput: move |e| scan_supplier.set(e.value()),
                            }
                        }
                        div { style: "display: flex; gap: 8px;",
                            div { style: "flex: 1;",
                                label { class: "field",
                                    span { "Numero de facture" }
                                    input {
                                        r#type: "text",
                                        value: "{scan_number}",
                                        oninput: move |e| scan_number.set(e.value()),
                                    }
                                }
                            }
                            div { style: "flex: 1;",
                                label { class: "field",
                                    span { "Date de facture (AAAA-MM-JJ)" }
                                    input {
                                        r#type: "text",
                                        value: "{scan_date}",
                                        oninput: move |e| scan_date.set(e.value()),
                                    }
                                }
                            }
                            div { style: "flex: 1;",
                                label { class: "field",
                                    span { "Date d'echeance (AAAA-MM-JJ)" }
                                    input {
                                        r#type: "text",
                                        value: "{scan_due_date}",
                                        oninput: move |e| scan_due_date.set(e.value()),
                                    }
                                }
                            }
                        }
                        div { style: "display: flex; gap: 8px;",
                            div { style: "flex: 1;",
                                label { class: "field",
                                    span { "Total HT" }
                                    input {
                                        r#type: "text",
                                        value: "{scan_ht}",
                                        oninput: move |e| scan_ht.set(e.value()),
                                    }
                                }
                            }
                            div { style: "flex: 1;",
                                label { class: "field",
                                    span { "TVA" }
                                    input {
                                        r#type: "text",
                                        value: "{scan_tva}",
                                        oninput: move |e| scan_tva.set(e.value()),
                                    }
                                }
                            }
                            div { style: "flex: 1;",
                                label { class: "field",
                                    span { "Total TTC" }
                                    input {
                                        r#type: "text",
                                        value: "{scan_ttc}",
                                        oninput: move |e| scan_ttc.set(e.value()),
                                    }
                                }
                            }
                            div { style: "width: 100px;",
                                label { class: "field",
                                    span { "Taux TVA %" }
                                    input {
                                        r#type: "text",
                                        value: "{scan_rate}",
                                        oninput: move |e| scan_rate.set(e.value()),
                                    }
                                }
                            }
                        }
                    }

                    if !p.vat_lines.is_empty() {
                        div { style: "margin-top: 14px; padding: 10px 14px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px;",
                            div { style: "font-size: 0.72rem; color: #94a3b8; margin-bottom: 6px; font-weight: 600;",
                                {format!("Detail TVA ({} taux detectes)", p.vat_lines.len())}
                            }
                            div { style: "display: flex; flex-direction: column; gap: 4px;",
                                for (i, vl) in p.vat_lines.iter().enumerate() {
                                    div {
                                        key: "{i}",
                                        style: "font-size: 0.75rem; color: #cbd5e1; font-family: monospace;",
                                        span { style: "color: #a78bfa;", {format!("TVA {}%", vl.rate)} }
                                        span { style: "color: #334155;", " | " }
                                        span { {format!("base {} EUR", vl.base_ht)} }
                                        span { style: "color: #334155;", " | " }
                                        span { {format!("TVA {} EUR", vl.amount_tva)} }
                                    }
                                }
                            }
                        }
                    }

                    div { style: "margin-top: 16px; padding-top: 12px; border-top: 1px solid var(--border);",
                        div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;",
                            "Attribution (optionnel)"
                        }
                        div { style: "display: flex; gap: 8px; flex-wrap: wrap;",
                            label { class: "field", style: "flex: 1; min-width: 180px;",
                                span { "Bien concerne" }
                                select {
                                    value: "{scan_bien}",
                                    onchange: move |e| {
                                        scan_bien.set(e.value());
                                        scan_lot.set(String::new());
                                    },
                                    option { value: "", "-- Aucun bien specifique --" }
                                    for pr in properties.read().as_deref().unwrap_or(&[]).iter() {
                                        option { value: "{pr.name}", "{pr.name}" }
                                    }
                                }
                            }
                            if !scan_bien().is_empty() {
                                label { class: "field", style: "flex: 1; min-width: 180px;",
                                    span { "Lot concerne" }
                                    select {
                                        value: "{scan_lot}",
                                        onchange: move |e| scan_lot.set(e.value()),
                                        option { value: "", "-- Tout le bien --" }
                                        for u in units.read().as_deref().unwrap_or(&[]).iter().filter(|u| u.property_name == scan_bien()) {
                                            option { value: "{u.code}", {format!("{} - {}", u.code, u.label)} }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if p.confidence < 30 {
                        div { style: "margin-top: 14px; padding: 10px 14px; background: rgba(251,191,36,0.08); border: 1px solid rgba(251,191,36,0.3); border-radius: 6px; font-size: 0.78rem; color: #fbbf24;",
                            "L'extraction automatique n'a pas bien fonctionne sur ce PDF. Copie manuellement les valeurs depuis le texte ci-dessous."
                        }
                    }

                    div { style: "margin-top: 14px;",
                        div { style: "font-size: 0.72rem; color: #94a3b8; margin-bottom: 6px; font-weight: 600;",
                            "Texte brut extrait du PDF"
                        }
                        pre {
                            style: "padding: 10px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px; font-size: 0.7rem; color: #cbd5e1; max-height: 260px; overflow: auto; white-space: pre-wrap; user-select: text;",
                            "{p.raw_text_preview}"
                        }
                    }

                    div { style: "display: flex; gap: 8px; margin-top: 18px;",
                        button {
                            class: "primary",
                            disabled: busy() || scan_supplier().trim().is_empty(),
                            onclick: move |_| {
                                let supplier = scan_supplier();
                                let number = scan_number();
                                let date = scan_date();
                                let due = scan_due_date();
                                let ht = scan_ht();
                                let tva = scan_tva();
                                let ttc = scan_ttc();
                                let rate = scan_rate();
                                let fname = file_name();
                                let bytes = file_bytes();
                                let sb = scan_bien();
                                let sl = scan_lot();
                                let vls = parsed().map(|p| p.vat_lines.clone()).unwrap_or_default();
                                busy.set(true);
                                flash.set(None);
                                spawn(async move {
                                    match crate::dolibarr::server_fns::dolibarr_create_supplier_invoice(
                                        supplier.clone(), number.clone(), date, due, ht, tva, ttc.clone(), rate, vls, fname.clone(), bytes, sb.clone(), sl.clone()
                                    ).await {
                                        Ok(response) => {
                                            let parts: Vec<&str> = response.splitn(2, '|').collect();
                                            let invoice_id = parts.get(0).unwrap_or(&"").to_string();
                                            let invoice_ref = parts.get(1).unwrap_or(&"").to_string();

                                            created_info.set(Some(CreatedInvoiceInfo {
                                                id: invoice_id.clone(),
                                                reference: invoice_ref.clone(),
                                                supplier: supplier.clone(),
                                                number: number.clone(),
                                                ttc: ttc.clone(),
                                                bien: sb.clone(),
                                                lot: sl.clone(),
                                                pdf_filename: fname.clone(),
                                            }));
                                            flash.set(Some((format!("Facture fournisseur creee (ID {} / ref {}).", invoice_id, invoice_ref), "success".into())));
                                            parsed.set(None);
                                            file_name.set(String::new());
                                            file_bytes.set(Vec::new());
                                            url_input.set(String::new());
                                            scan_supplier.set(String::new());
                                            scan_number.set(String::new());
                                            scan_date.set(String::new());
                                            scan_due_date.set(String::new());
                                            scan_ht.set(String::new());
                                            scan_tva.set(String::new());
                                            scan_ttc.set(String::new());
                                            scan_rate.set(String::new());
                                            scan_bien.set(String::new());
                                            scan_lot.set(String::new());
                                        }
                                        Err(e) => {
                                            flash.set(Some((format!("Erreur creation : {}", e), "error".into())));
                                        }
                                    }
                                    busy.set(false);
                                });
                            },
                            if busy() { "Creation..." } else { "Valider et creer dans Dolibarr" }
                        }
                        button {
                            class: "secondary",
                            onclick: move |_| {
                                parsed.set(None);
                                file_name.set(String::new());
                                file_bytes.set(Vec::new());
                                url_input.set(String::new());
                            },
                            "Annuler"
                        }
                    }
                }
            }
        }

        // ============================================================
        //  PANNEAU FACTURE CREEE
        // ============================================================
        if let Some(info) = created_info() {
            {
                let safe_fname = info.pdf_filename.replace(' ', "_");
                let file_param = format!("{}/{}", info.reference, safe_fname)
                    .replace('/', "%2F");
                let web_pdf_url = format!(
                    "http://localhost:8081/document.php?modulepart=facture_fournisseur&file={}",
                    file_param
                );
                let dolibarr_card_url = format!(
                    "http://localhost:8081/fourn/facture/card.php?facid={}",
                    info.id
                );

                rsx! {
                    section { class: "panel",
                        style: "background: rgba(74,222,128,0.06); border-color: rgba(74,222,128,0.3);",
                        div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;",
                            div {
                                div { style: "font-size: 0.7rem; color: #4ade80; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 700;",
                                    "Facture creee dans Dolibarr"
                                }
                                div { style: "font-size: 0.85rem; color: #e2e8f0; font-weight: 500; margin-top: 4px;",
                                    {format!("{} - {}", info.number, info.supplier)}
                                }
                            }
                            button {
                                style: "padding: 4px 12px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.72rem;",
                                onclick: move |_| created_info.set(None),
                                "Fermer"
                            }
                        }

                        div { style: "display: flex; gap: 20px; flex-wrap: wrap;",
                            div {
                                div { style: "font-size: 0.68rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em;",
                                    "Montant TTC"
                                }
                                div { style: "font-size: 1.1rem; color: #4ade80; font-weight: 600; margin-top: 4px;",
                                    {format!("{} EUR", info.ttc)}
                                }
                            }
                            div {
                                div { style: "font-size: 0.68rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em;",
                                    "ID Dolibarr"
                                }
                                div { style: "font-size: 1.1rem; color: #e2e8f0; font-weight: 500; margin-top: 4px;",
                                    "#{info.id}"
                                }
                            }
                            if !info.bien.is_empty() {
                                div {
                                    div { style: "font-size: 0.68rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em;",
                                        "Bien rattache"
                                    }
                                    div { style: "font-size: 0.85rem; color: #e2e8f0; margin-top: 4px;",
                                        if info.lot.is_empty() {
                                            "{info.bien}"
                                        } else {
                                            {format!("{} - {}", info.bien, info.lot)}
                                        }
                                    }
                                }
                            }
                        }

                        div { style: "display: flex; gap: 8px; margin-top: 16px; flex-wrap: wrap;",
                            button {
                                style: "padding: 8px 16px; background: #7c3aed; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.78rem; font-weight: 600;",
                                onclick: {
                                    let url = dolibarr_card_url.clone();
                                    move |_| {
                                        let script = format!("window.open('{}', '_blank');", url);
                                        let _ = document::eval(&script);
                                    }
                                },
                                "Ouvrir la facture dans Dolibarr"
                            }
                            button {
                                style: "padding: 8px 16px; background: transparent; color: #a78bfa; border: 1px solid #7c3aed; border-radius: 6px; cursor: pointer; font-size: 0.78rem; font-weight: 600;",
                                onclick: {
                                    let url = web_pdf_url.clone();
                                    move |_| {
                                        let script = format!("window.open('{}', '_blank');", url);
                                        let _ = document::eval(&script);
                                    }
                                },
                                "Voir le PDF attache"
                            }
                        }
                    }
                }
            }
        }

        // ============================================================
        //  MODE 2 : UPLOAD DOCUMENT
        // ============================================================
        if mode() == DocMode::UploadDocument {
            section { class: "panel",
                div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px;",
                    "A quoi ce document se rattache-t-il ?"
                }

                div { style: "display: flex; gap: 16px; flex-wrap: wrap; margin-bottom: 14px;",
                    label { style: "display: flex; align-items: center; gap: 8px; font-size: 0.82rem; color: #e2e8f0;",
                        input {
                            r#type: "checkbox",
                            checked: "{doc_sci}",
                            onchange: move |e| doc_sci.set(e.value() == "true"),
                        }
                        "La SCI"
                    }
                }

                div { style: "display: flex; gap: 12px; flex-wrap: wrap; margin-bottom: 14px;",
                    label { class: "field", style: "flex: 1; min-width: 200px;",
                        span { "Un bien (optionnel)" }
                        select {
                            value: "{doc_bien}",
                            onchange: move |e| {
                                doc_bien.set(e.value());
                                doc_lot.set(String::new());
                            },
                            option { value: "", "-- Aucun bien specifique --" }
                            for p in properties.read().as_deref().unwrap_or(&[]).iter() {
                                option { value: "{p.name}", "{p.name}" }
                            }
                        }
                    }
                    if !doc_bien().is_empty() {
                        label { class: "field", style: "flex: 1; min-width: 200px;",
                            span { "Un lot (optionnel)" }
                            select {
                                value: "{doc_lot}",
                                onchange: move |e| doc_lot.set(e.value()),
                                option { value: "", "-- Tout le bien --" }
                                for u in units.read().as_deref().unwrap_or(&[]).iter().filter(|u| u.property_name == doc_bien()) {
                                    option { value: "{u.code}", {format!("{} - {}", u.code, u.label)} }
                                }
                            }
                        }
                    }
                    label { class: "field", style: "flex: 1; min-width: 200px;",
                        span { "Un locataire / tiers (optionnel)" }
                        select {
                            value: "{doc_tenant}",
                            onchange: move |e| doc_tenant.set(e.value()),
                            option { value: "", "-- Aucun tiers specifique --" }
                            for t in thirds.read().as_ref().and_then(|r| r.as_ref().ok()).cloned().unwrap_or_default().iter() {
                                if t.name != "Patrimoine SCI" {
                                    option { value: "{t.id}", "{t.name}" }
                                }
                            }
                        }
                    }
                }

                label { class: "field",
                    span { "Description (optionnel)" }
                    input {
                        r#type: "text",
                        placeholder: "Ex : Bail signe 2024 - M. Dupont",
                        value: "{doc_reason}",
                        oninput: move |e| doc_reason.set(e.value()),
                    }
                }
            }

            section { class: "panel",
                div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px;",
                    "Choisis le fichier"
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
                                            Ok(bytes) => file_bytes.set(bytes.to_vec()),
                                            Err(_) => flash.set(Some(("Erreur lecture fichier".into(), "error".into()))),
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

                div { style: "display: flex; gap: 8px; align-items: center; flex-wrap: wrap; padding-top: 10px; border-top: 1px solid var(--border);",
                    div { style: "font-size: 0.72rem; color: #64748b;",
                        "Ou depuis une URL :"
                    }
                    input {
                        r#type: "text",
                        placeholder: "https://exemple.com/bail.pdf",
                        value: "{url_input}",
                        oninput: move |e| url_input.set(e.value()),
                        style: "flex: 1; min-width: 200px; padding: 6px 12px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px; color: #e2e8f0; font-size: 0.78rem;",
                    }
                    button {
                        disabled: busy() || url_input().trim().is_empty(),
                        onclick: move |_| {
                            let url = url_input();
                            busy.set(true);
                            flash.set(None);
                            spawn(async move {
                                match crate::dolibarr::server_fns::dolibarr_download_from_url(url).await {
                                    Ok((name, bytes)) => {
                                        file_name.set(name.clone());
                                        file_bytes.set(bytes.clone());
                                        flash.set(Some((format!("Fichier '{}' telecharge ({}).", name, format_size(bytes.len() as i64)), "success".into())));
                                    }
                                    Err(e) => {
                                        flash.set(Some((format!("Erreur URL : {}", e), "error".into())));
                                    }
                                }
                                busy.set(false);
                            });
                        },
                        style: "padding: 6px 14px; background: transparent; color: #a78bfa; border: 1px solid #7c3aed; border-radius: 6px; cursor: pointer; font-size: 0.78rem; font-weight: 600;",
                        "Charger depuis l'URL"
                    }
                }

                if !file_bytes().is_empty() {
                    div { style: "display: flex; gap: 8px; margin-top: 14px;",
                        button {
                            class: "primary",
                            disabled: busy(),
                            onclick: move |_| {
                                let fname = file_name();
                                let bytes = file_bytes();
                                let bien = doc_bien();
                                let lot = doc_lot();
                                let tenant_id = doc_tenant();
                                let reason = doc_reason();
                                let sci = doc_sci();
                                busy.set(true);
                                flash.set(None);
                                spawn(async move {
                                    let (bien_name_for_upload, lot_code_for_upload) = if !bien.is_empty() {
                                        (bien.clone(), lot.clone())
                                    } else {
                                        ("Divers".to_string(), String::new())
                                    };

                                    let result = crate::dolibarr::server_fns::dolibarr_upload_patrimoine_document(
                                        bien_name_for_upload,
                                        lot_code_for_upload,
                                        fname.clone(),
                                        bytes,
                                    ).await;

                                    let final_name = match result {
                                        Ok(n) => n,
                                        Err(e) => {
                                            flash.set(Some((format!("Erreur upload : {}", e), "error".into())));
                                            busy.set(false);
                                            return;
                                        }
                                    };

                                    let ecm_id = crate::dolibarr::server_fns::dolibarr_get_last_ecm_id().await.unwrap_or(0);

                                    if ecm_id > 0 && !tenant_id.is_empty() {
                                        let _ = crate::dolibarr::server_fns::dolibarr_link_document(
                                            ecm_id, tenant_id.clone(), "societe".to_string()
                                        ).await;
                                    }

                                    let mut msg = format!("Document '{}' depose.", final_name);
                                    if !reason.is_empty() {
                                        msg = format!("{} Description : {}", msg, reason);
                                    }
                                    if !bien.is_empty() {
                                        msg = format!("{} Rattaché au bien : {}", msg, bien);
                                    }
                                    if !tenant_id.is_empty() {
                                        msg = format!("{} + lien vers le tiers.", msg);
                                    }
                                    if sci && bien.is_empty() && tenant_id.is_empty() {
                                        msg = format!("{} Rattaché à la SCI.", msg);
                                    }
                                    flash.set(Some((msg, "success".into())));

                                    file_name.set(String::new());
                                    file_bytes.set(Vec::new());
                                    url_input.set(String::new());
                                    doc_bien.set(String::new());
                                    doc_lot.set(String::new());
                                    doc_tenant.set(String::new());
                                    doc_reason.set(String::new());
                                    busy.set(false);
                                });
                            },
                            if busy() { "Upload..." } else { "Déposer" }
                        }
                        button {
                            class: "secondary",
                            onclick: move |_| {
                                file_name.set(String::new());
                                file_bytes.set(Vec::new());
                                url_input.set(String::new());
                            },
                            "Annuler"
                        }
                    }
                }
            }

            section { class: "panel",
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;",
                    div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em;",
                        "Documents existants"
                    }
                    button {
                        style: "padding: 4px 10px; background: transparent; border: 1px solid var(--border); color: #94a3b8; border-radius: 6px; cursor: pointer; font-size: 0.7rem;",
                        disabled: busy() || doc_bien().is_empty(),
                        onclick: {
                            let bname = doc_bien();
                            move |_| {
                                if bname.is_empty() {
                                    flash.set(Some(("Choisis d'abord un bien pour filtrer.".into(), "error".into())));
                                    return;
                                }
                                let b = bname.clone();
                                busy.set(true);
                                spawn(async move {
                                    match crate::dolibarr::server_fns::dolibarr_list_patrimoine_documents(b).await {
                                        Ok(list) => {
                                            let n = list.len();
                                            docs.set(list);
                                            if n == 0 {
                                                flash.set(Some(("Aucun document pour ce bien.".into(), "error".into())));
                                            } else {
                                                flash.set(Some((format!("{} document(s).", n), "success".into())));
                                            }
                                        }
                                        Err(e) => flash.set(Some((format!("Erreur : {}", e), "error".into()))),
                                    }
                                    busy.set(false);
                                });
                            }
                        },
                        "Voir les documents du bien sélectionné"
                    }
                }

                if docs().is_empty() {
                    div { style: "padding: 14px; text-align: center; color: #64748b; font-size: 0.78rem; background: var(--bg-input); border: 1px dashed var(--border-strong); border-radius: 8px;",
                        "Sélectionne un bien et clique sur « Voir les documents » pour les afficher."
                    }
                } else {
                    div { style: "display: flex; flex-direction: column; gap: 6px;",
                        for (idx, d) in docs().iter().enumerate() {
                            {
                                let icon = file_icon(&d.filename);
                                let date = format_date(d.date);
                                let fname = d.filename.clone();
                                let relpath = d.relativename.clone();
                                let mp = d.modulepart.clone();
                                let key = format!("{}-{}", idx, fname);
                                let url = format!("{}/document.php?modulepart={}&file={}", dl_url, mp, relpath);
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
                                            div { style: "font-size: 0.68rem; color: #94a3b8; margin-top: 2px;",
                                                span { "{date}" }
                                                span { style: "color: #334155;", " / " }
                                                span { style: "font-family: monospace;", "{relpath}" }
                                            }
                                        }
                                        button {
                                            style: "padding: 5px 10px; background: transparent; border: 1px solid var(--border); color: #a78bfa; border-radius: 6px; cursor: pointer; font-size: 0.7rem; font-weight: 600;",
                                            onclick: move |_| {
                                                let u = url.clone();
                                                let script = format!("window.open('{}', '_blank');", u);
                                                let _ = document::eval(&script);
                                            },
                                            "Ouvrir"
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