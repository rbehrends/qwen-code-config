use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderPreset {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) base_url: String,
    pub(crate) default_env_key: String,
    pub(crate) default_protocol: SupportedProtocol,
    pub(crate) supports_fetch: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogModel {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) context_window_size: Option<u64>,
    pub(crate) supports_vision: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CustomProviderProfile {
    pub(crate) profile_id: String,
    pub(crate) label: String,
    pub(crate) base_url: String,
    pub(crate) env_key: String,
    pub(crate) protocol: SupportedProtocol,
    pub(crate) models: Vec<CatalogModel>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderDraftProfile {
    pub(crate) base_url: String,
    pub(crate) env_key: String,
    pub(crate) protocol: SupportedProtocol,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SupportedProtocol {
    Openai,
    Anthropic,
}

impl SupportedProtocol {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Openai => "openai",
            Self::Anthropic => "anthropic",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "openai" => Some(Self::Openai),
            "anthropic" => Some(Self::Anthropic),
            _ => None,
        }
    }
}

impl fmt::Display for SupportedProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelEntry {
    pub(crate) ui_id: String,
    pub(crate) protocol: SupportedProtocol,
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) base_url: String,
    pub(crate) env_key: String,
    pub(crate) context_window_size: Option<u64>,
    pub(crate) temperature: Option<f64>,
    pub(crate) top_p: Option<f64>,
    pub(crate) max_tokens: Option<u64>,
    pub(crate) reasoning_mode: ReasoningMode,
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
    pub(crate) reasoning_budget_tokens: Option<u64>,
    pub(crate) sampling_params: Map<String, Value>,
    pub(crate) extra_body: Map<String, Value>,
    pub(crate) raw_model: Value,
    pub(crate) is_default: bool,
    pub(crate) is_duplicate: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FastModelMode {
    Inherit,
    Specific,
    Invalid,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FastModelSelection {
    pub(crate) mode: FastModelMode,
    pub(crate) protocol: Option<SupportedProtocol>,
    pub(crate) model_id: Option<String>,
    pub(crate) raw_value: Option<String>,
}

impl Default for FastModelSelection {
    fn default() -> Self {
        Self {
            mode: FastModelMode::Inherit,
            protocol: None,
            model_id: None,
            raw_value: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ReasoningMode {
    #[default]
    Default,
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
    #[serde(rename = "xhigh")]
    XHigh,
    Max,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportantOptions {
    pub(crate) usage_statistics_enabled: bool,
    pub(crate) telemetry_enabled: bool,
    pub(crate) enable_auto_update: bool,
}

impl Default for ImportantOptions {
    fn default() -> Self {
        Self {
            usage_statistics_enabled: true,
            telemetry_enabled: false,
            enable_auto_update: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnvironmentVariable {
    pub(crate) key: String,
    pub(crate) value: String,
}
