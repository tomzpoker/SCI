use std::fs;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Cycle { name: String, steps: Vec<String> }

fn cycle() -> Cycle {
    serde_json::from_str(include_str!("../fixtures/e2e/locative_cycle.json")).expect("cycle fixture")
}

#[test]
fn full_locative_cycle_contract_is_present() {
    let c = cycle();
    assert_eq!(c.name, "cycle_locatif_complet");
    assert_eq!(c.steps.len(), 15);
    assert_eq!(c.steps[0], "SCI");
    assert_eq!(c.steps[14], "Archive");
}

#[test]
fn developer_recovery_contract_is_present() {
    let readme = fs::read_to_string("README.md").expect("README");
    assert!(readme.contains("bootstrap"));
    assert!(readme.contains("doctor"));
    assert!(readme.contains("tests"));
}

#[cfg(all(feature = "server", feature = "test-auth"))]
#[tokio::test]
async fn optional_runtime_e2e_requires_explicit_test_auth() {
    if std::env::var("SCI_TEST_AUTH").as_deref() != Ok("1") {
        eprintln!("SCI_TEST_AUTH=1 requis pour la phase DB E2E");
        return;
    }
    use serde_json::json;
    let _ = sci_family_pilot::security::auth_status().await.expect("auth bootstrap");
    let pool = sci_family_pilot::infrastructure::db().await.expect("DB auth");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM legal_entities WHERE active=true").fetch_one(pool).await.expect("legal entities");
    assert!(count >= 1);

    let key = format!("S19-E2E-{}", uuid::Uuid::new_v4());
    let run = sci_family_pilot::workflow::start_workflow(
        "ADMIN_ACTION".into(), json!({"scenario":"S19"}), key.clone(),
        Some("S19".into()), Some("CYCLE".into()), "HIGH".into()
    ).await.expect("workflow start");
    assert_eq!(run.status, "WAITING_VALIDATION");
    assert!(run.validation_request_id.is_some());

    let req = run.validation_request_id.unwrap();
    sqlx::query("DELETE FROM validation_requests WHERE id=$1").bind(req).execute(pool).await.expect("cleanup validation");
    sqlx::query("DELETE FROM workflow_runs WHERE id=$1").bind(run.workflow_run_id).execute(pool).await.expect("cleanup workflow run");
    sqlx::query("DELETE FROM job_executions WHERE legal_entity_id=$1 AND idempotency_key=$2").bind(sci_family_pilot::entity_scope::current_legal_entity_id()).bind(key).execute(pool).await.expect("cleanup idempotency");
}
