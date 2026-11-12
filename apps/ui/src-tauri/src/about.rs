use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use tauri::State;
use vestigium_ipc::PROTOCOL_VERSION;
use vestigium_store::SCHEMA_VERSION;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{PCWSTR, w};

use crate::agent::{AgentLink, Link, Peer, UiError};
use crate::commands::AppState;
use crate::history::History;
use crate::system::{WindowsVersion, file_version, windows_version};

const CHANGELOG: &str = include_str!("../../../../CHANGELOG.md");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const HELPER: &str = "Vestigium.Helper.exe";
const REPOSITORY: &str = "https://github.com/Filipe-Soares-de-Almeida/Vestigium";
const AUTHOR: &str = "https://github.com/Filipe-Soares-de-Almeida";

/// What the About dialog shows: each program's version, the formats, Windows and WebView2.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VersionsDto {
    pub version: &'static str,
    /// The day this version was released, "2026-11-11", from the bundled changelog.
    pub released: Option<String>,
    pub monitor: MonitorVersion,
    /// The file version of the helper next to the window; `None` when it is missing.
    pub helper: Option<String>,
    pub schema: u32,
    pub protocol: u32,
    pub windows: Option<WindowsVersion>,
    pub webview: Option<String>,
}

/// Whether the watcher runs, and which version it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MonitorVersion {
    pub run: MonitorRun,
    /// From the handshake while it runs, from `Vestigium.exe` while it is stopped.
    pub version: Option<String>,
    /// The protocol it speaks, when it refused this window's.
    pub protocol: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorRun {
    Running,
    Stopped,
    /// It speaks another protocol, or the pipe belongs to another program.
    Other,
}

#[tauri::command]
pub async fn about_versions(state: State<'_, AppState>) -> Result<VersionsDto, UiError> {
    let link = state.link.clone();
    tauri::async_runtime::spawn_blocking(move || versions(&link))
        .await
        .map_err(|err| UiError::Failed(err.to_string()))
}

fn versions(link: &Link) -> VersionsDto {
    let window = std::env::current_exe().ok();
    let monitor = monitor_version(&link.status(), link.peer(), || {
        Link::sibling_agent().and_then(|exe| file_version(&exe))
    });
    VersionsDto {
        version: VERSION,
        released: released(CHANGELOG, VERSION),
        monitor,
        helper: window.and_then(|exe| file_version(&exe.with_file_name(HELPER))),
        schema: SCHEMA_VERSION,
        protocol: PROTOCOL_VERSION,
        windows: windows_version(),
        webview: tauri::webview_version().ok(),
    }
}

/// The watcher as the link sees it; `installed` reads the version of the file when it is stopped.
fn monitor_version(
    link: &AgentLink,
    peer: Option<Peer>,
    installed: impl FnOnce() -> Option<String>,
) -> MonitorVersion {
    let peer = peer.unwrap_or_default();
    match link {
        AgentLink::Running(_) | AgentLink::Silent { .. } | AgentLink::Busy => MonitorVersion {
            run: MonitorRun::Running,
            version: peer.version,
            protocol: None,
        },
        AgentLink::Stopped => MonitorVersion {
            run: MonitorRun::Stopped,
            version: installed(),
            protocol: None,
        },
        AgentLink::Mismatch => MonitorVersion {
            run: MonitorRun::Other,
            version: peer.version,
            protocol: peer.protocol,
        },
    }
}

/// The date of `version`'s section in a Keep a Changelog text.
fn released(changelog: &str, version: &str) -> Option<String> {
    crate::update::releases(changelog)
        .into_iter()
        .find(|release| release.version == version)?
        .date
}

/// The pages the About dialog opens in the browser.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Page {
    Repository,
    Wiki,
    Privacy,
    License,
    Author,
}

fn address(page: Page) -> String {
    match page {
        Page::Repository => REPOSITORY.to_string(),
        Page::Wiki => format!("{REPOSITORY}/wiki"),
        Page::Privacy => format!("{REPOSITORY}/wiki/Privacy-and-data"),
        Page::License => format!("{REPOSITORY}/blob/main/LICENSE"),
        Page::Author => AUTHOR.to_string(),
    }
}

/// Opens one of the About dialog's pages in the default browser.
#[tauri::command]
pub fn open_page(page: Page) -> Result<(), UiError> {
    open_url(&address(page))
}

fn open_url(url: &str) -> Result<(), UiError> {
    let wide: Vec<u16> = url.encode_utf16().chain([0]).collect();
    // SAFETY: both strings are null-terminated and live through the call.
    let done = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW returns a value above 32 when it worked and an error code otherwise.
    match done.0 as usize {
        code if code > 32 => Ok(()),
        code => Err(UiError::Failed(format!(
            "the browser did not open (code {code})"
        ))),
    }
}

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
    // The final path, so Explorer finds a folder Windows redirects for a packaged parent process.
    let dir = std::fs::canonicalize(&dir).map_or(dir, plain);
    Command::new("explorer.exe")
        .arg(dir)
        .spawn()
        .map(drop)
        .map_err(|err| UiError::Failed(err.to_string()))
}

/// `path` without the `\\?\` prefix `canonicalize` adds, which Explorer does not accept.
fn plain(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{unc}"))
    } else if let Some(local) = text.strip_prefix(r"\\?\") {
        PathBuf::from(local)
    } else {
        path
    }
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
    fn explorer_gets_the_path_without_the_verbatim_prefix() {
        assert_eq!(
            plain(PathBuf::from(
                r"\\?\C:\Users\Ana\AppData\Local\Vestigium\data"
            )),
            PathBuf::from(r"C:\Users\Ana\AppData\Local\Vestigium\data")
        );
        assert_eq!(
            plain(PathBuf::from(r"\\?\UNC\server\share\data")),
            PathBuf::from(r"\\server\share\data")
        );
        assert_eq!(plain(PathBuf::from(r"D:\data")), PathBuf::from(r"D:\data"));
    }

    #[test]
    fn a_missing_folder_has_no_size() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(folder_bytes(&dir.path().join("data")).unwrap(), None);
    }

    #[test]
    fn a_running_watcher_has_the_version_it_greeted_with() {
        let peer = Peer {
            version: Some("0.1.0".into()),
            protocol: Some(PROTOCOL_VERSION),
        };
        let shown = monitor_version(&AgentLink::Busy, Some(peer), || panic!("not read"));
        assert_eq!(
            shown,
            MonitorVersion {
                run: MonitorRun::Running,
                version: Some("0.1.0".into()),
                protocol: None,
            }
        );
    }

    #[test]
    fn a_stopped_watcher_has_the_version_of_its_file() {
        let shown = monitor_version(&AgentLink::Stopped, None, || Some("0.1.1".into()));
        assert_eq!(shown.run, MonitorRun::Stopped);
        assert_eq!(shown.version.as_deref(), Some("0.1.1"));
    }

    #[test]
    fn a_watcher_of_another_protocol_names_it() {
        let peer = Peer {
            version: None,
            protocol: Some(13),
        };
        let shown = monitor_version(&AgentLink::Mismatch, Some(peer), || panic!("not read"));
        assert_eq!(
            shown,
            MonitorVersion {
                run: MonitorRun::Other,
                version: None,
                protocol: Some(13),
            }
        );
        let foreign = monitor_version(&AgentLink::Mismatch, None, || panic!("not read"));
        assert_eq!(foreign.protocol, None);
    }

    #[test]
    fn the_release_date_comes_from_the_version_section() {
        let changelog = "# Changelog\n\n## [Unreleased]\n\n### Added\n\n- A.\n\n\
                         ## [0.1.1] - 2026-11-11\n\n### Fixed\n\n- B.\n\n\
                         ## [0.1.0] - 2026-11-08\n\n### Added\n\n- C.\n";
        assert_eq!(released(changelog, "0.1.1").as_deref(), Some("2026-11-11"));
        assert_eq!(released(changelog, "0.1.0").as_deref(), Some("2026-11-08"));
        assert_eq!(released(changelog, "0.2.0"), None);
    }

    #[test]
    fn every_page_is_on_github_over_https() {
        let page: Page = serde_json::from_value(serde_json::json!("privacy")).unwrap();
        assert_eq!(
            address(page),
            "https://github.com/Filipe-Soares-de-Almeida/Vestigium/wiki/Privacy-and-data"
        );
        for page in [
            Page::Repository,
            Page::Wiki,
            Page::Privacy,
            Page::License,
            Page::Author,
        ] {
            assert!(address(page).starts_with("https://github.com/"), "{page:?}");
        }
        assert!(address(Page::License).ends_with("/blob/main/LICENSE"));
    }

    #[test]
    fn this_build_knows_its_own_formats() {
        let dto = versions(&Link::new(None));
        assert_eq!(dto.version, VERSION);
        assert_eq!(dto.schema, SCHEMA_VERSION);
        assert_eq!(dto.protocol, PROTOCOL_VERSION);
        assert_eq!(dto.monitor.run, MonitorRun::Stopped);
        assert!(dto.windows.is_some());
    }

    #[test]
    fn reaches_the_window_as_plain_values() {
        let json = serde_json::to_value(AboutDto {
            version: "1.2.3",
            data_bytes: Some(96),
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "version": "1.2.3", "data_bytes": 96 })
        );
    }
}
