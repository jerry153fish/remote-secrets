use log::{info, warn};

use utils::log::init_log;
use utils::metrics::register_custom_metrics;

use actix_web::{middleware, web::Data, App, HttpServer};

use anyhow::Result;
pub use controller::*;

#[tokio::main]
async fn main() -> Result<()> {
    init_log()?;
    register_custom_metrics();
    info!("Starting controller");

    let (manager, drainer) = Manager::new().await;

    // Infer the runtime environment and try to create a Kubernetes Client
    let server = HttpServer::new(move || {
        App::new()
            .app_data(Data::new(manager.clone()))
            .wrap(
                middleware::Logger::default()
                    .exclude("/healthz")
                    .exclude("/readyz"),
            )
            .service(web::health)
            .service(web::metrics)
            .service(web::ready)
    })
    .bind("0.0.0.0:8080")
    .expect("Can not bind to 0.0.0.0:8080")
    .shutdown_timeout(5);

    tokio::select! {
        _ = drainer => warn!("controller drained"),
        _ = server.run() => info!("actix exited"),
    }

    Ok(())
}
