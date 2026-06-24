use serde_json::{Map, Value};

pub(super) fn value_as_u64(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| {
        value
            .as_i64()
            .and_then(|value| if value >= 0 { Some(value as u64) } else { None })
    })
}

pub(super) fn value_as_f64(value: &Value) -> Option<f64> {
    value.as_f64()
}

pub(super) fn get_bool(json: &Value, path: &[&str]) -> Option<bool> {
    path.iter()
        .try_fold(json, |current, key| current.get(*key))
        .and_then(Value::as_bool)
}

pub(super) fn set_bool(json: &mut Value, path: &[&str], value: bool) -> Result<(), String> {
    set_path_value(json, path, Value::Bool(value))
}

pub(super) fn set_string(json: &mut Value, path: &[&str], value: String) -> Result<(), String> {
    set_path_value(json, path, Value::String(value))
}

pub(super) fn remove_path(json: &mut Value, path: &[&str]) {
    if path.is_empty() {
        return;
    }

    let mut current = json;

    for key in &path[..path.len() - 1] {
        let Some(next) = current.get_mut(*key) else {
            return;
        };
        current = next;
    }

    if let Some(object) = current.as_object_mut() {
        object.remove(path[path.len() - 1]);
    }
}

pub(super) fn set_path_value(json: &mut Value, path: &[&str], value: Value) -> Result<(), String> {
    if path.is_empty() {
        return Err("Cannot write an empty settings path".to_string());
    }

    let mut current = json;
    let mut traversed_path = Vec::new();

    for key in &path[..path.len() - 1] {
        if !current.is_object() {
            return Err(format!(
                "Cannot write `{}` because `{}` is {} instead of an object.",
                path.join("."),
                if traversed_path.is_empty() {
                    "<root>".to_string()
                } else {
                    traversed_path.join(".")
                },
                value_kind(current)
            ));
        }

        traversed_path.push(*key);
        let next = current
            .as_object_mut()
            .expect("value shape already checked")
            .entry((*key).to_string())
            .or_insert_with(|| Value::Object(Map::new()));

        if !next.is_object() {
            return Err(format!(
                "Cannot write `{}` because `{}` is {} instead of an object.",
                path.join("."),
                traversed_path.join("."),
                value_kind(next)
            ));
        }

        current = next;
    }

    let leaf = path.last().expect("path emptiness checked above");

    if !current.is_object() {
        return Err(format!(
            "Cannot write `{}` because `{}` is {} instead of an object.",
            path.join("."),
            if traversed_path.is_empty() {
                "<root>".to_string()
            } else {
                traversed_path.join(".")
            },
            value_kind(current)
        ));
    }

    current
        .as_object_mut()
        .expect("value shape already checked")
        .insert((*leaf).to_string(), value);

    Ok(())
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}
