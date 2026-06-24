use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::{ffi::CString, os::unix::ffi::OsStringExt};

pub(crate) fn expand_settings_path(path: &str) -> Result<PathBuf, String> {
    if path == "~" {
        return home_dir_for_current_user()
            .ok_or_else(|| "Could not resolve current user's home directory".to_string());
    }

    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir_for_current_user()
            .map(|home| home.join(rest))
            .ok_or_else(|| "Could not resolve current user's home directory".to_string());
    }

    if let Some(rest) = path.strip_prefix('~')
        && let Some((user, suffix)) = rest.split_once('/')
    {
        return home_dir_for_named_user(user)
            .map(|home| home.join(suffix))
            .ok_or_else(|| format!("Could not resolve home directory for user `{user}`"));
    }

    Ok(Path::new(path).to_path_buf())
}

pub(crate) fn home_dir_for_current_user() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(unix)]
fn home_dir_for_named_user(user: &str) -> Option<PathBuf> {
    let user = CString::new(user).ok()?;
    let passwd = unsafe { libc::getpwnam(user.as_ptr()) };

    if passwd.is_null() {
        return None;
    }

    let dir = unsafe { (*passwd).pw_dir };

    if dir.is_null() {
        return None;
    }

    let bytes = unsafe { std::ffi::CStr::from_ptr(dir) }.to_bytes().to_vec();
    Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}

#[cfg(not(unix))]
fn home_dir_for_named_user(_user: &str) -> Option<PathBuf> {
    None
}
