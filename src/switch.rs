use std::path::{Path, PathBuf};
use std::sync::{RwLock, RwLockReadGuard};

use walkdir::{DirEntry, WalkDir};

pub struct SwitchService {
    path: PathBuf,
    files: RwLock<Vec<SwitchFile>>,
}

impl SwitchService {
    pub fn new(path: &str) -> Self {
        let path = PathBuf::from(path);
        let files = SwitchService::scan_dir(&path);

        SwitchService {
            path,
            files: RwLock::new(files),
        }
    }

    fn scan_dir(path: &Path) -> Vec<SwitchFile> {
        let parse = |entry: DirEntry| {
            let metadata = entry.metadata().ok()?;
            if !metadata.is_file() {
                return None;
            }

            let rel_path = entry
                .path()
                .strip_prefix(path)
                .ok()?
                .to_string_lossy()
                .to_string();
            let name = entry.file_name().to_string_lossy().to_string();
            let size = metadata.len();

            Some(SwitchFile {
                rel_path,
                name,
                size,
            })
        };

        fn is_demos_dir(entry: &DirEntry) -> bool {
            entry.file_type().is_dir() && entry.file_name() == "demos"
        }
        fn is_hidden(entry: &DirEntry) -> bool {
            entry.file_name().to_string_lossy().starts_with(".")
        }

        WalkDir::new(path)
            .into_iter()
            .filter_entry(|e| !(is_hidden(e) || is_demos_dir(e)))
            .filter_map(|e| e.ok().and_then(parse))
            .collect()
    }

    pub fn scan(&self) {
        let files: Vec<SwitchFile> = SwitchService::scan_dir(&self.path);

        let mut lock = self
            .files
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        *lock = files;
    }

    pub fn list_files(&self) -> RwLockReadGuard<'_, Vec<SwitchFile>> {
        self.files.read().expect("Poisoned lock")
    }

    pub fn resolve_file(&self, path: &str) -> Option<PathBuf> {
        let mut resolved = self.path.clone();
        resolved.push(path);

        if resolved.starts_with(&self.path) && resolved.is_file() {
            Some(resolved)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct SwitchFile {
    pub name: String,
    pub rel_path: String,
    pub size: u64,
}
