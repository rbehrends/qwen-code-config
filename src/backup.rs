use crate::paths::home_dir_for_current_user;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const MIN_BACKUP_FILES_TO_KEEP: usize = 50;
const BACKUP_RETENTION_DAYS: u64 = 7;
const BACKUP_FILE_SUFFIX: &str = "-before-save.json";

pub(crate) fn write_backup_if_present(target_path: &Path) -> Result<Option<PathBuf>, String> {
    if !target_path.exists() {
        return Ok(None);
    }

    let backup_root = backup_root_dir()?;
    let backup_dir = backup_root.join(flattened_backup_key(target_path));
    fs::create_dir_all(&backup_dir).map_err(|error| {
        format!(
            "Failed to create backup directory {}: {error}",
            backup_dir.display()
        )
    })?;

    let backup_file = backup_dir.join(format!("{}{}", backup_timestamp(), BACKUP_FILE_SUFFIX));
    fs::copy(target_path, &backup_file).map_err(|error| {
        format!(
            "Failed to create backup {} from {}: {error}",
            backup_file.display(),
            target_path.display()
        )
    })?;

    Ok(Some(backup_file))
}

pub(crate) fn prune_backups_if_needed(target_path: &Path) {
    let backup_dir = match backup_root_dir() {
        Ok(root) => root.join(flattened_backup_key(target_path)),
        Err(_) => return,
    };

    let _ = prune_backup_dir(&backup_dir, SystemTime::now());
}

pub(crate) fn atomic_write_text(target_path: &Path, contents: &str) -> Result<(), String> {
    let target_path = resolve_write_target_path(target_path)?;
    let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Failed to create target directory {}: {error}",
            parent.display()
        )
    })?;

    let temp_path = unique_temp_path(&target_path);
    let mut temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|error| {
            format!(
                "Failed to create temp file {}: {error}",
                temp_path.display()
            )
        })?;

    #[cfg(unix)]
    set_temp_file_permissions(&temp_path, &target_path)?;

    if let Err(error) = temp_file.write_all(contents.as_bytes()) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "Failed to write temp file {}: {error}",
            temp_path.display()
        ));
    }
    if let Err(error) = temp_file.sync_all() {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "Failed to flush temp file {}: {error}",
            temp_path.display()
        ));
    }
    drop(temp_file);

    if let Err(error) = fs::rename(&temp_path, &target_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "Failed to replace {} with {}: {error}",
            target_path.display(),
            temp_path.display()
        ));
    }

    sync_parent_dir(parent)?;
    Ok(())
}

fn resolve_write_target_path(target_path: &Path) -> Result<PathBuf, String> {
    match fs::symlink_metadata(target_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            fs::canonicalize(target_path).map_err(|error| {
                format!(
                    "Failed to resolve symlink target {}: {error}",
                    target_path.display()
                )
            })
        }
        Ok(_) => Ok(target_path.to_path_buf()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(target_path.to_path_buf()),
        Err(error) => Err(format!(
            "Failed to inspect {} before saving: {error}",
            target_path.display()
        )),
    }
}

#[cfg(unix)]
fn set_temp_file_permissions(temp_path: &Path, target_path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mode = fs::metadata(target_path)
        .map(|metadata| metadata.permissions().mode())
        .unwrap_or(0o600);

    fs::set_permissions(temp_path, fs::Permissions::from_mode(mode)).map_err(|error| {
        format!(
            "Failed to set permissions on temp file {}: {error}",
            temp_path.display()
        )
    })
}

fn unique_temp_path(target_path: &Path) -> PathBuf {
    let file_name = target_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("settings.json");
    let stem = format!(".{file_name}.tmp-{}-{}", process::id(), backup_timestamp());
    target_path.with_file_name(stem)
}

fn backup_root_dir() -> Result<PathBuf, String> {
    Ok(app_storage_root_dir()?.join("backups"))
}

fn prune_backup_dir(backup_dir: &Path, now: SystemTime) -> Result<(), String> {
    let mut backups = list_backup_files(backup_dir)?;
    if backups.len() <= MIN_BACKUP_FILES_TO_KEEP {
        return Ok(());
    }

    backups.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| right.path.cmp(&left.path))
    });

    let retention_cutoff = now
        .checked_sub(Duration::from_secs(BACKUP_RETENTION_DAYS * 24 * 60 * 60))
        .unwrap_or(UNIX_EPOCH);

    for backup in backups.into_iter().skip(MIN_BACKUP_FILES_TO_KEEP) {
        if backup.modified >= retention_cutoff {
            continue;
        }

        fs::remove_file(&backup.path).map_err(|error| {
            format!(
                "Failed to remove expired backup {}: {error}",
                backup.path.display()
            )
        })?;
    }

    Ok(())
}

fn list_backup_files(backup_dir: &Path) -> Result<Vec<BackupFile>, String> {
    let mut backups = Vec::new();
    let entries = match fs::read_dir(backup_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(backups),
        Err(error) => {
            return Err(format!(
                "Failed to read backup directory {}: {error}",
                backup_dir.display()
            ));
        }
    };

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "Failed to read an entry in backup directory {}: {error}",
                backup_dir.display()
            )
        })?;
        let path = entry.path();
        if !is_backup_file(&path) {
            continue;
        }

        let metadata = entry
            .metadata()
            .map_err(|error| format!("Failed to read metadata for {}: {error}", path.display()))?;
        if !metadata.is_file() {
            continue;
        }

        backups.push(BackupFile {
            path,
            modified: metadata.modified().unwrap_or(UNIX_EPOCH),
        });
    }

    Ok(backups)
}

fn is_backup_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name.ends_with(BACKUP_FILE_SUFFIX))
}

#[derive(Debug)]
struct BackupFile {
    path: PathBuf,
    modified: SystemTime,
}

pub(crate) fn app_storage_root_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let home = home_dir_for_current_user()
            .ok_or_else(|| "Could not resolve current user's home directory".to_string())?;
        Ok(home
            .join("Library")
            .join("Application Support")
            .join("Qwen Code Config"))
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(state_home) = std::env::var_os("XDG_STATE_HOME") {
            return Ok(PathBuf::from(state_home).join("qwen-code-config"));
        }

        let home = home_dir_for_current_user()
            .ok_or_else(|| "Could not resolve current user's home directory".to_string())?;
        return Ok(home.join(".local").join("state").join("qwen-code-config"));
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let home = home_dir_for_current_user()
            .ok_or_else(|| "Could not resolve current user's home directory".to_string())?;
        Ok(home.join(".qwen-code-config"))
    }
}

pub(crate) fn flattened_backup_key(target_path: &Path) -> String {
    let absolute_path = absolutize_path(target_path);
    let key_path = backup_scope_path(&absolute_path);
    flatten_path_for_backup(&key_path)
}

fn backup_scope_path(target_path: &Path) -> PathBuf {
    let parent = target_path.parent().unwrap_or(target_path);
    if parent.file_name().and_then(|value| value.to_str()) == Some(".qwen") {
        parent
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| parent.to_path_buf())
    } else {
        parent.to_path_buf()
    }
}

pub(crate) fn flatten_path_for_backup(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let mut flattened = String::with_capacity(raw.len());

    for character in raw.chars() {
        match character {
            '/' | '\\' => flattened.push('-'),
            _ => flattened.push(character),
        }
    }

    if flattened.is_empty() {
        "-".to_string()
    } else {
        flattened
    }
}

fn absolutize_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    match std::env::current_dir() {
        Ok(current_dir) => current_dir.join(path),
        Err(_) => path.to_path_buf(),
    }
}

fn backup_timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    #[cfg(unix)]
    {
        let seconds = duration.as_secs() as libc::time_t;
        let millis = duration.subsec_millis();
        let mut local_time = std::mem::MaybeUninit::<libc::tm>::uninit();
        let mut buffer = [0u8; 32];

        unsafe {
            if !libc::localtime_r(&seconds, local_time.as_mut_ptr()).is_null() {
                let time = local_time.assume_init();
                let written = libc::strftime(
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    c"%Y%m%d-%H%M%S".as_ptr(),
                    &time,
                );
                if written > 0 {
                    let prefix = std::str::from_utf8(&buffer[..written]).unwrap_or("backup");
                    return format!("{prefix}-{millis:03}");
                }
            }
        }
    }

    format!("{}", duration.as_millis())
}

fn sync_parent_dir(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        let directory = File::open(path)
            .map_err(|error| format!("Failed to open {}: {error}", path.display()))?;
        directory
            .sync_all()
            .map_err(|error| format!("Failed to sync {}: {error}", path.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

    fn unique_test_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "qwenconf-backup-tests-{label}-{}-{}",
            process::id(),
            backup_timestamp()
        ))
    }

    #[test]
    fn flattened_backup_key_uses_project_root_for_dot_qwen_settings() {
        let path = Path::new("/home/tester/develop/util/.qwen/settings.json");
        assert_eq!(flattened_backup_key(path), "-home-tester-develop-util");
    }

    #[test]
    fn flattened_backup_key_uses_containing_directory_for_non_qwen_settings() {
        let path = Path::new("/home/tester/custom/settings.json");
        assert_eq!(flattened_backup_key(path), "-home-tester-custom");
    }

    #[test]
    fn flattened_backup_key_absolutizes_relative_paths() {
        let current_dir = std::env::current_dir().unwrap();
        let expected = flatten_path_for_backup(&current_dir);
        assert_eq!(
            flattened_backup_key(Path::new("qwen-settings.json")),
            expected
        );
    }

    #[test]
    fn backup_timestamp_is_human_readable() {
        let timestamp = backup_timestamp();

        #[cfg(unix)]
        {
            assert_eq!(timestamp.len(), 19);
            assert!(timestamp.chars().nth(8) == Some('-'));
            assert!(timestamp.chars().nth(15) == Some('-'));
            assert!(
                timestamp
                    .chars()
                    .enumerate()
                    .all(|(index, ch)| match index {
                        8 | 15 => ch == '-',
                        _ => ch.is_ascii_digit(),
                    })
            );
        }

        #[cfg(not(unix))]
        {
            assert!(!timestamp.is_empty());
        }
    }

    #[test]
    fn prune_backup_dir_keeps_newest_fifty_even_if_older_than_window() {
        let backup_dir = unique_test_dir("count-floor");
        fs::create_dir_all(&backup_dir).unwrap();

        for index in 0..(MIN_BACKUP_FILES_TO_KEEP + 5) {
            let path = backup_dir.join(format!("{index:03}{BACKUP_FILE_SUFFIX}"));
            fs::write(&path, format!("backup-{index}")).unwrap();
            thread::sleep(Duration::from_millis(2));
        }

        let now =
            SystemTime::now() + Duration::from_secs((BACKUP_RETENTION_DAYS + 1) * 24 * 60 * 60);
        prune_backup_dir(&backup_dir, now).unwrap();

        let remaining = list_backup_files(&backup_dir).unwrap();
        assert_eq!(remaining.len(), MIN_BACKUP_FILES_TO_KEEP);
    }

    #[test]
    fn prune_backup_dir_keeps_recent_files_beyond_fifty_file_floor() {
        let backup_dir = unique_test_dir("recent-window");
        fs::create_dir_all(&backup_dir).unwrap();

        for index in 0..MIN_BACKUP_FILES_TO_KEEP {
            let path = backup_dir.join(format!("old-{index:03}{BACKUP_FILE_SUFFIX}"));
            fs::write(&path, format!("old-{index}")).unwrap();
            thread::sleep(Duration::from_millis(2));
        }

        let cutoff_reference = SystemTime::now();
        for index in 0..3 {
            let path = backup_dir.join(format!("recent-{index:03}{BACKUP_FILE_SUFFIX}"));
            fs::write(&path, format!("recent-{index}")).unwrap();
            thread::sleep(Duration::from_millis(2));
        }

        let now =
            cutoff_reference + Duration::from_secs((BACKUP_RETENTION_DAYS - 1) * 24 * 60 * 60);
        prune_backup_dir(&backup_dir, now).unwrap();

        let remaining = list_backup_files(&backup_dir).unwrap();
        assert_eq!(remaining.len(), MIN_BACKUP_FILES_TO_KEEP + 3);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_text_preserves_existing_file_permissions() {
        let dir = unique_test_dir("atomic-perms");
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("settings.json");

        fs::write(&target, "old").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();

        atomic_write_text(&target, "new").unwrap();

        let mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_text_follows_symlink_targets() {
        let dir = unique_test_dir("atomic-symlink");
        fs::create_dir_all(&dir).unwrap();
        let real_target = dir.join("real-settings.json");
        let symlink_target = dir.join("settings.json");

        fs::write(&real_target, "old").unwrap();
        symlink(&real_target, &symlink_target).unwrap();

        atomic_write_text(&symlink_target, "new").unwrap();

        assert!(
            fs::symlink_metadata(&symlink_target)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&real_target).unwrap(), "new");
        assert_eq!(fs::read_to_string(&symlink_target).unwrap(), "new");
    }
}
