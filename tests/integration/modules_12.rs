use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use sci_family_pilot::server::{
    create_associate, create_bank_transaction, create_deadline, create_invoice_from_lease,
    create_lease, create_payment, create_property, create_tenant, create_unit, dashboard_snapshot,
    get_sci_profile, import_bank_csv, issue_invoice, list_associates, list_automation_rules,
    list_bank_transactions, list_deadlines, list_documents, list_invoices, list_leases,
    list_payments, list_tasks, list_tenants, list_units, module_counts, onboarding_status,
    archive_document_record, reconcile_bank_transaction, register_document, run_anticipation_cycle, update_sci_profile,
    vat_summary, auth_status,
};

const TEST_PREFIX: &str = "SCI-TEST-AUTO";

fn test_name(module: &str) -> String {
    format!("{TEST_PREFIX}-{module}-{}", Uuid::new_v4().simple())
}

async fn database() -> PgPool {
    sci_family_pilot::infrastructure::db()
        .await
        .expect("Impossible de se connecter à PostgreSQL")
        .clone()
}

async fn sci_id(pool: &PgPool) -> Uuid {
    sqlx::query(
        r#"
        SELECT id
        FROM scis
        ORDER BY created_at
        LIMIT 1
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("Aucune SCI trouvée")
    .get("id")
}

async fn cleanup(pool: &PgPool, sci_id: Uuid) {
    println!("\n[CLEANUP] suppression des données de test");

    /*
     * ------------------------------------------------------
     * Paiements liés aux factures de test.
     * ------------------------------------------------------
     */
    sqlx::query(
        r#"
        DELETE FROM payments
        WHERE invoice_id IN (
            SELECT i.id
            FROM invoices i
            JOIN leases l ON l.id = i.lease_id
            WHERE i.sci_id = $1
              AND l.reference LIKE 'SCI-TEST-AUTO-%'
        )
        "#,
    )
    .bind(sci_id)
    .execute(pool)
    .await
    .expect("cleanup payments failed");

    /*
     * ------------------------------------------------------
     * Factures de test.
     * ------------------------------------------------------
     */
    sqlx::query(
        r#"
        DELETE FROM invoices
        WHERE lease_id IN (
            SELECT id
            FROM leases
            WHERE reference LIKE 'SCI-TEST-AUTO-%'
        )
        "#,
    )
    .bind(sci_id)
    .execute(pool)
    .await
    .expect("cleanup invoices failed");

    /*
     * ------------------------------------------------------
     * Transactions bancaires de test.
     * ------------------------------------------------------
     */
    sqlx::query(
        r#"
        DELETE FROM bank_transactions
        WHERE sci_id = $1
          AND (
              external_id LIKE 'SCI-TEST-AUTO-%'
              OR label LIKE 'SCI-TEST-AUTO-%'
          )
        "#,
    )
    .bind(sci_id)
    .execute(pool)
    .await
    .expect("cleanup bank transactions failed");

    /*
     * ------------------------------------------------------
     * Documents de test.
     * ------------------------------------------------------
     */
    sqlx::query(
        r#"
        DELETE FROM documents
        WHERE sci_id = $1
          AND (
              title LIKE 'SCI-TEST-AUTO-%'
              OR storage_key LIKE 'SCI-TEST-AUTO-%'
        )
        "#,
    )
    .bind(sci_id)
    .execute(pool)
    .await
    .expect("cleanup documents failed");

    /*
     * ------------------------------------------------------
     * Échéances de test.
     * ------------------------------------------------------
     */
    sqlx::query(
        r#"
        DELETE FROM tax_deadlines
        WHERE sci_id = $1
          AND code LIKE 'SCI-TEST-AUTO-%'
        "#,
    )
    .bind(sci_id)
    .execute(pool)
    .await
    .expect("cleanup deadlines failed");

    /*
     * ------------------------------------------------------
     * Baux de test.
     * ------------------------------------------------------
     */
    sqlx::query(
        r#"
        DELETE FROM leases
        WHERE reference LIKE 'SCI-TEST-AUTO-%'
        "#,
    )
    .bind(sci_id)
    .execute(pool)
    .await
    .expect("cleanup leases failed");

    /*
     * ------------------------------------------------------
     * Unités appartenant aux propriétés de test.
     * ------------------------------------------------------
     */
    sqlx::query(
        r#"
        DELETE FROM units
        WHERE property_id IN (
            SELECT id
            FROM properties
            WHERE sci_id = $1
              AND name LIKE 'SCI-TEST-AUTO-%'
        )
        "#,
    )
    .bind(sci_id)
    .execute(pool)
    .await
    .expect("cleanup units failed");

    /*
     * ------------------------------------------------------
     * Locataires de test.
     * ------------------------------------------------------
     */
    sqlx::query(
        r#"
        DELETE FROM tenants
        WHERE sci_id = $1
          AND legal_name LIKE 'SCI-TEST-AUTO-%'
        "#,
    )
    .bind(sci_id)
    .execute(pool)
    .await
    .expect("cleanup tenants failed");

    /*
     * ------------------------------------------------------
     * Propriétés de test.
     * ------------------------------------------------------
     */
    sqlx::query(
        r#"
        DELETE FROM properties
        WHERE sci_id = $1
          AND name LIKE 'SCI-TEST-AUTO-%'
        "#,
    )
    .bind(sci_id)
    .execute(pool)
    .await
    .expect("cleanup properties failed");

    /*
     * ------------------------------------------------------
     * Associés de test.
     * ------------------------------------------------------
     */
    sqlx::query(
        r#"
        DELETE FROM associates
        WHERE sci_id = $1
          AND display_name LIKE 'SCI-TEST-AUTO-%'
        "#,
    )
    .bind(sci_id)
    .execute(pool)
    .await
    .expect("cleanup associates failed");

    println!("  CLEANUP PostgreSQL : OK");
}

#[tokio::test]
async fn integration_12_modules() {
    auth_status().await.expect("auth bootstrap");
    let pool = database().await;
    let sci_id = sci_id(&pool).await;

    println!();
    println!("==================================================");
    println!(" SCI MANAGER — TEST D'INTÉGRATION COMPLET");
    println!("==================================================");
    println!("SCI        : {sci_id}");
    println!("PostgreSQL : CONNECTÉ");
    println!("==================================================");

    /*
     * ------------------------------------------------------
     * 01 — SCI / PROFIL
     * ------------------------------------------------------
     */
    println!("\n[01/12] SCI / PROFIL");

    let profile = get_sci_profile().await.expect("get_sci_profile");

    assert!(!profile.legal_name.is_empty());

    update_sci_profile(profile.clone())
        .await
        .expect("update_sci_profile");

    let profile_after = get_sci_profile()
        .await
        .expect("get_sci_profile after update");

    assert_eq!(profile.legal_name, profile_after.legal_name);
    assert_eq!(profile.legal_name, profile_after.legal_name);

    println!("  GET profil       : OK");
    println!("  UPDATE profil    : OK");

    /*
     * ------------------------------------------------------
     * 02 — ASSOCIÉS
     * ------------------------------------------------------
     */
    println!("\n[02/12] ASSOCIÉS");

    let associates = list_associates().await.expect("list_associates");

    let ownership_total: Decimal = associates.iter().map(|a| a.ownership_pct).sum();

    println!("  existants        : {}", associates.len());
    println!("  détention totale : {ownership_total}");

    if ownership_total < Decimal::new(9999, 2) {
        let name = test_name("ASSOCIE");

        create_associate(name.clone(), Decimal::new(1, 2), 0)
            .await
            .expect("create_associate");

        let after = list_associates()
            .await
            .expect("list_associates after create");

        assert!(
            after.iter().any(|a| a.display_name == name),
            "Associé créé introuvable"
        );

        println!("  CREATE           : OK");
    } else {
        println!("  CREATE           : SKIP — détention déjà à 100%");
    }

    /*
     * ------------------------------------------------------
     * 03 — PROPRIÉTÉ
     * ------------------------------------------------------
     */
    println!("\n[03/12] PROPRIÉTÉ");

    let property_name = test_name("PROPERTY");

    create_property(
        property_name.clone(),
        "Adresse test automatique".to_string(),
        Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
        Some(100_000),
    )
    .await
    .expect("create_property");

    let property_id: Uuid = sqlx::query(
        r#"
        SELECT id
        FROM properties
        WHERE sci_id = $1
          AND name = $2
        LIMIT 1
        "#,
    )
    .bind(sci_id)
    .bind(&property_name)
    .fetch_one(&pool)
    .await
    .expect("property lookup")
    .get("id");

    println!("  CREATE           : OK");

    /*
     * ------------------------------------------------------
     * 04 — UNITÉ / LOT
     * ------------------------------------------------------
     */
    println!("\n[04/12] UNITÉ / LOT");

    let unit_code = test_name("UNIT");

    create_unit(
        property_id,
        unit_code.clone(),
        "Lot automatique".to_string(),
        "LOCAL_COMMERCIAL".to_string(),
        Some(Decimal::new(50, 0)),
        100_000,
        2000,
    )
    .await
    .expect("create_unit");

    let unit_id: Uuid = sqlx::query(
        r#"
        SELECT id
        FROM units
        WHERE property_id = $1
          AND code = $2
        LIMIT 1
        "#,
    )
    .bind(property_id)
    .bind(&unit_code)
    .fetch_one(&pool)
    .await
    .expect("unit lookup")
    .get("id");

    let units = list_units().await.expect("list_units");

    assert!(units.iter().any(|u| u.id == unit_id));

    println!("  CREATE           : OK");
    println!("  LIST             : OK");

    /*
     * ------------------------------------------------------
     * 05 — LOCATAIRE
     * ------------------------------------------------------
     */
    println!("\n[05/12] LOCATAIRE");

    let tenant_name = test_name("TENANT");

    create_tenant(
        tenant_name.clone(),
        "TEST-SIRET".to_string(),
        "test@example.invalid".to_string(),
        "0600000000".to_string(),
    )
    .await
    .expect("create_tenant");

    let tenant_id: Uuid = sqlx::query(
        r#"
        SELECT id
        FROM tenants
        WHERE sci_id = $1
          AND legal_name = $2
        LIMIT 1
        "#,
    )
    .bind(sci_id)
    .bind(&tenant_name)
    .fetch_one(&pool)
    .await
    .expect("tenant lookup")
    .get("id");

    let tenants = list_tenants().await.expect("list_tenants");

    assert!(tenants.iter().any(|t| t.id == tenant_id));

    println!("  CREATE           : OK");
    println!("  LIST             : OK");

    /*
     * ------------------------------------------------------
     * 06 — BAIL
     * ------------------------------------------------------
     */
    println!("\n[06/12] BAIL");

    let lease_reference = test_name("LEASE");

    create_lease(
        unit_id,
        tenant_id,
        lease_reference.clone(),
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        None,
        3,
        5,
        Some(1),
    )
    .await
    .expect("create_lease");

    let lease_id: Uuid = sqlx::query(
        r#"
        SELECT id
        FROM leases
        WHERE reference = $1
        LIMIT 1
        "#,
    )
    .bind(&lease_reference)
    .fetch_one(&pool)
    .await
    .expect("lease lookup")
    .get("id");

    let leases = list_leases().await.expect("list_leases");

    assert!(leases.iter().any(|l| l.id == lease_id));

    println!("  CREATE           : OK");
    println!("  LIST             : OK");

    /*
     * ------------------------------------------------------
     * 07 — FACTURATION
     *
     * Facture A = paiement classique.
     * Facture B = rapprochement bancaire.
     * ------------------------------------------------------
     */
    println!("\n[07/12] FACTURATION");

    create_invoice_from_lease(
        lease_id,
        NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
    )
    .await
    .expect("create_invoice A");

    let invoice_a = sqlx::query(
        r#"
        SELECT id, gross_cents
        FROM invoices
        WHERE lease_id = $1
        LIMIT 1
        "#,
    )
    .bind(lease_id)
    .fetch_one(&pool)
    .await
    .expect("invoice A lookup");

    let invoice_a_id: Uuid = invoice_a.get("id");
    let gross_a: i64 = invoice_a.get("gross_cents");

    assert!(gross_a > 0);

    issue_invoice(invoice_a_id).await.expect("issue_invoice A");

    /*
     * Deuxième facture : elle sera utilisée uniquement
     * pour le rapprochement bancaire.
     */
    create_invoice_from_lease(
        lease_id,
        NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
    )
    .await
    .expect("create_invoice B");

    let invoice_b = sqlx::query(
        r#"
        SELECT id, gross_cents
        FROM invoices
        WHERE lease_id = $1
          AND issue_date = DATE '2026-10-01'
        LIMIT 1
        "#,
    )
    .bind(lease_id)
    .fetch_one(&pool)
    .await
    .expect("invoice B lookup");

    let invoice_b_id: Uuid = invoice_b.get("id");
    let gross_b: i64 = invoice_b.get("gross_cents");

    assert!(gross_b > 0);
    assert_ne!(invoice_a_id, invoice_b_id);

    issue_invoice(invoice_b_id).await.expect("issue_invoice B");

    let invoices = list_invoices().await.expect("list_invoices");

    assert!(invoices.iter().any(|i| i.id == invoice_a_id));
    assert!(invoices.iter().any(|i| i.id == invoice_b_id));

    println!("  CREATE facture A : OK");
    println!("  CREATE facture B : OK");
    println!("  LIST             : OK");
    println!("  ISSUE            : OK");

    /*
     * ------------------------------------------------------
     * 08 — PAIEMENTS
     * ------------------------------------------------------
     */
    println!("\n[08/12] PAIEMENTS");

    let payment_reference = test_name("PAYMENT");

    create_payment(
        Some(invoice_a_id),
        Utc::now(),
        gross_a,
        payment_reference.clone(),
        "TEST_AUTOMATIQUE".to_string(),
    )
    .await
    .expect("create_payment");

    let payments = list_payments().await.expect("list_payments");

    assert!(
        payments
            .iter()
            .any(|p| { p.reference == payment_reference }),
        "Paiement de test introuvable"
    );

    let payment_total_a: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(amount_cents), 0)::bigint
        FROM payments
        WHERE invoice_id = $1
        "#,
    )
    .bind(invoice_a_id)
    .fetch_one(&pool)
    .await
    .expect("payment total A");

    assert_eq!(payment_total_a, gross_a);

    println!("  CREATE           : OK");
    println!("  LIST             : OK");
    println!("  TOTAL FACTURE A  : OK");

    /*
     * ------------------------------------------------------
     * 09 — BANQUE
     *
     * Facture B uniquement :
     * transaction bancaire -> rapprochement -> paiement.
     * ------------------------------------------------------
     */
    println!("\n[09/12] BANQUE");

    let external_id = test_name("BANK");

    create_bank_transaction(
        Utc::now(),
        Some(Utc::now().date_naive()),
        gross_b,
        format!("{external_id} transaction"),
        "TEST".to_string(),
        external_id.clone(),
    )
    .await
    .expect("create_bank_transaction");

    let bank_rows = list_bank_transactions()
        .await
        .expect("list_bank_transactions");

    let bank = bank_rows
        .iter()
        .find(|b| b.external_id == external_id)
        .expect("transaction bancaire introuvable");

    reconcile_bank_transaction(bank.id, invoice_b_id)
        .await
        .expect("reconcile_bank_transaction");

    let payment_total_b: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(amount_cents), 0)::bigint
        FROM payments
        WHERE invoice_id = $1
        "#,
    )
    .bind(invoice_b_id)
    .fetch_one(&pool)
    .await
    .expect("payment total B");

    assert_eq!(payment_total_b, gross_b);

    let reconciliation_status: String = sqlx::query_scalar(
        r#"
        SELECT reconciliation_status
        FROM bank_transactions
        WHERE id = $1
        "#,
    )
    .bind(bank.id)
    .fetch_one(&pool)
    .await
    .expect("bank reconciliation status");

    assert_eq!(reconciliation_status, "MATCHED");

    /*
     * Import CSV sur une transaction indépendante.
     */
    let csv_external = test_name("CSV");

    let csv = format!(
        "2026-09-15;{};SCI-TEST-AUTO-CSV;{}\n",
        gross_b as f64 / 100.0,
        csv_external
    );

    let imported = import_bank_csv(csv).await.expect("import_bank_csv");

    assert!(imported >= 1);

    println!("  CREATE           : OK");
    println!("  RECONCILIATION   : OK");
    println!("  PAIEMENT BANQUE  : OK");
    println!("  CSV IMPORT       : OK");

    /*
     * ------------------------------------------------------
     * 10 — TVA / ÉCHÉANCES
     * ------------------------------------------------------
     */
    println!("\n[10/12] TVA / ÉCHÉANCES");

    let vat = vat_summary("2026-09".to_string())
        .await
        .expect("vat_summary");

    assert!(vat.payments_count >= 0);

    let deadline_code = test_name("VAT");

    create_deadline(
        deadline_code.clone(),
        "Échéance TVA test".to_string(),
        NaiveDate::from_ymd_opt(2026, 9, 30).unwrap(),
        "2026-09".to_string(),
    )
    .await
    .expect("create_deadline");

    let deadlines = list_deadlines().await.expect("list_deadlines");

    assert!(deadlines.iter().any(|d| d.code == deadline_code));

    println!("  TVA SUMMARY       : OK");
    println!("  DEADLINE CREATE   : OK");
    println!("  DEADLINE LIST     : OK");

    /*
     * ------------------------------------------------------
     * 11 — AUTOMATISATION
     * ------------------------------------------------------
     */
    println!("\n[11/12] AUTOMATISATION");

    let rules = list_automation_rules()
        .await
        .expect("list_automation_rules");

    assert!(
        !rules.is_empty(),
        "Aucune règle d'automatisation disponible"
    );

    let cycle = run_anticipation_cycle()
        .await
        .expect("run_anticipation_cycle");

    let tasks = list_tasks().await.expect("list_tasks");

    assert!(
        !tasks.is_empty(),
        "Aucune tâche après le cycle d'anticipation"
    );

    println!("  RULES             : OK ({})", rules.len());
    println!("  CYCLE             : OK");
    println!("  RULES EVALUATED   : {}", cycle.evaluated_rules);
    println!("  TASKS CREATED     : {}", cycle.created_tasks);
    println!("  TASKS             : OK ({})", tasks.len());

    /*
     * ------------------------------------------------------
     * 12 — DOCUMENTS / PILOTAGE
     * ------------------------------------------------------
     */
    println!("\n[12/12] DOCUMENTS / PILOTAGE");

    let document_title = test_name("DOCUMENT");
    let storage_key = test_name("STORAGE");

    register_document(
        "BAIL".to_string(),
        document_title.clone(),
        "test.pdf".to_string(),
        storage_key,
        Some(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
        None,
    )
    .await
    .expect("register_document");

    let documents = list_documents().await.expect("list_documents");

    assert!(
        documents.iter().any(|d| d.title == document_title),
        "Document de test introuvable"
    );

    let document_id = documents.iter().find(|d| d.title == document_title).unwrap().id;
    archive_document_record(document_id, "Fin du cycle S19".into())
        .await
        .expect("archive_document_record");
    let archived_status: String = sqlx::query_scalar(
        "SELECT status FROM documents WHERE id=$1"
    ).bind(document_id).fetch_one(&pool).await.expect("archived status");
    assert_eq!(archived_status, "ARCHIVED");

    let dashboard = dashboard_snapshot().await.expect("dashboard_snapshot");

    assert!(!dashboard.risk_level.is_empty());

    let counts = module_counts().await.expect("module_counts");

    let onboarding = onboarding_status().await.expect("onboarding_status");

    println!("  DOCUMENT          : OK");
    println!("  ARCHIVE           : OK");
    println!("  DASHBOARD         : OK");
    println!("  MODULE COUNTS     : OK");
    println!("  ONBOARDING        : OK");
    println!("  COMPLETION        : {}%", onboarding.completion_pct);
    println!("  INVOICES          : {}", counts.invoices);
    println!("  PAYMENTS          : {}", counts.payments);
    println!("  BANK              : {}", counts.bank_transactions);

    /*
     * ------------------------------------------------------
     * FIN
     * ------------------------------------------------------
     */
    println!();
    println!("==================================================");
    println!("        TEST DES 12 MODULES — OK");
    println!("==================================================");
    println!("01  SCI / profil          OK");
    println!("02  Associés              OK");
    println!("03  Propriété             OK");
    println!("04  Unité / lot           OK");
    println!("05  Locataire             OK");
    println!("06  Bail                  OK");
    println!("07  Facturation           OK");
    println!("08  Paiements             OK");
    println!("09  Banque                OK");
    println!("10  TVA / échéances       OK");
    println!("11  Automatisation        OK");
    println!("12  Documents / pilotage  OK");
    println!("13  Archive              OK");
    println!("==================================================");

    cleanup(&pool, sci_id).await;

    println!("CLEANUP — OK");
}
