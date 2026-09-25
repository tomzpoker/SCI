#[cfg(feature = "server")]
use sqlx::{PgPool, postgres::PgPoolOptions};
#[cfg(feature = "server")]
use std::time::Duration;
#[cfg(feature = "server")]
use tokio::sync::OnceCell;

#[cfg(feature = "server")]
static DB: OnceCell<PgPool> = OnceCell::const_new();

#[cfg(feature = "server")]
pub async fn db_unchecked() -> Result<&'static PgPool, sqlx::Error> {
    DB.get_or_try_init(|| async {
        dotenvy::dotenv().ok();
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://sci:sci@127.0.0.1:55432/sci_family".to_string());
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&url)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(pool)
    })
    .await
}

#[cfg(feature="server")]
pub async fn db() -> Result<&'static PgPool, sqlx::Error> {
    let pool = db_unchecked().await?;
    crate::security::assert_authenticated(pool).await?;
    Ok(pool)
}
