use crate::types::{LayoutDensity, ThemeMode};
use tauri::AppHandle;

#[cfg(any(target_os = "macos", target_os = "linux"))]
use tauri::Emitter;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use tauri::menu::{
    CheckMenuItem, Menu, MenuEvent, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu,
};

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) const APP_MENU_COMMAND_EVENT: &str = "app-menu-command";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const MENU_ITEM_SAVE: &str = "save";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const MENU_ITEM_SAVE_AS: &str = "save-as";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const MENU_ITEM_RELOAD: &str = "reload";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const MENU_ITEM_CLOSE_WINDOW: &str = "close-window";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const MENU_ITEM_THEME_SYSTEM: &str = "theme-system";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const MENU_ITEM_THEME_LIGHT: &str = "theme-light";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const MENU_ITEM_THEME_DARK: &str = "theme-dark";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const MENU_ITEM_DENSITY_COMPACT: &str = "density-compact";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const MENU_ITEM_DENSITY_COMFORTABLE: &str = "density-comfortable";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const MENU_ITEM_DENSITY_SPACIOUS: &str = "density-spacious";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const MENU_ITEM_QUIT: &str = "quit";

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn set_check_item_checked<R: tauri::Runtime>(
    item: &CheckMenuItem<R>,
    checked: bool,
) -> Result<(), String> {
    item.set_checked(checked)
        .map_err(|error| format!("Failed to update menu check state: {error}"))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn set_menu_item_enabled<R: tauri::Runtime>(
    item: &MenuItem<R>,
    enabled: bool,
) -> Result<(), String> {
    item.set_enabled(enabled)
        .map_err(|error| format!("Failed to update menu item enabled state: {error}"))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn update_check_item_in_menu_items<R: tauri::Runtime>(
    items: &[MenuItemKind<R>],
    target_id: &str,
    checked: bool,
) -> Result<bool, String> {
    for item in items {
        if item.id().as_ref() == target_id {
            if let Some(check_item) = item.as_check_menuitem() {
                set_check_item_checked(check_item, checked)?;
                return Ok(true);
            }

            return Err(format!("Menu item '{target_id}' is not a check menu item"));
        }

        if let Some(submenu) = item.as_submenu() {
            let submenu_items = submenu
                .items()
                .map_err(|error| format!("Failed to inspect submenu items: {error}"))?;
            if update_check_item_in_menu_items(&submenu_items, target_id, checked)? {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn update_menu_item_enabled_in_menu_items<R: tauri::Runtime>(
    items: &[MenuItemKind<R>],
    target_id: &str,
    enabled: bool,
) -> Result<bool, String> {
    for item in items {
        if item.id().as_ref() == target_id {
            if let Some(menu_item) = item.as_menuitem() {
                set_menu_item_enabled(menu_item, enabled)?;
                return Ok(true);
            }

            return Err(format!(
                "Menu item '{target_id}' is not a regular menu item"
            ));
        }

        if let Some(submenu) = item.as_submenu() {
            let submenu_items = submenu
                .items()
                .map_err(|error| format!("Failed to inspect submenu items: {error}"))?;
            if update_menu_item_enabled_in_menu_items(&submenu_items, target_id, enabled)? {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn set_menu_check_item<R: tauri::Runtime>(
    app: &AppHandle<R>,
    item_id: &str,
    checked: bool,
) -> Result<(), String> {
    let Some(menu) = app.menu() else {
        return Ok(());
    };

    let items = menu
        .items()
        .map_err(|error| format!("Failed to inspect app menu items: {error}"))?;
    if update_check_item_in_menu_items(&items, item_id, checked)? {
        Ok(())
    } else {
        Err(format!("Menu item '{item_id}' not found"))
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn set_menu_item_enabled_by_id<R: tauri::Runtime>(
    app: &AppHandle<R>,
    item_id: &str,
    enabled: bool,
) -> Result<(), String> {
    let Some(menu) = app.menu() else {
        return Ok(());
    };

    let items = menu
        .items()
        .map_err(|error| format!("Failed to inspect app menu items: {error}"))?;
    if update_menu_item_enabled_in_menu_items(&items, item_id, enabled)? {
        Ok(())
    } else {
        Err(format!("Menu item '{item_id}' not found"))
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) fn sync_menu_ui_state_inner<R: tauri::Runtime>(
    app: &AppHandle<R>,
    layout_density: LayoutDensity,
    theme_mode: ThemeMode,
    can_save: bool,
) -> Result<(), String> {
    for (item_id, checked) in [
        (MENU_ITEM_THEME_SYSTEM, theme_mode == ThemeMode::System),
        (MENU_ITEM_THEME_LIGHT, theme_mode == ThemeMode::Light),
        (MENU_ITEM_THEME_DARK, theme_mode == ThemeMode::Dark),
        (
            MENU_ITEM_DENSITY_COMPACT,
            layout_density == LayoutDensity::Compact,
        ),
        (
            MENU_ITEM_DENSITY_COMFORTABLE,
            layout_density == LayoutDensity::Comfortable,
        ),
        (
            MENU_ITEM_DENSITY_SPACIOUS,
            layout_density == LayoutDensity::Spacious,
        ),
    ] {
        set_menu_check_item(app, item_id, checked)?;
    }

    set_menu_item_enabled_by_id(app, MENU_ITEM_SAVE, can_save)?;

    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn build_file_menu<R: tauri::Runtime>(
    app: &AppHandle<R>,
    include_quit: bool,
) -> tauri::Result<Submenu<R>> {
    let save = MenuItem::with_id(app, MENU_ITEM_SAVE, "Save", true, Some("CmdOrCtrl+S"))?;
    let save_as = MenuItem::with_id(
        app,
        MENU_ITEM_SAVE_AS,
        "Save As…",
        true,
        Some("CmdOrCtrl+Shift+S"),
    )?;
    let reload = MenuItem::with_id(app, MENU_ITEM_RELOAD, "Reload", true, Some("CmdOrCtrl+R"))?;
    let close_window = MenuItem::with_id(
        app,
        MENU_ITEM_CLOSE_WINDOW,
        "Close Window",
        true,
        Some("CmdOrCtrl+W"),
    )?;
    let separator_after_close = PredefinedMenuItem::separator(app)?;
    let separator_before_quit = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, MENU_ITEM_QUIT, "Quit", true, Some("CmdOrCtrl+Q"))?;

    let mut items: Vec<&dyn tauri::menu::IsMenuItem<R>> = vec![
        &close_window,
        &separator_after_close,
        &save,
        &save_as,
        &reload,
    ];
    if include_quit {
        items.push(&separator_before_quit);
        items.push(&quit);
    }

    Submenu::with_items(app, "File", true, &items)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn build_theme_menu<R: tauri::Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let system = CheckMenuItem::with_id(
        app,
        MENU_ITEM_THEME_SYSTEM,
        "System",
        true,
        true,
        None::<&str>,
    )?;
    let light = CheckMenuItem::with_id(
        app,
        MENU_ITEM_THEME_LIGHT,
        "Light",
        true,
        false,
        None::<&str>,
    )?;
    let dark =
        CheckMenuItem::with_id(app, MENU_ITEM_THEME_DARK, "Dark", true, false, None::<&str>)?;

    Submenu::with_items(app, "Theme", true, &[&system, &light, &dark])
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn build_edit_menu<R: tauri::Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let undo = PredefinedMenuItem::undo(app, None)?;
    let redo = PredefinedMenuItem::redo(app, None)?;
    let separator_one = PredefinedMenuItem::separator(app)?;
    let cut = PredefinedMenuItem::cut(app, None)?;
    let copy = PredefinedMenuItem::copy(app, None)?;
    let paste = PredefinedMenuItem::paste(app, None)?;
    let select_all = PredefinedMenuItem::select_all(app, None)?;

    Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &undo,
            &redo,
            &separator_one,
            &cut,
            &copy,
            &paste,
            &select_all,
        ],
    )
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn build_density_menu<R: tauri::Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let compact = CheckMenuItem::with_id(
        app,
        MENU_ITEM_DENSITY_COMPACT,
        "Compact",
        true,
        true,
        None::<&str>,
    )?;
    let comfortable = CheckMenuItem::with_id(
        app,
        MENU_ITEM_DENSITY_COMFORTABLE,
        "Comfortable",
        true,
        false,
        None::<&str>,
    )?;
    let spacious = CheckMenuItem::with_id(
        app,
        MENU_ITEM_DENSITY_SPACIOUS,
        "Spacious",
        true,
        false,
        None::<&str>,
    )?;

    Submenu::with_items(app, "Density", true, &[&compact, &comfortable, &spacious])
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) fn build_app_menu<R: tauri::Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let file_menu = build_file_menu(app, cfg!(target_os = "linux"))?;
    let edit_menu = build_edit_menu(app)?;
    let theme_menu = build_theme_menu(app)?;
    let density_menu = build_density_menu(app)?;
    let view_menu = Submenu::with_items(app, "View", true, &[&theme_menu, &density_menu])?;

    #[cfg(target_os = "macos")]
    {
        let services = PredefinedMenuItem::services(app, None)?;
        let separator_one = PredefinedMenuItem::separator(app)?;
        let hide = PredefinedMenuItem::hide(app, None)?;
        let hide_others = PredefinedMenuItem::hide_others(app, None)?;
        let separator_two = PredefinedMenuItem::separator(app)?;
        let quit = MenuItem::with_id(app, MENU_ITEM_QUIT, "Quit", true, Some("CmdOrCtrl+Q"))?;
        let app_menu = Submenu::with_items(
            app,
            app.package_info().name.clone(),
            true,
            &[
                &services,
                &separator_one,
                &hide,
                &hide_others,
                &separator_two,
                &quit,
            ],
        )?;

        return Menu::with_items(app, &[&app_menu, &file_menu, &edit_menu, &view_menu]);
    }

    #[cfg(target_os = "linux")]
    {
        return Menu::with_items(app, &[&file_menu, &edit_menu, &view_menu]);
    }

    #[allow(unreachable_code)]
    Menu::new(app)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) fn handle_app_menu_event<R: tauri::Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let command = match event.id().as_ref() {
        MENU_ITEM_SAVE => Some(MENU_ITEM_SAVE),
        MENU_ITEM_SAVE_AS => Some(MENU_ITEM_SAVE_AS),
        MENU_ITEM_RELOAD => Some(MENU_ITEM_RELOAD),
        MENU_ITEM_CLOSE_WINDOW => Some(MENU_ITEM_CLOSE_WINDOW),
        MENU_ITEM_THEME_SYSTEM => Some(MENU_ITEM_THEME_SYSTEM),
        MENU_ITEM_THEME_LIGHT => Some(MENU_ITEM_THEME_LIGHT),
        MENU_ITEM_THEME_DARK => Some(MENU_ITEM_THEME_DARK),
        MENU_ITEM_DENSITY_COMPACT => Some(MENU_ITEM_DENSITY_COMPACT),
        MENU_ITEM_DENSITY_COMFORTABLE => Some(MENU_ITEM_DENSITY_COMFORTABLE),
        MENU_ITEM_DENSITY_SPACIOUS => Some(MENU_ITEM_DENSITY_SPACIOUS),
        MENU_ITEM_QUIT => Some(MENU_ITEM_QUIT),
        _ => None,
    };

    if let Some(command) = command {
        let _ = app.emit(APP_MENU_COMMAND_EVENT, command);
    }
}
