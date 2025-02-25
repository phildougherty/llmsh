// src/utils/path_utils.rs
use std::path::{Path, PathBuf};
use std::env;
use std::fs;

pub fn find_executable(command: &str) -> Option<PathBuf> {
    // If the command contains a path separator, check if it exists directly
    if command.contains('/') {
        let path = Path::new(command);
        if path.exists() && is_executable(path) {
            return Some(path.to_path_buf());
        }
        return None;
    }

    // For common commands, try direct paths first
    let common_paths = [
        "/bin", "/usr/bin", "/usr/local/bin", "/sbin", "/usr/sbin"
    ];
    
    for dir in &common_paths {
        let path = Path::new(dir).join(command);
        if path.exists() && is_executable(&path) {
            return Some(path);
        }
    }

    // Otherwise, search in PATH
    if let Ok(path_var) = env::var("PATH") {
        for dir in path_var.split(':') {
            let path = Path::new(dir).join(command);
            if path.exists() && is_executable(&path) {
                return Some(path);
            }
        }
    }

    // If not found in PATH, just return the command itself
    // This allows the shell to handle the error more gracefully
    Some(PathBuf::from(command))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = fs::metadata(path) {
        let permissions = metadata.permissions();
        return permissions.mode() & 0o111 != 0;
    }
    false
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.exists()
}