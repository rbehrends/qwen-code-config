use crate::types::{FastModelMode, FastModelSelection, ModelEntry, SupportedProtocol};
use serde_json::Value;

use super::json::{remove_path, set_string};

pub(super) fn parse_fast_model(json: &Value) -> (FastModelSelection, Vec<String>) {
    let Some(raw_value) = json.get("fastModel") else {
        return (FastModelSelection::default(), Vec::new());
    };

    let Some(raw_string) = raw_value.as_str() else {
        return (
            FastModelSelection {
                mode: FastModelMode::Invalid,
                protocol: None,
                model_id: None,
                raw_value: Some(raw_value.to_string()),
            },
            vec!["Invalid fastModel value is preserved as-is until changed. Expected `protocol:model-id`.".to_string()],
        );
    };

    let trimmed = raw_string.trim();
    if trimmed.is_empty() {
        return (FastModelSelection::default(), Vec::new());
    }

    let Some((protocol_raw, model_id_raw)) = trimmed.split_once(':') else {
        return invalid_fast_model(trimmed);
    };
    let protocol_raw = protocol_raw.trim();
    let model_id = model_id_raw.trim();
    if protocol_raw.is_empty() || model_id.is_empty() || model_id.contains(':') {
        return invalid_fast_model(trimmed);
    }

    let Some(protocol) = SupportedProtocol::parse(protocol_raw) else {
        return invalid_fast_model(trimmed);
    };

    (
        FastModelSelection {
            mode: FastModelMode::Specific,
            protocol: Some(protocol),
            model_id: Some(model_id.to_string()),
            raw_value: None,
        },
        Vec::new(),
    )
}

pub(super) fn apply_fast_model(
    json: &mut Value,
    fast_model: Option<&FastModelSelection>,
) -> Result<(), String> {
    let Some(fast_model) = fast_model else {
        return Ok(());
    };

    match fast_model.mode {
        FastModelMode::Inherit => {
            remove_path(json, &["fastModel"]);
            Ok(())
        }
        FastModelMode::Specific => {
            let protocol = fast_model
                .protocol
                .ok_or_else(|| "Fast model protocol is required.".to_string())?;
            let model_id = fast_model
                .model_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "Fast model id is required.".to_string())?;
            set_string(
                json,
                &["fastModel"],
                format!("{}:{model_id}", protocol.as_str()),
            )
        }
        FastModelMode::Invalid => {
            Err("Fast model selection cannot be saved while invalid.".to_string())
        }
    }
}

pub(super) fn collect_fast_model_warnings(
    fast_model: &FastModelSelection,
    models: &[ModelEntry],
) -> Vec<String> {
    let mut warnings = Vec::new();

    if fast_model.mode == FastModelMode::Invalid {
        return warnings;
    }

    let Some(protocol) = fast_model.protocol else {
        return warnings;
    };
    let Some(model_id) = fast_model.model_id.as_deref() else {
        return warnings;
    };

    let matching_models: Vec<&ModelEntry> = models
        .iter()
        .filter(|model| model.protocol == protocol && model.id.trim() == model_id)
        .collect();

    if matching_models.is_empty() {
        warnings.push(format!(
            "fastModel `{}` does not match any configured {protocol} model entry shown in the structured editor.",
            format!("{}:{model_id}", protocol.as_str())
        ));
        return warnings;
    }

    if matching_models.len() > 1 {
        warnings.push(format!(
            "fastModel `{}` matches more than one configured entry. Qwen Code persists only protocol and model id, so resolution may be ambiguous.",
            format!("{}:{model_id}", protocol.as_str())
        ));
    }

    warnings
}

fn invalid_fast_model(raw_value: &str) -> (FastModelSelection, Vec<String>) {
    (
        FastModelSelection {
            mode: FastModelMode::Invalid,
            protocol: None,
            model_id: None,
            raw_value: Some(raw_value.to_string()),
        },
        vec![format!(
            "Invalid fastModel `{raw_value}` is preserved as-is until changed. Expected `protocol:model-id`."
        )],
    )
}
