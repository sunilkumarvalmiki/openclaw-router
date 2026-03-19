---
name: add-provider
description: Scaffold a new LLM provider following existing patterns. Provide the provider name as argument (e.g., "mistral", "cohere").
---

# Add LLM Provider

Scaffold a new LLM provider in the gateway, following the established patterns.

## Arguments
- `$ARGUMENTS` — the provider name in snake_case (e.g., `mistral`, `cohere`, `together`)

## Steps

1. **Read the template**: Read `gateway/src/providers/openai.rs` as the reference implementation
2. **Create provider file**: `gateway/src/providers/<name>.rs`
   - Struct with `endpoint`, `api_key`, optional `org_id`
   - `new()` constructor reading from env vars: `<NAME>_API_KEY`, `<NAME>_ENDPOINT`
   - `sanitize_request()` method that strips ALL `x_*` fields (x_cost_profile, x_tier_hint, x_priority, and any future ones)
   - `LlmProvider` trait implementation with `name()`, `is_healthy()`, `chat_completion()`, `health_check()`
   - Chat completion URL pointing to the provider's OpenAI-compatible endpoint
3. **Register in mod.rs**: Add `mod <name>;` and registration block in `gateway/src/providers/mod.rs`
   - Follow the existing pattern: check env var, create provider, push to vec
4. **Add models to registry**: Add the provider's key models in `gateway/src/routing/registry.rs` `seed_defaults()`
   - Include: model_id, provider name, tier, costs, context length, capabilities
5. **Update .env.example**: Add the new env vars with comments
6. **Remind user**: List what was created and suggest testing with:
   ```
   cargo test
   curl -X POST http://localhost:8080/v1/chat/completions -H "Content-Type: application/json" -d '{"model":"<model-id>","messages":[{"role":"user","content":"hello"}]}'
   ```

## Critical Reminders
- Every provider MUST strip `x_*` fields in `sanitize_request()` — check current providers for the full list
- Adding fields to `ChatCompletionRequest` in the future will require updating test constructors in ~8+ files
- The `LlmProvider` trait is in `gateway/src/providers/traits.rs`
