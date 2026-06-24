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

    Ok(())
}
