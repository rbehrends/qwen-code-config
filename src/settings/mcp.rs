use crate::types::{EnvironmentVariable, McpServerEntry, McpTransport};
use serde_json::{Map, Number, Value};
use std::collections::{BTreeMap, BTreeSet};

use super::json::value_as_u64;

pub(super) fn get_mcp_servers(json: &Value, warnings: &mut Vec<String>) -> Vec<McpServerEntry> {
    let Some(mcp_servers) = json.get("mcpServers") else {
        return Vec::new();
    };
    let Some(mcp_servers) = mcp_servers.as_object() else {
        warnings.push(
            "MCP: `mcpServers` is not a JSON object. It is preserved in the file but hidden from the structured editor."
                .to_string(),
        );
        return Vec::new();
    };

    let excluded_servers = excluded_server_names(json, warnings);
    let mut servers = Vec::new();

    for (index, (name, value)) in mcp_servers.iter().enumerate() {
        match extract_mcp_server(name, value, index, excluded_servers.contains(name)) {
            Ok(server) => servers.push(server),
            Err(warning) => warnings.push(warning),
        }
    }

    servers
}

pub(super) fn collect_mcp_warnings(json: &Value, servers: &[McpServerEntry]) -> Vec<String> {
    let mut warnings: Vec<String> = servers
        .iter()
        .filter_map(|server| {
            let hidden_keys = preserved_server_keys(server);
            (!hidden_keys.is_empty()).then(|| {
                format!(
                    "MCP: Server `{}` has preserved fields hidden from the structured editor: {}.",
                    server.name,
                    hidden_keys.join(", ")
                )
            })
        })
        .collect();

    if mcp_allowed_exists(json) {
        warnings.push(
            "MCP: Global allow-list rules exist in `mcp.allowed`. Per-server enabled state may not reflect the full effective policy."
                .to_string(),
        );
    }

    warnings
}

pub(super) fn apply_mcp_servers(
    json: &mut Value,
    mcp_servers: &[McpServerEntry],
) -> Result<(), String> {
    let root = json
        .as_object_mut()
        .ok_or_else(|| "Settings root must be a JSON object".to_string())?;

    let mut seen_names = BTreeSet::new();
    let mut saved_servers = Map::new();
    let mut excluded_visible_servers = BTreeSet::new();

    for server in mcp_servers {
        let name = server.name.trim();
        if name.is_empty() {
            return Err("MCP server names cannot be empty.".to_string());
        }
        if !seen_names.insert(name.to_string()) {
            return Err(format!("MCP server `{name}` is duplicated."));
        }
        if saved_servers.contains_key(name) {
            return Err(format!("MCP server `{name}` is duplicated."));
        }
        if !server.enabled {
            excluded_visible_servers.insert(name.to_string());
        }
        saved_servers.insert(name.to_string(), Value::Object(patch_mcp_server(server)?));
    }

    let preserved_servers = match root.get("mcpServers") {
        Some(Value::Object(existing)) => existing
            .iter()
            .filter_map(|(name, value)| {
                extract_mcp_server(name, value, 0, false)
                    .err()
                    .map(|_| (name.clone(), value.clone()))
            })
            .collect::<Map<String, Value>>(),
        Some(_) if saved_servers.is_empty() => return Ok(()),
        Some(_) => {
            return Err(
                "Cannot save structured MCP servers because `mcpServers` is not a JSON object."
                    .to_string(),
            );
        }
        None => Map::new(),
    };

    for name in preserved_servers.keys() {
        if saved_servers.contains_key(name) {
            return Err(format!(
                "Cannot save MCP server `{name}` because an unsupported or malformed saved entry with the same name is hidden from the structured editor."
            ));
        }
    }

    let mut merged = preserved_servers;
    merged.extend(saved_servers);

    if merged.is_empty() {
        root.remove("mcpServers");
    } else {
        root.insert("mcpServers".to_string(), Value::Object(merged));
    }

    apply_mcp_excluded(root, mcp_servers, excluded_visible_servers)?;

    Ok(())
}

fn extract_mcp_server(
    name: &str,
    value: &Value,
    index: usize,
    is_excluded: bool,
) -> Result<McpServerEntry, String> {
    let object = value.as_object().ok_or_else(|| {
        format!(
            "MCP: Ignored non-object entry in `mcpServers.{name}`. It is preserved in JSON but hidden from the structured editor."
        )
    })?;

    let stdio_command = object.get("command").and_then(Value::as_str);
    let http_url = object.get("httpUrl").and_then(Value::as_str);
    let sse_url = object.get("url").and_then(Value::as_str);

    let transport_count = usize::from(stdio_command.is_some())
        + usize::from(http_url.is_some())
        + usize::from(sse_url.is_some());

    if transport_count == 0 {
        return Err(format!(
            "MCP: Server `{name}` does not have a supported transport (`command`, `httpUrl`, or `url`) and is preserved in JSON but hidden from the structured editor."
        ));
    }

    if transport_count > 1 {
        return Err(format!(
            "MCP: Server `{name}` has multiple transports configured and is preserved in JSON but hidden from the structured editor."
        ));
    }

    let timeout = match object.get("timeout") {
        Some(value) => Some(value_as_u64(value).ok_or_else(|| {
            format!(
                "MCP: Server `{name}` has a non-numeric `timeout` and is preserved in JSON but hidden from the structured editor."
            )
        })?),
        None => None,
    };

    let (transport, command, args, cwd, env_vars, url, headers) =
        if let Some(command) = stdio_command {
            let args = get_string_array(object, "args").map_err(|_| {
                format!(
                    "MCP: Server `{name}` has a non-string `args` entry and is preserved in JSON but hidden from the structured editor."
                )
            })?;
            let cwd = get_optional_string(object, "cwd").map_err(|_| {
                format!(
                    "MCP: Server `{name}` has a non-string `cwd` and is preserved in JSON but hidden from the structured editor."
                )
            })?;
            let env_vars = get_string_object_entries(object, "env").map_err(|_| {
                format!(
                    "MCP: Server `{name}` has a non-string `env` entry and is preserved in JSON but hidden from the structured editor."
                )
            })?;
            (
                McpTransport::Stdio,
                command.to_string(),
                args,
                cwd.unwrap_or_default(),
                env_vars,
                String::new(),
                Vec::new(),
            )
        } else if let Some(url) = http_url {
            let headers = get_string_object_entries(object, "headers").map_err(|_| {
                format!(
                    "MCP: Server `{name}` has a non-string `headers` entry and is preserved in JSON but hidden from the structured editor."
                )
            })?;
            (
                McpTransport::Http,
                String::new(),
                Vec::new(),
                String::new(),
                Vec::new(),
                url.to_string(),
                headers,
            )
        } else {
            let url = sse_url.expect("transport count already checked");
            let headers = get_string_object_entries(object, "headers").map_err(|_| {
                format!(
                    "MCP: Server `{name}` has a non-string `headers` entry and is preserved in JSON but hidden from the structured editor."
                )
            })?;
            (
                McpTransport::Sse,
                String::new(),
                Vec::new(),
                String::new(),
                Vec::new(),
                url.to_string(),
                headers,
            )
        };

    Ok(McpServerEntry {
        ui_id: format!("saved-mcp-{index}-{name}"),
        name: name.to_string(),
        enabled: !is_excluded,
        transport,
        command,
        args,
        cwd,
        env_vars,
        url,
        headers,
        timeout,
        raw_server: value.clone(),
    })
}

fn excluded_server_names(json: &Value, warnings: &mut Vec<String>) -> BTreeSet<String> {
    let Some(root) = json.as_object() else {
        return BTreeSet::new();
    };

    let Some(mcp) = root.get("mcp") else {
        return BTreeSet::new();
    };

    let Some(mcp_object) = mcp.as_object() else {
        warnings.push(
            "MCP: `mcp` is not a JSON object. It is preserved in the file but hidden from the structured editor."
                .to_string(),
        );
        return BTreeSet::new();
    };

    let Some(excluded) = mcp_object.get("excluded") else {
        return BTreeSet::new();
    };

    let Some(excluded_array) = excluded.as_array() else {
        warnings.push(
            "MCP: `mcp.excluded` is not an array of strings. It is preserved in JSON but the structured enable/disable state may be incomplete."
                .to_string(),
        );
        return BTreeSet::new();
    };

    let mut excluded_names = BTreeSet::new();
    for value in excluded_array {
        let Some(name) = value.as_str() else {
            warnings.push(
                "MCP: `mcp.excluded` contains non-string entries. They are preserved in JSON but ignored by the structured editor."
                    .to_string(),
            );
            return BTreeSet::new();
        };
        excluded_names.insert(name.to_string());
    }

    excluded_names
}

fn mcp_allowed_exists(json: &Value) -> bool {
    json.get("mcp")
        .and_then(Value::as_object)
        .and_then(|mcp| mcp.get("allowed"))
        .is_some()
}

fn apply_mcp_excluded(
    root: &mut Map<String, Value>,
    mcp_servers: &[McpServerEntry],
    excluded_visible_servers: BTreeSet<String>,
) -> Result<(), String> {
    let mut preserved_excluded = BTreeSet::new();

    match root.get("mcp") {
        Some(Value::Object(existing_mcp)) => {
            if let Some(Value::Array(existing_excluded)) = existing_mcp.get("excluded") {
                for value in existing_excluded {
                    if let Some(name) = value.as_str()
                        && mcp_servers.iter().all(|server| server.name.trim() != name)
                    {
                        preserved_excluded.insert(name.to_string());
                    }
                }
            } else if existing_mcp.get("excluded").is_some() && excluded_visible_servers.is_empty() {
            } else if existing_mcp.get("excluded").is_some() {
                return Err(
                    "Cannot save MCP enabled state because `mcp.excluded` is not an array of strings."
                        .to_string(),
                );
            }
        }
        Some(_) if excluded_visible_servers.is_empty() => return Ok(()),
        Some(_) => {
            return Err("Cannot save MCP enabled state because `mcp` is not a JSON object.".to_string());
        }
        None => {}
    }

    let mut next_excluded = preserved_excluded;
    next_excluded.extend(excluded_visible_servers);

    if next_excluded.is_empty() {
        if let Some(Value::Object(mcp_object)) = root.get_mut("mcp") {
            mcp_object.remove("excluded");
            if mcp_object.is_empty() {
                root.remove("mcp");
            }
        }
        return Ok(());
    }

    let mcp_object = match root.get_mut("mcp") {
        Some(Value::Object(mcp)) => mcp,
        None => root
            .entry("mcp".to_string())
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .expect("value was just inserted as an object"),
        Some(_) => unreachable!("non-object case returned above"),
    };

    mcp_object.insert(
        "excluded".to_string(),
        Value::Array(
            next_excluded
                .into_iter()
                .map(Value::String)
                .collect::<Vec<_>>(),
        ),
    );

    Ok(())
}

fn patch_mcp_server(server: &McpServerEntry) -> Result<Map<String, Value>, String> {
    let mut object = server.raw_server.as_object().cloned().unwrap_or_default();

    match server.transport {
        McpTransport::Stdio => {
            let command = server.command.trim();
            if command.is_empty() {
                return Err(format!(
                    "MCP server `{}` must have a command for stdio transport.",
                    server.name.trim()
                ));
            }
            object.insert("command".to_string(), Value::String(command.to_string()));
            patch_string_array_field(&mut object, "args", &server.args);
            patch_optional_string_field(&mut object, "cwd", &server.cwd);
            patch_string_object_field(
                &mut object,
                "env",
                &server.env_vars,
                "environment variable",
                true,
            )?;
            object.remove("httpUrl");
            object.remove("url");
            object.remove("headers");
        }
        McpTransport::Http => {
            let url = server.url.trim();
            if url.is_empty() {
                return Err(format!(
                    "MCP server `{}` must have an HTTP URL for http transport.",
                    server.name.trim()
                ));
            }
            object.insert("httpUrl".to_string(), Value::String(url.to_string()));
            patch_string_object_field(&mut object, "headers", &server.headers, "header", false)?;
            object.remove("command");
            object.remove("args");
            object.remove("cwd");
            object.remove("env");
            object.remove("url");
        }
        McpTransport::Sse => {
            let url = server.url.trim();
            if url.is_empty() {
                return Err(format!(
                    "MCP server `{}` must have an SSE URL for sse transport.",
                    server.name.trim()
                ));
            }
            object.insert("url".to_string(), Value::String(url.to_string()));
            patch_string_object_field(&mut object, "headers", &server.headers, "header", false)?;
            object.remove("command");
            object.remove("args");
            object.remove("cwd");
            object.remove("env");
            object.remove("httpUrl");
        }
    }

    if let Some(timeout) = server.timeout {
        object.insert("timeout".to_string(), Value::Number(Number::from(timeout)));
    } else {
        object.remove("timeout");
    }

    Ok(object)
}

fn get_string_array(object: &Map<String, Value>, key: &str) -> Result<Vec<String>, ()> {
    let Some(value) = object.get(key) else {
        return Ok(Vec::new());
    };
    let Some(array) = value.as_array() else {
        return Err(());
    };

    array
        .iter()
        .map(|entry| entry.as_str().map(ToString::to_string).ok_or(()))
        .collect()
}

fn get_optional_string(object: &Map<String, Value>, key: &str) -> Result<Option<String>, ()> {
    match object.get(key) {
        Some(value) => value.as_str().map(|text| Some(text.to_string())).ok_or(()),
        None => Ok(None),
    }
}

fn get_string_object_entries(
    object: &Map<String, Value>,
    key: &str,
) -> Result<Vec<EnvironmentVariable>, ()> {
    let Some(value) = object.get(key) else {
        return Ok(Vec::new());
    };
    let Some(map) = value.as_object() else {
        return Err(());
    };

    let mut entries: Vec<_> = map
        .iter()
        .map(|(entry_key, entry_value)| {
            entry_value
                .as_str()
                .map(|entry_value| EnvironmentVariable {
                    key: entry_key.clone(),
                    value: entry_value.to_string(),
                })
                .ok_or(())
        })
        .collect::<Result<Vec<_>, _>>()?;

    entries.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(entries)
}

fn patch_string_array_field(object: &mut Map<String, Value>, key: &str, values: &[String]) {
    let normalized: Vec<_> = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| Value::String(value.to_string()))
        .collect();

    if normalized.is_empty() {
        object.remove(key);
    } else {
        object.insert(key.to_string(), Value::Array(normalized));
    }
}

fn patch_optional_string_field(object: &mut Map<String, Value>, key: &str, value: &str) {
    let normalized = value.trim();
    if normalized.is_empty() {
        object.remove(key);
    } else {
        object.insert(key.to_string(), Value::String(normalized.to_string()));
    }
}

fn patch_string_object_field(
    object: &mut Map<String, Value>,
    key: &str,
    entries: &[EnvironmentVariable],
    entry_label: &str,
    reject_equals: bool,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    let mut saved = BTreeMap::new();

    for entry in entries {
        let entry_key = entry.key.trim();
        if entry_key.is_empty() {
            return Err(format!("MCP {entry_label} names cannot be empty."));
        }
        if reject_equals && entry_key.contains('=') {
            return Err(format!(
                "MCP environment variable `{entry_key}` is invalid because names cannot contain `=`."
            ));
        }
        if !seen.insert(entry_key.to_string()) {
            return Err(format!("MCP {entry_label} `{entry_key}` is duplicated."));
        }
        saved.insert(entry_key.to_string(), Value::String(entry.value.clone()));
    }

    if saved.is_empty() {
        object.remove(key);
    } else {
        object.insert(
            key.to_string(),
            Value::Object(saved.into_iter().collect::<Map<String, Value>>()),
        );
    }

    Ok(())
}

fn preserved_server_keys(server: &McpServerEntry) -> Vec<String> {
    let Some(object) = server.raw_server.as_object() else {
        return Vec::new();
    };

    let known_keys: &[&str] = match server.transport {
        McpTransport::Stdio => &["command", "args", "cwd", "env", "timeout"],
        McpTransport::Http => &["httpUrl", "headers", "timeout"],
        McpTransport::Sse => &["url", "headers", "timeout"],
    };

    object
        .keys()
        .filter(|key| !known_keys.contains(&key.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn get_mcp_servers_extracts_supported_entries() {
        let json = json!({
            "mcpServers": {
                "stdioServer": {
                    "command": "node",
                    "args": ["dist/server.js"],
                    "cwd": "./tools",
                    "env": {
                        "API_KEY": "secret"
                    },
                    "trust": true
                },
                "httpServer": {
                    "httpUrl": "http://localhost:3000/mcp",
                    "headers": {
                        "Authorization": "Bearer token"
                    },
                    "timeout": 5000
                }
            }
        });

        let mut warnings = Vec::new();
        let servers = get_mcp_servers(&json, &mut warnings);

        assert_eq!(servers.len(), 2);
        assert!(warnings.is_empty());
        assert_eq!(servers[0].name, "httpServer");
        assert_eq!(servers[1].name, "stdioServer");
        assert!(servers.iter().all(|server| server.enabled));
    }

    #[test]
    fn get_mcp_servers_hides_malformed_entries() {
        let json = json!({
            "mcpServers": {
                "broken": {
                    "command": "node",
                    "args": [true]
                }
            }
        });

        let mut warnings = Vec::new();
        let servers = get_mcp_servers(&json, &mut warnings);

        assert!(servers.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("non-string `args`"));
    }

    #[test]
    fn apply_mcp_servers_preserves_hidden_saved_entries() {
        let mut json = json!({
            "mcpServers": {
                "broken": {
                    "command": "node",
                    "args": [true]
                },
                "saved": {
                    "url": "https://example.com/sse"
                }
            }
        });

        apply_mcp_servers(
            &mut json,
            &[McpServerEntry {
                ui_id: "saved".to_string(),
                name: "saved".to_string(),
                enabled: false,
                transport: McpTransport::Http,
                command: String::new(),
                args: Vec::new(),
                cwd: String::new(),
                env_vars: Vec::new(),
                url: "https://example.com/mcp".to_string(),
                headers: vec![EnvironmentVariable {
                    key: "Authorization".to_string(),
                    value: "Bearer token".to_string(),
                }],
                timeout: Some(5000),
                raw_server: json!({
                    "url": "https://example.com/sse",
                    "trust": true
                }),
            }],
        )
        .unwrap();

        assert!(json["mcpServers"]["broken"].is_object());
        assert_eq!(json["mcpServers"]["saved"]["httpUrl"], "https://example.com/mcp");
        assert_eq!(json["mcpServers"]["saved"]["trust"], true);
        assert!(json["mcpServers"]["saved"].get("url").is_none());
        assert_eq!(json["mcp"]["excluded"][0], "saved");
    }

    #[test]
    fn get_mcp_servers_marks_excluded_servers_disabled() {
        let json = json!({
            "mcp": {
                "excluded": ["disabled server"]
            },
            "mcpServers": {
                "disabled server": {
                    "command": "node"
                },
                "enabled server": {
                    "command": "python"
                }
            }
        });

        let mut warnings = Vec::new();
        let servers = get_mcp_servers(&json, &mut warnings);

        assert!(warnings.is_empty());
        assert_eq!(servers.len(), 2);
        assert!(!servers.iter().find(|server| server.name == "disabled server").unwrap().enabled);
        assert!(servers.iter().find(|server| server.name == "enabled server").unwrap().enabled);
    }
}
