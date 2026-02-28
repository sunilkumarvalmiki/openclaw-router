pub mod health;

use actix_web::web;

/// Register all route handlers with the Actix-Web application.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(health::health).service(health::ready);
}
