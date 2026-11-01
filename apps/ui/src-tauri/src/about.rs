use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::agent::UiError;
use crate::history::History;

/// What Settings shows about the program and the history it keeps on this PC.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AboutDto {
    pub version: &'static str,
    /// Bytes in the data folder; `None` before the watcher has created it.
    pub data_bytes: Option<u64>,
}

#[tauri::command]
pub async fn about() -> Result<AboutDto, UiError> {
    tauri::async_runtime::spawn_blocking(|| {
        let data_bytes = match data_dir() {
            Some(dir) => folder_bytes(&dir).map_err(|err| UiError::Failed(err.to_string()))?,
            None => None,
        };
        Ok(AboutDto {
            version: env!("CARGO_PKG_VERSION"),
            data_bytes,
        })
    })
    .await
    .map_err(|err| UiError::Failed(err.to_string()))?
}

/// Opens the data folder in File Explorer.
#[tauri::command]
pub fn open_data_folder() -> Result<(), UiError> {
    let dir = data_dir()
        .filter(|dir| dir.is_dir())
        .ok_or_else(|| UiError::Failed("the data folder does not exist yet".into()))?;
    Command::new("explorer.exe")
        .arg(dir)
        .spawn()
        .map(drop)
        .map_err(|err| UiError::Failed(err.to_string()))
}

/// The folder that holds the watcher's database.
fn data_dir() -> Option<PathBuf> {
    History::default_path()?.parent().map(Path::to_path_buf)
}

/// Total size of the files under `dir`; `None` when there is no such folder.
fn folder_bytes(dir: &Path) -> io::Result<Option<u64>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    let mut total = 0;
    for entry in entries {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            total += folder_bytes(&entry.path())?.unwrap_or(0);
        } else if kind.is_file() {
            total += entry.metadata()?.len();
        }
    }
    Ok(Some(total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_every_file_below_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("vestigium.db"), [0u8; 4096]).unwrap();
        std::fs::write(dir.path().join("vestigium.db-wal"), [0u8; 512]).unwrap();
        std::fs::create_dir(dir.path().join("old")).unwrap();
        std::fs::write(dir.path().join("old").join("note.txt"), [0u8; 10]).unwrap();
        assert_eq!(folder_bytes(dir.path()).unwrap(), Some(4618));
    }

    #[test]
    fn a_missing_folder_has_no_size() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(folder_bytes(&dir.path().join("data")).unwrap(), None);
    }

    #[test]
    fn reaches_the_window_as_plain_values() {
        let json = serde_json::to_value(AboutDto {
            version: "1.2.3",
            data_bytes: Some(96),
        })
        .unwrap();
        assert_eq!(json, serde_json::json!({ "version": "1.2.3", "data_bytes": 96 }));
    }
}
