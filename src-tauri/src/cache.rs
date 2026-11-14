//! Listings remembered on this computer, so the Services screen and the
//! Library open quickly from a shared folder (Google Drive streams a file's
//! contents only when it is read): a file is read again only when its
//! modification time or size has changed. Checking those reads no contents.
//!
//! The cache lives in the app's cache folder on this computer, never in the
//! shared folder, and can be deleted at any time.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::de::DeserializeOwned;
use serde::Serialize;
use tauri::{AppHandle, Manager};

/// A file's stamp: when it was last changed, and its size.
pub fn file_stamp(path: &Path) -> Option<String> {
    let m = std::fs::metadata(path).ok()?;
    let t = m.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos();
    Some(format!("{t}:{}", m.len()))
}

/// A folder's stamp: its newest file, how many files there are and their
/// total size, at every level.
pub fn folder_stamp(dir: &Path) -> String {
    let (mut newest, mut count, mut size) = (0u128, 0u64, 0u64);
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let Ok(m) = e.metadata() else { continue };
            if m.is_dir() {
                stack.push(e.path());
                continue;
            }
            count += 1;
            size += m.len();
            let t = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_nanos());
            newest = newest.max(t.unwrap_or(0));
        }
    }
    format!("{newest}:{count}:{size}")
}

/// Remembered values by path, each with the stamp it was read at.
pub struct Cache<T> {
    file: Option<PathBuf>,
    old: BTreeMap<String, (String, T)>,
    new: BTreeMap<String, (String, T)>,
}

impl<T: Serialize + DeserializeOwned + Clone> Cache<T> {
    /// The cache called `name` in this app's cache folder.
    pub fn open(app: &AppHandle, name: &str) -> Self {
        Self::at(app.path().app_cache_dir().ok().map(|d| d.join(format!("{name}.json"))))
    }

    /// The cache kept in `file` (none: nothing is remembered).
    pub fn at(file: Option<PathBuf>) -> Self {
        let old = file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Cache { file, old, new: BTreeMap::new() }
    }

    /// The value for `path` at `stamp`: remembered if the stamp is the same,
    /// else made by `make` (and remembered for next time).
    pub fn get(&mut self, path: &Path, stamp: &str, make: impl FnOnce() -> Option<T>) -> Option<T> {
        let key = path.to_string_lossy().into_owned();
        let value = match self.old.remove(&key) {
            Some((s, v)) if s == stamp => v,
            _ => make()?,
        };
        self.new.insert(key, (stamp.to_string(), value.clone()));
        Some(value)
    }

    /// Writes what was used this time (paths no longer listed are dropped).
    pub fn save(self) {
        let Some(file) = self.file else { return };
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string(&self.new) {
            let _ = std::fs::write(file, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_read_again_only_when_it_changes() {
        let dir = std::env::temp_dir().join(format!("episco-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let item = dir.join("sub/item.txt");
        std::fs::write(&item, "one").unwrap();
        let store = dir.with_extension("json");
        let reads = std::cell::Cell::new(0);
        let read = |p: &Path| {
            reads.set(reads.get() + 1);
            std::fs::read_to_string(p).ok()
        };

        let mut c: Cache<String> = Cache::at(Some(store.clone()));
        let s = folder_stamp(&dir);
        assert_eq!(c.get(&dir, &s, || read(&item)).as_deref(), Some("one"));
        c.save();
        let mut c: Cache<String> = Cache::at(Some(store.clone()));
        assert_eq!(c.get(&dir, &folder_stamp(&dir), || read(&item)).as_deref(), Some("one"));
        assert_eq!(reads.get(), 1, "unchanged: remembered");
        c.save();

        std::fs::write(&item, "two!").unwrap();
        let mut c: Cache<String> = Cache::at(Some(store.clone()));
        assert_eq!(c.get(&dir, &folder_stamp(&dir), || read(&item)).as_deref(), Some("two!"));
        assert_eq!(reads.get(), 2, "changed: read again");
        assert_ne!(file_stamp(&item), None);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(dir.with_extension("json"));
    }
}
