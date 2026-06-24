use crate::{
    models::prettify_model_name,
    types::{CatalogModel, ProviderPreset, SupportedProtocol},
};
use serde::Deserialize;
use std::time::Duration;

const MODEL_FETCH_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const MODEL_FETCH_READ_TIMEOUT: Duration = Duration::from_secs(10);
const MODEL_FETCH_WRITE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy)]
struct ProviderPresetDefinition {
    id: &'static str,
    label: &'static str,
    base_url: &'static str,
    default_env_key: &'static str,
    default_protocol: SupportedProtocol,
    supports_fetch: bool,
    catalog: Option<ProviderCatalog>,
}

#[derive(Debug, Clone, Copy)]
enum ProviderCatalog {
    OpenRouter,
    OpenCodeGo,
    OpenCodeZen,
    KiloCode,
    Nvidia,
    Ollama,
    LmStudio,
}

#[derive(Debug, Deserialize)]
struct BasicModelsDump {
    data: Vec<BasicModelRecord>,
}

#[derive(Debug, Deserialize)]
struct BasicModelRecord {
    id: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelsDump {
    data: Vec<OpenRouterModelRecord>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelRecord {
    id: String,
    context_length: Option<u64>,
    architecture: Option<OpenRouterArchitecture>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterArchitecture {
    input_modalities: Option<Vec<String>>,
}

const BUILTIN_PROVIDER_PRESETS: [ProviderPresetDefinition; 8] = [
    ProviderPresetDefinition {
        id: "openrouter",
        label: "OpenRouter",
        base_url: "https://openrouter.ai/api/v1",
        default_env_key: "OPENROUTER_API_KEY",
        default_protocol: SupportedProtocol::Openai,
        supports_fetch: true,
        catalog: Some(ProviderCatalog::OpenRouter),
    },
    ProviderPresetDefinition {
        id: "opencode-go",
        label: "OpenCode Go",
        base_url: "https://opencode.ai/zen/go/v1",
        default_env_key: "OPENCODE_API_KEY",
        default_protocol: SupportedProtocol::Openai,
        supports_fetch: true,
        catalog: Some(ProviderCatalog::OpenCodeGo),
    },
    ProviderPresetDefinition {
        id: "opencode-zen",
        label: "OpenCode Zen",
        base_url: "https://opencode.ai/zen/v1",
        default_env_key: "OPENCODE_API_KEY",
        default_protocol: SupportedProtocol::Openai,
        supports_fetch: true,
        catalog: Some(ProviderCatalog::OpenCodeZen),
    },
    ProviderPresetDefinition {
        id: "kilo-code",
        label: "Kilo Code",
        base_url: "https://api.kilo.ai/api/gateway",
        default_env_key: "KILOCODE_API_KEY",
        default_protocol: SupportedProtocol::Openai,
        supports_fetch: true,
        catalog: Some(ProviderCatalog::KiloCode),
    },
    ProviderPresetDefinition {
        id: "nvidia",
        label: "NVIDIA",
        base_url: "https://integrate.api.nvidia.com/v1",
        default_env_key: "NVIDIA_API_KEY",
        default_protocol: SupportedProtocol::Openai,
        supports_fetch: true,
        catalog: Some(ProviderCatalog::Nvidia),
    },
    ProviderPresetDefinition {
        id: "ollama",
        label: "Ollama",
        base_url: "http://localhost:11434/v1",
        default_env_key: "",
        default_protocol: SupportedProtocol::Openai,
        supports_fetch: true,
        catalog: Some(ProviderCatalog::Ollama),
    },
    ProviderPresetDefinition {
        id: "ollama-cloud",
        label: "Ollama Cloud",
        base_url: "https://ollama.com/v1",
        default_env_key: "OLLAMA_API_KEY",
        default_protocol: SupportedProtocol::Openai,
        supports_fetch: true,
        catalog: Some(ProviderCatalog::Ollama),
    },
    ProviderPresetDefinition {
        id: "lm-studio",
        label: "LM Studio",
        base_url: "http://localhost:1234/v1",
        default_env_key: "",
        default_protocol: SupportedProtocol::Openai,
        supports_fetch: true,
        catalog: Some(ProviderCatalog::LmStudio),
    },
];

pub(crate) fn builtin_provider_presets() -> Vec<ProviderPreset> {
    BUILTIN_PROVIDER_PRESETS
        .iter()
        .map(|preset| ProviderPreset {
            id: preset.id.to_string(),
            label: preset.label.to_string(),
            base_url: preset.base_url.to_string(),
            default_env_key: preset.default_env_key.to_string(),
            default_protocol: preset.default_protocol,
            supports_fetch: preset.supports_fetch,
        })
        .collect()
}

fn builtin_provider_preset(id: &str) -> Option<ProviderPresetDefinition> {
    BUILTIN_PROVIDER_PRESETS
        .iter()
        .copied()
        .find(|preset| preset.id == id)
}

pub(crate) fn fetch_catalog_models(provider_id: &str) -> Result<Vec<CatalogModel>, String> {
    let preset = builtin_provider_preset(provider_id)
        .ok_or_else(|| format!("Unknown provider preset `{provider_id}`"))?;
    fetch_catalog_models_for_preset(preset)
}

fn fetch_catalog_models_for_preset(
    preset: ProviderPresetDefinition,
) -> Result<Vec<CatalogModel>, String> {
    let endpoint = provider_models_url(preset.base_url);
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(MODEL_FETCH_CONNECT_TIMEOUT)
        .timeout_read(MODEL_FETCH_READ_TIMEOUT)
        .timeout_write(MODEL_FETCH_WRITE_TIMEOUT)
        .build();
    let response = agent
        .get(&endpoint)
        .set("Accept", "application/json")
        .set("User-Agent", "Qwen Code Config/0.1")
        .call()
        .map_err(|error| format!("Failed to fetch models from {endpoint}: {error}"))?;
    let body = response.into_string().map_err(|error| {
        format!("Failed to read model catalog response from {endpoint}: {error}")
    })?;

    match preset.catalog {
        Some(ProviderCatalog::OpenRouter) => parse_openrouter_catalog(&body, preset.label),
        Some(ProviderCatalog::OpenCodeGo)
        | Some(ProviderCatalog::OpenCodeZen)
        | Some(ProviderCatalog::KiloCode)
        | Some(ProviderCatalog::Nvidia)
        | Some(ProviderCatalog::Ollama)
        | Some(ProviderCatalog::LmStudio) => parse_basic_catalog(&body, preset.label),
        None => Ok(Vec::new()),
    }
}

fn parse_basic_catalog(json: &str, label: &str) -> Result<Vec<CatalogModel>, String> {
    let dump: BasicModelsDump = serde_json::from_str(json)
        .map_err(|error| format!("Failed to parse {label} model catalog: {error}"))?;
    Ok(dump
        .data
        .into_iter()
        .map(|record| CatalogModel {
            name: prettify_catalog_model_name(&record.id, label),
            id: record.id,
            context_window_size: None,
            supports_vision: false,
        })
        .collect())
}

fn parse_openrouter_catalog(json: &str, label: &str) -> Result<Vec<CatalogModel>, String> {
    let dump: OpenRouterModelsDump = serde_json::from_str(json)
        .map_err(|error| format!("Failed to parse {label} model catalog: {error}"))?;
    Ok(dump
        .data
        .into_iter()
        .map(|record| CatalogModel {
            name: prettify_catalog_model_name(&record.id, label),
            id: record.id,
            context_window_size: record.context_length,
            supports_vision: record
                .architecture
                .and_then(|architecture| architecture.input_modalities)
                .map(|modalities| {
                    modalities
                        .iter()
                        .any(|value| matches!(value.as_str(), "image" | "video"))
                })
                .unwrap_or(false),
        })
        .collect())
}

pub(crate) fn provider_models_url(base_url: &str) -> String {
    format!("{}/models", base_url.trim_end_matches('/'))
}

fn prettify_catalog_model_name(id: &str, label: &str) -> String {
    let display_id = id.rsplit('/').next().unwrap_or(id);
    format!("{} ({label})", prettify_model_name(display_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_models_url_appends_models_endpoint_once() {
        assert_eq!(
            provider_models_url("https://openrouter.ai/api/v1"),
            "https://openrouter.ai/api/v1/models"
        );
        assert_eq!(
            provider_models_url("https://openrouter.ai/api/v1/"),
            "https://openrouter.ai/api/v1/models"
        );
    }

    #[test]
    fn parse_basic_catalog_reads_live_style_model_response() {
        let json = r#"{"data":[{"id":"qwen3-coder-plus"}]}"#;
        let models = parse_basic_catalog(json, "OpenCode Go").unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "qwen3-coder-plus");
        assert_eq!(models[0].name, "Qwen3 Coder Plus (OpenCode Go)");
        assert_eq!(models[0].context_window_size, None);
        assert!(!models[0].supports_vision);
    }

    #[test]
    fn prettify_catalog_model_name_uses_suffix_after_last_slash() {
        assert_eq!(
            prettify_catalog_model_name("provider/family/qwen3-coder-plus", "OpenRouter"),
            "Qwen3 Coder Plus (OpenRouter)"
        );
    }

    #[test]
    fn parse_openrouter_catalog_reads_context_and_modalities() {
        let json = r#"{
            "data":[
                {
                    "id":"google/gemini-2.5-pro",
                    "context_length":1048576,
                    "architecture":{"input_modalities":["text","image"]}
                }
            ]
        }"#;
        let models = parse_openrouter_catalog(json, "OpenRouter").unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "google/gemini-2.5-pro");
        assert_eq!(models[0].name, "Gemini 2.5 Pro (OpenRouter)");
        assert_eq!(models[0].context_window_size, Some(1048576));
        assert!(models[0].supports_vision);
    }

    #[test]
    fn builtin_provider_presets_include_ollama_cloud() {
        let preset = builtin_provider_presets()
            .into_iter()
            .find(|preset| preset.id == "ollama-cloud")
            .expect("missing ollama-cloud preset");

        assert_eq!(preset.label, "Ollama Cloud");
        assert_eq!(preset.base_url, "https://ollama.com/v1");
        assert_eq!(preset.default_env_key, "OLLAMA_API_KEY");
        assert_eq!(preset.default_protocol, SupportedProtocol::Openai);
        assert!(preset.supports_fetch);
    }
}
