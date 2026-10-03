use dioxus::prelude::*;
use uuid::Uuid;

use crate::domain::UnitItem;
use crate::server::{list_properties, list_units};
use crate::tax_fonciere::calculation::apply_share_bp;
use crate::tax_fonciere::models::*;
use crate::tax_fonciere::tax_service::*;
use crate::ui::{euro, FormField, InfoTileOwned, ModuleHeader};

#[component]
pub fn TaxFoncierePage(refresh: Signal<u64>) -> Element {
    let bump = use_signal(|| 0u64);
    let mut selected_notice = use_signal(|| Option::<Uuid>::None);

    let notices = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_tax_notices().await.unwrap_or_default() }
    });

    let rows: Vec<(Uuid, String, String, Option<String>)> = notices
        .read()
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|n| {
            let title = format!("{} — {}", n.property_name, n.fiscal_year);
            let summary = format!(
                "{} adresse(s) • {} lot(s) • {}",
                n.addresses_count,
                n.lines_count,
                status_label(&n.status)
            );
            let fees = n.total_amount_cents.map(|t| {
                format!(
                    "Total {} • frais {}",
                    euro(t),
                    euro(n.management_fees_cents.unwrap_or(0))
                )
            });
            (n.id, title, summary, fees)
        })
        .collect();

    let rows_count = rows.len();

    rsx! {
        ModuleHeader {
            title: "Taxe foncière",
            kicker: "AVIS • RÉPARTITION • FACTURATION",
            detail: "Importez les 2 feuillets de l'avis (cotisations par adresse + montant total et frais), attribuez chaque adresse à un ou plusieurs lots, puis générez les factures Dolibarr."
        }

        NewNoticePanel { bump }

        if let Some(notice_id) = selected_notice() {
            NoticeDetailPanel {
                notice_id,
                on_close: move |_| selected_notice.set(None),
                bump,
            }
        }

        section { class: "panel",
            div { class: "panel-head",
                h3 { "Avis enregistrés" }
                span { class: "small", {format!("{} avis", rows_count)} }
            }
            if rows.is_empty() {
                div { class: "empty-state",
                    h3 { "Aucun avis" }
                    p { "Créez un premier avis ci-dessus." }
                }
            }
            for (nid, title, summary, fees) in rows.iter() {
                div { key: "{nid}", class: "data-row",
                    div {
                        div { class: "data-title", "{title}" }
                        div { class: "small", "{summary}" }
                        if let Some(line) = fees.as_ref() {
                            div { class: "small", "{line}" }
                        }
                    }
                    div { class: "row-actions",
                        {
                            let nid_copy = *nid;
                            rsx! {
                                button {
                                    class: "secondary",
                                    onclick: move |_| selected_notice.set(Some(nid_copy)),
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

#[component]
fn NewNoticePanel(mut bump: Signal<u64>) -> Element {
    let props = use_resource(move || async move {
        list_properties().await.unwrap_or_default()
    });
    let mut property_id = use_signal(String::new);
    let mut year = use_signal(|| {
        chrono::Utc::now()
            .format("%Y")
            .to_string()
            .parse::<i32>()
            .unwrap_or(2026)
    });
    let mut msg = use_signal(String::new);

    let props_list = props.read().as_deref().unwrap_or(&[]).to_vec();

    rsx! {
        section { class: "panel",
            div { class: "panel-head", h3 { "Nouvel avis" } }
            div { class: "form-grid",
                label { class: "field",
                    span { "Bien" }
                    select {
                        value: property_id(),
                        onchange: move |e: FormEvent| property_id.set(e.value()),
                        option { value: "", "— Sélectionner un bien —" }
                        for p in props_list.iter() {
                            option { value: p.id.to_string(), "{p.name}" }
                        }
                    }
                }
                FormField {
                    label: "Exercice",
                    value: year().to_string(),
                    oninput: move |e: FormEvent| {
                        if let Ok(y) = e.value().parse::<i32>() {
                            year.set(y);
                        }
                    }
                }
            }
            div { class: "action-row",
                button {
                    class: "primary",
                    onclick: move |_| {
                        let pid = property_id();
                        let y = year();
                        spawn(async move {
                            match Uuid::parse_str(&pid) {
                                Ok(id) => match create_or_get_notice(id, y).await {
                                    Ok(_) => {
                                        msg.set("Avis créé".into());
                                        bump += 1;
                                    }
                                    Err(e) => msg.set(e.to_string()),
                                },
                                Err(_) => msg.set("Sélectionnez un bien".into()),
                            }
                        });
                    },
                    "Créer l'avis"
                }
                if !msg().is_empty() {
                    span { class: "save-ok", "{msg}" }
                }
            }
        }
    }
}

#[component]
fn NoticeDetailPanel(
    notice_id: Uuid,
    on_close: EventHandler<()>,
    mut bump: Signal<u64>,
) -> Element {
    let mut local_bump = use_signal(|| 0u64);
    let detail = use_resource(move || {
        let _ = local_bump();
        async move { get_tax_notice_detail(notice_id).await.ok() }
    });

    let all_units = use_resource(move || async move {
        list_units().await.unwrap_or_default()
    });

    let mut action_msg = use_signal(String::new);
    let mut loading = use_signal(|| false);

    let detail_snap: Option<TaxNoticeDetail> =
        detail.read().as_ref().and_then(|x| x.clone());
    let all_units_list = all_units.read().as_deref().unwrap_or(&[]).to_vec();

    rsx! {
        section { class: "panel",
            div { class: "panel-head",
                div {
                    h3 { "Détail de l'avis" }
                    if let Some(d) = &detail_snap {
                        span { class: "small",
                            {format!("{} • {} • {}",
                                d.property_name, d.fiscal_year, status_label(&d.status))} }
                    }
                }
                div { style: "display: flex; gap: 8px;",
                    {
                        let mut confirming = use_signal(|| false);
                        let is_confirming = confirming();
                        rsx! {
                            if is_confirming {
                                button {
                                    class: "secondary",
                                    style: "color: #f87171; border-color: rgba(248,113,113,0.3);",
                                    onclick: move |_| {
                                        spawn(async move {
                                            match delete_tax_notice(notice_id).await {
                                                Ok(_) => {
                                                    bump += 1;
                                                    on_close.call(());
                                                }
                                                Err(e) => action_msg.set(format!("Erreur : {e}")),
                                            }
                                        });
                                    },
                                    "Confirmer la suppression"
                                }
                                button {
                                    class: "secondary",
                                    onclick: move |_| confirming.set(false),
                                    "Annuler"
                                }
                            } else {
                                button {
                                    class: "secondary",
                                    style: "color: #f87171; border-color: rgba(248,113,113,0.3);",
                                    onclick: move |_| confirming.set(true),
                                    "🗑 Supprimer l'avis"
                                }
                                button {
                                    class: "secondary",
                                    onclick: move |_| on_close.call(()),
                                    "Fermer"
                                }
                            }
                        }
                    }
                }
            }

            if let Some(d) = detail_snap {
                div { class: "facts-row",
                    InfoTileOwned {
                        label: "Total avis",
                        value: d.total_amount_cents.map(euro).unwrap_or_else(|| "—".into())
                    }
                    InfoTileOwned {
                        label: "Cotisations",
                        value: d.cotisations_amount_cents.map(euro).unwrap_or_else(|| "—".into())
                    }
                    InfoTileOwned {
                        label: "Frais gestion",
                        value: d.management_fees_cents.map(euro).unwrap_or_else(|| "—".into())
                    }
                    InfoTileOwned {
                        label: "Documents",
                        value: d.documents.len().to_string()
                    }
                }

                div { class: "two-col",
                    UploadSlot {
                        notice_id,
                        kind: "BASES",
                        title: "Feuillet 1 — Cotisations par adresse",
                        already_uploaded: d.documents.iter().any(|x| x.document_kind == "BASES"),
                        local_bump,
                        action_msg,
                    }
                    UploadSlot {
                        notice_id,
                        kind: "FEES",
                        title: "Feuillet 2 — Montant total & frais",
                        already_uploaded: d.documents.iter().any(|x| x.document_kind == "FEES"),
                        local_bump,
                        action_msg,
                    }
                }

                if !action_msg().is_empty() {
                    div { class: "save-ok", "{action_msg()}" }
                }

                if let Some(inv_id) = d.dgfip_invoice_id {
                    div {
                        style: "margin: 12px 0; padding: 12px 16px; border-radius: 8px; background: rgba(16,185,129,0.08); border: 1px solid rgba(16,185,129,0.2); display: flex; justify-content: space-between; align-items: center; gap: 12px;",
                        div {
                            div {
                                style: "font-size: 0.82rem; color: #94a3b8;",
                                "Facture fournisseur DGFiP"
                            }
                            div {
                                style: "font-size: 1rem; font-weight: 700; color: #10b981;",
                                {format!("{} — {}",
                                    d.dgfip_invoice_ref.clone().unwrap_or_else(|| "—".into()),
                                    euro(d.total_amount_cents.unwrap_or(0)))}
                            }
                        }
                        a {
                            href: "http://localhost:8081/fourn/facture/card.php?facid={inv_id}",
                            target: "_blank",
                            style: "font-size: 0.85rem; padding: 8px 14px; border-radius: 6px; background: #10b981; color: #04121f; font-weight: 700; text-decoration: none; white-space: nowrap;",
                            "📄 Voir la facture fournisseur"
                        }
                    }
                }

                if !d.addresses.is_empty() {
                    {
                        let fees_total = d.fees.iter().map(|f| f.fee_amount_cents).sum::<i64>();
                        // Cotisations totales des adresses SCINDÉES uniquement (pour le prorata)
                        let cotisations_scindees = d.addresses.iter()
                            .filter(|a| a.units.len() > 1)
                            .map(|a| a.tax_amount_cents)
                            .sum::<i64>();
                        let fees_uploaded = d.documents.iter().any(|x| x.document_kind == "FEES");
                        rsx! {
                            AddressesList {
                                addresses: d.addresses.clone(),
                                all_units: all_units_list.clone(),
                                property_id: d.property_id,
                                fees_total_cents: fees_total,
                                cotisations_scindees_cents: cotisations_scindees,
                                fees_uploaded,
                                local_bump,
                                action_msg,
                            }
                        }
                    }
                } else {
                    div { class: "empty-state",
                        h3 { "Aucune adresse" }
                        p { "Importez le feuillet 1 (cotisations par adresse) pour extraire les adresses." }
                    }
                }

                {
                    let has_bases = d.documents.iter().any(|x| x.document_kind == "BASES");
                    let has_fees = d.documents.iter().any(|x| x.document_kind == "FEES");
                    let all_reparties = !d.addresses.is_empty() && d.addresses.iter().all(|a| {
                        let total_bp: i32 = a.units.iter().map(|u| u.share_bp).sum();
                        total_bp == 10_000
                    });
                    let total_present = d.total_amount_cents.unwrap_or(0) > 0;
                    let can_generate = has_bases && has_fees && total_present && all_reparties && !loading();

                    let facturables = d.addresses.iter()
                        .flat_map(|a| a.units.iter())
                        .filter(|u| !u.is_vacant)
                        .count();
                    let vacants = d.addresses.iter()
                        .flat_map(|a| a.units.iter())
                        .filter(|u| u.is_vacant)
                        .count();

                    rsx! {
                        div { class: "action-row",
                            div { style: "flex: 1;",
                                if !has_bases {
                                    div {
                                        style: "font-size: 0.82rem; color: #f87171; margin-bottom: 8px;",
                                        "⚠ Importez d'abord le feuillet 1 (cotisations par adresse)."
                                    }
                                } else if !has_fees {
                                    div {
                                        style: "font-size: 0.82rem; color: #f87171; margin-bottom: 8px;",
                                        "⚠ Importez le feuillet 2 (montant total & frais) pour activer la génération des factures."
                                    }
                                } else if !all_reparties {
                                    div {
                                        style: "font-size: 0.82rem; color: #f87171; margin-bottom: 8px;",
                                        "⚠ Toutes les adresses doivent être réparties à 100 % avant de générer les factures."
                                    }
                                } else {
                                    div {
                                        style: "font-size: 0.82rem; color: #10b981; margin-bottom: 8px;",
                                        if vacants > 0 {
                                            {format!(
                                                "✓ Prêt à générer : {} facture(s) locataire(s) + 1 facture fournisseur DGFiP de {} ({} lot(s) vacant(s) — reste à charge SCI)",
                                                facturables,
                                                euro(d.total_amount_cents.unwrap_or(0)),
                                                vacants
                                            )}
                                        } else {
                                            {format!(
                                                "✓ Prêt à générer : {} facture(s) locataire(s) + 1 facture fournisseur DGFiP de {}",
                                                facturables,
                                                euro(d.total_amount_cents.unwrap_or(0))
                                            )}
                                        }
                                    }
                                }
                            }
                            button {
                                class: "primary",
                                disabled: !can_generate,
                                onclick: move |_| {
                                    spawn(async move {
                                        loading.set(true);
                                        match generate_tax_notice_invoices(notice_id).await {
                                            Ok(r) => {
                                                action_msg.set(format!(
                                                    "✓ {} facture(s) locataire(s) créée(s) • {} vacant(s) ignoré(s) • {} échec(s) • Facture fournisseur DGFiP enregistrée",
                                                    r.created, r.skipped_vacant, r.failed
                                                ));
                                                bump += 1;
                                                local_bump += 1;
                                            }
                                            Err(e) => action_msg.set(format!("Erreur : {e}")),
                                        }
                                        loading.set(false);
                                    });
                                },
                                if loading() { "⏳ Facturation en cours…" } else { "Générer les factures Dolibarr" }
                            }
                            if loading() {
                                div {
                                    style: "flex-basis: 100%; margin-top: 8px;",
                                    div { class: "progress-bar" }
                                    div { class: "progress-hint",
                                        "Création des factures dans Dolibarr…"
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "loading-grid",
                    div { class: "hero-card skeleton" }
                }
            }
        }
    }
}

#[component]
fn UploadSlot(
    notice_id: Uuid,
    kind: &'static str,
    title: &'static str,
    already_uploaded: bool,
    mut local_bump: Signal<u64>,
    mut action_msg: Signal<String>,
) -> Element {
    let mut busy = use_signal(|| false);
    let mut confirming_delete = use_signal(|| false);

    rsx! {
        div { class: "panel",
            div { style: "display: flex; justify-content: space-between; align-items: center; gap: 8px;",
                h3 { "{title}" }
                if already_uploaded {
                    span {
                        style: "font-size: 0.75rem; font-weight: 700; padding: 4px 10px; border-radius: 6px; background: rgba(16,185,129,0.15); color: #10b981;",
                        "✓ Importé"
                    }
                }
            }

            div {
                style: "font-size: 0.78rem; color: #94a3b8; margin: 8px 0 12px 0;",
                if already_uploaded {
                    "Ce feuillet est déjà enregistré. Vous pouvez le remplacer en cas d'erreur."
                } else {
                    "Formats acceptés : PDF, JPG, PNG. L'ordre d'import n'a pas d'importance."
                }
            }

            if !already_uploaded || confirming_delete() {
                input {
                    r#type: "file",
                    accept: ".pdf,.png,.jpg,.jpeg",
                    disabled: busy(),
                    onchange: move |evt: FormEvent| {
                        let mut files = evt.files();
                        if files.is_empty() {
                            return;
                        }
                        let file = files.remove(0);
                        let name = file.name();
                        spawn(async move {
                            busy.set(true);
                            action_msg.set(format!("Lecture de {} …", name));
                            match file.read_bytes().await {
                                Ok(bytes) => {
                                    let bytes_vec: Vec<u8> = bytes.to_vec();
                                    match upload_tax_notice_document(
                                        notice_id,
                                        kind.to_string(),
                                        name.clone(),
                                        bytes_vec,
                                    )
                                    .await
                                    {
                                        Ok(r) => {
                                            action_msg.set(format!(
                                                "✓ {} importé — OCR {}% — statut {}",
                                                kind, r.ocr_confidence, r.notice_status
                                            ));
                                            confirming_delete.set(false);
                                            local_bump += 1;
                                        }
                                        Err(e) => action_msg.set(format!("Erreur OCR : {e}")),
                                    }
                                }
                                Err(e) => action_msg.set(format!("Erreur lecture fichier : {e}")),
                            }
                            busy.set(false);
                        });
                    }
                }
            }

            if already_uploaded && !confirming_delete() {
                div { style: "margin-top: 8px;",
                    button {
                        class: "secondary",
                        disabled: busy(),
                        onclick: move |_| confirming_delete.set(true),
                        "🔄 Remplacer ce feuillet"
                    }
                }
            } else if already_uploaded && confirming_delete() {
                div {
                    style: "margin-top: 8px; padding: 10px 14px; border-radius: 6px; background: rgba(248,113,113,0.08); border: 1px solid rgba(248,113,113,0.3);",
                    div {
                        style: "font-size: 0.82rem; color: #f87171; margin-bottom: 10px;",
                        "⚠ Cette action va supprimer le feuillet actuel et ses données associées. Continuer ?"
                    }
                    div { style: "display: flex; gap: 8px;",
                        button {
                            class: "secondary",
                            style: "color: #f87171; border-color: rgba(248,113,113,0.3);",
                            disabled: busy(),
                            onclick: move |_| {
                                spawn(async move {
                                    busy.set(true);
                                    match delete_tax_notice_document(notice_id, kind.to_string()).await {
                                        Ok(_) => {
                                            action_msg.set(format!("✓ Feuillet {} supprimé. Vous pouvez en importer un nouveau.", kind));
                                            confirming_delete.set(false);
                                            local_bump += 1;
                                        }
                                        Err(e) => action_msg.set(format!("Erreur : {e}")),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Confirmer la suppression"
                        }
                        button {
                            class: "secondary",
                            disabled: busy(),
                            onclick: move |_| confirming_delete.set(false),
                            "Annuler"
                        }
                    }
                }
            }

            if busy() {
                div {
                    div { class: "progress-bar" }
                    div { class: "progress-hint",
                        "Analyse OCR en cours… (cela peut prendre 10 à 30 secondes)"
                    }
                }
            }
        }
    }
}

#[component]
fn AddressesList(
    addresses: Vec<TaxNoticeAddress>,
    all_units: Vec<UnitItem>,
    property_id: Uuid,
    fees_total_cents: i64,
    cotisations_scindees_cents: i64,
    fees_uploaded: bool,
    mut local_bump: Signal<u64>,
    mut action_msg: Signal<String>,
) -> Element {
    // Filtre 1 : ne garder que les lots du bien concerné par cet avis
    let units_of_property: Vec<UnitItem> = all_units
        .iter()
        .filter(|u| u.property_id == property_id)
        .cloned()
        .collect();

    // Filtre 2 : ensemble des unit_id déjà utilisés (toutes adresses confondues)
    let mut all_used: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
    for addr in &addresses {
        for u in &addr.units {
            all_used.insert(u.unit_id);
        }
    }
    let all_used_snapshot = all_used.clone();

    rsx! {
        section { class: "panel",
            div { class: "panel-head", h3 { "Adresses et attribution des lots" } }

            for addr in addresses.iter() {
                {
                    // used_elsewhere = lots utilisés SAUF ceux de cette adresse
                    let mut used_elsewhere: std::collections::HashSet<Uuid> = all_used_snapshot.clone();
                    for u in &addr.units {
                        used_elsewhere.remove(&u.unit_id);
                    }
                    let addr_clone = addr.clone();
                    rsx! {
                        AddressRow {
                            address: addr_clone,
                            all_units: units_of_property.clone(),
                            used_elsewhere,
                            fees_total_cents,
                            cotisations_scindees_cents,
                            fees_uploaded,
                            local_bump,
                            action_msg,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn AddressRow(
    address: TaxNoticeAddress,
    all_units: Vec<UnitItem>,
    used_elsewhere: std::collections::HashSet<Uuid>,
    fees_total_cents: i64,
    cotisations_scindees_cents: i64,
    fees_uploaded: bool,
    mut local_bump: Signal<u64>,
    mut action_msg: Signal<String>,
) -> Element {
    let addr_id = address.id;
    let tax_cents = address.tax_amount_cents;
    let used_elsewhere_snapshot = used_elsewhere.clone();

    let mut shares = use_signal(|| {
        address
            .units
            .iter()
            .map(|u| (u.unit_id, u.share_bp))
            .collect::<Vec<_>>()
    });

    let mut new_unit_id = use_signal(String::new);
    let mut new_share_pct = use_signal(|| "100".to_string());

    let shares_snapshot = shares();
    let total_bp: i32 = shares_snapshot.iter().map(|(_, bp)| *bp).sum();
    let total_pct = total_bp / 100;
    let valid = total_bp == 10_000;
    let remaining_bp = 10_000 - total_bp;
    let remaining_pct = remaining_bp / 100;
    let remaining_cents = if remaining_bp > 0 {
        apply_share_bp(tax_cents, remaining_bp)
    } else {
        0
    };

    // L'adresse est-elle scindée (= plusieurs lots) ?
    let is_scindee = shares_snapshot.len() > 1;

    let find_label = |uid: Uuid| -> String {
        all_units
            .iter()
            .find(|u| u.id == uid)
            .map(|u| format!("{} • {}", u.property_name, u.label))
            .unwrap_or_else(|| uid.to_string())
    };

    // Frais de gestion : uniquement si l'adresse est scindée (plusieurs lots)
    // et prorata sur le total des cotisations des adresses scindées.
    let calc_frais = |amount_cents: i64| -> i64 {
        if !is_scindee || cotisations_scindees_cents <= 0 || fees_total_cents <= 0 {
            0
        } else {
            ((amount_cents as i128 * fees_total_cents as i128) / cotisations_scindees_cents as i128) as i64
        }
    };

    rsx! {
        div {
            class: "panel",
            style: "margin-bottom: 16px; border-left: 4px solid var(--accent);",

            div {
                class: "panel-head",
                style: "display: flex; justify-content: space-between; align-items: center; gap: 16px;",
                div {
                    div {
                        style: "font-size: 1.1rem; font-weight: 700; color: var(--accent);",
                        "🏠 {address.address_label}"
                    }
                    div {
                        style: "font-size: 1.35rem; font-weight: 700; color: #10b981; margin-top: 4px;",
                        "💰 Cotisation totale : {euro(tax_cents)}"
                    }
                }
                div {
                    style: "text-align: right;",
                    span {
                        style: if valid {
                            "font-size: 1rem; font-weight: 700; padding: 6px 12px; border-radius: 6px; background: rgba(16,185,129,0.15); color: #10b981;"
                        } else if total_bp == 0 {
                            "font-size: 1rem; font-weight: 700; padding: 6px 12px; border-radius: 6px; background: rgba(148,163,184,0.15); color: #94a3b8;"
                        } else {
                            "font-size: 1rem; font-weight: 700; padding: 6px 12px; border-radius: 6px; background: rgba(248,113,113,0.15); color: #f87171;"
                        },
                        if valid {
                            "✓ {total_pct} % répartis"
                        } else if total_bp == 0 {
                            "Aucun lot attribué"
                        } else {
                            "⚠ {total_pct} % — reste {remaining_pct} %"
                        }
                    }
                }
            }

            div {
                style: "font-size: 0.78rem; color: #94a3b8; margin: 8px 0 12px 0; padding: 8px 12px; background: rgba(148,163,184,0.08); border-radius: 6px;",
                if shares_snapshot.is_empty() {
                    "Ajoutez chaque lot avec sa part en %. Sauvegarde automatique."
                } else if !valid && remaining_bp > 0 {
                    {format!(
                        "Il reste {} à répartir ({} %).",
                        euro(remaining_cents),
                        remaining_pct
                    )}
                } else if valid {
                    if is_scindee {
                        "✓ Répartition complète. Frais de gestion répartis au prorata."
                    } else {
                        "✓ Répartition complète."
                    }
                } else {
                    "⚠ Le total dépasse 100 %."
                }
            }

            if !shares_snapshot.is_empty() {
                div {
                    style: "display: flex; flex-direction: column; gap: 6px; margin-bottom: 12px;",
                    for (unit_id, bp) in shares_snapshot.iter().cloned() {
                        {
                            let cotisation_lot = apply_share_bp(tax_cents, bp);
                            let frais_lot = calc_frais(cotisation_lot);
                            let total_facture = cotisation_lot + frais_lot;
                            let is_vacant_lot = address.units.iter()
                                .find(|u| u.unit_id == unit_id)
                                .map(|u| u.is_vacant)
                                .unwrap_or(false);
                            let inv_id = address.units.iter()
                                .find(|u| u.unit_id == unit_id)
                                .and_then(|u| u.dolibarr_invoice_id);

                            rsx! {
                                div {
                                    key: "{unit_id}",
                                    class: "data-row",
                                    style: "background: rgba(255,255,255,0.03); border-radius: 6px; padding: 12px 14px; display: flex; justify-content: space-between; align-items: center; gap: 12px;",
                                    div {
                                        style: "flex: 1; min-width: 0;",
                                        div { style: "display: flex; align-items: center; gap: 8px;",
                                            span { class: "data-title", "{find_label(unit_id)}" }
                                            if is_vacant_lot {
                                                span {
                                                    style: "font-size: 0.65rem; font-weight: 700; padding: 2px 8px; border-radius: 4px; background: rgba(251,146,60,0.15); color: #fb923c; white-space: nowrap;",
                                                    "VACANT"
                                                }
                                            }
                                        }
                                        div {
                                            style: "font-size: 0.75rem; color: #94a3b8; margin-top: 2px;",
                                            {format!("{} % du total", bp / 100)}
                                        }
                                    }
                                    div {
                                        style: "display: flex; align-items: center; gap: 12px;",
                                        if is_vacant_lot {
                                            div {
                                                style: "text-align: right; min-width: 180px;",
                                                div {
                                                    style: "font-size: 0.85rem; color: #94a3b8; font-style: italic;",
                                                    "Reste à charge SCI"
                                                }
                                            }
                                        } else if is_scindee && fees_uploaded {
                                            // Adresse scindée + feuillet 2 : détail cotisation + frais
                                            div {
                                                style: "text-align: right; min-width: 180px;",
                                                div {
                                                    style: "font-size: 0.85rem; color: #94a3b8;",
                                                    {format!("Cotisation : {}", euro(cotisation_lot))}
                                                }
                                                if frais_lot > 0 {
                                                    div {
                                                        style: "font-size: 0.85rem; color: #94a3b8;",
                                                        {format!("+ Frais de gestion : {}", euro(frais_lot))}
                                                    }
                                                }
                                                div {
                                                    style: "font-size: 1.05rem; font-weight: 700; color: #10b981; margin-top: 4px; padding-top: 4px; border-top: 1px solid rgba(255,255,255,0.1);",
                                                    {format!("= Facture : {}", euro(total_facture))}
                                                }
                                            }
                                        } else if is_scindee && !fees_uploaded {
                                            // Adresse scindée, feuillet 2 pas encore importé
                                            div {
                                                style: "text-align: right; min-width: 180px;",
                                                div {
                                                    style: "font-size: 1.05rem; font-weight: 700; color: #f1f5f9;",
                                                    {euro(cotisation_lot)}
                                                }
                                                div {
                                                    style: "font-size: 0.72rem; color: #94a3b8; margin-top: 2px;",
                                                    "En attente du feuillet 2"
                                                }
                                            }
                                        }
                                        // Adresse non scindée (1 seul lot) : rien à droite,
                                        // le montant est déjà dans le header "Cotisation totale"
                                        if let Some(id) = inv_id {
                                            a {
                                                href: "http://localhost:8081/compta/facture/card.php?facid={id}",
                                                target: "_blank",
                                                style: "font-size: 0.75rem; padding: 6px 10px; border-radius: 6px; background: rgba(16,185,129,0.15); color: #10b981; text-decoration: none; white-space: nowrap;",
                                                "📄 Voir la facture"
                                            }
                                        }
                                        button {
                                            class: "secondary",
                                            style: "font-size: 0.78rem; padding: 6px 12px; color: #f87171; border-color: rgba(248,113,113,0.3); white-space: nowrap;",
                                            onclick: {
                                                let uuid = unit_id;
                                                move |_| {
                                                    let mut cur = shares();
                                                    cur.retain(|(u, _)| *u != uuid);
                                                    shares.set(cur.clone());

                                                    let inputs: Vec<ShareInput> = cur
                                                        .iter()
                                                        .map(|(u, bp)| ShareInput { unit_id: *u, share_bp: *bp })
                                                        .collect();
                                                    spawn(async move {
                                                        match set_address_shares(addr_id, inputs).await {
                                                            Ok(_) => local_bump += 1,
                                                            Err(e) => action_msg.set(format!("Erreur sauvegarde : {e}")),
                                                        }
                                                    });
                                                }
                                            },
                                            "🗑 Effacer"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !valid {
                div {
                    class: "form-grid",
                    style: "grid-template-columns: 2fr 1fr auto; gap: 8px; align-items: end;",
                    label { class: "field",
                        span { "Ajouter un lot" }
                        select {
                            value: new_unit_id(),
                            onchange: move |e: FormEvent| new_unit_id.set(e.value()),
                            option { value: "", "— Sélectionner un lot —" }
                            for u in all_units.iter() {
                                if !used_elsewhere_snapshot.contains(&u.id)
                                    && !shares_snapshot.iter().any(|(uid, _)| *uid == u.id)
                                {
                                    option {
                                        value: u.id.to_string(),
                                        {format!("{} • {}", u.property_name, u.label)}
                                    }
                                }
                            }
                        }
                    }
                    label { class: "field",
                        span { "Part en % (1 à 100)" }
                        input {
                            r#type: "number",
                            min: "1",
                            max: "100",
                            value: "{new_share_pct}",
                            oninput: move |e: FormEvent| new_share_pct.set(e.value()),
                        }
                    }
                    button {
                        class: "secondary",
                        onclick: move |_| {
                            let uid = new_unit_id();
                            if uid.is_empty() { return; }
                            let pct: i32 = new_share_pct().parse().unwrap_or(0);
                            if !(1..=100).contains(&pct) {
                                action_msg.set("La part doit être entre 1 et 100 %".into());
                                return;
                            }
                            let bp = pct * 100;
                            let Ok(uuid) = Uuid::parse_str(&uid) else { return; };
                            let mut cur = shares();
                            if cur.iter().any(|(u, _)| *u == uuid) {
                                action_msg.set("Ce lot est déjà attribué à cette adresse".into());
                                return;
                            }
                            if used_elsewhere_snapshot.contains(&uuid) {
                                action_msg.set("Ce lot est déjà attribué à une autre adresse de cet avis".into());
                                return;
                            }
                            cur.push((uuid, bp));
                            shares.set(cur.clone());
                            new_unit_id.set(String::new());
                            new_share_pct.set("100".into());

                            let inputs: Vec<ShareInput> = cur
                                .iter()
                                .map(|(u, bp)| ShareInput { unit_id: *u, share_bp: *bp })
                                .collect();
                            let label = address.address_label.clone();
                            spawn(async move {
                                match set_address_shares(addr_id, inputs).await {
                                    Ok(_) => {
                                        action_msg.set(format!("✓ {} : répartition enregistrée", label));
                                        local_bump += 1;
                                    }
                                    Err(e) => action_msg.set(format!("Erreur sauvegarde : {e}")),
                                }
                            });
                        },
                        "+ Ajouter le lot"
                    }
                }
            }
        }
    }
}

fn status_label(s: &str) -> &'static str {
    match s {
        "AWAITING_DOCS"   => "En attente de feuillets",
        "AWAITING_REVIEW" => "À valider",
        "READY"           => "Prêt à facturer",
        "INVOICED"        => "Facturé",
        "ARCHIVED"        => "Archivé",
        _ => "?",
    }
}