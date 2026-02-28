mod config;
mod error;
mod providers;
mod routes;
mod routing;
mod types;

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::{fmt, EnvFilter};

use crate::config::AppConfig;
use crate::providers::ProviderFactory;
use crate::routes::completions::AppState;
use crate::routing::RequestScorer;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // -----------------------------------------------------------------------
    // 1. Load environment variables
    // -----------------------------------------------------------------------
    dotenvy::dotenv().ok(); // .env is optional; production uses real env vars

    // -----------------------------------------------------------------------
    // 2. Initialize structured (JSON) tracing
    // -----------------------------------------------------------------------
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("openclaw_gateway=info,actix_web=info,sqlx=warn"));

    fmt()
        .json()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Starting OpenClaw Gateway v{}", env!("CARGO_PKG_VERSION"));

    // -----------------------------------------------------------------------
    // 3. Load application configuration
    // -----------------------------------------------------------------------
    let config = AppConfig::from_env();
    let bind_address = config.bind_address();

    tracing::info!(
        host = %config.host,
        port = config.port,
        cost_profile = %config.default_cost_profile,
        "Configuration loaded"
    );

    // -----------------------------------------------------------------------
    // 4. Connect to PostgreSQL (optional — standalone mode if unavailable)
    // -----------------------------------------------------------------------
    let db_pool = if let Some(ref db_url) = config.database_url {
        match PgPoolOptions::new()
            .max_connections(20)
            .connect(db_url)
            .await
        {
            Ok(pool) => {
                tracing::info!("Connected to PostgreSQL");

                // 5. Run pending migrations
                if let Err(e) = sqlx::migrate!("./migrations").run(&pool).await {
                    tracing::warn!("Failed to run migrations: {e}");
                } else {
                    tracing::info!("Database migrations applied");
                }

                Some(pool)
            }
            Err(e) => {
                tracing::warn!("PostgreSQL unavailable ({e}), running in standalone mode");
                None
            }
        }
    } else {
        tracing::info!("No DATABASE_URL set — running in standalone mode (no persistence)");
        None
    };

    // -----------------------------------------------------------------------
    // 6. Build LLM providers from environment variables
    // -----------------------------------------------------------------------
    let llm_providers = ProviderFactory::create_from_env();

    // -----------------------------------------------------------------------
    // 7. Build shared application state
    // -----------------------------------------------------------------------
    let config_data = web::Data::new(config.clone());
    let pool_data = web::Data::new(db_pool); // Option<PgPool>
    let app_state = web::Data::new(AppState {
        scorer: RequestScorer::new(),
        providers: llm_providers,
    });

    // -----------------------------------------------------------------------
    // 8. Start the HTTP server
    // -----------------------------------------------------------------------
    let workers = num_cpus::get();
    tracing::info!(workers, "Starting HTTP server on {}", &bind_address);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&config.dashboard_url)
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(TracingLogger::default())
            .wrap(cors)
            .wrap(middleware::Compress::default())
            .app_data(config_data.clone())
            .app_data(pool_data.clone())
            .app_data(app_state.clone())
            .configure(routes::configure)
    })
    .workers(workers)
    .keep_alive(std::time::Duration::from_secs(75))
    .max_connections(25_000)
    .backlog(2048)
    .bind(&bind_address)?
    .run()
    .await
}
