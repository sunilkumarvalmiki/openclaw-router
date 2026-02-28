pub mod completions;
pub mod dashboard;
pub mod health;

use actix_web::web;

/// Register all route handlers with the Actix-Web application.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(health::health)
        .service(health::ready)
        .route(
            "/v1/chat/completions",
            web::post().to(completions::chat_completions),
        )
        // Dashboard API endpoints
        .route("/api/v1/stats", web::get().to(dashboard::get_stats))
        .route("/api/v1/stats/providers", web::get().to(dashboard::get_provider_stats))
        .route("/api/v1/stats/tiers", web::get().to(dashboard::get_tier_distribution))
        .route("/api/v1/stats/providers/distribution", web::get().to(dashboard::get_provider_distribution))
        .route("/api/v1/stats/hourly", web::get().to(dashboard::get_hourly_data))
        .route("/api/v1/recent-requests", web::get().to(dashboard::get_recent_requests))
        .route("/api/v1/models", web::get().to(dashboard::get_models))
        .route("/api/v1/config", web::get().to(dashboard::get_config));
}
