#[cfg(feature = "server")]
use sqlx::{PgPool, postgres::PgPoolOptions};
#[cfg(feature = "server")]
use std::time::Duration;
#[cfg(feature = "server")]
use tokio::sync::OnceCell;

#[cfg(feature = "server")]
static DB: OnceCell<PgPool> = OnceCell::const_new();

#[cfg(feature = "server")]
pub async fn db() -> Result<&'static PgPool, sqlx::Error> {
    DB.get_or_try_init(|| async {
        dotenvy::dotenv().ok();
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| sqlx::Error::Configuration("DATABASE_URL manquant".into()))?;
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&url)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(pool)
    }).await
}
