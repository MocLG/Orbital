use std::fs;
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
            match scan_directory(&dir) {
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

/// Scan a single directory level, computing recursive sizes for subdirectories.
fn scan_directory(dir: &Path) -> Result<ScanResult, String> {
    let read_dir = fs::read_dir(dir).map_err(|e| format!("Cannot read {}: {}", dir.display(), e))?;

    let mut entries = Vec::new();
    let mut total_size: u64 = 0;

    for entry in read_dir.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let is_dir = meta.is_dir();
        let modified = meta.modified().ok();
        let readonly = meta.permissions().readonly();

        let size = if is_dir {
            dir_size_jwalk(&path)
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

    // Sort by size descending
    entries.sort_by(|a, b| b.size.cmp(&a.size));

    Ok(ScanResult {
        path: dir.to_path_buf(),
        entries,
        total_size,
    })
}

/// Use jwalk for fast parallel recursive directory size calculation.
fn dir_size_jwalk(path: &Path) -> u64 {
    jwalk::WalkDir::new(path)
        .skip_hidden(false)
        .follow_links(false)
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
