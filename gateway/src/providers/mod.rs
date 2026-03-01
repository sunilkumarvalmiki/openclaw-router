pub mod bedrock;
pub mod custom;
pub mod deepseek;
pub mod gemini;
pub mod github_copilot;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod traits;
pub mod xai;

pub use bedrock::BedrockProvider;
pub use custom::CustomProvider;
pub use deepseek::DeepSeekProvider;
pub use gemini::GeminiProvider;
pub use github_copilot::GitHubCopilotProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use openrouter::OpenRouterProvider;
pub use traits::LlmProvider;
pub use xai::XaiProvider;

use std::collections::HashSet;
use std::sync::Arc;

use crate::openclaw_config;

/// Read an env var, returning `Some(value)` only if it is set and non-empty.
fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

/// Factory that auto-detects providers from the OpenClaw configuration and
/// falls back to environment variables for any providers not discovered.
pub struct ProviderFactory;

impl ProviderFactory {
    /// Auto-detect providers using this priority:
    ///
    /// 1. **OpenClaw config** (`~/.openclaw/openclaw.json` + auth-profiles) —
    ///    reads credentials directly from the OpenClaw installation so the user
    ///    doesn't need to duplicate keys.
    /// 2. **Environment variables** — for providers not found in OpenClaw or for
    ///    providers that OpenClaw doesn't manage (Ollama, custom, etc.).
    pub fn create_from_env() -> Vec<Arc<dyn LlmProvider>> {
        let mut providers: Vec<Arc<dyn LlmProvider>> = Vec::new();
        let mut registered: HashSet<String> = HashSet::new();

        // =================================================================
        // Phase 1: Auto-detect from OpenClaw configuration
        // =================================================================
        let discovered = openclaw_config::discover_providers();

        if !discovered.is_empty() {
            tracing::info!(
                count = discovered.len(),
                "Auto-detected providers from OpenClaw config"
            );
        }

        for dp in &discovered {
            match dp.provider.as_str() {
                // ---------------------------------------------------------
                // OpenAI Codex (OAuth) → register as "openai" provider
                // ---------------------------------------------------------
                "openai-codex" => {
                    let base_url = env_non_empty("OPENAI_BASE_URL");
                    let provider = OpenAiProvider::new(dp.token.clone(), base_url);
                    providers.push(Arc::new(provider));
                    registered.insert("openai".to_string());
                    tracing::info!(
                        auth_mode = ?dp.auth_mode,
                        "Provider registered: openai (via OpenClaw OAuth)"
                    );
                }
                // ---------------------------------------------------------
                // GitHub Copilot (token) → register as "github-copilot"
                // ---------------------------------------------------------
                "github-copilot" => {
                    let base_url = env_non_empty("GITHUB_COPILOT_BASE_URL");
                    let provider = GitHubCopilotProvider::new(dp.token.clone(), base_url);
                    providers.push(Arc::new(provider));
                    registered.insert("github-copilot".to_string());
                    tracing::info!(
                        auth_mode = ?dp.auth_mode,
                        "Provider registered: github-copilot (via OpenClaw)"
                    );
                }
                other => {
                    tracing::debug!(
                        provider = %other,
                        "Unknown OpenClaw provider — skipping auto-registration"
                    );
                }
            }
        }

        // =================================================================
        // Phase 2: Environment variable fallbacks
        //
        // When OpenClaw auto-detection found providers, env vars are ONLY
        // used as fallbacks for those same providers (openai, github-copilot).
        // Other providers from system env vars are ignored to prevent
        // accidental registration (e.g. GEMINI_API_KEY set globally).
        //
        // When OpenClaw is NOT installed, ALL env-var-based providers are
        // scanned (the original behaviour).
        // =================================================================
        let openclaw_active = !discovered.is_empty();

        // OpenAI (env var fallback)
        if !registered.contains("openai") {
            if let Some(api_key) = env_non_empty("OPENAI_API_KEY") {
                let base_url = env_non_empty("OPENAI_BASE_URL");
                let provider = OpenAiProvider::new(api_key, base_url);
                providers.push(Arc::new(provider));
                registered.insert("openai".to_string());
                tracing::info!("Provider registered: openai (via env var)");
            }
        }

        // GitHub Copilot (env var fallback)
        if !registered.contains("github-copilot") {
            if let Some(api_key) = env_non_empty("GITHUB_COPILOT_API_KEY") {
                let base_url = env_non_empty("GITHUB_COPILOT_BASE_URL");
                let provider = GitHubCopilotProvider::new(api_key, base_url);
                providers.push(Arc::new(provider));
                registered.insert("github-copilot".to_string());
                tracing::info!("Provider registered: github-copilot (via env var)");
            }
        }

        // The providers below are only registered from env vars when
        // OpenClaw auto-detection did NOT find any providers (i.e. OpenClaw
        // is not installed or not configured). This prevents stray system
        // env vars from registering unwanted providers.
        if !openclaw_active {
            // Ollama (requires explicit OLLAMA_ENDPOINT)
            if let Some(endpoint) = env_non_empty("OLLAMA_ENDPOINT") {
                let provider = OllamaProvider::new(Some(endpoint));
                providers.push(Arc::new(provider));
                registered.insert("ollama".to_string());
                tracing::info!("Provider registered: ollama");
            }

            // OpenRouter
            if let Some(api_key) = env_non_empty("OPENROUTER_API_KEY") {
                let provider = OpenRouterProvider::new(api_key);
                providers.push(Arc::new(provider));
                registered.insert("openrouter".to_string());
                tracing::info!("Provider registered: openrouter");
            }

            // Google Gemini
            if let Some(api_key) = env_non_empty("GEMINI_API_KEY") {
                let provider = GeminiProvider::new(api_key);
                providers.push(Arc::new(provider));
                registered.insert("gemini".to_string());
                tracing::info!("Provider registered: gemini");
            }

            // xAI / Grok
            if let Some(api_key) = env_non_empty("XAI_API_KEY") {
                let provider = XaiProvider::new(api_key);
                providers.push(Arc::new(provider));
                registered.insert("xai".to_string());
                tracing::info!("Provider registered: xai");
            }

            // DeepSeek
            if let Some(api_key) = env_non_empty("DEEPSEEK_API_KEY") {
                let provider = DeepSeekProvider::new(api_key);
                providers.push(Arc::new(provider));
                registered.insert("deepseek".to_string());
                tracing::info!("Provider registered: deepseek");
            }

            // AWS Bedrock
            if let (Some(access_key), Some(secret_key)) = (
                env_non_empty("AWS_ACCESS_KEY_ID"),
                env_non_empty("AWS_SECRET_ACCESS_KEY"),
            ) {
                let region = std::env::var("AWS_REGION")
                    .unwrap_or_else(|_| "us-east-1".to_string());
                let provider = BedrockProvider::new(region, access_key, secret_key);
                providers.push(Arc::new(provider));
                registered.insert("bedrock".to_string());
                tracing::info!("Provider registered: bedrock");
            }

            // Custom provider
            if let Some(base_url) = env_non_empty("CUSTOM_LLM_BASE_URL") {
                let name = std::env::var("CUSTOM_LLM_NAME")
                    .unwrap_or_else(|_| "custom".to_string());
                let api_key = env_non_empty("CUSTOM_LLM_API_KEY");
                let provider = CustomProvider::new(name.clone(), base_url, api_key);
                providers.push(Arc::new(provider));
                registered.insert(name.clone());
                tracing::info!(provider_name = %name, "Provider registered: custom");
            }
        }

        // =================================================================
        // Summary
        // =================================================================
        if providers.is_empty() {
            tracing::warn!(
                "No LLM providers configured. Either install OpenClaw with \
                 configured providers, or set environment variables \
                 (OPENAI_API_KEY, GITHUB_COPILOT_API_KEY, etc.)"
            );
        } else {
            let names: Vec<&str> = providers.iter().map(|p| p.name()).collect();
            tracing::info!(
                count = providers.len(),
                providers = ?names,
                "Total LLM providers registered"
            );
        }

        providers
    }
}
