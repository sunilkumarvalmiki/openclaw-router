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

use std::sync::Arc;

/// Read an env var, returning `Some(value)` only if it is set and non-empty.
fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

/// Factory that reads environment variables and creates all configured LLM
/// provider instances.
pub struct ProviderFactory;

impl ProviderFactory {
    /// Scan environment variables and instantiate every provider whose
    /// required configuration is present (non-empty API key / endpoint).
    ///
    /// Providers are returned in a `Vec<Arc<dyn LlmProvider>>` suitable for
    /// sharing across Actix-Web worker threads.
    pub fn create_from_env() -> Vec<Arc<dyn LlmProvider>> {
        let mut providers: Vec<Arc<dyn LlmProvider>> = Vec::new();

        // -----------------------------------------------------------------
        // OpenAI (requires OPENAI_API_KEY)
        // -----------------------------------------------------------------
        if let Some(api_key) = env_non_empty("OPENAI_API_KEY") {
            let base_url = env_non_empty("OPENAI_BASE_URL");
            let provider = OpenAiProvider::new(api_key, base_url);
            providers.push(Arc::new(provider));
            tracing::info!("Provider registered: openai");
        }

        // -----------------------------------------------------------------
        // GitHub Copilot / GitHub Models (requires GITHUB_COPILOT_API_KEY)
        // -----------------------------------------------------------------
        if let Some(api_key) = env_non_empty("GITHUB_COPILOT_API_KEY") {
            let base_url = env_non_empty("GITHUB_COPILOT_BASE_URL");
            let provider = GitHubCopilotProvider::new(api_key, base_url);
            providers.push(Arc::new(provider));
            tracing::info!("Provider registered: github-copilot");
        }

        // -----------------------------------------------------------------
        // Ollama (requires OLLAMA_ENDPOINT, e.g. http://localhost:11434)
        // -----------------------------------------------------------------
        if let Some(endpoint) = env_non_empty("OLLAMA_ENDPOINT") {
            let provider = OllamaProvider::new(Some(endpoint));
            providers.push(Arc::new(provider));
            tracing::info!("Provider registered: ollama");
        }

        // -----------------------------------------------------------------
        // OpenRouter (requires OPENROUTER_API_KEY)
        // -----------------------------------------------------------------
        if let Some(api_key) = env_non_empty("OPENROUTER_API_KEY") {
            let provider = OpenRouterProvider::new(api_key);
            providers.push(Arc::new(provider));
            tracing::info!("Provider registered: openrouter");
        }

        // -----------------------------------------------------------------
        // Google Gemini (requires GEMINI_API_KEY)
        // -----------------------------------------------------------------
        if let Some(api_key) = env_non_empty("GEMINI_API_KEY") {
            let provider = GeminiProvider::new(api_key);
            providers.push(Arc::new(provider));
            tracing::info!("Provider registered: gemini");
        }

        // -----------------------------------------------------------------
        // xAI / Grok (requires XAI_API_KEY)
        // -----------------------------------------------------------------
        if let Some(api_key) = env_non_empty("XAI_API_KEY") {
            let provider = XaiProvider::new(api_key);
            providers.push(Arc::new(provider));
            tracing::info!("Provider registered: xai");
        }

        // -----------------------------------------------------------------
        // DeepSeek (requires DEEPSEEK_API_KEY)
        // -----------------------------------------------------------------
        if let Some(api_key) = env_non_empty("DEEPSEEK_API_KEY") {
            let provider = DeepSeekProvider::new(api_key);
            providers.push(Arc::new(provider));
            tracing::info!("Provider registered: deepseek");
        }

        // -----------------------------------------------------------------
        // AWS Bedrock (requires AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY,
        //              and optionally AWS_REGION — defaults to us-east-1)
        // -----------------------------------------------------------------
        if let (Some(access_key), Some(secret_key)) = (
            env_non_empty("AWS_ACCESS_KEY_ID"),
            env_non_empty("AWS_SECRET_ACCESS_KEY"),
        ) {
            let region = std::env::var("AWS_REGION")
                .unwrap_or_else(|_| "us-east-1".to_string());
            let provider = BedrockProvider::new(region, access_key, secret_key);
            providers.push(Arc::new(provider));
            tracing::info!("Provider registered: bedrock (stub — use openrouter for Bedrock models)");
        }

        // -----------------------------------------------------------------
        // Custom provider (requires CUSTOM_LLM_BASE_URL;
        //                  CUSTOM_LLM_NAME and CUSTOM_LLM_API_KEY are optional)
        // -----------------------------------------------------------------
        if let Some(base_url) = env_non_empty("CUSTOM_LLM_BASE_URL") {
            let name = std::env::var("CUSTOM_LLM_NAME")
                .unwrap_or_else(|_| "custom".to_string());
            let api_key = env_non_empty("CUSTOM_LLM_API_KEY");
            let provider = CustomProvider::new(name.clone(), base_url, api_key);
            providers.push(Arc::new(provider));
            tracing::info!(provider_name = %name, "Provider registered: custom");
        }

        if providers.is_empty() {
            tracing::warn!(
                "No LLM providers configured — set at least one provider API key \
                 (OPENAI_API_KEY, GEMINI_API_KEY, OPENROUTER_API_KEY, etc.)"
            );
        } else {
            tracing::info!(
                count = providers.len(),
                "Total LLM providers registered"
            );
        }

        providers
    }
}
