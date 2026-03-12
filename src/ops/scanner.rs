use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;

/// A single entry (file or directory) with its computed size.
#[derive(Clone, Debug)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub modified: Option<SystemTime>,
    pub readonly: bool,
}

/// Scan result for a directory.
#[derive(Clone, Debug)]
pub struct ScanResult {
    pub path: PathBuf,
    pub entries: Vec<DirEntry>,
    pub total_size: u64,
}

/// State shared between the scanner thread and the UI.
#[derive(Clone, Debug)]
pub enum ScanState {
    Idle,
    Scanning(PathBuf),
    Done(ScanResult),
    Error(String),
}

/// Handle to the background scanner.
pub struct Scanner {
    pub state: Arc<Mutex<ScanState>>,
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ScanState::Idle)),
        }
    }

    /// Kick off a background scan of `dir`. Non-blocking.
    pub fn scan(&self, dir: PathBuf) {
        let state = Arc::clone(&self.state);

        // Mark as scanning
        if let Ok(mut s) = state.lock() {
            *s = ScanState::Scanning(dir.clone());
        }

        thread::spawn(move || {
            let root_dev = device_id(&dir);
            match scan_directory(&dir, root_dev) {
                Ok(result) => {
                    if let Ok(mut s) = state.lock() {
                        *s = ScanState::Done(result);
                    }
                }
                Err(e) => {
                    if let Ok(mut s) = state.lock() {
                        *s = ScanState::Error(e);
                    }
                }
            }
        });
    }

    /// Get current scan state (cloned).
    pub fn get_state(&self) -> ScanState {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// Virtual filesystem mount points that must be skipped entirely.
const VIRTUAL_FS: &[&str] = &["/proc", "/sys", "/dev", "/run", "/tmp"];

/// Returns true if the path is inside a virtual/pseudo filesystem.
fn is_virtual_fs(path: &Path) -> bool {
    let s = path.to_string_lossy();
    VIRTUAL_FS.iter().any(|vfs| s == *vfs || s.starts_with(&format!("{vfs}/")))
}

/// Get the device ID (st_dev) for a path, used for same-device boundary checks.
fn device_id(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().map(|m| m.dev())
}

/// Scan a single directory level, computing recursive sizes for all subdirs.
/// For cross-device mount points (e.g. /home on a separate partition),
/// scans them using their own device ID so sizes are computed correctly.
fn scan_directory(dir: &Path, root_dev: Option<u64>) -> Result<ScanResult, String> {
    let read_dir = fs::read_dir(dir).map_err(|e| format!("Cannot read {}: {}", dir.display(), e))?;

    let mut entries = Vec::new();
    let mut total_size: u64 = 0;

    for entry in read_dir.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if is_virtual_fs(&path) {
            continue;
        }

        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let is_dir = meta.is_dir();
        let modified = meta.modified().ok();
        let readonly = meta.permissions().readonly();

        let size = if is_dir {
            // For cross-device dirs, use the child's own device ID
            // so jwalk scans within that filesystem
            let child_dev = Some(meta.dev());
            dir_size_jwalk(&path, child_dev)
        } else {
            meta.len()
        };

        total_size += size;

        entries.push(DirEntry {
            name,
            path,
            size,
            is_dir,
            modified,
            readonly,
        });
    }

    entries.sort_by(|a, b| b.size.cmp(&a.size));

    Ok(ScanResult {
        path: dir.to_path_buf(),
        entries,
        total_size,
    })
}

/// Maximum depth for recursive scanning (safety limit for first pass).
const MAX_SCAN_DEPTH: usize = 10;

/// Use jwalk for fast parallel recursive directory size calculation.
/// Respects device boundary and skips virtual FS paths.
fn dir_size_jwalk(path: &Path, root_dev: Option<u64>) -> u64 {
    jwalk::WalkDir::new(path)
        .skip_hidden(false)
        .follow_links(false)
        .max_depth(MAX_SCAN_DEPTH)
        .process_read_dir(move |_depth, _path, _state, children| {
            children.retain(|child_result| {
                if let Ok(child) = child_result {
                    let child_path = child.path();

                    // Skip virtual filesystems
                    if is_virtual_fs(&child_path) {
                        return false;
                    }

                    // Skip cross-device mount points
                    if child.file_type.is_dir() {
                        if let Some(root) = root_dev {
                            if let Ok(m) = fs::symlink_metadata(&child_path) {
                                if m.dev() != root {
                                    return false;
                                }
                            }
                        }
                    }
                }
                true
            });
        })
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

/// Protected paths that must never be deleted.
const PROTECTED_PATHS: &[&str] = &[
    "/boot",
    "/etc",
    "/usr",
    "/bin",
    "/sbin",
    "/lib",
    "/lib64",
    "/var",
    "/proc",
    "/sys",
    "/dev",
    "/home",
    "/root",
    "/snap",
    "/lost+found",
    "/mnt",
    "/media",
    "/srv",
    "/opt",
];

/// Returns true if path is safe to delete (not a critical system directory).
pub fn is_safe_to_delete(path: &Path) -> bool {
    let abs = match fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => path.to_path_buf(),
    };
    let s = abs.to_string_lossy();

    // Never allow deleting filesystem root
    if s == "/" {
        return false;
    }

    for protected in PROTECTED_PATHS {
        if s == *protected || s.starts_with(&format!("{protected}/")) {
            return false;
        }
    }

    true
}

/// Delete a file or directory. Returns Ok on success.
pub fn delete_entry(path: &Path) -> Result<(), String> {
    if !is_safe_to_delete(path) {
        return Err(format!("BLOCKED: {} is a protected system path", path.display()));
    }

    if path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to delete {}: {}", path.display(), e))
    } else {
        fs::remove_file(path)
            .map_err(|e| format!("Failed to delete {}: {}", path.display(), e))
    }
}

/// Format byte sizes human-readably.
pub fn format_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * 1024 * 1024;
    const TIB: u64 = 1024 * 1024 * 1024 * 1024;

    if bytes >= TIB {
        format!("{:.1} TiB", bytes as f64 / TIB as f64)
    } else if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}
