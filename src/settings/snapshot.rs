use crate::{
    catalog::builtin_provider_presets,
    types::{ImportantOptions, SettingsSnapshot},
};
use serde_json::Value;

use super::{
    env::{collect_env_warnings, get_env_vars},
    fast_model::{collect_fast_model_warnings, parse_fast_model},
    json::get_bool,
    mcp::{collect_mcp_warnings, get_mcp_servers},
    model_editing::{collect_editor_warnings, get_models},
};

pub(super) fn build_snapshot(
    path: String,
    json: Value,
    last_backup_path: Option<String>,
) -> Result<SettingsSnapshot, String> {
    let mut options = ImportantOptions::default();
    if let Some(value) = get_bool(&json, &["privacy", "usageStatisticsEnabled"]) {
        options.usage_statistics_enabled = value;
    }
    if let Some(value) = get_bool(&json, &["telemetry", "enabled"]) {
        options.telemetry_enabled = value;
    }
    if let Some(value) = get_bool(&json, &["general", "enableAutoUpdate"]) {
        options.enable_auto_update = value;
    }
    let mut parse_warnings = Vec::new();
    let models = get_models(&json, &mut parse_warnings);
    let mcp_servers = get_mcp_servers(&json, &mut parse_warnings);
    let (fast_model, fast_model_warnings) = parse_fast_model(&json);
    let warnings = collect_editor_warnings(&json, &models)
        .into_iter()
        .chain(collect_env_warnings(&json))
        .chain(collect_mcp_warnings(&json, &mcp_servers))
        .chain(collect_fast_model_warnings(&fast_model, &models))
        .chain(fast_model_warnings)
        .chain(parse_warnings)
        .collect();

    Ok(SettingsSnapshot {
        path,
        options,
        env_vars: get_env_vars(&json),
        models,
        mcp_servers,
        fast_model,
        providers: builtin_provider_presets(),
        warnings,
        last_backup_path,
        json: serde_json::to_string_pretty(&json)
            .map_err(|error| format!("Failed to format settings JSON: {error}"))?,
    })
}
