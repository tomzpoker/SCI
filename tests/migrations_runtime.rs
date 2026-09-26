#[cfg(feature="server")]
#[tokio::test]
async fn current_database_schema_is_migrated_when_runtime_test_is_enabled() {
    if std::env::var("SCI_RUN_DB_TESTS").as_deref() != Ok("1") { return; }
    let url=std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool=sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(&url).await.expect("connexion DB");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations");
    let n:i64=sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success=true").fetch_one(&pool).await.expect("migration count");
    assert_eq!(n,28);
    for table in ["invoices","bank_transactions","vat_declarations","arrears_cases","generated_documents","auth_users","system_backups"] {
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name=$1)").bind(table).fetch_one(&pool).await.expect("table existence");
        assert!(exists,"table absente: {table}");
    }
}

#[cfg(not(feature="server"))]
#[test]
fn migration_runtime_test_requires_server_feature() {}
