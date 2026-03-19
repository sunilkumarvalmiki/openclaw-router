# OpenClaw Router

## Quick Reference
- Build: `cd gateway && cargo build`
- Test: `cd gateway && cargo test`
- Run: `cd gateway && cargo run` (serves on 0.0.0.0:8080)
- Config: `gateway/.env` (copy from `.env.example` if missing)

## Architecture
Rust/Actix-Web LLM routing gateway. Single crate at `gateway/`.

### Key directories
- `src/providers/` — LLM provider implementations (openai, ollama, deepseek, xai, gemini, openrouter, github_copilot, bedrock, custom). Each implements `LlmProvider` trait from `traits.rs`. Registration in `mod.rs`.
- `src/routing/` — Model selection: `scorer.rs` (15-dim scoring engine), `registry.rs` (model catalog), `selector.rs` (model picker), `cascade.rs` (cascade router)
- `src/routes/completions.rs` — Main request handler. Priority-based routing: low→ollama, high→cloud, normal→auto-detect
- `src/types/` — `request.rs` (ChatCompletionRequest), `response.rs` (ChatCompletionResponse), `routing.rs` (Tier, ScoringResult)
- `src/resilience/` — Circuit breakers, fallback chains, rate limiting, retry logic
- `src/optimization/` — Prompt compression, semantic caching, request dedup
- `src/openclaw_config.rs` — Auto-discovers providers from `~/.openclaw/openclaw.json`
- `tests/` — 12 integration test files, 236+ tests total

## Gotchas
- **Adding fields to `ChatCompletionRequest`** requires updating EVERY constructor literal across src/ and tests/ (~8+ files). Search for `x_tier_hint: None` to find all locations.
- **Windows binary locking**: `cargo build` fails if the gateway exe is running. Stop it first: `powershell -Command "Stop-Process -Name openclaw-gateway -Force"`
- **Provider `sanitize_request()`**: Every provider strips `x_*` routing fields before forwarding. When adding new `x_*` fields, update ALL provider sanitize methods.
- **Ollama registration**: Lives OUTSIDE the `if !openclaw_active` block in `providers/mod.rs` so it's always available as local free-tier.
- **PowerShell in bash**: Avoid `$_` and `$()` in PowerShell commands run via bash — they get mangled. Use simplified commands.

## Code Style
- Rust 2021 edition, `serde` with `skip_serializing_if = "Option::is_none"` on optional fields
- `#[async_trait]` for async trait impls
- Integration tests in `tests/` directory, unit tests in `#[cfg(test)]` modules within source files
- Error type: `AppError` enum in `src/error.rs` with `thiserror` derive
