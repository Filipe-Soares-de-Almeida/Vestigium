use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::async_runtime::JoinHandle;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::agent::UiError;

const CHANGELOG: &str = include_str!("../../../../CHANGELOG.md");
const CHANGELOG_PT_BR: &str = include_str!("../../../../locales/changelog/pt-BR.md");
const PREFS_FILE: &str = "updates.json";

/// One heading of a version's notes ("Added", "Fixed") and its entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Group {
    pub kind: String,
    pub items: Vec<String>,
}

/// A released version and what changed in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Release {
    pub version: String,
    pub date: Option<String>,
    pub groups: Vec<Group>,
}

/// What the window remembers about updates, in its own data folder.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatePrefs {
    /// `None` until the person answers the first-open question.
    pub check: Option<bool>,
    pub last_check_ms: Option<i64>,
    /// The version whose notes were last shown; `None` on a fresh install.
    pub seen_version: Option<String>,
}

/// The answer of a check: nothing newer, or the version on offer with its notes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CheckResult {
    UpToDate,
    Available {
        version: String,
        date_ms: Option<i64>,
        groups: Vec<Group>,
    },
}

/// How a download goes, sent to the window as it happens.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Progress {
    Chunk { done: u64, total: Option<u64> },
    Installing,
    Failed { message: String },
}

/// The update a check found, and the download under way.
#[derive(Default)]
pub struct UpdateState {
    found: Mutex<Option<Update>>,
    download: Mutex<Option<JoinHandle<()>>>,
}

/// Every released section of Keep a Changelog text, newest first as written; Unreleased is left out.
pub fn releases(text: &str) -> Vec<Release> {
    let mut out: Vec<Release> = Vec::new();
    let mut body = String::new();
    let mut current: Option<(String, Option<String>)> = None;
    let mut flush = |current: &mut Option<(String, Option<String>)>, body: &mut String| {
        if let Some((version, date)) = current.take() {
            out.push(Release {
                version,
                date,
                groups: groups(body),
            });
        }
        body.clear();
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("## [") {
            flush(&mut current, &mut body);
            let Some((version, tail)) = rest.split_once(']') else {
                continue;
            };
            if version.eq_ignore_ascii_case("unreleased") {
                continue;
            }
            let date = tail
                .trim()
                .strip_prefix('-')
                .map(|d| d.trim().to_string())
                .filter(|d| !d.is_empty());
            current = Some((version.to_string(), date));
        } else if current.is_some() {
            body.push_str(line);
            body.push('\n');
        }
    }
    flush(&mut current, &mut body);
    out
}

/// The groups of one version's notes: `### Heading` lines and `- ` entries, wrapped lines joined.
pub fn groups(body: &str) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    let mut open_item = false;
    for line in body.lines() {
        if let Some(kind) = line.strip_prefix("### ") {
            out.push(Group {
                kind: kind.trim().to_string(),
                items: Vec::new(),
            });
            open_item = false;
        } else if let Some(entry) = line.strip_prefix("- ") {
            let Some(group) = out.last_mut() else {
                continue;
            };
            group.items.push(plain(entry));
            open_item = true;
        } else if open_item && line.starts_with("  ") && !line.trim().is_empty() {
            if let Some(item) = out.last_mut().and_then(|g| g.items.last_mut()) {
                item.push(' ');
                item.push_str(&plain(line));
            }
        } else if line.trim().is_empty() || line.starts_with('[') {
            open_item = false;
        }
    }
    out.retain(|g| !g.items.is_empty());
    out
}

/// An entry as the window shows it: no code marks, no surrounding space.
fn plain(text: &str) -> String {
    text.trim().replace('`', "")
}

/// `1.2.3` as numbers to compare; anything after a `-` or `+` is ignored.
fn version_key(version: &str) -> Option<(u64, u64, u64)> {
    let core = version
        .trim()
        .trim_start_matches('v')
        .split(['-', '+'])
        .next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    Some((
        parts.next()??,
        parts.next().unwrap_or(Some(0))?,
        parts.next().unwrap_or(Some(0))?,
    ))
}

/// The releases newer than `after` up to `current`, in the window's language where translated.
pub fn notes_between(
    english: &str,
    translated: Option<&str>,
    after: Option<&str>,
    current: &str,
) -> Vec<Release> {
    let local = translated.map(releases).unwrap_or_default();
    let floor = after.and_then(version_key);
    let ceiling = version_key(current);
    releases(english)
        .into_iter()
        .filter(|r| {
            let key = version_key(&r.version);
            key.is_some()
                && (floor.is_none() || key > floor)
                && (ceiling.is_none() || key <= ceiling)
        })
        .map(|r| {
            local
                .iter()
                .find(|l| l.version == r.version)
                .cloned()
                .unwrap_or(r)
        })
        .collect()
}

/// The notes a feed carries: a JSON object of languages, or plain notes in English.
pub fn feed_groups(body: &str, lang: &str) -> Vec<Group> {
    let text = match serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(body) {
        Ok(map) => map
            .get(lang)
            .or_else(|| map.get("en"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        Err(_) => body.to_string(),
    };
    groups(&text)
}

fn translation(lang: &str) -> Option<&'static str> {
    (lang == "pt-BR").then_some(CHANGELOG_PT_BR)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn prefs_path(app: &AppHandle) -> Result<PathBuf, UiError> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|err| UiError::Failed(err.to_string()))?;
    Ok(dir.join(PREFS_FILE))
}

fn read_prefs(app: &AppHandle) -> Result<UpdatePrefs, UiError> {
    match std::fs::read(prefs_path(app)?) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes).unwrap_or_default()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(UpdatePrefs::default()),
        Err(err) => Err(UiError::Failed(err.to_string())),
    }
}

fn write_prefs(app: &AppHandle, prefs: &UpdatePrefs) -> Result<(), UiError> {
    let path = prefs_path(app)?;
    let failed = |err: std::io::Error| UiError::Failed(err.to_string());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(failed)?;
    }
    let bytes = serde_json::to_vec_pretty(prefs).map_err(|err| UiError::Failed(err.to_string()))?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, bytes).map_err(failed)?;
    std::fs::rename(&temp, &path).map_err(failed)
}

#[tauri::command]
pub fn update_prefs(app: AppHandle) -> Result<UpdatePrefs, UiError> {
    read_prefs(&app)
}

#[tauri::command]
pub fn set_update_prefs(app: AppHandle, prefs: UpdatePrefs) -> Result<(), UiError> {
    write_prefs(&app, &prefs)
}

/// The notes of the versions after `after` up to the one running, newest first.
#[tauri::command]
pub fn release_notes(lang: String, after: Option<String>) -> Vec<Release> {
    notes_between(
        CHANGELOG,
        translation(&lang),
        after.as_deref(),
        env!("CARGO_PKG_VERSION"),
    )
}

/// Asks the release feed for a newer version; a feed with no release yet counts as up to date.
#[tauri::command]
pub async fn update_check(
    app: AppHandle,
    state: State<'_, UpdateState>,
    lang: String,
) -> Result<CheckResult, UiError> {
    let updater = app
        .updater()
        .map_err(|err| UiError::Failed(err.to_string()))?;
    let found = match updater.check().await {
        Ok(found) => found,
        Err(tauri_plugin_updater::Error::ReleaseNotFound) => None,
        Err(err) => return Err(UiError::Failed(err.to_string())),
    };
    let mut prefs = read_prefs(&app)?;
    prefs.last_check_ms = Some(now_ms());
    write_prefs(&app, &prefs)?;
    let result = match &found {
        None => CheckResult::UpToDate,
        Some(update) => CheckResult::Available {
            version: update.version.clone(),
            date_ms: update
                .date
                .map(|d| (d.unix_timestamp_nanos() / 1_000_000) as i64),
            groups: feed_groups(update.body.as_deref().unwrap_or_default(), &lang),
        },
    };
    *state.found.lock().unwrap_or_else(|p| p.into_inner()) = found;
    Ok(result)
}

/// Downloads the update a check found and starts its installer, which closes this window.
#[tauri::command]
pub fn update_install(
    state: State<'_, UpdateState>,
    progress: Channel<Progress>,
) -> Result<(), UiError> {
    let update = state
        .found
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
        .ok_or_else(|| UiError::Failed("no update was found to install".into()))?;
    let task = tauri::async_runtime::spawn(async move {
        let mut done = 0u64;
        let sent = progress.clone();
        let bytes = update
            .download(
                |chunk, total| {
                    done += chunk as u64;
                    let _ = sent.send(Progress::Chunk { done, total });
                },
                || {},
            )
            .await;
        let outcome = bytes.and_then(|bytes| {
            let _ = progress.send(Progress::Installing);
            update.install(bytes)
        });
        if let Err(err) = outcome {
            let _ = progress.send(Progress::Failed {
                message: err.to_string(),
            });
        }
    });
    if let Some(earlier) = state
        .download
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .replace(task)
    {
        earlier.abort();
    }
    Ok(())
}

/// Stops a download under way; the installer never starts.
#[tauri::command]
pub fn update_cancel(state: State<'_, UpdateState>) {
    if let Some(task) = state
        .download
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take()
    {
        task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Changelog\n\n<!--\n- Every change adds its entries.\n-->\n\n## [Unreleased]\n\n### Added\n\n- Not out yet.\n\n## [0.2.0] - 2026-11-18\n\n### Added\n\n- Holding Ctrl pauses the `Now` screen,\n  like Task Manager.\n- See all programs.\n\n### Fixed\n\n- Health opens with its data.\n\n## [0.1.0] - 2026-11-01\n\n### Added\n\n- First version.\n\n[Unreleased]: https://example.invalid/compare/v0.2.0...HEAD\n[0.2.0]: https://example.invalid/v0.2.0\n";

    #[test]
    fn released_sections_are_read_and_unreleased_is_left_out() {
        let list = releases(SAMPLE);
        assert_eq!(
            list.iter().map(|r| r.version.as_str()).collect::<Vec<_>>(),
            ["0.2.0", "0.1.0"]
        );
        assert_eq!(list[0].date.as_deref(), Some("2026-11-18"));
        assert_eq!(list[0].groups[0].kind, "Added");
        assert_eq!(
            list[0].groups[0].items[0],
            "Holding Ctrl pauses the Now screen, like Task Manager."
        );
        assert_eq!(list[0].groups[1].items, ["Health opens with its data."]);
        assert_eq!(list[1].groups[0].items, ["First version."]);
    }

    #[test]
    fn notes_cover_the_versions_since_the_last_one_seen() {
        let since = notes_between(SAMPLE, None, Some("0.1.0"), "0.2.0");
        assert_eq!(since.len(), 1);
        assert_eq!(since[0].version, "0.2.0");
        assert!(notes_between(SAMPLE, None, Some("0.2.0"), "0.2.0").is_empty());
        assert_eq!(notes_between(SAMPLE, None, None, "0.1.0").len(), 1);
    }

    #[test]
    fn a_translated_version_replaces_the_english_one() {
        let local = "## [0.2.0] - 2026-11-18\n\n### Added\n\n- The translated entry.\n";
        let list = notes_between(SAMPLE, Some(local), None, "0.2.0");
        assert_eq!(list[0].groups[0].items, ["The translated entry."]);
        assert_eq!(list[1].groups[0].items, ["First version."]);
    }

    #[test]
    fn feed_notes_pick_the_language_and_fall_back_to_english() {
        let body = serde_json::json!({
            "en": "### Fixed\n\n- A fix.",
            "pt-BR": "### Fixed\n\n- The translated fix.",
        })
        .to_string();
        assert_eq!(
            feed_groups(&body, "pt-BR")[0].items,
            ["The translated fix."]
        );
        assert_eq!(feed_groups(&body, "fr")[0].items, ["A fix."]);
        assert_eq!(
            feed_groups("### Added\n\n- Plain notes.", "pt-BR")[0].items,
            ["Plain notes."]
        );
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(version_key("0.10.0") > version_key("0.9.3"));
        assert_eq!(version_key("v1.2"), Some((1, 2, 0)));
        assert_eq!(version_key("1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(version_key("Unreleased"), None);
    }

    #[test]
    fn the_shipped_changelogs_parse() {
        for release in releases(CHANGELOG)
            .iter()
            .chain(releases(CHANGELOG_PT_BR).iter())
        {
            assert!(
                version_key(&release.version).is_some(),
                "{}",
                release.version
            );
            assert!(
                !release.groups.is_empty(),
                "{} has no entries",
                release.version
            );
        }
    }
}
