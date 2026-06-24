use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    CatalogModel, CustomProviderProfile, EnvironmentVariable, FastModelSelection, ImportantOptions,
    LayoutDensity, McpServerEntry, ModelEntry, ProviderDraftProfile, ProviderPreset, ThemeMode,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveOptionsRequest {
    pub(crate) path: String,
    pub(crate) options: ImportantOptions,
    pub(crate) env_vars: Vec<EnvironmentVariable>,
    pub(crate) models: Vec<ModelEntry>,
    pub(crate) mcp_servers: Vec<McpServerEntry>,
    pub(crate) fast_model: Option<FastModelSelection>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveOptionsAsRequest {
    pub(crate) source_path: String,
    pub(crate) target_path: String,
    pub(crate) options: ImportantOptions,
    pub(crate) env_vars: Vec<EnvironmentVariable>,
    pub(crate) models: Vec<ModelEntry>,
    pub(crate) mcp_servers: Vec<McpServerEntry>,
    pub(crate) fast_model: Option<FastModelSelection>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FetchProviderModelsRequest {
    pub(crate) provider_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildModelDraftRequest {
    pub(crate) profile: ProviderDraftProfile,
    pub(crate) model: CatalogModel,
    pub(crate) has_existing_default: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveUiStateRequest {
    pub(crate) layout_density: LayoutDensity,
    pub(crate) theme_mode: ThemeMode,
    pub(crate) custom_provider_profiles: Vec<CustomProviderProfile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewSettingsRequest {
    pub(crate) base_json: Value,
    pub(crate) options: ImportantOptions,
    pub(crate) env_vars: Vec<EnvironmentVariable>,
    pub(crate) models: Vec<ModelEntry>,
    pub(crate) mcp_servers: Vec<McpServerEntry>,
    pub(crate) fast_model: Option<FastModelSelection>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncMenuUiStateRequest {
    pub(crate) layout_density: LayoutDensity,
    pub(crate) theme_mode: ThemeMode,
    pub(crate) can_save: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PendingOpenPathResponse {
    pub(crate) path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsSnapshot {
    pub(crate) path: String,
    pub(crate) options: ImportantOptions,
    pub(crate) env_vars: Vec<EnvironmentVariable>,
    pub(crate) models: Vec<ModelEntry>,
    pub(crate) mcp_servers: Vec<McpServerEntry>,
    pub(crate) fast_model: FastModelSelection,
    pub(crate) providers: Vec<ProviderPreset>,
    pub(crate) warnings: Vec<String>,
    pub(crate) last_backup_path: Option<String>,
    pub(crate) json: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderFetchResult {
    pub(crate) provider_id: String,
    pub(crate) models: Vec<CatalogModel>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildModelDraftResult {
    pub(crate) model: ModelEntry,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelNameRewriteRule {
    pub(crate) pattern: String,
    pub(crate) replacement: Option<String>,
    pub(crate) uppercase_captures: Vec<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewSettingsResult {
    pub(crate) canonical_json: String,
    pub(crate) preview_json: String,
    pub(crate) warnings: Vec<String>,
    pub(crate) models: Vec<ModelEntry>,
    pub(crate) mcp_servers: Vec<McpServerEntry>,
    pub(crate) fast_model: FastModelSelection,
}
