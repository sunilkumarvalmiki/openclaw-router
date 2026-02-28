mod config;
mod error;
mod routes;

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::{fmt, EnvFilter};

use crate::config::AppConfig;

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
    // 4. Connect to PostgreSQL
    // -----------------------------------------------------------------------
    let db_pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    tracing::info!("Connected to PostgreSQL");

    // -----------------------------------------------------------------------
    // 5. Run pending migrations
    // -----------------------------------------------------------------------
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run database migrations");

    tracing::info!("Database migrations applied");

    // -----------------------------------------------------------------------
    // 6. Build shared application state
    // -----------------------------------------------------------------------
    let config_data = web::Data::new(config.clone());
    let pool_data = web::Data::new(db_pool);

    // -----------------------------------------------------------------------
    // 7. Start the HTTP server
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
