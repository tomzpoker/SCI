use dioxus::prelude::*;
use uuid::Uuid;

use super::models::*;
use super::parser::{parse_bases_sheet, parse_fees_sheet};

// ============================================================
//  Facture fournisseur DGFiP (à payer par la SCI)
// ============================================================

#[cfg(feature = "server")]
const DGFIP_SUPPLIER_NAME: &str = "DGFiP - TRESOR PUBLIC";

#[cfg(feature = "server")]
async fn create_dgfip_supplier_invoice(
    pool_fourn: &sqlx::PgPool,
    notice_id: Uuid,
    total_cents: i64,
    fiscal_year: i32,
    property_name: &str,
    notice_reference: &str,
) -> Result<i64, String> {
    if total_cents <= 0 {
        return Err("Montant total invalide".into());
    }

    // Suffixe = nom du bien (majuscules, sans espaces, tronqué à 30 caractères)
    let property_slug: String = property_name
        .to_uppercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    let property_slug = if property_slug.is_empty() {
        "BIEN".to_string()
    } else {
        property_slug.chars().take(30).collect()
    };

    let ref_facture = format!("TF{}-{}", fiscal_year, property_slug);
    let ref_safe = ref_facture.replace('\'', "''");

    let check_sql = format!(
        "SELECT rowid FROM llx_facture_fourn WHERE entity=1 AND ref = '{}' LIMIT 1;",
        ref_safe
    );
    let check_out = tokio::process::Command::new("docker")
        .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", &check_sql])
        .output()
        .await
        .map_err(|e| format!("Erreur SQL check facture : {}", e))?;
    let existing = String::from_utf8_lossy(&check_out.stdout).trim().to_string();
    if let Ok(id) = existing.parse::<i64>() {
        // Facture déjà créée : on s'assure que la notice est bien liée
        let _ = sqlx::query(
            "UPDATE property_tax_notices SET dgfip_invoice_id = $2, dgfip_invoice_ref = $3 WHERE id = $1",
        )
        .bind(notice_id)
        .bind(id)
        .bind(&ref_facture)
        .execute(pool_fourn)
        .await;
        return Ok(id);
    }

    let supplier_safe = DGFIP_SUPPLIER_NAME.replace('\'', "''");
    let sql_find = format!(
        "SELECT rowid FROM llx_societe WHERE entity=1 AND nom = '{}' AND fournisseur = 1 LIMIT 1;",
        supplier_safe
    );
    let find_out = tokio::process::Command::new("docker")
        .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", &sql_find])
        .output()
        .await
        .map_err(|e| format!("Erreur SQL find supplier : {}", e))?;
    let found = String::from_utf8_lossy(&find_out.stdout).trim().to_string();

    let supplier_id: i64 = if found.is_empty() {
        let sql_create = format!(
            "INSERT INTO llx_societe (nom, entity, client, fournisseur, status, datec) \
             VALUES ('{}', 1, 0, 1, 1, NOW()) RETURNING rowid;",
            supplier_safe
        );
        let create_out = tokio::process::Command::new("docker")
            .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", &sql_create])
            .output()
            .await
            .map_err(|e| format!("Erreur SQL create supplier : {}", e))?;
        String::from_utf8_lossy(&create_out.stdout).trim().parse::<i64>()
            .map_err(|_| "Impossible de créer le fournisseur DGFiP".to_string())?
    } else {
        found.parse().map_err(|_| "ID fournisseur invalide".to_string())?
    };

    let total_eur = (total_cents as f64) / 100.0;

    let note = format!(
        "Taxe foncière {} — Bien : {} — Avis : {}",
        fiscal_year,
        property_name,
        if notice_reference.is_empty() { "—" } else { notice_reference }
    ).replace('\'', "''");

    let sql_inv = format!(
        "INSERT INTO llx_facture_fourn (ref, ref_supplier, entity, fk_soc, datec, datef, date_lim_reglement, total_ht, total_tva, total_ttc, fk_statut, paye, note_private) \
         VALUES ('{}', '{}', 1, {}, NOW(), CURRENT_DATE, NULL, {}, 0, {}, 0, 0, '{}');",
        ref_safe, ref_safe, supplier_id, total_eur, total_eur, note
    );
    let inv_out = tokio::process::Command::new("docker")
        .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-c", &sql_inv])
        .output()
        .await
        .map_err(|e| format!("Erreur SQL facture fourn : {}", e))?;

    if !inv_out.status.success() {
        return Err(format!(
            "Erreur création facture fournisseur : {}",
            String::from_utf8_lossy(&inv_out.stderr)
        ));
    }

    let get_out = tokio::process::Command::new("docker")
        .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-t", "-A", "-c", &check_sql])
        .output()
        .await
        .map_err(|e| format!("Erreur SQL get id : {}", e))?;
    let invoice_id: i64 = String::from_utf8_lossy(&get_out.stdout).trim().parse::<i64>()
        .map_err(|_| "ID facture fournisseur introuvable".to_string())?;

    let desc = format!("Taxe foncière {} — {}", fiscal_year, property_name).replace('\'', "''");
    let sql_line = format!(
        "INSERT INTO llx_facture_fourn_det (fk_facture_fourn, description, qty, tva_tx, total_ht, total_tva, total_ttc) \
         VALUES ({}, '{}', 1, 0, {}, 0, {});",
        invoice_id, desc, total_eur, total_eur
    );
    let _ = tokio::process::Command::new("docker")
        .args(["exec", "sci-family-dolibarr-db", "psql", "-U", "dolibarr", "-d", "dolibarr", "-c", &sql_line])
        .output()
        .await;

    // Persiste l'ID et la ref sur la notice
    let _ = sqlx::query(
        "UPDATE property_tax_notices SET dgfip_invoice_id = $2, dgfip_invoice_ref = $3 WHERE id = $1",
    )
    .bind(notice_id)
    .bind(invoice_id)
    .bind(&ref_facture)
    .execute(pool_fourn)
    .await;

    Ok(invoice_id)
}

// ============================================================
//  Rattachement des feuillets à Dolibarr (GED)
// ============================================================

#[cfg(feature = "server")]
async fn attach_notice_documents_to_dolibarr(
    notice_id: Uuid,
    property_name: &str,
    fourn_invoice_id: Option<i64>,
    fourn_invoice_ref: Option<&str>,
) -> Result<usize, String> {
    use sqlx::Row;

    let pool = crate::infrastructure::db().await.map_err(|e| e.to_string())?;

    let docs = sqlx::query(
        "SELECT id, document_kind, file_name, file_bytes FROM property_tax_notice_documents \
         WHERE notice_id = $1 AND file_bytes IS NOT NULL",
    )
    .bind(notice_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut uploaded = 0usize;

    for d in docs {
        let doc_id: Uuid = d.get("id");
        let kind: String = d.get("document_kind");
        let file_name: String = d.get("file_name");
        let file_bytes: Option<Vec<u8>> = d.get("file_bytes");
        let Some(bytes) = file_bytes else { continue; };
        if bytes.is_empty() { continue; }

        let prefix = format!("TF-{}-{}", property_name.replace(' ', "_"), kind);
        let full_name = format!("{}_{}", prefix, file_name);

        if let Err(e) = crate::dolibarr::server_fns::dolibarr_upload_patrimoine_document(
            property_name.to_string(),
            String::new(),
            full_name.clone(),
            bytes.clone(),
        )
        .await
        {
            tracing::warn!("Upload GED patrimoine échoué pour {} : {}", full_name, e);
        } else {
            uploaded += 1;
        }

        if let (Some(inv_id), Some(inv_ref)) = (fourn_invoice_id, fourn_invoice_ref) {
            let _ = crate::dolibarr::server_fns::dolibarr_upload_document_sql(
                "facture_fournisseur".to_string(),
                inv_id.to_string(),
                inv_ref.to_string(),
                full_name.clone(),
                bytes.clone(),
            )
            .await;

            if let Ok(ecm_id) = crate::dolibarr::server_fns::dolibarr_get_last_ecm_id().await {
                let _ = sqlx::query(
                    "UPDATE property_tax_notice_documents SET dolibarr_ecm_id = $2 WHERE id = $1",
                )
                .bind(doc_id)
                .bind(ecm_id)
                .execute(pool)
                .await;
            }
        }
    }

    Ok(uploaded)
}

// ============================================================
//  Liste des avis TF
// ============================================================

#[server]
pub async fn list_tax_notices() -> Result<Vec<TaxNoticeSummary>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = crate::entity_scope::current_legal_entity_id();

        let rows = sqlx::query(
            r#"SELECT n.id, n.property_id, p.name AS property_name, n.fiscal_year,
                      n.total_amount_cents, n.cotisations_amount_cents,
                      n.management_fees_cents, n.status, n.created_at,
                      (SELECT COUNT(*)::bigint FROM property_tax_notice_addresses a
                         WHERE a.notice_id = n.id) AS addresses_count,
                      (SELECT COUNT(*)::bigint FROM property_tax_notice_lines l
                         JOIN property_tax_notice_addresses a ON a.id = l.address_id
                         WHERE a.notice_id = n.id) AS lines_count
                 FROM property_tax_notices n
                 JOIN properties p ON p.id = n.property_id
                WHERE n.legal_entity_id = $1
                ORDER BY n.fiscal_year DESC, p.name"#,
        )
        .bind(entity)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(rows.into_iter().map(|r| TaxNoticeSummary {
            id: r.get("id"),
            property_id: r.get("property_id"),
            property_name: r.get("property_name"),
            fiscal_year: r.get("fiscal_year"),
            total_amount_cents: r.get("total_amount_cents"),
            cotisations_amount_cents: r.get("cotisations_amount_cents"),
            management_fees_cents: r.get("management_fees_cents"),
            status: r.get("status"),
            addresses_count: r.get("addresses_count"),
            lines_count: r.get("lines_count"),
            created_at: r.get("created_at"),
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_tax_notices est exécutée côté serveur"))
}

#[server]
pub async fn create_or_get_notice(
    property_id: Uuid,
    fiscal_year: i32,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = crate::entity_scope::current_legal_entity_id();

        let row = sqlx::query_scalar::<_, Uuid>(
            r#"INSERT INTO property_tax_notices
                   (legal_entity_id, property_id, fiscal_year, status)
               VALUES ($1, $2, $3, 'AWAITING_DOCS')
               ON CONFLICT (legal_entity_id, property_id, fiscal_year)
               DO UPDATE SET updated_at = NOW()
               RETURNING id"#,
        )
        .bind(entity)
        .bind(property_id)
        .bind(fiscal_year)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(row)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("create_or_get_notice est exécutée côté serveur"))
}

#[server]
pub async fn get_tax_notice_detail(notice_id: Uuid) -> Result<TaxNoticeDetail, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = crate::entity_scope::current_legal_entity_id();

        let n = sqlx::query(
            r#"SELECT n.id, n.property_id, p.name AS property_name, n.fiscal_year,
                      COALESCE(n.notice_reference,'') AS notice_reference,
                      n.total_amount_cents, n.cotisations_amount_cents,
                      n.management_fees_cents, n.status, n.notes,
                      n.dgfip_invoice_id, n.dgfip_invoice_ref,
                      n.created_at, n.updated_at
                 FROM property_tax_notices n
                 JOIN properties p ON p.id = n.property_id
                WHERE n.id = $1 AND n.legal_entity_id = $2"#,
        )
        .bind(notice_id)
        .bind(entity)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Avis introuvable"))?;

        let docs_rows = sqlx::query(
            r#"SELECT id, notice_id, document_kind, file_name, ocr_confidence, uploaded_at
                 FROM property_tax_notice_documents WHERE notice_id = $1
                ORDER BY uploaded_at"#,
        )
        .bind(notice_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let documents: Vec<TaxNoticeDocument> = docs_rows.into_iter().map(|r| TaxNoticeDocument {
            id: r.get("id"),
            notice_id: r.get("notice_id"),
            document_kind: r.get("document_kind"),
            file_name: r.get("file_name"),
            ocr_confidence: r.get("ocr_confidence"),
            uploaded_at: r.get("uploaded_at"),
        }).collect();

        let addr_rows = sqlx::query(
            r#"SELECT id, notice_id, address_label, base_amount_cents, tax_amount_cents, display_order
                 FROM property_tax_notice_addresses
                WHERE notice_id = $1
                ORDER BY display_order, address_label"#,
        )
        .bind(notice_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut addresses: Vec<TaxNoticeAddress> = Vec::new();
        for a in addr_rows {
            let addr_id: Uuid = a.get("id");
            let unit_rows = sqlx::query(
                r#"SELECT id, unit_id, share_bp, amount_cents, is_vacant, status,
                          dolibarr_invoice_id, dolibarr_invoice_ref, error_message
                     FROM property_tax_notice_lines WHERE address_id = $1"#,
            )
            .bind(addr_id)
            .fetch_all(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            let units: Vec<TaxNoticeAddressUnit> = unit_rows.into_iter().map(|u| {
                let uid: Uuid = u.get("unit_id");
                TaxNoticeAddressUnit {
                    id: Some(u.get("id")),
                    unit_id: uid,
                    unit_label: String::new(),
                    share_bp: u.get("share_bp"),
                    amount_cents: u.get("amount_cents"),
                    is_vacant: u.get("is_vacant"),
                    status: u.get("status"),
                    dolibarr_invoice_id: u.get("dolibarr_invoice_id"),
                    dolibarr_invoice_ref: u.get("dolibarr_invoice_ref"),
                    error_message: u.get("error_message"),
                }
            }).collect();
            addresses.push(TaxNoticeAddress {
                id: addr_id,
                notice_id: a.get("notice_id"),
                address_label: a.get("address_label"),
                base_amount_cents: a.get("base_amount_cents"),
                tax_amount_cents: a.get("tax_amount_cents"),
                display_order: a.get("display_order"),
                units,
            });
        }

        let fees_rows = sqlx::query(
            r#"SELECT id, notice_id, fee_label, fee_amount_cents, distribution_mode
                 FROM property_tax_notice_fees WHERE notice_id = $1 ORDER BY created_at"#,
        )
        .bind(notice_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let fees: Vec<TaxNoticeFee> = fees_rows.into_iter().map(|r| TaxNoticeFee {
            id: r.get("id"),
            notice_id: r.get("notice_id"),
            fee_label: r.get("fee_label"),
            fee_amount_cents: r.get("fee_amount_cents"),
            distribution_mode: r.get("distribution_mode"),
        }).collect();

        Ok(TaxNoticeDetail {
            id: n.get("id"),
            property_id: n.get("property_id"),
            property_name: n.get("property_name"),
            fiscal_year: n.get("fiscal_year"),
            notice_reference: n.get("notice_reference"),
            total_amount_cents: n.get("total_amount_cents"),
            cotisations_amount_cents: n.get("cotisations_amount_cents"),
            management_fees_cents: n.get("management_fees_cents"),
            status: n.get("status"),
            notes: n.get("notes"),
            documents,
            addresses,
            fees,
            dgfip_invoice_id: n.get("dgfip_invoice_id"),
            dgfip_invoice_ref: n.get("dgfip_invoice_ref"),
            created_at: n.get("created_at"),
            updated_at: n.get("updated_at"),
        })
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("get_tax_notice_detail est exécutée côté serveur"))
}

#[server]
pub async fn upload_tax_notice_document(
    notice_id: Uuid,
    document_kind: String,
    file_name: String,
    bytes: Vec<u8>,
) -> Result<UploadTaxNoticeResult, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if !["BASES", "FEES", "OTHER"].contains(&document_kind.as_str()) {
            return Err(ServerFnError::new("Type de feuillet inconnu"));
        }

        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;

        let raw_text = if bytes.starts_with(b"%PDF") {
            match crate::dolibarr::invoice_parser::extract_text_from_pdf(&bytes) {
                Ok(t) if t.trim().len() > 100 => t,
                _ => crate::dolibarr::server_fns::dolibarr_ocr_pdf(bytes.clone())
                    .await
                    .map_err(ServerFnError::new)?,
            }
        } else {
            crate::dolibarr::server_fns::dolibarr_ocr_pdf(bytes.clone())
                .await
                .map_err(ServerFnError::new)?
        };

        let (parsed_summary, confidence) = match document_kind.as_str() {
            "BASES" => {
                let p = parse_bases_sheet(&raw_text);
                let right_text = crate::dolibarr::server_fns::dolibarr_ocr_right_column(bytes.clone())
                    .await
                    .unwrap_or_default();
                tracing::info!("OCR colonne droite brute :\n{}", right_text);
                let labels: Vec<String> = p.addresses.iter().map(|a| a.label.clone()).collect();
                let merged = super::parser::merge_addresses_with_totals(&labels, &right_text);
                let conf = p.confidence;
                let s = ParsedOcrSummary {
                    notice_reference: p.notice_reference.clone(),
                    fiscal_year: None,
                    addresses: merged.iter().map(|a| ParsedAddressSummary {
                        label: a.label.clone(),
                        base_cents: a.base_cents,
                        tax_cents: a.tax_cents,
                    }).collect(),
                    total_amount_cents: None,
                    cotisations_amount_cents: None,
                    management_fees_cents: None,
                };
                (s, conf)
            }
            "FEES" => {
                let column_text = crate::dolibarr::server_fns::dolibarr_ocr_fees_column(bytes.clone())
                    .await
                    .unwrap_or_default();
                tracing::info!("OCR colonne droite FEES brute :\n{}", column_text);
                let p = parse_fees_sheet(&raw_text, &column_text);
                let conf = p.confidence;
                let s = ParsedOcrSummary {
                    notice_reference: p.notice_reference.clone(),
                    fiscal_year: p.fiscal_year,
                    addresses: vec![],
                    total_amount_cents: Some(p.total_amount_cents),
                    cotisations_amount_cents: Some(p.cotisations_amount_cents),
                    management_fees_cents: Some(p.management_fees_cents),
                };
                (s, conf)
            }
            _ => (
                ParsedOcrSummary {
                    notice_reference: String::new(),
                    fiscal_year: None,
                    addresses: vec![],
                    total_amount_cents: None,
                    cotisations_amount_cents: None,
                    management_fees_cents: None,
                },
                0,
            ),
        };

        let extracted_json = serde_json::to_value(&parsed_summary).unwrap_or(serde_json::json!({}));

        let mime_type = if bytes.starts_with(b"%PDF") {
            "application/pdf"
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            "image/jpeg"
        } else if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            "image/png"
        } else {
            "application/octet-stream"
        };

        sqlx::query(
            r#"INSERT INTO property_tax_notice_documents
                   (notice_id, document_kind, file_name, ocr_raw_text, ocr_extracted, ocr_confidence, file_bytes, mime_type)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(notice_id)
        .bind(&document_kind)
        .bind(&file_name)
        .bind(&raw_text)
        .bind(&extracted_json)
        .bind(confidence)
        .bind(&bytes)
        .bind(mime_type)
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        if document_kind == "BASES" {
            sqlx::query("DELETE FROM property_tax_notice_addresses WHERE notice_id = $1")
                .bind(notice_id)
                .execute(pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

            for (i, a) in parsed_summary.addresses.iter().enumerate() {
                sqlx::query(
                    r#"INSERT INTO property_tax_notice_addresses
                           (notice_id, address_label, base_amount_cents, tax_amount_cents, display_order)
                       VALUES ($1, $2, $3, $4, $5)"#,
                )
                .bind(notice_id)
                .bind(&a.label)
                .bind(a.base_cents)
                .bind(a.tax_cents)
                .bind(i as i32)
                .execute(pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            }
        } else if document_kind == "FEES" {
            sqlx::query("DELETE FROM property_tax_notice_fees WHERE notice_id = $1")
                .bind(notice_id)
                .execute(pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

            if let Some(total) = parsed_summary.total_amount_cents {
                sqlx::query(
                    r#"UPDATE property_tax_notices
                          SET total_amount_cents = $2,
                              cotisations_amount_cents = $3,
                              management_fees_cents = $4,
                              notice_reference = COALESCE(NULLIF($5,''), notice_reference),
                              updated_at = NOW()
                        WHERE id = $1"#,
                )
                .bind(notice_id)
                .bind(total)
                .bind(parsed_summary.cotisations_amount_cents)
                .bind(parsed_summary.management_fees_cents)
                .bind(&parsed_summary.notice_reference)
                .execute(pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            }

            if let Some(fees) = parsed_summary.management_fees_cents {
                if fees > 0 {
                    sqlx::query(
                        r#"INSERT INTO property_tax_notice_fees
                               (notice_id, fee_label, fee_amount_cents, distribution_mode)
                           VALUES ($1, 'Frais de gestion', $2, 'PRORATA_TAX')"#,
                    )
                    .bind(notice_id)
                    .bind(fees)
                    .execute(pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
                }
            }
        }

        let row: (i64, i64) = sqlx::query_as(
            r#"SELECT
                 (SELECT COUNT(*)::bigint FROM property_tax_notice_addresses WHERE notice_id = $1),
                 (SELECT COUNT(*)::bigint FROM property_tax_notice_documents WHERE notice_id = $1)"#,
        )
        .bind(notice_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let (addr_count, doc_count) = row;

        let new_status = if addr_count > 0 && doc_count >= 2 {
            "AWAITING_REVIEW"
        } else {
            "AWAITING_DOCS"
        };

        sqlx::query(
            "UPDATE property_tax_notices SET status = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(notice_id)
        .bind(new_status)
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(UploadTaxNoticeResult {
            document_kind,
            ocr_confidence: confidence,
            notice_status: new_status.to_string(),
            parsed_summary,
        })
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("upload_tax_notice_document est exécutée côté serveur"))
}

#[server]
pub async fn update_tax_notice_addresses(
    input: UpdateAddressesInput,
) -> Result<Vec<TaxNoticeAddress>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let mut tx = pool.begin().await.map_err(|e| ServerFnError::new(e.to_string()))?;

        sqlx::query("DELETE FROM property_tax_notice_addresses WHERE notice_id = $1")
            .bind(input.notice_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        for (i, a) in input.addresses.iter().enumerate() {
            sqlx::query(
                r#"INSERT INTO property_tax_notice_addresses
                       (notice_id, address_label, base_amount_cents, tax_amount_cents, display_order)
                   VALUES ($1, $2, $3, $4, $5)"#,
            )
            .bind(input.notice_id)
            .bind(&a.label)
            .bind(a.base_cents)
            .bind(a.tax_cents)
            .bind(i as i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        tx.commit().await.map_err(|e| ServerFnError::new(e.to_string()))?;

        let detail = get_tax_notice_detail(input.notice_id).await?;
        Ok(detail.addresses)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("update_tax_notice_addresses est exécutée côté serveur"))
}

#[server]
pub async fn set_address_shares(
    address_id: Uuid,
    shares: Vec<ShareInput>,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use super::calculation::apply_share_bp;

        let bp: Vec<i32> = shares.iter().map(|s| s.share_bp).collect();
        let total_bp: i32 = bp.iter().sum();
        if total_bp > 10_000 {
            return Err(ServerFnError::new(format!(
                "La somme des parts ne peut pas dépasser 100% (actuelle : {}%)",
                total_bp / 100
            )));
        }

        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;

        let tax_cents: i64 = sqlx::query_scalar(
            "SELECT tax_amount_cents FROM property_tax_notice_addresses WHERE id = $1",
        )
        .bind(address_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        // Vérifie qu'aucun de ces unit_id n'est déjà attribué à une AUTRE adresse de la même notice
        let notice_id: Uuid = sqlx::query_scalar(
            "SELECT notice_id FROM property_tax_notice_addresses WHERE id = $1",
        )
        .bind(address_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        if !shares.is_empty() {
            let unit_ids: Vec<Uuid> = shares.iter().map(|s| s.unit_id).collect();
            let conflicts: i64 = sqlx::query_scalar(
                r#"SELECT COUNT(*)::bigint
                     FROM property_tax_notice_lines l
                     JOIN property_tax_notice_addresses a ON a.id = l.address_id
                    WHERE a.notice_id = $1
                      AND a.id <> $2
                      AND l.unit_id = ANY($3)"#,
            )
            .bind(notice_id)
            .bind(address_id)
            .bind(&unit_ids)
            .fetch_one(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

            if conflicts > 0 {
                return Err(ServerFnError::new(
                    "Un ou plusieurs lots sont déjà attribués à une autre adresse de cet avis"
                        .to_string(),
                ));
            }
        }

        let mut tx = pool.begin().await.map_err(|e| ServerFnError::new(e.to_string()))?;

        let invoiced: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM property_tax_notice_lines \
             WHERE address_id = $1 AND status = 'invoiced'",
        )
        .bind(address_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        if invoiced > 0 {
            return Err(ServerFnError::new(
                "Une adresse déjà facturée ne peut plus être modifiée".to_string(),
            ));
        }

        sqlx::query("DELETE FROM property_tax_notice_lines WHERE address_id = $1")
            .bind(address_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        for s in &shares {
            let amount = apply_share_bp(tax_cents, s.share_bp);

            let has_active_lease: bool = sqlx::query_scalar(
                r#"SELECT EXISTS(
                       SELECT 1 FROM leases l
                        WHERE l.unit_id = $1
                          AND l.active = true
                          AND (l.end_date IS NULL OR l.end_date >= CURRENT_DATE)
                   )"#,
            )
            .bind(s.unit_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

            sqlx::query(
                r#"INSERT INTO property_tax_notice_lines
                       (address_id, unit_id, share_bp, amount_cents, is_vacant, status)
                   VALUES ($1, $2, $3, $4, $5, $6)"#,
            )
            .bind(address_id)
            .bind(s.unit_id)
            .bind(s.share_bp)
            .bind(amount)
            .bind(!has_active_lease)
            .bind(if has_active_lease { "pending" } else { "skipped_vacant" })
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        tx.commit().await.map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("set_address_shares est exécutée côté serveur"))
}

#[server]
pub async fn update_tax_notice_fees(input: UpdateFeesInput) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let mut tx = pool.begin().await.map_err(|e| ServerFnError::new(e.to_string()))?;

        sqlx::query("DELETE FROM property_tax_notice_fees WHERE notice_id = $1")
            .bind(input.notice_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        for f in &input.fees {
            sqlx::query(
                r#"INSERT INTO property_tax_notice_fees
                       (notice_id, fee_label, fee_amount_cents, distribution_mode)
                   VALUES ($1, $2, $3, $4)"#,
            )
            .bind(input.notice_id)
            .bind(&f.label)
            .bind(f.amount_cents)
            .bind(&f.distribution_mode)
            .execute(&mut *tx)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        sqlx::query(
            r#"UPDATE property_tax_notices
                  SET total_amount_cents = $2,
                      cotisations_amount_cents = $3,
                      management_fees_cents = $4,
                      updated_at = NOW()
                WHERE id = $1"#,
        )
        .bind(input.notice_id)
        .bind(input.total_amount_cents)
        .bind(input.cotisations_amount_cents)
        .bind(input.management_fees_cents)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        tx.commit().await.map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("update_tax_notice_fees est exécutée côté serveur"))
}

#[server]
pub async fn generate_tax_notice_invoices(
    notice_id: Uuid,
) -> Result<GenerateInvoicesResult, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::dolibarr::client::DolibarrClient;
        use crate::dolibarr::models::DolibarrInvoiceLine;
        use sqlx::Row;
        use std::collections::HashMap;

        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;

        let notice_info = sqlx::query(
            r#"SELECT n.total_amount_cents, n.notice_reference, n.fiscal_year,
                      p.name AS property_name
                 FROM property_tax_notices n
                 JOIN properties p ON p.id = n.property_id
                WHERE n.id = $1"#,
        )
        .bind(notice_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Avis introuvable"))?;

        let total_cents: Option<i64> = notice_info.get("total_amount_cents");
        let Some(total_cents) = total_cents else {
            return Err(ServerFnError::new(
                "Le feuillet 2 (montant total de l'impôt) doit être importé avant de générer les factures"
            ));
        };

        let fiscal_year: i32 = notice_info.get("fiscal_year");
        let notice_ref: Option<String> = notice_info.get("notice_reference");
        let property_name: String = notice_info.get("property_name");

        let rows = sqlx::query(
            r#"SELECT l.id AS line_id, l.unit_id, l.share_bp, l.amount_cents,
                      l.is_vacant, l.status AS line_status,
                      a.id AS address_id, a.address_label, a.tax_amount_cents,
                      u.label AS unit_label
                 FROM property_tax_notice_lines l
                 JOIN property_tax_notice_addresses a ON a.id = l.address_id
                 JOIN units u ON u.id = l.unit_id
                WHERE a.notice_id = $1"#,
        )
        .bind(notice_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        if rows.is_empty() {
            return Err(ServerFnError::new(
                "Aucun lot attribué : configurez d'abord les adresses".to_string(),
            ));
        }

        let fee_total: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(fee_amount_cents), 0)::bigint FROM property_tax_notice_fees WHERE notice_id = $1",
        )
        .bind(notice_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        // Frais de gestion : uniquement sur les adresses SCINDÉES (> 1 lot par adresse)
        // 1) Compte les lots par adresse
        let mut lines_per_address: HashMap<Uuid, usize> = HashMap::new();
        for r in &rows {
            let addr_id: Uuid = r.get("address_id");
            *lines_per_address.entry(addr_id).or_insert(0) += 1;
        }

        // 2) Total des cotisations des seules adresses scindées
        let total_tax_scinded: i64 = rows.iter()
            .filter(|r| {
                let addr_id: Uuid = r.get("address_id");
                lines_per_address.get(&addr_id).copied().unwrap_or(0) > 1
            })
            .map(|r| r.get::<i64, _>("amount_cents"))
            .sum();

        // 3) Prorata des frais sur ces lignes uniquement
        let scinded_lines: Vec<&sqlx::postgres::PgRow> = rows.iter()
            .filter(|r| {
                let addr_id: Uuid = r.get("address_id");
                lines_per_address.get(&addr_id).copied().unwrap_or(0) > 1
            })
            .collect();

        let fee_share_per_line: HashMap<Uuid, i64> = if fee_total > 0 && total_tax_scinded > 0 {
            let mut map = HashMap::new();
            let mut allocated = 0i64;
            for (idx, r) in scinded_lines.iter().enumerate() {
                let line_id: Uuid = r.get("line_id");
                let amount: i64 = r.get("amount_cents");
                let part = if idx + 1 == scinded_lines.len() {
                    fee_total - allocated
                } else {
                    ((amount as i128 * fee_total as i128) / total_tax_scinded as i128) as i64
                };
                map.insert(line_id, part);
                allocated += part;
            }
            map
        } else {
            HashMap::new()
        };

        let client = DolibarrClient::from_env().map_err(ServerFnError::new)?;
        let third_parties = client.list_third_parties(500).await.map_err(ServerFnError::new)?;
        let mut by_name: HashMap<String, String> = HashMap::new();
        let mut by_siret: HashMap<String, String> = HashMap::new();
        for tp in &third_parties {
            if !tp.name.is_empty() {
                by_name.insert(tp.name.trim().to_lowercase(), tp.id.clone());
            }
            if !tp.siret.is_empty() {
                by_siret.insert(tp.siret.trim().to_string(), tp.id.clone());
            }
        }

        let today_ts = chrono::Utc::now().timestamp();
        let mut created = 0usize;
        let mut skipped = 0usize;
        let mut failed = 0usize;

        for r in &rows {
            let line_id: Uuid = r.get("line_id");
            let unit_id: Uuid = r.get("unit_id");
            let line_status: String = r.get("line_status");
            let is_vacant: bool = r.get("is_vacant");
            let amount: i64 = r.get("amount_cents");
            let _address_label: String = r.get("address_label");
            let _unit_label: String = r.get("unit_label");

            if line_status == "invoiced" {
                continue;
            }
            if is_vacant || line_status == "skipped_vacant" {
                skipped += 1;
                continue;
            }

            let tp_row = sqlx::query(
                r#"SELECT COALESCE(t.legal_name,'') AS tenant_name,
                          COALESCE(t.siret,'')      AS tenant_siret
                     FROM leases l
                     JOIN tenants t ON t.id = l.tenant_id
                    WHERE l.unit_id = $1
                      AND l.active = true
                      AND (l.end_date IS NULL OR l.end_date >= CURRENT_DATE)
                    ORDER BY l.start_date DESC LIMIT 1"#,
            )
            .bind(unit_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

            let Some(tp_row) = tp_row else {
                failed += 1;
                sqlx::query(
                    "UPDATE property_tax_notice_lines SET status='failed', error_message=$2 WHERE id=$1",
                )
                .bind(line_id)
                .bind("Pas de bail actif")
                .execute(pool)
                .await
                .ok();
                continue;
            };

            let name: String = tp_row.get("tenant_name");
            let siret: String = tp_row.get("tenant_siret");
            let dolibarr_id: Option<String> = {
                if !siret.is_empty() {
                    by_siret.get(siret.trim()).cloned()
                } else {
                    None
                }
                .or_else(|| {
                    if !name.is_empty() {
                        by_name.get(&name.trim().to_lowercase()).cloned()
                    } else {
                        None
                    }
                })
            };

            let Some(tp_id) = dolibarr_id else {
                failed += 1;
                sqlx::query(
                    "UPDATE property_tax_notice_lines SET status='failed', error_message=$2 WHERE id=$1",
                )
                .bind(line_id)
                .bind(format!("Tiers Dolibarr introuvable pour '{}'", name))
                .execute(pool)
                .await
                .ok();
                continue;
            };

            let fee_part = fee_share_per_line.get(&line_id).copied().unwrap_or(0);
            let cotisation_eur = (amount as f64) / 100.0;
            let frais_eur = (fee_part as f64) / 100.0;

            let mut invoice_lines = vec![
                DolibarrInvoiceLine::new("Taxe foncière".to_string(), 1.0, cotisation_eur, 0.0)
            ];
            if frais_eur > 0.0 {
                invoice_lines.push(DolibarrInvoiceLine::new(
                    "Frais de gestion".to_string(),
                    1.0,
                    frais_eur,
                    0.0,
                ));
            }

            match client.create_invoice(&tp_id, today_ts, &invoice_lines).await {
                Ok(inv_id) => {
                    if let Err(e) = client.validate_invoice(&inv_id).await {
                        tracing::warn!("Validation facture {inv_id} échouée : {e}");
                    }
                    let inv_id_i64: Option<i64> = inv_id.parse().ok();
                    sqlx::query(
                        r#"UPDATE property_tax_notice_lines
                              SET status='invoiced',
                                  dolibarr_invoice_id=$2,
                                  dolibarr_invoice_ref=$3,
                                  error_message=NULL
                            WHERE id=$1"#,
                    )
                    .bind(line_id)
                    .bind(inv_id_i64)
                    .bind(&inv_id)
                    .execute(pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
                    created += 1;
                }
                Err(e) => {
                    failed += 1;
                    sqlx::query(
                        "UPDATE property_tax_notice_lines SET status='failed', error_message=$2 WHERE id=$1",
                    )
                    .bind(line_id)
                    .bind(&e)
                    .execute(pool)
                    .await
                    .ok();
                }
            }
        }

        // Crée la facture fournisseur DGFiP (à payer par la SCI)
        let mut fourn_id: Option<i64> = None;
        let fourn_ref = format!("TF{}-{}", fiscal_year, notice_id.to_string().chars().take(8).collect::<String>());

        if created > 0 || skipped > 0 {
            match create_dgfip_supplier_invoice(
                &pool,
                notice_id,
                total_cents,
                fiscal_year,
                &property_name,
                notice_ref.as_deref().unwrap_or(""),
            )
            .await
            {
                Ok(inv_id) => {
                    tracing::info!("Facture fournisseur DGFiP #{} créée pour {} €", inv_id, total_cents as f64 / 100.0);
                    fourn_id = Some(inv_id);
                }
                Err(e) => {
                    tracing::warn!("Création facture fournisseur DGFiP échouée : {}", e);
                }
            }
        }

        // Attache les feuillets à la GED Dolibarr
        if fourn_id.is_some() {
            let _ = attach_notice_documents_to_dolibarr(
                notice_id,
                &property_name,
                fourn_id,
                Some(&fourn_ref),
            )
            .await;
        }

        if created > 0 && failed == 0 {
            sqlx::query("UPDATE property_tax_notices SET status='INVOICED', updated_at=NOW() WHERE id=$1")
                .bind(notice_id)
                .execute(pool)
                .await
                .ok();
        }

        let detail = get_tax_notice_detail(notice_id).await?;
        let mut all_lines = Vec::new();
        for a in detail.addresses {
            all_lines.extend(a.units);
        }

        Ok(GenerateInvoicesResult {
            notice_id,
            created,
            skipped_vacant: skipped,
            failed,
            lines: all_lines,
        })
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("generate_tax_notice_invoices est exécutée côté serveur"))
}

#[server]
pub async fn delete_tax_notice(notice_id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let invoiced: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM property_tax_notice_lines l \
             JOIN property_tax_notice_addresses a ON a.id = l.address_id \
             WHERE a.notice_id = $1 AND l.status = 'invoiced'",
        )
        .bind(notice_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        if invoiced > 0 {
            return Err(ServerFnError::new(
                "Impossible : des factures ont déjà été émises".to_string(),
            ));
        }
        sqlx::query("DELETE FROM property_tax_notices WHERE id = $1")
            .bind(notice_id)
            .execute(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("delete_tax_notice est exécutée côté serveur"))
}

#[server]
pub async fn delete_tax_notice_document(
    notice_id: Uuid,
    document_kind: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        if !["BASES", "FEES", "OTHER"].contains(&document_kind.as_str()) {
            return Err(ServerFnError::new("Type de feuillet inconnu"));
        }

        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;

        let invoiced: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM property_tax_notice_lines l \
             JOIN property_tax_notice_addresses a ON a.id = l.address_id \
             WHERE a.notice_id = $1 AND l.status = 'invoiced'",
        )
        .bind(notice_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        if invoiced > 0 {
            return Err(ServerFnError::new(
                "Impossible : des factures ont déjà été émises".to_string(),
            ));
        }

        sqlx::query(
            "DELETE FROM property_tax_notice_documents WHERE notice_id = $1 AND document_kind = $2",
        )
        .bind(notice_id)
        .bind(&document_kind)
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        if document_kind == "BASES" {
            sqlx::query("DELETE FROM property_tax_notice_addresses WHERE notice_id = $1")
                .bind(notice_id)
                .execute(pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
        } else if document_kind == "FEES" {
            sqlx::query("DELETE FROM property_tax_notice_fees WHERE notice_id = $1")
                .bind(notice_id)
                .execute(pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            sqlx::query(
                r#"UPDATE property_tax_notices
                      SET total_amount_cents = NULL,
                          cotisations_amount_cents = NULL,
                          management_fees_cents = NULL,
                          updated_at = NOW()
                    WHERE id = $1"#,
            )
            .bind(notice_id)
            .execute(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        }

        let row: (i64, i64) = sqlx::query_as(
            r#"SELECT
                 (SELECT COUNT(*)::bigint FROM property_tax_notice_addresses WHERE notice_id = $1),
                 (SELECT COUNT(*)::bigint FROM property_tax_notice_documents WHERE notice_id = $1)"#,
        )
        .bind(notice_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let (addr_count, doc_count) = row;

        let new_status = if addr_count > 0 && doc_count >= 2 {
            "AWAITING_REVIEW"
        } else {
            "AWAITING_DOCS"
        };

        sqlx::query(
            "UPDATE property_tax_notices SET status = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(notice_id)
        .bind(new_status)
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("delete_tax_notice_document est exécutée côté serveur"))
}