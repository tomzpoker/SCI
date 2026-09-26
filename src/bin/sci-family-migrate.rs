use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL est absent de l'environnement du migrateur")?;

    if database_url.trim().is_empty() {
        return Err("DATABASE_URL est vide".into());
    }

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;

    println!("SCI migration runner");
    println!("Database connection: OK");

    let migrator = sqlx::migrate!("./migrations");

    println!("Pending migration execution starting...");
    migrator.run(&pool).await?;
    println!("Pending migration execution: OK");

    let applied_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM _sqlx_migrations WHERE success = true",
    )
    .fetch_one(&pool)
    .await?;

    let failed_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM _sqlx_migrations WHERE success = false",
    )
    .fetch_one(&pool)
    .await?;

    if failed_count > 0 {
        return Err(format!(
            "L'historique SQLx contient {} migration(s) en échec",
            failed_count
        )
        .into());
    }

    let latest: Option<(i64, String)> = sqlx::query_as(
        "SELECT version, description
         FROM _sqlx_migrations
         WHERE success = true
         ORDER BY version DESC
         LIMIT 1",
    )
    .fetch_optional(&pool)
    .await?;

    match latest {
        Some((version, description)) => {
            println!("Successful migrations: {}", applied_count);
            println!("Latest migration: {} ({})", version, description);
        }
        None => {
            println!("Successful migrations: 0");
            println!("Latest migration: none");
        }
    }

    pool.close().await;

    Ok(())
}
