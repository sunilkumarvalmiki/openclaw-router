# OpenClaw Router -- Architecture

> **Version**: 2.0 | **Last updated**: 2026-03-01

---

## Table of Contents

- [System Overview](#system-overview)
- [Architecture Diagram](#architecture-diagram)
- [Request Pipeline](#request-pipeline)
- [15-Dimensional Scoring Engine](#15-dimensional-scoring-engine)
- [Tier Classification](#tier-classification)
- [Cost Profiles and Model Selection](#cost-profiles-and-model-selection)
- [Cascade Routing](#cascade-routing)
- [Provider Implementations](#provider-implementations)
- [Caching Strategy](#caching-strategy)
- [Resilience Patterns](#resilience-patterns)
- [Data Flow](#data-flow)
- [Dashboard Architecture](#dashboard-architecture)
- [Technology Choices](#technology-choices)

---

## System Overview

OpenClaw Router is an intelligent LLM routing gateway that sits between clients (Cursor, Claude Code, OpenAI SDKs, custom applications) and multiple LLM providers. It analyzes every incoming request across 15 dimensions, classifies its complexity, and routes it to the optimal model/provider combination based on cost profile, quality requirements, and provider health.

The system consists of three main components:

1. **Rust Gateway** -- High-performance API server built with Actix-Web 4. Handles request scoring, model selection, provider dispatch, caching, rate limiting, and circuit breaking. Designed for sub-millisecond routing overhead.

2. **Next.js Dashboard** -- Administrative web interface at sunilkumarvalmiki.xyz. Provides real-time visibility into request analytics, cost tracking (INR/USD/EUR), provider health, cache performance, and system configuration.

3. **Infrastructure Services** -- PostgreSQL 16 for persistent storage, Redis 7 for caching and rate limiting, and Nginx as a reverse proxy with TLS termination.

---

## Architecture Diagram

```
                    +----------------------------------------------+
                    |       Clients (Cursor / Claude Code / API)    |
                    +---------------------+------------------------+
                                          | HTTPS
                    +---------------------v------------------------+
                    |             Nginx Reverse Proxy               |
                    |   TLS termination | Rate limiting | Gzip      |
                    |   /v1/* -> Gateway | /* -> Dashboard          |
                    +----------+------------------+----------------+
                               |                  |
              +----------------v---+    +---------v-----------+
              |   Rust Gateway     |    |  Next.js Dashboard  |
              |   (Actix-Web 4)    |    |  (Next.js 16)       |
              |                    |    |                      |
              |  +------+------+  |    |  Overview            |
              |  | Auth | Rate |  |    |  Analytics            |
              |  | Layer| Limit|  |    |  Costs (INR/USD/EUR) |
              |  +--+---+--+--+  |    |  Providers            |
              |     |      |     |    |  Cache                |
              |  +--v------v--+  |    |  Settings             |
              |  |  Request   |  |    +----------+------------+
              |  |  Pipeline  |  |               |
              |  |            |  |               |
              |  | Score (15d)|  |               |
              |  | Classify   |  |               |
              |  | Select     |  |               |
              |  | Optimize   |  |               |
              |  +-----+------+  |               |
              |        |         |               |
              |  +-----v------+  |               |
              |  |  Provider  |  |               |
              |  |  Manager   |  |               |
              |  |            |  |               |
              |  | OpenAI     |  |               |
              |  | Gemini     |  |               |
              |  | OpenRouter |  |               |
              |  | Ollama     |  |               |
              |  | xAI        |  |               |
              |  | DeepSeek   |  |               |
              |  | Bedrock    |  |               |
              |  | Custom     |  |               |
              |  +-----+------+  |               |
              |        |         |               |
              |  +-----+------+  |               |
              |  | Resilience |  |               |
              |  | Circuit Brk|  |               |
              |  | Retry      |  |               |
              |  | Fallback   |  |               |
              |  +------------+  |               |
              +---+----------+---+               |
                  |          |                   |
       +----------v--+  +---v-----------+       |
       | PostgreSQL   |  |    Redis      |       |
       | 16-Alpine    |  |    7-Alpine   |<------+
       |              |  |               |
       | - Migrations |  | - Response    |
       | - Analytics  |  |   Cache       |
       | - Users      |  | - Rate Limits |
       | - API Keys   |  | - Cache Stats |
       +--------------+  +---------------+
```

---

## Request Pipeline

Every request flows through a 10-step pipeline designed for minimal latency and maximum routing intelligence.

```
Client Request
     |
     v
+----+------+     +----------+     +-----------+     +---------+
| 1. Auth   |---->| 2. Rate  |---->| 3. Format |---->| 4. Score|
|   Validate|     |   Limit  |     | Translate |     | (15-dim)|
+-----------+     +----------+     +-----------+     +----+----+
                                                          |
+----------+     +-----------+     +----------+     +-----v-----+
|10. Return|<----| 9. Provider|<----| 8. Prompt|<----| 5. Classify|
|  Response|     |   Dispatch|     | Optimize |     |   Tier    |
+----------+     +-----------+     +----------+     +-----+-----+
     ^                                                    |
     |           +-----------+     +----------+     +-----v-----+
     +-----------| 7. Model  |<----| 6. Cache |<----| Classified|
      (metrics)  |  Selection|     |  Check   |     |  Request  |
                 +-----------+     +----------+     +-----------+
```

### Step Details

| Step | Component | Latency | Description |
|------|-----------|---------|-------------|
| 1 | Auth Layer | <0.1ms | Validate API key or JWT token from `Authorization` header. |
| 2 | Rate Limiter | <0.1ms | Token-aware rate limiting per API key. Uses in-memory token bucket with 60-second sliding windows. Checks both RPM and TPM limits. |
| 3 | Format Translator | <0.1ms | Detect request format (OpenAI or Anthropic) and normalize to internal representation. Custom `x_*` routing fields are extracted. |
| 4 | Scoring Engine | <1ms | Analyze request across 15 dimensions. Pure computation, no I/O. Produces a composite complexity score (0--100). |
| 5 | Tier Classifier | <0.01ms | Map complexity score to tier: SIMPLE (<20), MEDIUM (20--39), COMPLEX (40--64), REASONING (65+). |
| 6 | Cache Check | <2ms | Compute SHA-256 hash of normalized request. Look up response in Redis. Return immediately on cache hit. |
| 7 | Model Selection | <0.1ms | Query the model registry for candidates matching the tier, cost profile, and required capabilities (vision, tools). Select primary + up to 3 fallbacks. |
| 8 | Prompt Optimization | <0.5ms | Remove empty messages, deduplicate system prompts, collapse redundant whitespace. Reduces token usage by 10--30%. |
| 9 | Provider Dispatch | 100--3000ms | Forward the request to the selected provider. Circuit breaker guards against unhealthy providers. On failure, cascade to fallback models. |
| 10 | Response | <0.5ms | Attach `x_router_metadata`, cache the response, update metrics, and return to client. |

---

## 15-Dimensional Scoring Engine

The scoring engine (`gateway/src/routing/scorer.rs`) performs a pure-computation analysis of every request. It uses zero I/O and completes in sub-millisecond time.

### Dimensions

| # | Dimension | Range | Weight | How It Is Measured |
|---|-----------|-------|--------|--------------------|
| 1 | Input token count | 0--inf | 0--15 | Character count / 4 (rough token estimate). |
| 2 | Context length requirement | 0--100 | 0--10 | Ratio of tokens to 128K context window. |
| 3 | Code percentage | 0--100 | 0--15 | Keyword density of code constructs (`fn`, `def`, `class`, `{`, `=>`, etc.). |
| 4 | Programming language count | 0--6 | 0--10 | Count of languages with >= 2 keyword matches (Rust, Python, JS, TS, Go, Java). |
| 5 | Step count | 0--inf | 0--10 | Count of step markers ("step", "first", "then", numbered lists, bullet points). |
| 6 | Tool call count | 0--inf | 0--10 | Number of tools provided in the `tools` array. |
| 7 | Output requirement | 0--100 | 0--10 | Structured output score: `response_format` (+40), `tools` (+30), high `max_tokens` (+10--20). |
| 8 | Accuracy need | 0--100 | 0--10 | Presence of precision keywords ("exact", "calculate", "verify", "formula", etc.). |
| 9 | Latency need | 0--100 | Informational | Short queries (< 50 tokens) score 90; long queries (> 1000 tokens) score 30. |
| 10 | Consistency need | 0--100 | Informational | Based on `response_format` (+40) and low `temperature` (+15--30). |
| 11 | Needs reasoning | bool | 0--10 | Presence of reasoning keywords ("analyze", "think step by step", "pros and cons"). |
| 12 | Needs vision | bool | 0--5 | Presence of `image_url` content parts in any message. |
| 13 | Needs tools | bool | 0--5 | Whether the `tools` array is non-empty. |
| 14 | User tier | string | Informational | From `x_tier_hint` field: `"free"`, `"pro"`, `"enterprise"`. |
| 15 | Budget remaining | 0--100 | Informational | Normalized remaining budget (defaults to 100.0). |

### Composite Complexity Score

The composite score is computed by summing weighted contributions from dimensions 1--8 and 11--13:

```
score = token_contribution          (0-15)
      + context_length / 100 * 10   (0-10)
      + code_percentage / 100 * 15  (0-15)
      + min(language_count, 3) * 3.33 (0-10)
      + min(step_count, 10)         (0-10)
      + min(tool_count, 5) * 2      (0-10)
      + output_requirement / 100 * 10 (0-10)
      + accuracy_need / 100 * 10    (0-10)
      + (needs_reasoning ? 10 : 0)  (0-10)
      + (needs_vision ? 5 : 0)      (0-5)
      + (needs_tools ? 5 : 0)       (0-5)

Total range: 0-110, clamped to 0-100
```

### Token Count Contribution Brackets

| Input Tokens | Contribution |
|-------------|-------------|
| < 100 | 2 |
| 100--499 | 5 |
| 500--1999 | 8 |
| 2000--7999 | 12 |
| >= 8000 | 15 |

---

## Tier Classification

The composite complexity score maps to one of four tiers:

| Tier | Score Range | Typical Requests | Example Models |
|------|------------|------------------|----------------|
| **SIMPLE** | 0 -- 19 | Short questions, greetings, simple lookups | GPT-3.5 Turbo, Gemini 2.5 Flash Lite, Llama 3.1 8B |
| **MEDIUM** | 20 -- 39 | Moderate tasks, short code, summarization | GPT-4o Mini, Gemini 2.5 Flash, DeepSeek Chat |
| **COMPLEX** | 40 -- 64 | Multi-step code, detailed analysis, tool use | GPT-4o, Llama 3.1 70B, Mistral Large |
| **REASONING** | 65 -- 100 | Chain-of-thought, proofs, advanced reasoning | Gemini 2.5 Pro, Grok 3, DeepSeek Reasoner |

### Confidence Score

The confidence score (0.0--1.0) reflects how far the composite score is from the nearest tier boundary. A score near a boundary (e.g., 19 or 39) produces low confidence, while a score in the middle of a tier range produces high confidence.

```
boundaries = [20, 40, 65]
min_distance = min(|score - b| for b in boundaries)
confidence = min(min_distance / 17.5, 1.0)
```

---

## Cost Profiles and Model Selection

### Model Registry

The model registry (`gateway/src/routing/registry.rs`) maintains a thread-safe collection of all known models using `DashMap` for lock-free concurrent access. Each model entry contains:

- Provider and model identifiers
- Pricing (input/output cost per million tokens)
- Capabilities (vision, tools, streaming)
- Maximum context window
- Tier assignment
- Enable/disable flag

The registry ships with 17 pre-seeded models across 6 providers and can be updated at runtime via the pricing sync task.

### Selection Algorithm

The `ModelSelector` (`gateway/src/routing/selector.rs`) selects the best model in three steps:

1. **Filter by tier**: Retrieve all enabled models capable of handling the request's tier. Models suited for higher tiers can also handle lower tiers.

2. **Filter by capabilities**: Remove models that lack required features (vision, tool calling).

3. **Sort by cost profile**:
   - **ECO**: Sort ascending by blended cost (cheapest first).
   - **FREE**: Only include zero-cost models, sorted by cost.
   - **AUTO**: Sort by a balanced score combining 60% cost weight and 40% quality weight.
   - **PREMIUM**: Sort by tier quality rank descending (highest quality first), then by cost ascending within the same quality level.

4. **Select primary + fallbacks**: The top candidate becomes the primary model. Up to 3 additional candidates become fallback models for cascade routing.

### Balanced Score Formula (AUTO Profile)

```
quality_rank = {Simple: 1, Medium: 2, Complex: 3, Reasoning: 4}
blended_cost = (input_cost_per_million + output_cost_per_million) / 2
quality_component = (5 - quality_rank) * 2
balanced_score = blended_cost * 0.6 + quality_component * 0.4
```

Lower balanced scores are preferred.

---

## Cascade Routing

The cascade router (`gateway/src/routing/cascade.rs`) orchestrates the full routing pipeline with automatic failover and quality escalation.

### Algorithm

```
1. Score the request (15 dimensions)
2. Select primary model + up to 3 fallbacks
3. Send request to primary model
4. IF primary succeeds:
   a. Check response quality (heuristic)
   b. IF quality is low AND request is not a simple question:
      - Try each fallback model in order
      - Return first successful response
   c. IF quality is OK:
      - Return response
5. IF primary fails:
   a. Log warning
   b. Try each fallback model in order
   c. Return first successful response
   d. IF all fail: return error
```

### Quality Heuristic

A response is flagged as low quality when:
- The completion has fewer than 10 tokens, AND
- The actual response text is shorter than 40 characters, AND
- The request does not look like a simple yes/no or short-answer question.

Simple questions are identified by:
- Message length under 50 characters, OR
- Presence of patterns like "is it", "what is", "yes or no", "true or false".

---

## Provider Implementations

All providers implement the `LlmProvider` trait:

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_healthy(&self) -> bool;
    async fn chat_completion(
        &self, request: &ChatCompletionRequest, model_id: &str,
    ) -> Result<ChatCompletionResponse, AppError>;
    async fn health_check(&self) -> bool;
}
```

### Provider Summary

| Provider | Source File | API Base URL | Auth | Key Env Variable |
|----------|-----------|-------------|------|------------------|
| **OpenAI** | `providers/openai.rs` | `https://api.openai.com` | Bearer token | `OPENAI_API_KEY` |
| **Gemini** | `providers/gemini.rs` | `https://generativelanguage.googleapis.com` | API key | `GEMINI_API_KEY` |
| **OpenRouter** | `providers/openrouter.rs` | `https://openrouter.ai/api` | Bearer token | `OPENROUTER_API_KEY` |
| **Ollama** | `providers/ollama.rs` | `http://localhost:11434` | None | `OLLAMA_ENDPOINT` (optional) |
| **xAI** | `providers/xai.rs` | `https://api.x.ai` | Bearer token | `XAI_API_KEY` |
| **DeepSeek** | `providers/deepseek.rs` | `https://api.deepseek.com` | Bearer token | `DEEPSEEK_API_KEY` |
| **Bedrock** | `providers/bedrock.rs` | AWS SDK | IAM credentials | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` |
| **Custom** | `providers/custom.rs` | User-defined | Optional bearer | `CUSTOM_LLM_BASE_URL` |

### Format Translation

All providers receive requests in a normalized OpenAI-compatible format. Before forwarding, custom `x_*` routing fields are stripped from the request body. Responses are normalized to the `ChatCompletionResponse` format regardless of the upstream provider's native format.

### Provider Registration

Providers are instantiated at startup by `ProviderFactory::create_from_env()`. Only providers whose required environment variables are set will be registered. Ollama is always registered (with a configurable endpoint) since it runs locally and requires no API key.

---

## Caching Strategy

### Overview

The response cache (`gateway/src/optimization/cache.rs`) is backed by Redis and provides exact-match caching based on request content. Identical requests are served from cache in under 2ms, compared to 100--3000ms for an LLM call.

### Cache Key Generation

Cache keys are SHA-256 hashes of the normalized request content:

1. **Model normalization**: Include the model identifier if specified.
2. **Message normalization**: Extract text from all messages, collapse whitespace, sort messages by (role, content) for deterministic ordering.
3. **Parameter inclusion**: Include `temperature` and `max_tokens` when set (these affect output).
4. **Exclusion**: Routing hints (`x_cost_profile`, `x_tier_hint`) are excluded since they do not affect the LLM output.

The result is a 64-character hex-encoded SHA-256 digest used as the Redis key with a `cache:` prefix.

### Whitespace Normalization

Multiple spaces, tabs, and newlines are collapsed to a single space. Leading and trailing whitespace is trimmed. This ensures that requests differing only in whitespace produce the same cache key.

### Cache Configuration

| Parameter | Default | Env Variable | Description |
|-----------|---------|-------------|-------------|
| TTL | 3600s (1 hour) | `CACHE_TTL_SECONDS` | Time-to-live for cached responses. |
| Max entries | 10,000 | `MAX_CACHE_ENTRIES` | Soft limit on cache size. |
| Eviction | LRU | N/A | Redis `allkeys-lru` policy at 256MB memory limit. |

### Failure Handling

Redis failures are treated as cache misses -- the gateway never crashes or blocks due to a Redis outage. All Redis errors are logged as warnings and the request proceeds to the LLM provider.

### Cache Statistics

The cache tracks hits and misses via Redis counters (`cache:stats:hits`, `cache:stats:misses`). These can be queried via the dashboard's cache page.

---

## Resilience Patterns

### Circuit Breaker

The circuit breaker (`gateway/src/resilience/circuit_breaker.rs`) prevents cascading failures by tracking provider health.

```
         record_success()
    +---------->-----------+
    |                      |
    v                      |
+--------+   threshold   +------+   cooldown   +-----------+
| CLOSED |---failures--->| OPEN |---expires--->| HALF-OPEN |
+--------+               +------+              +-----------+
    ^                                               |
    |              record_success()                 |
    +<----------------------------------------------+
    |              record_failure()                 |
    +--------------------->+------+                 |
                           | OPEN |<----------------+
                           +------+  (re-opens on failure)
```

| Parameter | Default | Description |
|-----------|---------|-------------|
| Failure threshold | 5 | Consecutive failures before the circuit opens. |
| Cooldown period | 30s | Seconds to wait in OPEN before transitioning to HALF-OPEN. |

**Implementation details**:
- Lock-free using `AtomicU8` for state and `AtomicU32` for failure count.
- Per-provider independent circuit breakers managed by `ProviderCircuitBreakers` registry (backed by `DashMap`).
- OPEN to HALF-OPEN transition uses compare-and-swap (CAS) to prevent race conditions.

### Rate Limiter

The rate limiter (`gateway/src/resilience/rate_limiter.rs`) implements per-API-key token-aware rate limiting.

**Algorithm**: Token bucket with 60-second sliding windows.

- Each API key gets an independent `RateLimitBucket` with RPM and TPM counters.
- Tokens are estimated before the request and deducted optimistically.
- After the response, actual usage is reconciled: surplus tokens are refunded, additional tokens are deducted.
- Windows reset automatically when 60 seconds have elapsed since the window start.

### Retry and Fallback

- **Retry**: Built into the cascade router. Failed primary model requests automatically cascade to fallback models.
- **Fallback**: Up to 3 fallback models are pre-selected during the routing decision. They are tried in order of preference (cheapest for ECO, best quality for PREMIUM).

---

## Data Flow

### Happy Path (Cache Miss)

```
Client --POST /v1/chat/completions--> Nginx
  |
  v
Nginx --proxy_pass--> Gateway:8080
  |
  v
Gateway:
  1. Validate auth
  2. Check rate limit (in-memory)
  3. Score request (<1ms)
  4. Classify tier
  5. Check Redis cache -> MISS
  6. Select model from registry
  7. Compress prompt
  8. Call provider API (100-3000ms)
  9. Receive response
  10. Cache response in Redis (async)
  11. Attach x_router_metadata
  12. Return response
  |
  v
Client <-- 200 OK + JSON response
```

### Happy Path (Cache Hit)

```
Client --POST /v1/chat/completions--> Nginx --> Gateway
  |
  v
Gateway:
  1. Validate auth
  2. Check rate limit
  3. Score request (<1ms)
  4. Check Redis cache -> HIT (<2ms)
  5. Attach x_router_metadata (cache_hit: true)
  6. Return cached response
  |
  v
Client <-- 200 OK + cached response (total: <5ms)
```

### Provider Failure (Cascade)

```
Gateway:
  1. Score + Select: primary=gpt-4o, fallbacks=[gemini-2.5-pro, llama-3.1-70b]
  2. Call gpt-4o -> TIMEOUT
  3. Circuit breaker: record_failure() for openai
  4. Call gemini-2.5-pro -> 200 OK
  5. Return gemini response + metadata (provider: "gemini")
```

---

## Dashboard Architecture

### Technology Stack

- **Framework**: Next.js 16 with App Router
- **UI**: Tailwind CSS 4, Lucide React icons
- **Charts**: Recharts 3
- **Auth**: NextAuth.js v5 (Google OAuth + credentials)
- **Theme**: next-themes (dark/light mode with system preference detection)
- **HTTP Client**: Axios

### Pages

| Route | Page | Description |
|-------|------|-------------|
| `/` | Landing | Public landing page. |
| `/login` | Login | Google OAuth + email/password authentication. |
| `/dashboard` | Overview | Total requests, success rate, avg latency, uptime, cost savings. |
| `/dashboard/analytics` | Analytics | Request volume charts by provider, tier, model. Latency distributions. |
| `/dashboard/costs` | Costs | Cost with/without router in INR, USD, EUR. Daily/weekly/monthly trends. |
| `/dashboard/providers` | Providers | Provider health status, circuit breaker states, model list. |
| `/dashboard/cache` | Cache | Cache hit/miss rates, memory usage, top cached queries. |
| `/dashboard/settings` | Settings | Provider API keys, cost profile, rate limits, theme toggle. |

---

## Technology Choices

| Component | Technology | Justification |
|-----------|-----------|---------------|
| **Gateway** | Rust + Actix-Web 4 | Sub-millisecond latency, memory safety, zero-cost abstractions, async I/O. Actix-Web consistently ranks among the fastest web frameworks. |
| **Scoring Engine** | Rust (zero-alloc pure computation) | No I/O, no allocations in the hot path. Completes in <1ms for typical requests. |
| **Concurrent State** | DashMap + atomics | Lock-free concurrent access for model registry, circuit breakers, and rate limiters. No mutex contention under load. |
| **Database** | PostgreSQL 16 | ACID transactions, JSON support, analytical queries for the dashboard, mature migration tooling (sqlx). |
| **Cache** | Redis 7 | Sub-millisecond reads, TTL-based expiration, LRU eviction, counter support for cache statistics. |
| **Dashboard** | Next.js 16 + Tailwind CSS 4 | Server-side rendering for fast initial load, App Router for modern routing, Tailwind for consistent styling without custom CSS. |
| **Charts** | Recharts 3 | Composable React chart library. Clean API for line, bar, and area charts used in analytics. |
| **Auth** | NextAuth.js v5 | Mature authentication library with Google OAuth and credentials providers. JWT session tokens. |
| **HTTP Client** | reqwest (Rust) | Async HTTP client with rustls for TLS. Connection pooling and timeout support. |
| **Serialization** | serde + serde_json | Industry-standard Rust serialization. Zero-copy deserialization where possible. |
| **Observability** | tracing + tracing-subscriber | Structured JSON logging with span context. Compatible with OpenTelemetry. |
| **Reverse Proxy** | Nginx | TLS termination, rate limiting, gzip compression, WebSocket support for streaming. |
| **Containerization** | Docker Compose | Single-command deployment of all 5 services. Health checks ensure correct startup ordering. |
| **CI/CD** | GitHub Actions | Automated build, test, and lint on every push. Separate workflows for gateway and dashboard. |
