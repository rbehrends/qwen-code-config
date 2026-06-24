mod commands;
mod menu;
mod open_files;

use crate::types::AppState;
use std::sync::Mutex;

use open_files::initial_open_path_from_args;

pub(crate) fn run() {
    let builder = tauri::Builder::default()
        .manage(AppState {
            pending_open_path: Mutex::new(initial_open_path_from_args()),
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::load_ui_state,
            commands::save_ui_state,
            commands::sync_menu_ui_state,
            commands::load_settings,
            commands::save_options,
            commands::save_options_as,
            commands::fetch_provider_models,
            commands::build_model_draft_command,
            commands::get_model_name_capitalizations,
            commands::get_model_name_rewrite_rules,
            commands::preview_settings_command,
            commands::get_pending_open_path,
            commands::quit_app
        ]);

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let builder = builder
        .menu(menu::build_app_menu)
        .on_menu_event(menu::handle_app_menu_event);

    let app = builder
        .build(tauri::generate_context!())
        .expect("error while building Qwen Code Config");

    app.run(|app, event| {
        #[cfg(target_os = "macos")]
        open_files::handle_run_event(app, event);

        #[cfg(not(target_os = "macos"))]
        let _ = (app, event);
    });
}
