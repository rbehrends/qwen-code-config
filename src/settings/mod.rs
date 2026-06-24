mod env;
mod fast_model;
mod json;
mod mcp;
mod model_editing;
mod options;
mod snapshot;

use crate::{
    backup::{atomic_write_text, prune_backups_if_needed, write_backup_if_present},
    paths::expand_settings_path,
    types::{
        EnvironmentVariable, FastModelSelection, ImportantOptions, McpServerEntry, ModelEntry,
        PreviewSettingsResult, SettingsSnapshot,
    },
};
use serde_json::{Map, Value};
use std::fs;

use self::{
    env::{apply_env_vars, collect_env_warnings, mask_preview_env_values},
    fast_model::{apply_fast_model, collect_fast_model_warnings, parse_fast_model},
    mcp::{apply_mcp_servers, collect_mcp_warnings},
    model_editing::{
        apply_models, collect_editor_warnings, ensure_default_model, normalize_editor_models,
    },
    options::apply_important_options,
    snapshot::build_snapshot,
};

const DEFAULT_SETTINGS_PATH: &str = "~/.qwen/settings.json";

pub(crate) fn load_settings_snapshot(path: Option<String>) -> Result<SettingsSnapshot, String> {
    let path = path.unwrap_or_else(|| DEFAULT_SETTINGS_PATH.to_string());
    let path_buf = expand_settings_path(&path)?;
    let json = load_settings_json_or_empty(&path_buf)?;

    build_snapshot(path, json, None)
}

pub(crate) fn save_settings_to_path(
    source_path: &str,
    target_path: &str,
    options: ImportantOptions,
    env_vars: Vec<EnvironmentVariable>,
    models: Vec<ModelEntry>,
    mcp_servers: Vec<McpServerEntry>,
    fast_model: Option<FastModelSelection>,
) -> Result<SettingsSnapshot, String> {
    let source_path_buf = expand_settings_path(source_path)?;
    let target_path_buf = expand_settings_path(target_path)?;
    let json = load_settings_json_or_empty(&source_path_buf)?;
    let json = build_settings_json(
        json,
        &options,
        &env_vars,
        &models,
        &mcp_servers,
        fast_model.as_ref(),
    )?;

    let formatted = serde_json::to_string_pretty(&json)
        .map_err(|error| format!("Failed to format settings JSON: {error}"))?;
    let formatted = format!("{formatted}\n");

    let backup_path = write_backup_if_present(&target_path_buf)?;
    atomic_write_text(&target_path_buf, &formatted)?;
    prune_backups_if_needed(&target_path_buf);

    build_snapshot(
        target_path.to_string(),
        json,
        backup_path.map(|path| path.display().to_string()),
    )
}

fn load_settings_json_or_empty(path: &std::path::Path) -> Result<Value, String> {
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents)
            .map_err(|error| format!("Failed to parse {}: {error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Value::Object(Map::new())),
        Err(error) => Err(format!("Failed to read {}: {error}", path.display())),
    }
}

pub(crate) fn preview_settings(
    base_json: Value,
    options: ImportantOptions,
    env_vars: Vec<EnvironmentVariable>,
    models: Vec<ModelEntry>,
    mcp_servers: Vec<McpServerEntry>,
    fast_model: Option<FastModelSelection>,
) -> Result<PreviewSettingsResult, String> {
    let canonical_json = build_settings_json(
        base_json,
        &options,
        &env_vars,
        &models,
        &mcp_servers,
        fast_model.as_ref(),
    )?;
    let normalized_models = normalize_editor_models(&canonical_json, &models);
    let mut mcp_parse_warnings = Vec::new();
    let normalized_mcp_servers = mcp::get_mcp_servers(&canonical_json, &mut mcp_parse_warnings);
    let (normalized_fast_model, fast_model_parse_warnings) = parse_fast_model(&canonical_json);
    let warnings = collect_editor_warnings(&canonical_json, &normalized_models)
        .into_iter()
        .chain(collect_env_warnings(&canonical_json))
        .chain(collect_mcp_warnings(&canonical_json, &normalized_mcp_servers))
        .chain(collect_fast_model_warnings(
            &normalized_fast_model,
            &normalized_models,
        ))
        .chain(mcp_parse_warnings)
        .chain(fast_model_parse_warnings)
        .collect();
    let preview_json = mask_preview_env_values(canonical_json.clone());

    Ok(PreviewSettingsResult {
        canonical_json: serde_json::to_string_pretty(&canonical_json)
            .map_err(|error| format!("Failed to format settings JSON: {error}"))?,
        preview_json: serde_json::to_string_pretty(&preview_json)
            .map_err(|error| format!("Failed to format preview JSON: {error}"))?,
        warnings,
        models: normalized_models,
        mcp_servers: normalized_mcp_servers,
        fast_model: normalized_fast_model,
    })
}

fn build_settings_json(
    mut json: Value,
    options: &ImportantOptions,
    env_vars: &[EnvironmentVariable],
    models: &[ModelEntry],
    mcp_servers: &[McpServerEntry],
    fast_model: Option<&FastModelSelection>,
) -> Result<Value, String> {
    apply_important_options(&mut json, options)?;
    apply_env_vars(&mut json, env_vars)?;
    apply_models(&mut json, models)?;
    apply_mcp_servers(&mut json, mcp_servers)?;
    ensure_default_model(&mut json, models)?;
    apply_fast_model(&mut json, fast_model)?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        paths::home_dir_for_current_user,
        settings::{
            env::apply_env_vars,
            json::{get_bool, set_bool},
            model_editing::{
                apply_models, ensure_default_model, get_models, normalize_editor_models,
            },
        },
        types::{ReasoningEffort, ReasoningMode, SupportedProtocol},
    };
    use serde_json::{Map, Value};
    use std::path::PathBuf;

    #[test]
    fn set_bool_updates_nested_value_without_removing_other_settings() {
        let mut json = serde_json::json!({
            "privacy": {
                "usageStatisticsEnabled": true,
                "otherPrivacySetting": "preserve"
            },
            "modelProviders": {
                "openai": [
                    {
                        "id": "qwen3.5-plus",
                        "name": "Qwen 3.5 Plus"
                    }
                ]
            }
        });

        set_bool(&mut json, &["privacy", "usageStatisticsEnabled"], false).unwrap();
        set_bool(&mut json, &["telemetry", "enabled"], false).unwrap();

        assert_eq!(
            get_bool(&json, &["privacy", "usageStatisticsEnabled"]),
            Some(false)
        );
        assert_eq!(get_bool(&json, &["telemetry", "enabled"]), Some(false));
        assert_eq!(json["privacy"]["otherPrivacySetting"], "preserve");
        assert_eq!(json["modelProviders"]["openai"][0]["id"], "qwen3.5-plus");
    }

    #[test]
    fn apply_env_vars_replaces_string_env_entries_without_removing_other_settings() {
        let mut json = serde_json::json!({
            "env": {
                "OLD_KEY": "old"
            },
            "privacy": {
                "usageStatisticsEnabled": false
            }
        });

        apply_env_vars(
            &mut json,
            &[
                EnvironmentVariable {
                    key: "DUMMY_KEY".to_string(),
                    value: "dummy".to_string(),
                },
                EnvironmentVariable {
                    key: "API_KEY".to_string(),
                    value: "secret".to_string(),
                },
            ],
        )
        .unwrap();

        assert_eq!(json["env"]["API_KEY"], "secret");
        assert_eq!(json["env"]["DUMMY_KEY"], "dummy");
        assert!(json["env"].get("OLD_KEY").is_none());
        assert_eq!(json["privacy"]["usageStatisticsEnabled"], false);
    }

    #[test]
    fn apply_env_vars_preserves_non_string_entries() {
        let mut json = serde_json::json!({
            "env": {
                "API_KEY": "old",
                "RETRY_COUNT": 3,
                "FEATURE_FLAGS": {
                    "beta": true
                }
            }
        });

        apply_env_vars(
            &mut json,
            &[EnvironmentVariable {
                key: "API_KEY".to_string(),
                value: "new".to_string(),
            }],
        )
        .unwrap();

        assert_eq!(json["env"]["API_KEY"], "new");
        assert_eq!(json["env"]["RETRY_COUNT"], 3);
        assert_eq!(json["env"]["FEATURE_FLAGS"]["beta"], true);
    }

    #[test]
    fn apply_env_vars_preserves_non_object_env_when_editor_has_no_entries() {
        let mut json = serde_json::json!({
            "env": "preserve me"
        });

        apply_env_vars(&mut json, &[]).unwrap();

        assert_eq!(json["env"], "preserve me");
    }

    #[test]
    fn apply_env_vars_rejects_duplicate_keys() {
        let mut json = serde_json::json!({});
        let error = apply_env_vars(
            &mut json,
            &[
                EnvironmentVariable {
                    key: "API_KEY".to_string(),
                    value: "first".to_string(),
                },
                EnvironmentVariable {
                    key: "API_KEY".to_string(),
                    value: "second".to_string(),
                },
            ],
        )
        .unwrap_err();

        assert!(error.contains("duplicated"));
    }

    #[test]
    fn load_settings_json_or_empty_returns_empty_object_for_missing_file() {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "qwenconf-missing-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let json = load_settings_json_or_empty(&path).unwrap();
        assert_eq!(json, Value::Object(Map::new()));
    }

    #[test]
    fn apply_models_preserves_unknown_generation_fields() {
        let mut json = serde_json::json!({
            "modelProviders": {
                "openai": [
                    {
                        "id": "test-model",
                        "name": "Test Model",
                        "envKey": "OPENAI_API_KEY",
                        "baseUrl": "https://example.com/v1",
                        "generationConfig": {
                            "samplingParams": {
                                "temperature": 0.1,
                                "top_p": 0.9,
                                "custom_knob": true
                            },
                            "extra_body": {
                                "keep_me": "yes"
                            },
                            "reasoning": {
                                "enabled": true,
                                "effort": "high"
                            }
                        }
                    }
                ]
            }
        });

        let model = get_models(&json, &mut Vec::new()).remove(0);
        assert_eq!(model.reasoning_mode, ReasoningMode::Enabled);
        assert_eq!(model.reasoning_effort, Some(ReasoningEffort::High));
        assert_eq!(model.reasoning_budget_tokens, None);
        let mut edited = model.clone();
        edited.temperature = Some(0.7);
        edited.max_tokens = Some(4096);
        edited.reasoning_mode = ReasoningMode::Disabled;
        edited.reasoning_effort = Some(ReasoningEffort::Max);
        edited.reasoning_budget_tokens = Some(2048);

        apply_models(&mut json, &[edited]).unwrap();

        assert_eq!(
            json["modelProviders"]["openai"][0]["generationConfig"]["samplingParams"]["custom_knob"],
            true
        );
        assert_eq!(
            json["modelProviders"]["openai"][0]["generationConfig"]["samplingParams"]["temperature"],
            0.7
        );
        assert_eq!(
            json["modelProviders"]["openai"][0]["generationConfig"]["samplingParams"]["max_tokens"],
            4096
        );
        assert_eq!(
            json["modelProviders"]["openai"][0]["generationConfig"]["extra_body"]["keep_me"],
            "yes"
        );
        assert_eq!(
            json["modelProviders"]["openai"][0]["generationConfig"]["reasoning"]["enabled"],
            false
        );
        assert_eq!(
            json["modelProviders"]["openai"][0]["generationConfig"]["reasoning"]["effort"],
            "max"
        );
        assert_eq!(
            json["modelProviders"]["openai"][0]["generationConfig"]["reasoning"]["budget_tokens"],
            2048
        );
    }

    #[test]
    fn apply_models_preserves_unsupported_protocol_buckets() {
        let mut json = serde_json::json!({
            "modelProviders": {
                "gemini": [
                    {
                        "id": "gemini-2.5-pro"
                    }
                ]
            }
        });

        let models = vec![ModelEntry {
            ui_id: "manual-1".to_string(),
            protocol: SupportedProtocol::Openai,
            id: "gpt-4o".to_string(),
            name: "GPT 4o".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            env_key: "OPENAI_API_KEY".to_string(),
            context_window_size: Some(128000),
            temperature: Some(0.2),
            top_p: None,
            max_tokens: None,
            reasoning_mode: ReasoningMode::Default,
            reasoning_effort: None,
            reasoning_budget_tokens: None,
            sampling_params: Map::new(),
            extra_body: Map::new(),
            raw_model: Value::Object(Map::new()),
            is_default: false,
            is_duplicate: false,
        }];

        apply_models(&mut json, &models).unwrap();

        assert_eq!(json["modelProviders"]["gemini"][0]["id"], "gemini-2.5-pro");
        assert_eq!(json["modelProviders"]["openai"][0]["id"], "gpt-4o");
    }

    #[test]
    fn apply_models_preserves_non_array_supported_bucket() {
        let mut json = serde_json::json!({
            "modelProviders": {
                "openai": {
                    "unexpected": true
                },
                "anthropic": [
                    {
                        "id": "claude-3-7-sonnet",
                        "name": "Claude 3.7 Sonnet"
                    }
                ]
            }
        });

        let models = vec![ModelEntry {
            ui_id: "manual-1".to_string(),
            protocol: SupportedProtocol::Anthropic,
            id: "claude-3-7-sonnet".to_string(),
            name: "Claude 3.7 Sonnet".to_string(),
            base_url: String::new(),
            env_key: String::new(),
            context_window_size: None,
            temperature: Some(0.4),
            top_p: None,
            max_tokens: None,
            reasoning_mode: ReasoningMode::Default,
            reasoning_effort: None,
            reasoning_budget_tokens: None,
            sampling_params: Map::new(),
            extra_body: Map::new(),
            raw_model: Value::Object(Map::new()),
            is_default: false,
            is_duplicate: false,
        }];

        apply_models(&mut json, &models).unwrap();

        assert_eq!(json["modelProviders"]["openai"]["unexpected"], true);
        assert_eq!(
            json["modelProviders"]["anthropic"][0]["generationConfig"]["samplingParams"]["temperature"],
            0.4
        );
    }

    #[test]
    fn apply_models_rejects_non_object_model_providers_when_structured_models_exist() {
        let mut json = serde_json::json!({
            "modelProviders": "broken"
        });
        let models = vec![ModelEntry {
            ui_id: "manual-1".to_string(),
            protocol: SupportedProtocol::Openai,
            id: "gpt-4o".to_string(),
            name: "GPT 4o".to_string(),
            base_url: String::new(),
            env_key: String::new(),
            context_window_size: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            reasoning_mode: ReasoningMode::Default,
            reasoning_effort: None,
            reasoning_budget_tokens: None,
            sampling_params: Map::new(),
            extra_body: Map::new(),
            raw_model: Value::Object(Map::new()),
            is_default: false,
            is_duplicate: false,
        }];

        let error = apply_models(&mut json, &models).unwrap_err();
        assert!(error.contains("modelProviders"));
    }

    #[test]
    fn get_models_warns_when_supported_bucket_is_not_an_array() {
        let json = serde_json::json!({
            "modelProviders": {
                "openai": {
                    "unexpected": true
                }
            }
        });
        let mut warnings = Vec::new();

        let models = get_models(&json, &mut warnings);

        assert!(models.is_empty());
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("modelProviders.openai"))
        );
    }

    #[test]
    fn set_bool_rejects_non_object_intermediate_values() {
        let mut json = serde_json::json!({
            "privacy": "unexpected"
        });

        let error = set_bool(&mut json, &["privacy", "usageStatisticsEnabled"], false).unwrap_err();

        assert!(error.contains("privacy"));
        assert_eq!(json["privacy"], "unexpected");
    }

    #[test]
    fn expand_settings_path_supports_current_user_home() {
        let home = home_dir_for_current_user().unwrap();

        assert_eq!(expand_settings_path("~").unwrap(), home);
        assert_eq!(
            expand_settings_path("~/settings.json").unwrap(),
            home.join("settings.json")
        );
        assert_eq!(
            expand_settings_path("qwen-settings.json").unwrap(),
            PathBuf::from("qwen-settings.json")
        );
    }

    #[test]
    fn ensure_default_model_clears_default_fields_when_no_models_remain() {
        let mut json = serde_json::json!({
            "model": {
                "name": "qwen3.5-plus"
            },
            "security": {
                "auth": {
                    "selectedType": "openai"
                }
            }
        });

        ensure_default_model(&mut json, &[]).unwrap();

        assert!(json["model"].get("name").is_none());
        assert!(json["security"]["auth"].get("selectedType").is_none());
    }

    #[test]
    fn normalize_editor_models_marks_effective_default_after_removal() {
        let base_json = serde_json::json!({
            "model": {
                "name": "model-a"
            },
            "security": {
                "auth": {
                    "selectedType": "openai"
                }
            }
        });

        let models = vec![ModelEntry {
            ui_id: "manual-1".to_string(),
            protocol: SupportedProtocol::Openai,
            id: "model-b".to_string(),
            name: "Model B".to_string(),
            base_url: String::new(),
            env_key: String::new(),
            context_window_size: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            reasoning_mode: ReasoningMode::Default,
            reasoning_effort: None,
            reasoning_budget_tokens: None,
            sampling_params: Map::new(),
            extra_body: Map::new(),
            raw_model: Value::Object(Map::new()),
            is_default: false,
            is_duplicate: false,
        }];

        let canonical_json = build_settings_json(
            base_json,
            &ImportantOptions::default(),
            &[],
            &models,
            &[],
            None,
        )
        .unwrap();
        let normalized = normalize_editor_models(&canonical_json, &models);
        assert_eq!(normalized.len(), 1);
        assert!(normalized[0].is_default);
    }

    #[test]
    fn normalize_editor_models_marks_duplicate_rows() {
        let base_json = serde_json::json!({});
        let models = vec![
            ModelEntry {
                ui_id: "manual-1".to_string(),
                protocol: SupportedProtocol::Openai,
                id: "gpt-4o".to_string(),
                name: "GPT 4o".to_string(),
                base_url: "https://api.example.com/v1".to_string(),
                env_key: String::new(),
                context_window_size: None,
                temperature: None,
                top_p: None,
                max_tokens: None,
                reasoning_mode: ReasoningMode::Default,
                reasoning_effort: None,
                reasoning_budget_tokens: None,
                sampling_params: Map::new(),
                extra_body: Map::new(),
                raw_model: Value::Object(Map::new()),
                is_default: false,
                is_duplicate: false,
            },
            ModelEntry {
                ui_id: "manual-2".to_string(),
                protocol: SupportedProtocol::Openai,
                id: "gpt-4o".to_string(),
                name: "GPT 4o Copy".to_string(),
                base_url: "https://api.example.com/v1".to_string(),
                env_key: String::new(),
                context_window_size: None,
                temperature: None,
                top_p: None,
                max_tokens: None,
                reasoning_mode: ReasoningMode::Default,
                reasoning_effort: None,
                reasoning_budget_tokens: None,
                sampling_params: Map::new(),
                extra_body: Map::new(),
                raw_model: Value::Object(Map::new()),
                is_default: false,
                is_duplicate: false,
            },
        ];

        let canonical_json = build_settings_json(
            base_json,
            &ImportantOptions::default(),
            &[],
            &models,
            &[],
            None,
        )
        .unwrap();
        let normalized = normalize_editor_models(&canonical_json, &models);
        assert!(normalized.iter().all(|model| model.is_duplicate));
    }

    #[test]
    fn get_models_marks_only_first_duplicate_as_default() {
        let json = serde_json::json!({
            "model": {
                "name": "gpt-4o"
            },
            "security": {
                "auth": {
                    "selectedType": "openai"
                }
            },
            "modelProviders": {
                "openai": [
                    {
                        "id": "gpt-4o",
                        "name": "GPT 4o",
                        "baseUrl": "https://provider-a.example/v1"
                    },
                    {
                        "id": "gpt-4o",
                        "name": "GPT 4o Copy",
                        "baseUrl": "https://provider-b.example/v1"
                    }
                ]
            }
        });

        let models = get_models(&json, &mut Vec::new());

        assert_eq!(models.len(), 2);
        assert!(models[0].is_default);
        assert!(!models[1].is_default);
    }

    #[test]
    fn normalize_editor_models_marks_only_first_duplicate_as_default() {
        let base_json = serde_json::json!({
            "model": {
                "name": "gpt-4o"
            },
            "security": {
                "auth": {
                    "selectedType": "openai"
                }
            }
        });
        let models = vec![
            ModelEntry {
                ui_id: "manual-1".to_string(),
                protocol: SupportedProtocol::Openai,
                id: "gpt-4o".to_string(),
                name: "GPT 4o".to_string(),
                base_url: "https://provider-a.example/v1".to_string(),
                env_key: String::new(),
                context_window_size: None,
                temperature: None,
                top_p: None,
                max_tokens: None,
                reasoning_mode: ReasoningMode::Default,
                reasoning_effort: None,
                reasoning_budget_tokens: None,
                sampling_params: Map::new(),
                extra_body: Map::new(),
                raw_model: Value::Object(Map::new()),
                is_default: false,
                is_duplicate: false,
            },
            ModelEntry {
                ui_id: "manual-2".to_string(),
                protocol: SupportedProtocol::Openai,
                id: "gpt-4o".to_string(),
                name: "GPT 4o Copy".to_string(),
                base_url: "https://provider-b.example/v1".to_string(),
                env_key: String::new(),
                context_window_size: None,
                temperature: None,
                top_p: None,
                max_tokens: None,
                reasoning_mode: ReasoningMode::Default,
                reasoning_effort: None,
                reasoning_budget_tokens: None,
                sampling_params: Map::new(),
                extra_body: Map::new(),
                raw_model: Value::Object(Map::new()),
                is_default: false,
                is_duplicate: false,
            },
        ];

        let canonical_json = build_settings_json(
            base_json,
            &ImportantOptions::default(),
            &[],
            &models,
            &[],
            None,
        )
        .unwrap();
        let normalized = normalize_editor_models(&canonical_json, &models);

        assert_eq!(normalized.len(), 2);
        assert!(normalized[0].is_default);
        assert!(!normalized[1].is_default);
    }

    #[test]
    fn build_settings_json_preserves_invalid_fast_model_when_untouched() {
        let base_json = serde_json::json!({
            "fastModel": "not-a-valid-fast-model"
        });

        let canonical_json = build_settings_json(
            base_json,
            &ImportantOptions::default(),
            &[],
            &[],
            &[],
            None,
        )
        .unwrap();

        assert_eq!(canonical_json["fastModel"], "not-a-valid-fast-model");
    }

    #[test]
    fn build_settings_json_clears_fast_model_when_inherit_is_selected() {
        let base_json = serde_json::json!({
            "fastModel": "openai:deepseek-v4-flash"
        });

        let canonical_json = build_settings_json(
            base_json,
            &ImportantOptions::default(),
            &[],
            &[],
            &[],
            Some(&FastModelSelection::default()),
        )
        .unwrap();

        assert!(canonical_json.get("fastModel").is_none());
    }

    #[test]
    fn build_settings_json_sets_fast_model_override() {
        let canonical_json = build_settings_json(
            serde_json::json!({}),
            &ImportantOptions::default(),
            &[],
            &[],
            &[],
            Some(&FastModelSelection {
                mode: crate::types::FastModelMode::Specific,
                protocol: Some(SupportedProtocol::Openai),
                model_id: Some("deepseek-v4-flash".to_string()),
                raw_value: None,
            }),
        )
        .unwrap();

        assert_eq!(canonical_json["fastModel"], "openai:deepseek-v4-flash");
    }

    #[test]
    fn preview_settings_warns_for_ambiguous_fast_model() {
        let models = vec![
            ModelEntry {
                ui_id: "manual-1".to_string(),
                protocol: SupportedProtocol::Openai,
                id: "gpt-4o-mini".to_string(),
                name: "GPT 4o Mini".to_string(),
                base_url: "https://provider-a.example/v1".to_string(),
                env_key: String::new(),
                context_window_size: None,
                temperature: None,
                top_p: None,
                max_tokens: None,
                reasoning_mode: ReasoningMode::Default,
                reasoning_effort: None,
                reasoning_budget_tokens: None,
                sampling_params: Map::new(),
                extra_body: Map::new(),
                raw_model: Value::Object(Map::new()),
                is_default: false,
                is_duplicate: false,
            },
            ModelEntry {
                ui_id: "manual-2".to_string(),
                protocol: SupportedProtocol::Openai,
                id: "gpt-4o-mini".to_string(),
                name: "GPT 4o Mini Copy".to_string(),
                base_url: "https://provider-b.example/v1".to_string(),
                env_key: String::new(),
                context_window_size: None,
                temperature: None,
                top_p: None,
                max_tokens: None,
                reasoning_mode: ReasoningMode::Default,
                reasoning_effort: None,
                reasoning_budget_tokens: None,
                sampling_params: Map::new(),
                extra_body: Map::new(),
                raw_model: Value::Object(Map::new()),
                is_default: false,
                is_duplicate: false,
            },
        ];

        let preview = preview_settings(
            serde_json::json!({}),
            ImportantOptions::default(),
            Vec::new(),
            models,
            Vec::new(),
            Some(FastModelSelection {
                mode: crate::types::FastModelMode::Specific,
                protocol: Some(SupportedProtocol::Openai),
                model_id: Some("gpt-4o-mini".to_string()),
                raw_value: None,
            }),
        )
        .unwrap();

        assert!(preview.warnings.iter().any(|warning| {
            warning.contains("fastModel `openai:gpt-4o-mini` matches more than one")
        }));
    }

    #[test]
    fn preview_settings_trims_fast_model_id_before_matching_models() {
        let models = vec![ModelEntry {
            ui_id: "manual-1".to_string(),
            protocol: SupportedProtocol::Openai,
            id: "gpt-4o-mini".to_string(),
            name: "GPT 4o Mini".to_string(),
            base_url: String::new(),
            env_key: String::new(),
            context_window_size: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            reasoning_mode: ReasoningMode::Default,
            reasoning_effort: None,
            reasoning_budget_tokens: None,
            sampling_params: Map::new(),
            extra_body: Map::new(),
            raw_model: Value::Object(Map::new()),
            is_default: false,
            is_duplicate: false,
        }];

        let preview = preview_settings(
            serde_json::json!({
                "fastModel": "openai: gpt-4o-mini"
            }),
            ImportantOptions::default(),
            Vec::new(),
            models,
            Vec::new(),
            None,
        )
        .unwrap();

        assert_eq!(preview.fast_model.model_id.as_deref(), Some("gpt-4o-mini"));
        assert!(
            !preview
                .warnings
                .iter()
                .any(|warning| warning.contains("does not match any configured"))
        );
    }
}
