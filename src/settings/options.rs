use crate::types::ImportantOptions;
use serde_json::Value;

use super::json::set_bool;

pub(crate) fn apply_important_options(
    json: &mut Value,
    options: &ImportantOptions,
) -> Result<(), String> {
    set_bool(
        json,
        &["privacy", "usageStatisticsEnabled"],
        options.usage_statistics_enabled,
    )?;
    set_bool(json, &["telemetry", "enabled"], options.telemetry_enabled)?;
    set_bool(
        json,
        &["general", "enableAutoUpdate"],
        options.enable_auto_update,
    )?;
    set_bool(
        json,
        &["security", "folderTrust", "enabled"],
        options.folder_trust_enabled,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
