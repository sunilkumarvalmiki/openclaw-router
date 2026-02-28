use actix_web::{get, web, HttpResponse};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct ReadyResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct ReadyErrorResponse {
    status: &'static str,
    error: String,
}

/// `GET /health` -- lightweight liveness probe.
///
/// Always returns 200 OK with the service version. Does not check
/// downstream dependencies so it can never false-negative.
#[get("/health")]
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// `GET /ready` -- readiness probe.
///
/// Pings the PostgreSQL database with `SELECT 1` to confirm connectivity.
/// Returns 200 if the database is reachable, 503 otherwise.
#[get("/ready")]
pub async fn ready(pool: web::Data<PgPool>) -> HttpResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool.get_ref())
        .await
    {
        Ok(_) => HttpResponse::Ok().json(ReadyResponse { status: "ready" }),
        Err(err) => {
            tracing::error!(error = %err, "Readiness check failed: database unreachable");
            HttpResponse::ServiceUnavailable().json(ReadyErrorResponse {
                status: "not_ready",
                error: format!("Database unreachable: {err}"),
            })
        }
    }
}
