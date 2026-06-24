use crate::{
    catalog::fetch_catalog_models,
    models::{build_model_draft, common_model_name_capitalizations, model_name_rewrite_rules},
    settings::{load_settings_snapshot, preview_settings, save_settings_to_path},
    types::{
        AppState, BuildModelDraftRequest, BuildModelDraftResult, FetchProviderModelsRequest,
        ModelNameRewriteRule, PendingOpenPathResponse, PreviewSettingsRequest,
        PreviewSettingsResult, ProviderFetchResult, SaveOptionsAsRequest, SaveOptionsRequest,
        SaveUiStateRequest, SyncMenuUiStateRequest, UiStateSnapshot,
    },
    ui_state::{load_ui_state_snapshot, save_ui_state_snapshot},
};
use std::collections::BTreeMap;
use tauri::{AppHandle, State};

use super::menu::sync_menu_ui_state_inner;

#[tauri::command]
pub(crate) fn load_settings(
    path: Option<String>,
) -> Result<crate::types::SettingsSnapshot, String> {
    load_settings_snapshot(path)
}

#[tauri::command]
pub(crate) fn load_ui_state() -> Result<UiStateSnapshot, String> {
    load_ui_state_snapshot()
}

#[tauri::command]
pub(crate) fn save_ui_state(
    app: AppHandle,
    request: SaveUiStateRequest,
) -> Result<UiStateSnapshot, String> {
    let snapshot = save_ui_state_snapshot(
        request.layout_density,
        request.theme_mode,
        request.custom_provider_profiles,
    )?;
    sync_menu_ui_state_inner(&app, snapshot.layout_density, snapshot.theme_mode, false)?;
    Ok(snapshot)
}

#[tauri::command]
pub(crate) fn sync_menu_ui_state(
    app: AppHandle,
    request: SyncMenuUiStateRequest,
) -> Result<(), String> {
    sync_menu_ui_state_inner(
        &app,
        request.layout_density,
        request.theme_mode,
        request.can_save,
    )
}

#[tauri::command]
pub(crate) fn save_options(
    request: SaveOptionsRequest,
) -> Result<crate::types::SettingsSnapshot, String> {
    save_settings_to_path(
        &request.path,
        &request.path,
        request.options,
        request.env_vars,
        request.models,
        request.mcp_servers,
        request.fast_model,
    )
}

#[tauri::command]
pub(crate) fn save_options_as(
    request: SaveOptionsAsRequest,
) -> Result<crate::types::SettingsSnapshot, String> {
    save_settings_to_path(
        &request.source_path,
        &request.target_path,
        request.options,
        request.env_vars,
        request.models,
        request.mcp_servers,
        request.fast_model,
    )
}

#[tauri::command]
pub(crate) fn fetch_provider_models(
    request: FetchProviderModelsRequest,
) -> Result<ProviderFetchResult, String> {
    let models = fetch_catalog_models(&request.provider_id)?;

    Ok(ProviderFetchResult {
        provider_id: request.provider_id,
        models,
    })
}

#[tauri::command]
pub(crate) fn build_model_draft_command(
    request: BuildModelDraftRequest,
) -> Result<BuildModelDraftResult, String> {
    Ok(BuildModelDraftResult {
        model: build_model_draft(request.profile, request.model, request.has_existing_default)?,
    })
}

#[tauri::command]
pub(crate) fn get_model_name_capitalizations() -> Result<BTreeMap<String, String>, String> {
    Ok(common_model_name_capitalizations())
}

#[tauri::command]
pub(crate) fn get_model_name_rewrite_rules() -> Result<Vec<ModelNameRewriteRule>, String> {
    Ok(model_name_rewrite_rules())
}

#[tauri::command]
pub(crate) fn preview_settings_command(
    request: PreviewSettingsRequest,
) -> Result<PreviewSettingsResult, String> {
    preview_settings(
        request.base_json,
        request.options,
        request.env_vars,
        request.models,
        request.mcp_servers,
        request.fast_model,
    )
}

#[tauri::command]
pub(crate) fn get_pending_open_path(
    state: State<'_, AppState>,
) -> Result<PendingOpenPathResponse, String> {
    let mut pending = state
        .pending_open_path
        .lock()
        .map_err(|_| "Failed to lock app state".to_string())?;
    Ok(PendingOpenPathResponse {
        path: pending.take(),
    })
}

#[tauri::command]
pub(crate) fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}
