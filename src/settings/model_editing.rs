use crate::{
    models::{
        apply_effective_default_flags, duplicate_key, mark_duplicate_models, normalized_model_name,
        prettify_model_name,
    },
    types::{ModelEntry, ReasoningEffort, ReasoningMode, SupportedProtocol},
};
use serde_json::{Map, Number, Value};
use std::collections::BTreeMap;

use super::json::{remove_path, set_string, value_as_f64, value_as_u64};

const SUPPORTED_PROTOCOLS: [SupportedProtocol; 2] =
    [SupportedProtocol::Openai, SupportedProtocol::Anthropic];

pub(super) fn get_models(json: &Value, warnings: &mut Vec<String>) -> Vec<ModelEntry> {
    let mut models = Vec::new();
    let Some(providers) = json.get("modelProviders") else {
        return models;
    };
    let Some(providers) = providers.as_object() else {
        warnings.push(
            "`modelProviders` is not a JSON object. It is preserved in the file but hidden from the structured editor."
                .to_string(),
        );
        return models;
    };

    for protocol in SUPPORTED_PROTOCOLS {
        let Some(bucket_value) = providers.get(protocol.as_str()) else {
            continue;
        };
        let Some(bucket) = bucket_value.as_array() else {
            warnings.push(format!(
                "`modelProviders.{}` is not an array. It is preserved in the file but hidden from the structured editor.",
                protocol.as_str()
            ));
            continue;
        };

        for (index, raw_model) in bucket.iter().enumerate() {
            let Some(model_object) = raw_model.as_object() else {
                warnings.push(format!(
                    "Ignored non-object entry in modelProviders.{} at index {}.",
                    protocol.as_str(),
                    index
                ));
                continue;
            };

            let Some(id) = model_object.get("id").and_then(Value::as_str) else {
                warnings.push(format!(
                    "Ignored model entry without a string id in modelProviders.{} at index {}.",
                    protocol.as_str(),
                    index
                ));
                continue;
            };

            let generation_config = model_object
                .get("generationConfig")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let sampling_params = generation_config
                .get("samplingParams")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let extra_body = generation_config
                .get("extra_body")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let reasoning = generation_config
                .get("reasoning")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();

            models.push(ModelEntry {
                ui_id: format!("saved-{}-{index}-{id}", protocol.as_str()),
                protocol,
                id: id.to_string(),
                name: model_object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| prettify_model_name(id)),
                base_url: model_object
                    .get("baseUrl")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                env_key: model_object
                    .get("envKey")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                context_window_size: generation_config
                    .get("contextWindowSize")
                    .and_then(value_as_u64),
                temperature: sampling_params.get("temperature").and_then(value_as_f64),
                top_p: sampling_params.get("top_p").and_then(value_as_f64),
                max_tokens: sampling_params.get("max_tokens").and_then(value_as_u64),
                reasoning_mode: reasoning_mode_from_value(reasoning.get("enabled")),
                reasoning_effort: reasoning_effort_from_value(reasoning.get("effort")),
                reasoning_budget_tokens: reasoning.get("budget_tokens").and_then(value_as_u64),
                sampling_params,
                extra_body,
                raw_model: raw_model.clone(),
                is_default: false,
                is_duplicate: false,
            });
        }
    }

    mark_duplicate_models(&mut models);
    apply_effective_default_flags(&mut models, json);
    models
}

pub(super) fn collect_editor_warnings(json: &Value, models: &[ModelEntry]) -> Vec<String> {
    let mut warnings = Vec::new();

    if let Some(providers) = json.get("modelProviders").and_then(Value::as_object) {
        for key in providers.keys() {
            if SupportedProtocol::parse(key).is_none() {
                warnings.push(format!(
                    "Models under unsupported protocol `{key}` are preserved in JSON but hidden from the structured editor."
                ));
            }
        }
    }

    let mut counts = BTreeMap::new();
    for model in models {
        let key = duplicate_key(&model.protocol, &model.id, &model.base_url);
        *counts.entry(key).or_insert(0usize) += 1;
    }

    for (key, count) in counts {
        if count > 1 {
            let mut parts = key.split('\0');
            let protocol = parts.next().unwrap_or_default();
            let model_id = parts.next().unwrap_or_default();
            let base_url = parts.next().unwrap_or_default();
            warnings.push(format!(
                "Duplicate model entry in {protocol}: {model_id}{}. Qwen Code only honors the first match for the same protocol and base URL.",
                if base_url.is_empty() {
                    String::new()
                } else {
                    format!(" @ {base_url}")
                }
            ));
        }
    }

    if let Some(default_model) = models.iter().find(|model| model.is_default)
        && models.iter().any(|model| {
            model.ui_id != default_model.ui_id && model.id.trim() == default_model.id.trim()
        })
    {
        warnings.push(format!(
            "Default model id `{}` exists more than once across the configured buckets. Qwen Code persists only the model id, so selection may be ambiguous.",
            default_model.id
        ));
    }

    warnings
}

pub(super) fn normalize_editor_models(
    canonical_json: &Value,
    models: &[ModelEntry],
) -> Vec<ModelEntry> {
    let mut normalized = models.to_vec();
    mark_duplicate_models(&mut normalized);
    apply_effective_default_flags(&mut normalized, canonical_json);
    normalized
}

pub(super) fn apply_models(json: &mut Value, models: &[ModelEntry]) -> Result<(), String> {
    let root = json
        .as_object_mut()
        .ok_or_else(|| "Settings root must be a JSON object".to_string())?;

    let mut grouped: BTreeMap<SupportedProtocol, Vec<Value>> = BTreeMap::new();

    for model in models {
        if model.id.trim().is_empty() {
            return Err("Model ids cannot be empty.".to_string());
        }

        let saved_model = patch_model_entry(model)?;
        grouped
            .entry(model.protocol)
            .or_default()
            .push(Value::Object(saved_model));
    }

    let providers_object = match root.get_mut("modelProviders") {
        Some(Value::Object(providers)) => providers,
        Some(_) if grouped.is_empty() => return Ok(()),
        Some(_) => {
            return Err(
                "Cannot save structured models because `modelProviders` is not a JSON object."
                    .to_string(),
            );
        }
        None if grouped.is_empty() => return Ok(()),
        None => root
            .entry("modelProviders".to_string())
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .expect("value was just inserted as an object"),
    };

    for protocol in SUPPORTED_PROTOCOLS {
        let Some(existing_value) = providers_object.get(protocol.as_str()) else {
            continue;
        };

        if existing_value.is_array() {
            providers_object.remove(protocol.as_str());
            continue;
        }

        if grouped.contains_key(&protocol) {
            return Err(format!(
                "Cannot save structured {} models because `modelProviders.{}` is not an array.",
                protocol.as_str(),
                protocol.as_str()
            ));
        }
    }

    for protocol in SUPPORTED_PROTOCOLS {
        if let Some(entries) = grouped.remove(&protocol)
            && !entries.is_empty()
        {
            providers_object.insert(protocol.as_str().to_string(), Value::Array(entries));
        }
    }

    Ok(())
}

fn patch_model_entry(model: &ModelEntry) -> Result<Map<String, Value>, String> {
    let mut object = model.raw_model.as_object().cloned().unwrap_or_default();

    object.insert("id".to_string(), Value::String(model.id.trim().to_string()));
    object.insert(
        "name".to_string(),
        Value::String(normalized_model_name(model.name.trim(), &model.id)),
    );

    if model.base_url.trim().is_empty() {
        object.remove("baseUrl");
    } else {
        object.insert(
            "baseUrl".to_string(),
            Value::String(model.base_url.trim().to_string()),
        );
    }

    if model.env_key.trim().is_empty() {
        object.remove("envKey");
    } else {
        object.insert(
            "envKey".to_string(),
            Value::String(model.env_key.trim().to_string()),
        );
    }

    let mut generation_config = object
        .get("generationConfig")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    if let Some(context_window_size) = model.context_window_size {
        generation_config.insert(
            "contextWindowSize".to_string(),
            Value::Number(Number::from(context_window_size)),
        );
    } else {
        generation_config.remove("contextWindowSize");
    }

    let mut sampling_params = model.sampling_params.clone();
    patch_numeric_field(&mut sampling_params, "temperature", model.temperature)?;
    patch_numeric_field(&mut sampling_params, "top_p", model.top_p)?;
    patch_integer_field(&mut sampling_params, "max_tokens", model.max_tokens);

    if sampling_params.is_empty() {
        generation_config.remove("samplingParams");
    } else {
        generation_config.insert("samplingParams".to_string(), Value::Object(sampling_params));
    }

    if model.extra_body.is_empty() {
        generation_config.remove("extra_body");
    } else {
        generation_config.insert(
            "extra_body".to_string(),
            Value::Object(model.extra_body.clone()),
        );
    }

    let mut reasoning = generation_config
        .get("reasoning")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    patch_reasoning_mode(&mut reasoning, model.reasoning_mode);
    patch_reasoning_effort(&mut reasoning, model.reasoning_effort);
    patch_integer_field(
        &mut reasoning,
        "budget_tokens",
        model.reasoning_budget_tokens,
    );

    if reasoning.is_empty() {
        generation_config.remove("reasoning");
    } else {
        generation_config.insert("reasoning".to_string(), Value::Object(reasoning));
    }

    if generation_config.is_empty() {
        object.remove("generationConfig");
    } else {
        object.insert(
            "generationConfig".to_string(),
            Value::Object(generation_config),
        );
    }

    Ok(object)
}

fn patch_numeric_field(
    object: &mut Map<String, Value>,
    key: &str,
    value: Option<f64>,
) -> Result<(), String> {
    if let Some(value) = value {
        let number =
            Number::from_f64(value).ok_or_else(|| format!("`{key}` must be a finite number"))?;
        object.insert(key.to_string(), Value::Number(number));
    } else {
        object.remove(key);
    }

    Ok(())
}

fn patch_integer_field(object: &mut Map<String, Value>, key: &str, value: Option<u64>) {
    if let Some(value) = value {
        object.insert(key.to_string(), Value::Number(Number::from(value)));
    } else {
        object.remove(key);
    }
}

fn patch_reasoning_mode(object: &mut Map<String, Value>, value: ReasoningMode) {
    match value {
        ReasoningMode::Default => {
            object.remove("enabled");
        }
        ReasoningMode::Enabled | ReasoningMode::Disabled => {
            object.insert(
                "enabled".to_string(),
                Value::Bool(matches!(value, ReasoningMode::Enabled)),
            );
        }
    }
}

fn reasoning_mode_from_value(value: Option<&Value>) -> ReasoningMode {
    match value {
        Some(Value::Bool(true)) => ReasoningMode::Enabled,
        Some(Value::Bool(false)) => ReasoningMode::Disabled,
        _ => ReasoningMode::Default,
    }
}

fn patch_reasoning_effort(object: &mut Map<String, Value>, value: Option<ReasoningEffort>) {
    if let Some(value) = value {
        object.insert(
            "effort".to_string(),
            Value::String(reasoning_effort_to_str(value).to_string()),
        );
    } else {
        object.remove("effort");
    }
}

fn reasoning_effort_from_value(value: Option<&Value>) -> Option<ReasoningEffort> {
    match value.and_then(Value::as_str) {
        Some("minimal") => Some(ReasoningEffort::Minimal),
        Some("low") => Some(ReasoningEffort::Low),
        Some("medium") => Some(ReasoningEffort::Medium),
        Some("high") => Some(ReasoningEffort::High),
        Some("xhigh") => Some(ReasoningEffort::XHigh),
        Some("max") => Some(ReasoningEffort::Max),
        _ => None,
    }
}

fn reasoning_effort_to_str(value: ReasoningEffort) -> &'static str {
    match value {
        ReasoningEffort::Minimal => "minimal",
        ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        ReasoningEffort::High => "high",
        ReasoningEffort::XHigh => "xhigh",
        ReasoningEffort::Max => "max",
    }
}

pub(super) fn ensure_default_model(json: &mut Value, models: &[ModelEntry]) -> Result<(), String> {
    if models.is_empty() {
        remove_path(json, &["model", "name"]);
        remove_path(json, &["security", "auth", "selectedType"]);
        return Ok(());
    }

    let explicit_default = models.iter().find(|model| model.is_default);

    if let Some(model) = explicit_default {
        set_string(json, &["model", "name"], model.id.trim().to_string())?;
        set_string(
            json,
            &["security", "auth", "selectedType"],
            model.protocol.as_str().to_string(),
        )?;
        return Ok(());
    }

    let existing_protocol = json
        .get("security")
        .and_then(|value| value.get("auth"))
        .and_then(|value| value.get("selectedType"))
        .and_then(Value::as_str);
    let existing_model = json
        .get("model")
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str);

    if let (Some(protocol), Some(model_id)) = (existing_protocol, existing_model)
        && model_exists_in_json(json, protocol, model_id)
    {
        return Ok(());
    }

    if let Some(first_model) = models.first() {
        set_string(json, &["model", "name"], first_model.id.trim().to_string())?;
        set_string(
            json,
            &["security", "auth", "selectedType"],
            first_model.protocol.as_str().to_string(),
        )?;
    }

    Ok(())
}

fn model_exists_in_json(json: &Value, protocol: &str, model_id: &str) -> bool {
    json.get("modelProviders")
        .and_then(Value::as_object)
        .and_then(|providers| providers.get(protocol))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .any(|model| model.get("id").and_then(Value::as_str) == Some(model_id))
}
