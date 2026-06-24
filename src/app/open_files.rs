#[cfg(target_os = "macos")]
use crate::types::AppState;

#[cfg(target_os = "macos")]
use tauri::{Emitter, Manager, RunEvent};

#[cfg(target_os = "macos")]
pub(super) const SETTINGS_FILE_OPENED_EVENT: &str = "settings-file-opened";

pub(super) fn initial_open_path_from_args() -> Option<String> {
    std::env::args_os()
        .nth(1)
        .map(|value| value.to_string_lossy().into_owned())
}

#[cfg(target_os = "macos")]
fn first_file_path_from_urls(urls: &[tauri::Url]) -> Option<String> {
    urls.iter().find_map(|url| {
        if url.scheme() == "file" {
            url.to_file_path()
                .ok()
                .map(|path| path.to_string_lossy().into_owned())
        } else {
            None
        }
    })
}

#[cfg(target_os = "macos")]
pub(super) fn handle_run_event<R: tauri::Runtime>(app: &tauri::AppHandle<R>, event: RunEvent) {
    if let RunEvent::Opened { urls } = event
        && let Some(path) = first_file_path_from_urls(&urls)
    {
        if let Some(state) = app.try_state::<AppState>()
            && let Ok(mut pending) = state.pending_open_path.lock()
        {
            *pending = Some(path.clone());
        }
        let _ = app.emit(SETTINGS_FILE_OPENED_EVENT, path);
    }
}
