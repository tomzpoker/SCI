mod application;
mod domain;
mod infrastructure;
mod server;
mod ui;

use dioxus::prelude::*;
use ui::App;

fn main() {
    #[cfg(feature = "server")]
    {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
            .init();
        dioxus::serve(|| async move {
            let pool = infrastructure::db().await.map_err(dioxus::prelude::ServerFnError::new)?;
            tokio::spawn(async move {
                loop {
                    if let Err(error) = server::automation_tick(pool).await {
                        tracing::error!(?error, "automation cycle failed");
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                }
            });
            Ok(dioxus::server::router(App))
        });
    }

    #[cfg(not(feature = "server"))]
    dioxus::launch(App);
}
