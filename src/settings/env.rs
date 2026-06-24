use crate::types::EnvironmentVariable;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn get_env_vars(json: &Value) -> Vec<EnvironmentVariable> {
    let mut env_vars: Vec<_> = json
        .get("env")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|env| env.iter())
        .filter_map(|(key, value)| {
            value.as_str().map(|value| EnvironmentVariable {
                key: key.clone(),
                value: value.to_string(),
            })
        })
        .collect();

    env_vars.sort_by(|left, right| left.key.cmp(&right.key));
    env_vars
}

pub(super) fn apply_env_vars(
    json: &mut Value,
    env_vars: &[EnvironmentVariable],
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    let mut env = BTreeMap::new();
    let preserved_env = match preserved_non_string_env_entries(json) {
        Ok(entries) => entries,
        Err(EnvShapeError::NonObject) if env_vars.is_empty() => return Ok(()),
        Err(EnvShapeError::NonObject) => {
            return Err(
                "Cannot save environment variables because `env` is not a JSON object.".to_string(),
            );
        }
        Err(EnvShapeError::RootNotObject) => {
            return Err("Settings root must be a JSON object".to_string());
        }
    };

    for env_var in env_vars {
        let key = env_var.key.trim();

        if key.is_empty() {
            return Err("Environment variable names cannot be empty".to_string());
        }

        if key.contains('=') {
            return Err(format!(
                "Environment variable `{key}` is invalid because names cannot contain `=`"
            ));
        }

        if !seen.insert(key.to_string()) {
            return Err(format!("Environment variable `{key}` is duplicated"));
        }

        env.insert(key.to_string(), env_var.value.clone());
    }

    let object = json
        .as_object_mut()
        .ok_or_else(|| "Settings root must be a JSON object".to_string())?;

    if env.is_empty() && preserved_env.is_empty() {
        object.remove("env");
    } else {
        let mut merged_env: Map<String, Value> = env
            .into_iter()
            .map(|(key, value)| (key, Value::String(value)))
            .collect();
        merged_env.extend(preserved_env);
        object.insert("env".to_string(), Value::Object(merged_env));
    }

    Ok(())
}

pub(super) fn collect_env_warnings(json: &Value) -> Vec<String> {
    let Some(env) = json.get("env") else {
        return Vec::new();
    };

    let Some(env_object) = env.as_object() else {
        return vec![
            "`env` is not a JSON object. It is preserved in the file but hidden from the structured editor."
                .to_string(),
        ];
    };

    let mut warnings = Vec::new();
    let hidden_keys: Vec<_> = env_object
        .iter()
        .filter_map(|(key, value)| (!value.is_string()).then_some(key.as_str()))
        .collect();

    if !hidden_keys.is_empty() {
        warnings.push(format!(
            "Non-string `env` entries are preserved in JSON but hidden from the structured editor: {}.",
            hidden_keys.join(", ")
        ));
    }

    warnings
}

fn preserved_non_string_env_entries(json: &Value) -> Result<Map<String, Value>, EnvShapeError> {
    let Some(root) = json.as_object() else {
        return Err(EnvShapeError::RootNotObject);
    };

    let Some(env) = root.get("env") else {
        return Ok(Map::new());
    };

    let Some(env_object) = env.as_object() else {
        return Err(EnvShapeError::NonObject);
    };

    Ok(env_object
        .iter()
        .filter_map(|(key, value)| (!value.is_string()).then_some((key.clone(), value.clone())))
        .collect())
}

enum EnvShapeError {
    RootNotObject,
    NonObject,
}

pub(super) fn mask_preview_env_values(mut json: Value) -> Value {
    if let Some(root) = json.as_object_mut()
        && let Some(env) = root.get_mut("env").and_then(Value::as_object_mut)
    {
        for value in env.values_mut() {
            *value = Value::String("********".to_string());
        }
    }

    json
}
