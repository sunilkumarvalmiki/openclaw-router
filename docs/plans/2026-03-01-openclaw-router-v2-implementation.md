# OpenClaw Router v2.0 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a production-grade LLM routing gateway in Rust with a Next.js dashboard, achieving sub-millisecond routing and 60-92% cost savings.

**Architecture:** Monolithic Rust gateway (Actix-Web 4) handling request scoring, routing, caching, rate limiting, and multi-provider proxy. Next.js 15 dashboard with Google OAuth + local auth. PostgreSQL + Redis as data layer. Docker Compose for deployment to sunilkumarvalmiki.xyz.

**Tech Stack:** Rust/Actix-Web, Next.js 15, Tailwind CSS 4, NextAuth.js v5, PostgreSQL 16, Redis 7, Docker, Nginx, OpenTelemetry

**Notion Tracking:** https://www.notion.so/OpenClaw-Development-315a03019d5281f6a17ce1bf3b9b1df1

---

## Phase 1: Foundation — Rust Gateway Scaffold

### Task 1.1: Initialize Rust Project

**Files:**
- Create: `gateway/Cargo.toml`
- Create: `gateway/src/main.rs`
- Create: `gateway/.env.example`
- Create: `gateway/rust-toolchain.toml`

**Step 1: Create the gateway directory and initialize Cargo project**

```bash
cd c:/Users/Narasamma/workspace/openclaw-router
mkdir -p gateway
cd gateway
cargo init --name openclaw-gateway
```

**Step 2: Write Cargo.toml with all dependencies**

```toml
[package]
name = "openclaw-gateway"
version = "0.1.0"
edition = "2021"
description = "High-performance LLM routing gateway for OpenClaw"

[dependencies]
# Web Framework
actix-web = "4"
actix-rt = "2"
actix-cors = "0.7"

# Async Runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# HTTP Client
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "stream"] }

# Database
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio-rustls", "macros", "uuid", "chrono", "json", "migrate"] }

# Redis
deadpool-redis = "0.18"
redis = { version = "0.27", features = ["tokio-comp"] }

# Environment
dotenvy = "0.15"

# UUID
uuid = { version = "1", features = ["serde", "v4", "v7"] }

# Date/Time
chrono = { version = "0.4", features = ["serde"] }

# Logging/Tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "json"] }
tracing-actix-web = "0.7"

# Metrics
prometheus = "0.13"

# Utils
anyhow = "1"
thiserror = "2"
async-trait = "0.1"
dashmap = "6"
num_cpus = "1"
tiktoken-rs = "0.6"

[dev-dependencies]
actix-test = "0.1"
tokio-test = "0.4"
criterion = { version = "0.5", features = ["async_tokio"] }
pretty_assertions = "1"

[[bench]]
name = "routing"
harness = false
```

**Step 3: Create .env.example**

```bash
# Server
HOST=0.0.0.0
PORT=8080
RUST_LOG=info

# Database
DATABASE_URL=postgres://openclaw:openclaw@localhost:5432/openclaw_router

# Redis
REDIS_URL=redis://127.0.0.1:6379

# Providers (add keys as needed)
OPENAI_API_KEY=
OPENROUTER_API_KEY=
GEMINI_API_KEY=
XAI_API_KEY=
DEEPSEEK_API_KEY=
OLLAMA_ENDPOINT=http://localhost:11434

# Routing
DEFAULT_COST_PROFILE=AUTO
CACHE_TTL_SECONDS=3600
MAX_CACHE_ENTRIES=10000
PRICING_SYNC_INTERVAL_SECONDS=300

# Dashboard
DASHBOARD_URL=http://localhost:3000
```

**Step 4: Create rust-toolchain.toml**

```toml
[toolchain]
channel = "stable"
```

**Step 5: Verify project compiles**

Run: `cd gateway && cargo check`
Expected: Compiles without errors

**Step 6: Commit**

```bash
git add gateway/
git commit -m "feat: initialize Rust gateway project with dependencies"
```

---

### Task 1.2: Core Error Types and Config

**Files:**
- Create: `gateway/src/error.rs`
- Create: `gateway/src/config.rs`

**Step 1: Write error types**

```rust
// gateway/src/error.rs
use actix_web::{HttpResponse, ResponseError};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Internal(String),
    BadRequest(String),
    Unauthorized(String),
    NotFound(String),
    RateLimit(String),
    ProviderError(String),
    CacheError(String),
    DatabaseError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
            Self::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            Self::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::RateLimit(msg) => write!(f, "Rate limited: {}", msg),
            Self::ProviderError(msg) => write!(f, "Provider error: {}", msg),
            Self::CacheError(msg) => write!(f, "Cache error: {}", msg),
            Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let (status, error_type) = match self {
            Self::Internal(_) => (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
            Self::BadRequest(_) => (actix_web::http::StatusCode::BAD_REQUEST, "bad_request"),
            Self::Unauthorized(_) => (actix_web::http::StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::NotFound(_) => (actix_web::http::StatusCode::NOT_FOUND, "not_found"),
            Self::RateLimit(_) => (actix_web::http::StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            Self::ProviderError(_) => (actix_web::http::StatusCode::BAD_GATEWAY, "provider_error"),
            Self::CacheError(_) => (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "cache_error"),
            Self::DatabaseError(_) => (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "database_error"),
        };
        HttpResponse::build(status).json(serde_json::json!({
            "error": { "type": error_type, "message": self.to_string() }
        }))
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::DatabaseError(err.to_string())
    }
}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        Self::CacheError(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        Self::ProviderError(err.to_string())
    }
}
```

**Step 2: Write config module**

```rust
// gateway/src/config.rs
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub default_cost_profile: String,
    pub cache_ttl_seconds: u64,
    pub max_cache_entries: usize,
    pub pricing_sync_interval_seconds: u64,
    pub dashboard_url: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("PORT must be a number"),
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            default_cost_profile: std::env::var("DEFAULT_COST_PROFILE")
                .unwrap_or_else(|_| "AUTO".to_string()),
            cache_ttl_seconds: std::env::var("CACHE_TTL_SECONDS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .unwrap_or(3600),
            max_cache_entries: std::env::var("MAX_CACHE_ENTRIES")
                .unwrap_or_else(|_| "10000".to_string())
                .parse()
                .unwrap_or(10000),
            pricing_sync_interval_seconds: std::env::var("PRICING_SYNC_INTERVAL_SECONDS")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            dashboard_url: std::env::var("DASHBOARD_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        }
    }
}
```

**Step 3: Verify compiles**

Run: `cd gateway && cargo check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add gateway/src/error.rs gateway/src/config.rs
git commit -m "feat: add core error types and config module"
```

---

### Task 1.3: Main Server Setup with Health Check

**Files:**
- Modify: `gateway/src/main.rs`
- Create: `gateway/src/routes/mod.rs`
- Create: `gateway/src/routes/health.rs`

**Step 1: Write health check handler**

```rust
// gateway/src/routes/health.rs
use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // TODO: track uptime
    })
}

pub async fn ready_check(db: web::Data<PgPool>) -> HttpResponse {
    match sqlx::query("SELECT 1").execute(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({"status": "ready"})),
        Err(_) => HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({"status": "not_ready"})),
    }
}
```

**Step 2: Write routes module**

```rust
// gateway/src/routes/mod.rs
pub mod health;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .route("/health", web::get().to(health::health_check))
            .route("/ready", web::get().to(health::ready_check))
    );
}
```

**Step 3: Write main.rs**

```rust
// gateway/src/main.rs
use actix_web::{web, App, HttpServer, middleware};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

mod config;
mod error;
mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let config = config::AppConfig::from_env();
    let bind_addr = format!("{}:{}", config.host, config.port);

    tracing::info!("Connecting to database...");
    let db_pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    tracing::info!("Running migrations...");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run migrations");

    let workers = num_cpus::get();
    tracing::info!("Starting server on {} with {} workers", bind_addr, workers);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(config.clone()))
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .wrap(middleware::NormalizePath::trim())
            .configure(routes::configure)
    })
    .workers(workers)
    .keep_alive(std::time::Duration::from_secs(75))
    .max_connections(25000)
    .backlog(2048)
    .bind(&bind_addr)?
    .run()
    .await
}
```

**Step 4: Create initial migration**

```bash
cd gateway
mkdir -p migrations
```

Create `gateway/migrations/0001_init.sql`:
```sql
-- Initial schema for OpenClaw Router

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- API keys for gateway authentication
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    key_hash VARCHAR(128) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    user_id UUID,
    cost_profile VARCHAR(20) NOT NULL DEFAULT 'AUTO',
    rate_limit_rpm INT NOT NULL DEFAULT 60,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Request logs for analytics
CREATE TABLE request_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    api_key_id UUID REFERENCES api_keys(id),
    request_id VARCHAR(64) NOT NULL,
    model_used VARCHAR(128) NOT NULL,
    provider VARCHAR(64) NOT NULL,
    tier VARCHAR(20) NOT NULL,
    cost_profile VARCHAR(20) NOT NULL,
    input_tokens INT NOT NULL DEFAULT 0,
    output_tokens INT NOT NULL DEFAULT 0,
    total_tokens INT NOT NULL DEFAULT 0,
    cost_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
    latency_ms INT NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'success',
    cache_hit BOOLEAN NOT NULL DEFAULT false,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_request_logs_api_key ON request_logs(api_key_id);
CREATE INDEX idx_request_logs_created_at ON request_logs(created_at);
CREATE INDEX idx_request_logs_provider ON request_logs(provider);
CREATE INDEX idx_request_logs_model ON request_logs(model_used);

-- Provider configurations
CREATE TABLE providers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(64) NOT NULL UNIQUE,
    base_url VARCHAR(512) NOT NULL,
    api_key_env VARCHAR(128),
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    priority INT NOT NULL DEFAULT 100,
    max_rpm INT NOT NULL DEFAULT 1000,
    circuit_breaker_threshold INT NOT NULL DEFAULT 5,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Model registry with pricing
CREATE TABLE models (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    provider_id UUID NOT NULL REFERENCES providers(id),
    model_id VARCHAR(128) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    tier VARCHAR(20) NOT NULL,
    input_cost_per_million DOUBLE PRECISION NOT NULL DEFAULT 0,
    output_cost_per_million DOUBLE PRECISION NOT NULL DEFAULT 0,
    max_context_length INT NOT NULL DEFAULT 4096,
    supports_vision BOOLEAN NOT NULL DEFAULT false,
    supports_tools BOOLEAN NOT NULL DEFAULT false,
    supports_streaming BOOLEAN NOT NULL DEFAULT true,
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider_id, model_id)
);

CREATE INDEX idx_models_provider ON models(provider_id);
CREATE INDEX idx_models_tier ON models(tier);

-- Dashboard users
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255),
    password_hash VARCHAR(255),
    google_id VARCHAR(255),
    role VARCHAR(20) NOT NULL DEFAULT 'user',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Settings (key-value store for app configuration)
CREATE TABLE settings (
    key VARCHAR(128) PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Step 5: Verify compiles**

Run: `cd gateway && cargo check`
Expected: Compiles (may warn about unused imports, that's OK)

**Step 6: Commit**

```bash
git add gateway/
git commit -m "feat: add Actix-Web server with health check, DB migrations"
```

---

### Task 1.4: Core Types — Request, Response, Routing Models

**Files:**
- Create: `gateway/src/types/mod.rs`
- Create: `gateway/src/types/request.rs`
- Create: `gateway/src/types/response.rs`
- Create: `gateway/src/types/routing.rs`

**Step 1: Write request types (OpenAI + Anthropic compatible)**

```rust
// gateway/src/types/request.rs
use serde::{Deserialize, Serialize};

/// OpenAI-compatible chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: Option<String>,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<serde_json::Value>,
    // Routing hints (native API extension)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_cost_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_tier_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub r#type: String,
    pub function: FunctionDef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Anthropic Messages API request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicRequest {
    pub model: Option<String>,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: serde_json::Value,
}
```

**Step 2: Write response types**

```rust
// gateway/src/types/response.rs
use serde::{Deserialize, Serialize};

/// OpenAI-compatible chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
    // Router metadata extension
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_router_metadata: Option<RouterMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: super::request::Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterMetadata {
    pub provider: String,
    pub tier: String,
    pub cost_profile: String,
    pub cost_usd: f64,
    pub cost_without_router_usd: f64,
    pub savings_percent: f64,
    pub cache_hit: bool,
    pub latency_ms: u64,
    pub scoring_ms: f64,
}
```

**Step 3: Write routing types**

```rust
// gateway/src/types/routing.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    Simple,
    Medium,
    Complex,
    Reasoning,
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tier::Simple => write!(f, "SIMPLE"),
            Tier::Medium => write!(f, "MEDIUM"),
            Tier::Complex => write!(f, "COMPLEX"),
            Tier::Reasoning => write!(f, "REASONING"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CostProfile {
    Eco,
    Auto,
    Premium,
    Free,
}

impl CostProfile {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "ECO" => Self::Eco,
            "PREMIUM" => Self::Premium,
            "FREE" => Self::Free,
            _ => Self::Auto,
        }
    }
}

impl std::fmt::Display for CostProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CostProfile::Eco => write!(f, "ECO"),
            CostProfile::Auto => write!(f, "AUTO"),
            CostProfile::Premium => write!(f, "PREMIUM"),
            CostProfile::Free => write!(f, "FREE"),
        }
    }
}

/// 15-dimensional scoring result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringResult {
    pub input_tokens: u32,
    pub context_length: u32,
    pub code_percentage: f64,
    pub language_count: u8,
    pub step_count: u8,
    pub tool_call_count: u8,
    pub output_requirement: f64,
    pub accuracy_need: f64,
    pub latency_need: f64,
    pub consistency_need: f64,
    pub needs_reasoning: bool,
    pub needs_vision: bool,
    pub needs_tools: bool,
    pub user_tier: String,
    pub budget_remaining: f64,
    // Result
    pub tier: Tier,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelection {
    pub model_id: String,
    pub provider: String,
    pub tier: Tier,
    pub input_cost_per_million: f64,
    pub output_cost_per_million: f64,
    pub max_context: u32,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub supports_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub scoring: ScoringResult,
    pub cost_profile: CostProfile,
    pub selected_model: ModelSelection,
    pub fallback_models: Vec<ModelSelection>,
    pub decision_time_us: u64,
}
```

**Step 4: Write types mod.rs**

```rust
// gateway/src/types/mod.rs
pub mod request;
pub mod response;
pub mod routing;

pub use request::*;
pub use response::*;
pub use routing::*;
```

**Step 5: Update main.rs to include types module**

Add `mod types;` to main.rs.

**Step 6: Verify compiles**

Run: `cd gateway && cargo check`
Expected: Compiles without errors

**Step 7: Commit**

```bash
git add gateway/src/types/
git commit -m "feat: add core types for requests, responses, and routing"
```

---

### Task 1.5: Scoring Engine (15-Dimensional)

**Files:**
- Create: `gateway/src/routing/mod.rs`
- Create: `gateway/src/routing/scorer.rs`
- Create: `gateway/tests/scoring_tests.rs`

**Step 1: Write failing tests for scoring engine**

```rust
// gateway/tests/scoring_tests.rs
use openclaw_gateway::routing::scorer::RequestScorer;
use openclaw_gateway::types::request::{ChatCompletionRequest, Message};
use openclaw_gateway::types::routing::Tier;

#[test]
fn test_simple_query_scores_as_simple() {
    let scorer = RequestScorer::new();
    let request = ChatCompletionRequest {
        model: None,
        messages: vec![Message {
            role: "user".to_string(),
            content: Some(serde_json::json!("What is 2 + 2?")),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }],
        temperature: None,
        max_tokens: None,
        stream: None,
        top_p: None,
        tools: None,
        tool_choice: None,
        response_format: None,
        x_cost_profile: None,
        x_tier_hint: None,
    };
    let result = scorer.score(&request);
    assert_eq!(result.tier, Tier::Simple);
    assert!(result.confidence > 0.5);
}

#[test]
fn test_code_request_scores_higher() {
    let scorer = RequestScorer::new();
    let request = ChatCompletionRequest {
        model: None,
        messages: vec![Message {
            role: "user".to_string(),
            content: Some(serde_json::json!("Write a Rust function that implements a binary search tree with insert, delete, and search operations. Include comprehensive error handling and unit tests.")),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }],
        temperature: None,
        max_tokens: None,
        stream: None,
        top_p: None,
        tools: None,
        tool_choice: None,
        response_format: None,
        x_cost_profile: None,
        x_tier_hint: None,
    };
    let result = scorer.score(&request);
    assert!(result.tier == Tier::Medium || result.tier == Tier::Complex);
}

#[test]
fn test_tool_calling_request() {
    let scorer = RequestScorer::new();
    let request = ChatCompletionRequest {
        model: None,
        messages: vec![Message {
            role: "user".to_string(),
            content: Some(serde_json::json!("Get the weather in London")),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }],
        temperature: None,
        max_tokens: None,
        stream: None,
        top_p: None,
        tools: Some(vec![]),
        tool_choice: None,
        response_format: None,
        x_cost_profile: None,
        x_tier_hint: None,
    };
    let result = scorer.score(&request);
    assert!(result.needs_tools);
}

#[test]
fn test_scoring_completes_under_1ms() {
    let scorer = RequestScorer::new();
    let request = ChatCompletionRequest {
        model: None,
        messages: vec![Message {
            role: "user".to_string(),
            content: Some(serde_json::json!("Explain quantum computing in detail with examples")),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }],
        temperature: None,
        max_tokens: None,
        stream: None,
        top_p: None,
        tools: None,
        tool_choice: None,
        response_format: None,
        x_cost_profile: None,
        x_tier_hint: None,
    };

    let start = std::time::Instant::now();
    let _result = scorer.score(&request);
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 1, "Scoring took {}ms, must be <1ms", elapsed.as_millis());
}
```

**Step 2: Run tests — should fail**

Run: `cd gateway && cargo test --test scoring_tests`
Expected: FAIL (module not found)

**Step 3: Implement scoring engine**

```rust
// gateway/src/routing/scorer.rs
use crate::types::request::ChatCompletionRequest;
use crate::types::routing::{ScoringResult, Tier};

pub struct RequestScorer;

impl RequestScorer {
    pub fn new() -> Self {
        Self
    }

    pub fn score(&self, request: &ChatCompletionRequest) -> ScoringResult {
        let content = self.extract_content(request);
        let content_len = content.len();

        // Dimension 1: Input tokens (rough estimate: 1 token ≈ 4 chars)
        let input_tokens = (content_len / 4).max(1) as u32;

        // Dimension 2: Context length requirement
        let context_length = input_tokens * 2; // Estimate needed context

        // Dimension 3: Code percentage
        let code_percentage = self.detect_code_percentage(&content);

        // Dimension 4: Language count
        let language_count = self.count_languages(&content);

        // Dimension 5: Step count (multi-step indicators)
        let step_count = self.count_steps(&content);

        // Dimension 6: Tool calls
        let tool_call_count = request.tools.as_ref().map_or(0, |t| t.len() as u8);

        // Dimension 7: Output requirement complexity
        let output_requirement = self.assess_output_requirement(request);

        // Dimension 8: Accuracy need
        let accuracy_need = self.assess_accuracy_need(&content);

        // Dimension 9: Latency need
        let latency_need = if content_len < 100 { 0.8 } else { 0.4 };

        // Dimension 10: Consistency need
        let consistency_need = if request.response_format.is_some() { 0.9 } else { 0.3 };

        // Dimension 11: Reasoning model needed
        let needs_reasoning = self.needs_reasoning(&content);

        // Dimension 12: Vision support
        let needs_vision = self.needs_vision(request);

        // Dimension 13: Tool calling
        let needs_tools = tool_call_count > 0;

        // Dimension 14: User tier (default)
        let user_tier = "free".to_string();

        // Dimension 15: Budget remaining (default)
        let budget_remaining = 100.0;

        // Calculate composite score and classify tier
        let complexity_score = self.calculate_complexity(
            input_tokens, code_percentage, language_count, step_count,
            tool_call_count, output_requirement, accuracy_need,
            needs_reasoning, needs_vision, needs_tools,
        );

        let (tier, confidence) = self.classify_tier(complexity_score);

        ScoringResult {
            input_tokens,
            context_length,
            code_percentage,
            language_count,
            step_count,
            tool_call_count,
            output_requirement,
            accuracy_need,
            latency_need,
            consistency_need,
            needs_reasoning,
            needs_vision,
            needs_tools,
            user_tier,
            budget_remaining,
            tier,
            confidence,
        }
    }

    fn extract_content(&self, request: &ChatCompletionRequest) -> String {
        request.messages.iter()
            .filter_map(|m| m.content.as_ref())
            .map(|c| match c {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn detect_code_percentage(&self, content: &str) -> f64 {
        let code_indicators = [
            "```", "function ", "def ", "class ", "impl ", "fn ",
            "const ", "let ", "var ", "import ", "require(", "pub ",
            "async ", "await ", "return ", "if (", "for (", "while (",
            "=>", "->", "::", "&&", "||", "!=", "==",
        ];
        let words: Vec<&str> = content.split_whitespace().collect();
        if words.is_empty() { return 0.0; }
        let code_word_count = words.iter()
            .filter(|w| code_indicators.iter().any(|ind| w.contains(ind)))
            .count();
        (code_word_count as f64 / words.len() as f64).min(1.0)
    }

    fn count_languages(&self, content: &str) -> u8 {
        let langs = [
            ("rust", &["fn ", "impl ", "pub ", "mod ", "use ", "cargo"][..]),
            ("python", &["def ", "import ", "class ", "self.", "pip"][..]),
            ("javascript", &["const ", "let ", "var ", "function ", "=>", "npm"][..]),
            ("typescript", &["interface ", "type ", "enum ", ": string", ": number"][..]),
            ("go", &["func ", "package ", "import (", "go mod"][..]),
            ("java", &["public class", "private ", "protected ", "void "][..]),
        ];
        let lower = content.to_lowercase();
        langs.iter()
            .filter(|(_, keywords)| keywords.iter().any(|k| lower.contains(k)))
            .count() as u8
    }

    fn count_steps(&self, content: &str) -> u8 {
        let step_indicators = [
            "step ", "first", "then", "next", "after that", "finally",
            "1.", "2.", "3.", "- ", "* ",
        ];
        let lower = content.to_lowercase();
        step_indicators.iter()
            .filter(|ind| lower.contains(*ind))
            .count()
            .min(10) as u8
    }

    fn assess_output_requirement(&self, request: &ChatCompletionRequest) -> f64 {
        let mut score = 0.3;
        if request.response_format.is_some() { score += 0.3; }
        if request.tools.is_some() { score += 0.2; }
        if request.max_tokens.unwrap_or(0) > 2000 { score += 0.2; }
        score.min(1.0)
    }

    fn assess_accuracy_need(&self, content: &str) -> f64 {
        let accuracy_indicators = [
            "exact", "precise", "accurate", "correct", "verify",
            "calculate", "compute", "math", "equation",
        ];
        let lower = content.to_lowercase();
        let count = accuracy_indicators.iter()
            .filter(|ind| lower.contains(*ind))
            .count();
        (0.3 + count as f64 * 0.15).min(1.0)
    }

    fn needs_reasoning(&self, content: &str) -> bool {
        let reasoning_indicators = [
            "reason", "analyze", "compare", "evaluate", "prove",
            "deduce", "infer", "conclude", "argue", "debate",
            "pros and cons", "trade-off", "think step by step",
        ];
        let lower = content.to_lowercase();
        reasoning_indicators.iter().any(|ind| lower.contains(ind))
    }

    fn needs_vision(&self, request: &ChatCompletionRequest) -> bool {
        request.messages.iter().any(|m| {
            if let Some(content) = &m.content {
                if let serde_json::Value::Array(parts) = content {
                    return parts.iter().any(|p| {
                        p.get("type").and_then(|t| t.as_str()) == Some("image_url")
                    });
                }
            }
            false
        })
    }

    fn calculate_complexity(
        &self,
        input_tokens: u32,
        code_pct: f64,
        lang_count: u8,
        step_count: u8,
        tool_count: u8,
        output_req: f64,
        accuracy: f64,
        needs_reasoning: bool,
        needs_vision: bool,
        needs_tools: bool,
    ) -> f64 {
        let mut score = 0.0;

        // Token-based complexity (0-25)
        score += match input_tokens {
            0..=50 => 5.0,
            51..=200 => 10.0,
            201..=1000 => 15.0,
            1001..=5000 => 20.0,
            _ => 25.0,
        };

        // Code complexity (0-20)
        score += code_pct * 15.0 + lang_count as f64 * 2.5;

        // Multi-step complexity (0-15)
        score += step_count as f64 * 2.0;

        // Tool complexity (0-10)
        score += tool_count as f64 * 3.0;
        if needs_tools { score += 2.0; }

        // Output complexity (0-10)
        score += output_req * 10.0;

        // Accuracy requirement (0-10)
        score += accuracy * 10.0;

        // Special requirements (0-10)
        if needs_reasoning { score += 8.0; }
        if needs_vision { score += 5.0; }

        score
    }

    fn classify_tier(&self, score: f64) -> (Tier, f64) {
        match score {
            s if s < 20.0 => (Tier::Simple, 1.0 - s / 20.0 * 0.3),
            s if s < 40.0 => (Tier::Medium, 0.7 + (s - 20.0) / 20.0 * 0.2),
            s if s < 65.0 => (Tier::Complex, 0.7 + (s - 40.0) / 25.0 * 0.2),
            s => (Tier::Reasoning, (0.7 + (s - 65.0) / 35.0 * 0.3).min(1.0)),
        }
    }
}
```

**Step 4: Write routing mod.rs**

```rust
// gateway/src/routing/mod.rs
pub mod scorer;
```

**Step 5: Make types and routing public in lib.rs**

Create `gateway/src/lib.rs`:
```rust
pub mod types;
pub mod routing;
pub mod error;
pub mod config;
```

**Step 6: Run tests — should pass**

Run: `cd gateway && cargo test --test scoring_tests`
Expected: All 4 tests pass, scoring under 1ms

**Step 7: Commit**

```bash
git add gateway/src/routing/ gateway/src/lib.rs gateway/tests/
git commit -m "feat: implement 15-dimensional scoring engine with tests"
```

---

### Task 1.6: Provider Trait and OpenAI Provider

**Files:**
- Create: `gateway/src/providers/mod.rs`
- Create: `gateway/src/providers/traits.rs`
- Create: `gateway/src/providers/openai.rs`

**Step 1: Write provider trait**

```rust
// gateway/src/providers/traits.rs
use async_trait::async_trait;
use crate::error::AppError;
use crate::types::request::ChatCompletionRequest;
use crate::types::response::ChatCompletionResponse;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_healthy(&self) -> bool;
    async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
        model_id: &str,
    ) -> Result<ChatCompletionResponse, AppError>;
    async fn health_check(&self) -> bool;
}
```

**Step 2: Implement OpenAI provider**

```rust
// gateway/src/providers/openai.rs
use async_trait::async_trait;
use reqwest::Client;
use crate::error::AppError;
use crate::types::request::ChatCompletionRequest;
use crate::types::response::ChatCompletionResponse;
use super::traits::LlmProvider;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
    healthy: AtomicBool,
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com".to_string()),
            healthy: AtomicBool::new(true),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
        model_id: &str,
    ) -> Result<ChatCompletionResponse, AppError> {
        let mut req_body = serde_json::to_value(request)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        req_body["model"] = serde_json::json!(model_id);
        // Remove custom fields
        req_body.as_object_mut().map(|o| {
            o.remove("x_cost_profile");
            o.remove("x_tier_hint");
        });

        let response = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            self.healthy.store(false, Ordering::Relaxed);
            return Err(AppError::ProviderError(
                format!("OpenAI returned {}: {}", status, body)
            ));
        }

        self.healthy.store(true, Ordering::Relaxed);
        let result: ChatCompletionResponse = response.json().await
            .map_err(|e| AppError::ProviderError(e.to_string()))?;
        Ok(result)
    }

    async fn health_check(&self) -> bool {
        let result = self.client
            .get(format!("{}/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await;
        let healthy = result.map_or(false, |r| r.status().is_success());
        self.healthy.store(healthy, Ordering::Relaxed);
        healthy
    }
}
```

**Step 3: Write providers mod.rs**

```rust
// gateway/src/providers/mod.rs
pub mod traits;
pub mod openai;

pub use traits::LlmProvider;
pub use openai::OpenAiProvider;
```

**Step 4: Add to lib.rs**

Add `pub mod providers;` to `gateway/src/lib.rs`.

**Step 5: Verify compiles**

Run: `cd gateway && cargo check`
Expected: Compiles

**Step 6: Commit**

```bash
git add gateway/src/providers/ gateway/src/lib.rs
git commit -m "feat: add provider trait and OpenAI provider implementation"
```

---

### Task 1.7: OpenAI-Compatible Proxy Endpoint

**Files:**
- Create: `gateway/src/routes/completions.rs`
- Modify: `gateway/src/routes/mod.rs`
- Modify: `gateway/src/main.rs`

**Step 1: Write the completions handler**

```rust
// gateway/src/routes/completions.rs
use actix_web::{web, HttpRequest, HttpResponse};
use crate::error::AppError;
use crate::types::request::ChatCompletionRequest;
use crate::types::response::RouterMetadata;
use crate::routing::scorer::RequestScorer;
use crate::providers::traits::LlmProvider;
use std::sync::Arc;
use std::time::Instant;

pub struct AppState {
    pub scorer: RequestScorer,
    pub providers: Vec<Arc<dyn LlmProvider>>,
}

pub async fn chat_completions(
    req: HttpRequest,
    body: web::Json<ChatCompletionRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let start = Instant::now();
    let request = body.into_inner();

    // 1. Score the request
    let scoring = state.scorer.score(&request);
    let scoring_time = start.elapsed();

    // 2. Select model (for now, use the model from request or default)
    let model_id = request.model.clone()
        .unwrap_or_else(|| "gpt-4o-mini".to_string());

    // 3. Find a healthy provider
    let provider = state.providers.iter()
        .find(|p| p.is_healthy())
        .ok_or_else(|| AppError::ProviderError("No healthy providers available".to_string()))?;

    // 4. Forward request
    let mut response = provider.chat_completion(&request, &model_id).await?;
    let total_time = start.elapsed();

    // 5. Add router metadata
    response.x_router_metadata = Some(RouterMetadata {
        provider: provider.name().to_string(),
        tier: scoring.tier.to_string(),
        cost_profile: "AUTO".to_string(),
        cost_usd: 0.0, // TODO: calculate from token usage
        cost_without_router_usd: 0.0,
        savings_percent: 0.0,
        cache_hit: false,
        latency_ms: total_time.as_millis() as u64,
        scoring_ms: scoring_time.as_secs_f64() * 1000.0,
    });

    Ok(HttpResponse::Ok().json(response))
}
```

**Step 2: Update routes/mod.rs**

```rust
// gateway/src/routes/mod.rs
pub mod health;
pub mod completions;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .route("/health", web::get().to(health::health_check))
            .route("/ready", web::get().to(health::ready_check))
            .route("/v1/chat/completions", web::post().to(completions::chat_completions))
    );
}
```

**Step 3: Update main.rs to wire up state**

Update main.rs to create AppState with scorer and providers, and pass it via `web::Data`.

**Step 4: Verify compiles**

Run: `cd gateway && cargo check`
Expected: Compiles

**Step 5: Commit**

```bash
git add gateway/src/routes/ gateway/src/main.rs
git commit -m "feat: add OpenAI-compatible /v1/chat/completions endpoint"
```

---

## Phase 2: Intelligent Routing

### Task 2.1: Model Registry

**Files:**
- Create: `gateway/src/routing/registry.rs`
- Create: `gateway/tests/registry_tests.rs`

The model registry is a thread-safe in-memory store of all available models with their pricing and capabilities. It's populated from the database on startup and refreshed periodically from OpenRouter/Gemini pricing APIs.

**Implementation:** Use `DashMap` for lock-free concurrent access. Store `ModelSelection` structs keyed by `(provider, model_id)`. Provide methods: `get_models_for_tier(tier, cost_profile)`, `get_cheapest_model(tier)`, `update_pricing(provider, model_id, new_pricing)`.

---

### Task 2.2: Model Selector

**Files:**
- Create: `gateway/src/routing/selector.rs`
- Create: `gateway/tests/selector_tests.rs`

Given a `ScoringResult` and `CostProfile`, the selector picks the optimal model from the registry. Logic:
1. Filter models by tier capability
2. Filter by required features (vision, tools, streaming)
3. Sort by cost (ascending for ECO, balanced for AUTO, quality for PREMIUM)
4. Return top match + fallback list

---

### Task 2.3: Cascade Router

**Files:**
- Create: `gateway/src/routing/cascade.rs`
- Create: `gateway/tests/cascade_tests.rs`

Implements cascade routing: try cheapest model first, if response quality is low (detected via heuristics like response length, confidence markers), escalate to next tier.

---

### Task 2.4: Real-Time Pricing Sync

**Files:**
- Create: `gateway/src/pricing/mod.rs`
- Create: `gateway/src/pricing/openrouter.rs`
- Create: `gateway/src/pricing/gemini.rs`

Background task that fetches `GET https://openrouter.ai/api/v1/models` every 5 minutes and updates the model registry with current pricing.

---

## Phase 3: Optimization Layer

### Task 3.1: Semantic Cache (Redis-Based)

**Files:**
- Create: `gateway/src/optimization/mod.rs`
- Create: `gateway/src/optimization/cache.rs`
- Create: `gateway/tests/cache_tests.rs`

Implement semantic caching: hash request content, store in Redis with TTL. For exact cache: use SHA-256 hash of normalized messages. For semantic cache: use cosine similarity on embeddings (requires embedding endpoint).

---

### Task 3.2: Prompt Compression

**Files:**
- Create: `gateway/src/optimization/compression.rs`
- Create: `gateway/tests/compression_tests.rs`

Implement lightweight prompt compression: remove redundant whitespace, consolidate repeated instructions, trim system prompts to essential parts. Target 10-30% token reduction.

---

### Task 3.3: Request Deduplication

**Files:**
- Create: `gateway/src/optimization/dedup.rs`

Use `DashMap` to track in-flight requests by content hash. If a duplicate arrives within 1 second, share the response from the first request.

---

## Phase 4: Resilience

### Task 4.1: Circuit Breaker

**Files:**
- Create: `gateway/src/resilience/mod.rs`
- Create: `gateway/src/resilience/circuit_breaker.rs`
- Create: `gateway/tests/circuit_breaker_tests.rs`

Per-provider circuit breaker with three states: Closed (normal), Open (skip provider for cooldown), Half-Open (probe with single request). Track failure counts, auto-open after threshold (5 failures in 60 seconds), cooldown of 30 seconds.

---

### Task 4.2: Rate Limiter (Token-Aware)

**Files:**
- Create: `gateway/src/resilience/rate_limiter.rs`
- Create: `gateway/tests/rate_limiter_tests.rs`

Redis-based token bucket rate limiter. Per API key. Track both request count and estimated token usage. Use Redis `MULTI/EXEC` for atomic increment and check.

---

### Task 4.3: Retry with Exponential Backoff

**Files:**
- Create: `gateway/src/resilience/retry.rs`

Implement retry logic: `delay = base_delay * 2^attempt + jitter`. Max 3 retries. Only retry on 429 (rate limit) and 5xx errors. Skip 4xx errors.

---

### Task 4.4: Fallback Chain

**Files:**
- Create: `gateway/src/resilience/fallback.rs`

When primary provider fails (after retries), automatically try next provider in priority order. Integrate with circuit breaker (skip open circuits).

---

## Phase 5: All Providers

### Task 5.1-5.7: Provider Implementations

For each provider (Ollama, OpenRouter, Gemini, xAI, DeepSeek, Bedrock, Custom), implement the `LlmProvider` trait. Each provider file follows the same pattern as `openai.rs` but with provider-specific API format, auth, and response parsing.

**Files per provider:**
- `gateway/src/providers/{name}.rs`
- `gateway/tests/{name}_provider_tests.rs`

---

## Phase 6: Dashboard

### Task 6.1: Initialize Next.js Project

**Files:**
- Create: `dashboard/` (entire Next.js project)

```bash
cd c:/Users/Narasamma/workspace/openclaw-router
npx create-next-app@latest dashboard --typescript --tailwind --app --src-dir
cd dashboard
npm install next-auth@5 next-themes recharts axios
```

---

### Task 6.2: Auth Setup (Google + Local)

**Files:**
- Create: `dashboard/src/auth.ts`
- Create: `dashboard/src/auth.config.ts`
- Create: `dashboard/src/app/api/auth/[...nextauth]/route.ts`
- Create: `dashboard/src/app/login/page.tsx`
- Create: `dashboard/src/middleware.ts`

NextAuth.js v5 with Google OAuth provider + Credentials provider. Protected routes via middleware.

---

### Task 6.3: Dashboard Layout

**Files:**
- Create: `dashboard/src/app/dashboard/layout.tsx` (sidebar + header)
- Create: `dashboard/src/components/Sidebar.tsx`
- Create: `dashboard/src/components/Header.tsx`
- Create: `dashboard/src/components/ThemeToggle.tsx`
- Create: `dashboard/src/app/providers.tsx`

Sidebar navigation with links to all dashboard pages. Header with user info and theme toggle. Dark/light mode via next-themes.

---

### Task 6.4: Overview Page

**Files:**
- Create: `dashboard/src/app/dashboard/page.tsx`
- Create: `dashboard/src/components/MetricCard.tsx`

Display: total requests, success rate, avg latency, uptime, cost savings summary. Metric cards with sparkline charts.

---

### Task 6.5: Analytics Page

**Files:**
- Create: `dashboard/src/app/dashboard/analytics/page.tsx`

Recharts: request volume over time (line chart), requests by provider (bar chart), requests by tier (pie chart), latency distribution (histogram).

---

### Task 6.6: Cost Tracker Page

**Files:**
- Create: `dashboard/src/app/dashboard/costs/page.tsx`

Cost with router vs without, in INR and USD. Daily/weekly/monthly trends. Savings percentage. Currency toggle (INR default).

---

### Task 6.7: Provider Health Page

**Files:**
- Create: `dashboard/src/app/dashboard/providers/page.tsx`

Provider status cards (healthy/degraded/down), circuit breaker state, rate limit usage bars, model list per provider.

---

### Task 6.8: Settings Page

**Files:**
- Create: `dashboard/src/app/dashboard/settings/page.tsx`

Provider API keys (masked input), cost profile selector, rate limit config, theme toggle, notification preferences.

---

### Task 6.9: Gateway Admin API

**Files:**
- Create: `gateway/src/routes/admin.rs`

REST API for dashboard to read metrics, update settings, manage providers. Endpoints:
- `GET /api/admin/metrics` — aggregated metrics
- `GET /api/admin/providers` — provider status
- `POST /api/admin/settings` — update settings
- `GET /api/admin/requests` — request log query

---

## Phase 7: Deployment

### Task 7.1: Dockerfiles

**Files:**
- Create: `docker/gateway.Dockerfile` (multi-stage Rust build)
- Create: `docker/dashboard.Dockerfile` (Next.js standalone)
- Create: `docker/docker-compose.yml`
- Create: `docker/nginx.conf`

---

### Task 7.2: Domain + SSL Setup

Configure Nginx reverse proxy:
- `/` → dashboard (port 3000)
- `/v1/` → gateway (port 8080)
- `/api/admin/` → gateway (port 8080)
- SSL via Let's Encrypt certbot

---

### Task 7.3: CI/CD Pipeline

**Files:**
- Modify: `.github/workflows/test.yml`
- Create: `.github/workflows/deploy.yml`

Test on push (Rust tests + Next.js tests), deploy on main merge (Docker build + push + restart).

---

## Phase 8: Testing

### Task 8.1: Rust Unit Tests (100+)

Comprehensive unit tests for: scoring engine, model selector, cascade router, cache, compression, dedup, circuit breaker, rate limiter, retry logic, format translation, each provider.

### Task 8.2: Rust Integration Tests (30+)

Tests with real Redis/PostgreSQL (Docker containers): request pipeline end-to-end, provider failover, cache hit/miss flows, rate limiting behavior.

### Task 8.3: Benchmark Tests (Criterion)

**Files:**
- Create: `gateway/benches/routing.rs`

Benchmark: scoring (<1ms), model selection (<5ms), cache lookup (<2ms), full pipeline (<100ms).

### Task 8.4: Dashboard Tests

Vitest for components, Playwright for E2E user journeys (login, view dashboard, change settings).

### Task 8.5: Load Testing

k6 script hitting `/v1/chat/completions` with 100+ concurrent users. Target: >1000 RPS, P99 <50ms.

---

## Phase 9: Documentation & Push

### Task 9.1: API Documentation

**Files:**
- Create: `docs/API.md`

Full API reference for all endpoints with request/response examples.

### Task 9.2: Architecture Documentation

**Files:**
- Create: `docs/ARCHITECTURE.md`

System architecture, data flow diagrams, component descriptions.

### Task 9.3: Deployment Guide

**Files:**
- Create: `docs/DEPLOYMENT.md`

Step-by-step guide to deploy on any server.

### Task 9.4: README with Badges

**Files:**
- Rewrite: `README.md`

Project overview, features, quick start, badges (CI, license, version).

### Task 9.5: Push to GitHub

```bash
git push origin main
```

---

## Execution Order Summary

| Phase | Tasks | Estimated Steps | Dependencies |
|-------|-------|----------------|--------------|
| 1. Foundation | 1.1–1.7 | ~45 | None |
| 2. Routing | 2.1–2.4 | ~30 | Phase 1 |
| 3. Optimization | 3.1–3.3 | ~25 | Phase 1 |
| 4. Resilience | 4.1–4.4 | ~25 | Phase 1 |
| 5. Providers | 5.1–5.7 | ~35 | Phases 1-2 |
| 6. Dashboard | 6.1–6.9 | ~60 | Phases 1-4 (gateway API needed) |
| 7. Deployment | 7.1–7.3 | ~20 | Phases 1-6 |
| 8. Testing | 8.1–8.5 | ~40 | Phases 1-6 |
| 9. Docs & Push | 9.1–9.5 | ~15 | All phases |

**Phases 2, 3, and 4 can be developed in parallel** after Phase 1 is complete.
**Phase 6 (Dashboard) can start after Phase 1** — gateway API endpoints are added incrementally.
