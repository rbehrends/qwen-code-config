use crate::types::ImportantOptions;
use serde_json::Value;

use super::json::{get_bool, set_bool};

pub(crate) fn apply_important_options(
    json: &mut Value,
    options: &ImportantOptions,
) -> Result<(), String> {
    set_bool_preserving_omitted_default(
        json,
        &["privacy", "usageStatisticsEnabled"],
        options.usage_statistics_enabled,
        true,
    )?;
    set_bool_preserving_omitted_default(
        json,
        &["telemetry", "enabled"],
        options.telemetry_enabled,
        false,
    )?;
    set_bool_preserving_omitted_default(
        json,
        &["general", "enableAutoUpdate"],
        options.enable_auto_update,
        true,
    )?;
    set_bool_preserving_omitted_default(
        json,
        &["security", "folderTrust", "enabled"],
        options.folder_trust_enabled,
        false,
    )?;

    Ok(())
}

fn set_bool_preserving_omitted_default(
    json: &mut Value,
    path: &[&str],
    value: bool,
    default: bool,
) -> Result<(), String> {
    if get_bool(json, path).is_some() || value != default {
        set_bool(json, path, value)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_important_options_does_not_materialize_omitted_defaults() {
        let mut json = serde_json::json!({});

        apply_important_options(&mut json, &ImportantOptions::default()).unwrap();

        assert_eq!(json, serde_json::json!({}));
    }

    #[test]
    fn apply_important_options_preserves_explicit_defaults() {
        let mut json = serde_json::json!({
            "privacy": {
                "usageStatisticsEnabled": true
            },
            "telemetry": {
                "enabled": false
            },
            "general": {
                "enableAutoUpdate": true
            },
            "security": {
                "folderTrust": {
                    "enabled": false
                }
            }
        });
        let expected = json.clone();

        apply_important_options(&mut json, &ImportantOptions::default()).unwrap();

        assert_eq!(json, expected);
    }

    #[test]
    fn apply_important_options_writes_folder_trust_without_removing_security_settings() {
        let mut json = serde_json::json!({
            "security": {
                "otherSecuritySetting": "preserve"
            }
        });
        let options = ImportantOptions {
            folder_trust_enabled: true,
            ..ImportantOptions::default()
        };

        apply_important_options(&mut json, &options).unwrap();

        assert_eq!(json["security"]["folderTrust"]["enabled"], true);
        assert_eq!(json["security"]["otherSecuritySetting"], "preserve");
    }
}
