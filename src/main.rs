use sci_family_pilot::infrastructure;
use sci_family_pilot::observability::{self, LogDomain};
use sci_family_pilot::server;
use sci_family_pilot::ui::App;

fn main() {
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
                loop {
                    if let Err(error) = server::automation_tick(pool).await {
                        observability::record_error(
                            LogDomain::Automation,
                            &error,
                            "automation cycle failed",
                        );
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
