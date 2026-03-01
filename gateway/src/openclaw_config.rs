//! Reads the OpenClaw configuration (`~/.openclaw/openclaw.json`) and per-agent
//! auth-profiles to auto-detect which LLM providers are available and extract
//! their credentials.
//!
//! Two authentication modes are supported:
//!   - **token** – a static API key / PAT (used by GitHub Copilot).
//!   - **oauth** – an OAuth2 access + refresh token pair (used by OpenAI Codex).

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A discovered provider with its credentials ready for use.
#[derive(Debug, Clone)]
pub struct DiscoveredProvider {
    /// The OpenClaw provider id (e.g. "github-copilot", "openai-codex").
    pub provider: String,
    /// The credential to use as a Bearer token.
    pub token: String,
    /// Authentication mode.
    pub auth_mode: AuthMode,
    /// For OAuth tokens: expiry timestamp (Unix epoch **milliseconds**).
    pub expires: Option<u64>,
    /// For OAuth tokens: the refresh token (for future use).
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMode {
    Token,
    OAuth,
}

// ---------------------------------------------------------------------------
// JSON shapes (deserialized from OpenClaw files)
// ---------------------------------------------------------------------------

/// Top-level `openclaw.json`.
#[derive(Deserialize)]
struct OpenClawConfig {
    auth: Option<AuthSection>,
    agents: Option<AgentsSection>,
}

#[derive(Deserialize)]
struct AuthSection {
    profiles: Option<HashMap<String, AuthProfileMeta>>,
}

#[derive(Deserialize)]
struct AuthProfileMeta {
    provider: String,
    mode: String,
}

#[derive(Deserialize)]
struct AgentsSection {
    list: Option<Vec<AgentEntry>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentEntry {
    #[allow(dead_code)]
    id: String,
    agent_dir: Option<String>,
}

/// Per-agent `auth-profiles.json`.
#[derive(Deserialize)]
struct AgentAuthProfiles {
    profiles: Option<HashMap<String, AgentAuthProfileEntry>>,
}

#[derive(Deserialize)]
struct AgentAuthProfileEntry {
    #[serde(rename = "type")]
    auth_type: String,
    provider: String,
    /// For token auth.
    token: Option<String>,
    /// For OAuth: the access (JWT) token.
    access: Option<String>,
    /// For OAuth: the refresh token.
    refresh: Option<String>,
    /// For OAuth: expiry in Unix epoch milliseconds.
    expires: Option<u64>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Scan the OpenClaw installation and return all providers whose credentials
/// are available.
///
/// Resolution order:
/// 1. Read `~/.openclaw/openclaw.json` to discover which auth profiles exist.
/// 2. Iterate agent directories listed in the config looking for
///    `auth-profiles.json` files that contain actual credentials.
/// 3. Return one `DiscoveredProvider` per unique provider whose credentials
///    were found.
pub fn discover_providers() -> Vec<DiscoveredProvider> {
    let openclaw_dir = match openclaw_home() {
        Some(d) => d,
        None => {
            tracing::debug!("OpenClaw home directory not found");
            return Vec::new();
        }
    };

    let config_path = openclaw_dir.join("openclaw.json");
    let config: OpenClawConfig = match read_json(&config_path) {
        Some(c) => c,
        None => {
            tracing::debug!("Could not read {:?}", config_path);
            return Vec::new();
        }
    };

    // Collect profile keys and their expected modes from the top-level config.
    let profile_metas: HashMap<String, AuthProfileMeta> = config
        .auth
        .and_then(|a| a.profiles)
        .unwrap_or_default();

    if profile_metas.is_empty() {
        tracing::debug!("No auth profiles declared in openclaw.json");
        return Vec::new();
    }

    // Resolve agent directories where we can look for auth-profiles.json.
    let agent_dirs = resolve_agent_dirs(&openclaw_dir, &config.agents);

    // Search agent auth-profiles.json files for actual credentials.
    let mut found: HashMap<String, DiscoveredProvider> = HashMap::new();

    for agent_dir in &agent_dirs {
        let auth_path = agent_dir.join("auth-profiles.json");
        let auth_profiles: AgentAuthProfiles = match read_json(&auth_path) {
            Some(a) => a,
            None => continue,
        };

        let profiles = match auth_profiles.profiles {
            Some(p) => p,
            None => continue,
        };

        for (profile_key, entry) in &profiles {
            // Only process profiles that are declared in the top-level config.
            if !profile_metas.contains_key(profile_key) {
                continue;
            }

            // Skip if we already found credentials for this provider.
            if found.contains_key(&entry.provider) {
                continue;
            }

            match entry.auth_type.as_str() {
                "token" => {
                    if let Some(ref tok) = entry.token {
                        if !tok.is_empty() {
                            found.insert(
                                entry.provider.clone(),
                                DiscoveredProvider {
                                    provider: entry.provider.clone(),
                                    token: tok.clone(),
                                    auth_mode: AuthMode::Token,
                                    expires: None,
                                    refresh_token: None,
                                },
                            );
                            tracing::info!(
                                provider = %entry.provider,
                                "Auto-detected provider credentials (token)"
                            );
                        }
                    }
                }
                "oauth" => {
                    if let Some(ref access) = entry.access {
                        if !access.is_empty() {
                            let is_expired = entry
                                .expires
                                .map(|exp| {
                                    let now_ms = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64;
                                    now_ms >= exp
                                })
                                .unwrap_or(false);

                            if is_expired {
                                tracing::warn!(
                                    provider = %entry.provider,
                                    "OAuth access token is expired — provider will be registered \
                                     but may need token refresh"
                                );
                            }

                            found.insert(
                                entry.provider.clone(),
                                DiscoveredProvider {
                                    provider: entry.provider.clone(),
                                    token: access.clone(),
                                    auth_mode: AuthMode::OAuth,
                                    expires: entry.expires,
                                    refresh_token: entry.refresh.clone(),
                                },
                            );
                            tracing::info!(
                                provider = %entry.provider,
                                expired = is_expired,
                                "Auto-detected provider credentials (oauth)"
                            );
                        }
                    }
                }
                other => {
                    tracing::debug!(
                        provider = %entry.provider,
                        auth_type = %other,
                        "Unknown auth type — skipping"
                    );
                }
            }
        }
    }

    found.into_values().collect()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Return the path to `~/.openclaw` (or `%USERPROFILE%/.openclaw` on Windows).
fn openclaw_home() -> Option<PathBuf> {
    let home = if cfg!(windows) {
        std::env::var("USERPROFILE").ok()
    } else {
        std::env::var("HOME").ok()
    };

    home.map(|h| PathBuf::from(h).join(".openclaw"))
        .filter(|p| p.is_dir())
}

/// Collect agent directories from the config, falling back to scanning the
/// agents/ folder.
fn resolve_agent_dirs(openclaw_dir: &Path, agents: &Option<AgentsSection>) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    // Try explicit agentDir entries first.
    if let Some(ref section) = agents {
        if let Some(ref list) = section.list {
            for agent in list {
                if let Some(ref dir_str) = agent.agent_dir {
                    let expanded = expand_tilde(dir_str);
                    let path = PathBuf::from(&expanded);
                    if path.is_dir() {
                        dirs.push(path);
                    }
                }
            }
        }
    }

    // Fallback: scan ~/.openclaw/agents/*/agent/
    if dirs.is_empty() {
        let agents_root = openclaw_dir.join("agents");
        if agents_root.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&agents_root) {
                for entry in entries.flatten() {
                    let agent_path = entry.path().join("agent");
                    if agent_path.is_dir() {
                        dirs.push(agent_path);
                    }
                }
            }
        }
    }

    dirs
}

/// Expand `~` at the start of a path to the user's home directory.
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") || path.starts_with("~\\") {
        let home = if cfg!(windows) {
            std::env::var("USERPROFILE").unwrap_or_default()
        } else {
            std::env::var("HOME").unwrap_or_default()
        };
        format!("{}{}", home, &path[1..])
    } else {
        path.to_string()
    }
}

/// Read and deserialize a JSON file, returning `None` on any error.
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}
