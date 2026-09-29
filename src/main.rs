use sci_family_pilot::infrastructure;
use sci_family_pilot::observability::{self, LogDomain};
use sci_family_pilot::server;
use sci_family_pilot::ui::App;

fn main() {
    // Charge le .env AVANT tout (variables Gmail, DB, etc.)
    #[cfg(feature = "server")]
    {
        let _ = dotenvy::dotenv();
    }

    observability::init();

    #[cfg(feature = "server")]
    {
        dioxus::serve(|| async move {
            let pool = infrastructure::db_unchecked()
                .await
                .map_err(dioxus::prelude::ServerFnError::new)?;

            tracing::info!(
                domain = %LogDomain::Application,
                event = "server_started",
                "SCI Family server started"
            );

            tokio::spawn(async move {
                // Premier passage immédiat au démarrage
                if let Err(error) = server::automation_tick(pool).await {
                    observability::record_error(
                        LogDomain::Automation,
                        &error,
                        "automation cycle failed at startup",
                    );
                }
                // Puis une fois par jour (24h)
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(24 * 3600)).await;
                    if let Err(error) = server::automation_tick(pool).await {
                        observability::record_error(
                            LogDomain::Automation,
                            &error,
                            "automation cycle failed",
                        );
                    }
                }
            });

            Ok(dioxus::server::router(App))
        });
    }

    #[cfg(not(feature = "server"))]
    dioxus::launch(App);
}