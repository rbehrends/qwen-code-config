use std::sync::Mutex;

#[derive(Default)]
pub(crate) struct AppState {
    pub(crate) pending_open_path: Mutex<Option<String>>,
}
