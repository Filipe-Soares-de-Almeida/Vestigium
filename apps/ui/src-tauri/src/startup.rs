use serde::Serialize;
use vestigium_collect::{read_setting, startup_entries};
use vestigium_core::health::{BootFacts, BootPhases, StartupRun, TempSize};
use vestigium_core::{StartupEntry, TweakId, Value};

use crate::agent::UiError;
use crate::history::History;

/// Boots the startup page lists, the latest ones.
pub const BOOTS_SHOWN: u32 = 60;

/// One boot: what the logs say, with its phases worked out.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BootDto {
    pub kernel_start_ms: i64,
    pub boot_type: Option<u8>,
    #[serde(flatten)]
    pub phases: BootPhases,
    pub apps: Vec<StartupRun>,
    pub logger_full: bool,
    pub failed_drivers: Vec<String>,
}

impl From<&BootFacts> for BootDto {
    fn from(boot: &BootFacts) -> BootDto {
        BootDto {
            kernel_start_ms: boot.kernel_start_ms,
            boot_type: boot.boot_type,
            phases: boot.phases(),
            apps: boot
                .apps
                .iter()
                .map(|run| StartupRun {
                    command: program_name(&run.command),
                    ..run.clone()
                })
                .collect(),
            logger_full: boot.logger_full,
            failed_drivers: boot.failed_drivers.clone(),
        }
    }
}

/// The file name of the program a Run command starts, without its folder, arguments or quotes.
fn program_name(command: &str) -> String {
    let command = command.trim().trim_start_matches('"');
    let program = command.split(['"']).next().unwrap_or(command);
    let program = program.split(".exe ").next().map_or(program, |stem| {
        if stem.len() < program.len() {
            &program[..stem.len() + 4]
        } else {
            program
        }
    });
    program
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(program)
        .trim()
        .to_string()
}

#[cfg(test)]
mod program_name_tests {
    use super::program_name;

    #[test]
    fn a_run_command_shows_its_program_file() {
        assert_eq!(program_name("Lightshot.exe"), "Lightshot.exe");
        assert_eq!(program_name("Vestigium.exe\""), "Vestigium.exe");
        assert_eq!(
            program_name(r#""C:\Program Files\Example\Example.exe" --tray"#),
            "Example.exe"
        );
        assert_eq!(program_name(r"C:\Tools\Sync.exe /min"), "Sync.exe");
    }
}

/// A program Windows starts at sign-in; the program's path stays on this side.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StartupAppDto {
    pub name: String,
    /// The address of this entry's row on the optimizations page.
    pub key: String,
    pub enabled: bool,
    /// A security product: the page never offers to switch it off.
    pub protected: bool,
}

/// The startup page: the latest boots, oldest first, the Temp folder and the startup programs.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct StartupAreaDto {
    pub boots: Vec<BootDto>,
    pub temp: Option<TempSize>,
    pub startup_apps: Vec<StartupAppDto>,
}

/// Builds the page from stored boots (oldest first), the newest Temp count and the entries with
/// the approval value Explorer keeps for each.
pub fn area(
    boots: &[BootFacts],
    temp: Option<TempSize>,
    entries: &[(StartupEntry, Option<Value>)],
) -> StartupAreaDto {
    StartupAreaDto {
        boots: boots.iter().map(BootDto::from).collect(),
        temp,
        startup_apps: entries
            .iter()
            .map(|(entry, approval)| {
                let state = entry.state(approval.as_ref());
                StartupAppDto {
                    name: entry.name.clone(),
                    key: format!("{:?}:{}", TweakId::StartupApp, entry.name),
                    enabled: state.enabled,
                    protected: state.protected,
                }
            })
            .collect(),
    }
}

/// Reads the stored boots and the startup programs Windows runs now.
pub fn read(history: &mut History) -> Result<StartupAreaDto, UiError> {
    let (boots, temp) = history.boots(BOOTS_SHOWN)?;
    let entries: Vec<(StartupEntry, Option<Value>)> = startup_entries()
        .into_iter()
        .map(|entry| {
            let approval = read_setting(&entry.approval());
            (entry, approval)
        })
        .collect();
    Ok(area(&boots, temp, &entries))
}

#[cfg(test)]
mod tests {
    use vestigium_core::{Hive, StartupList};

    use super::*;

    fn boot(start_ms: i64) -> BootFacts {
        BootFacts {
            kernel_start_ms: start_ms,
            boot_type: Some(0),
            firmware_ms: Some(14_000),
            signin_ms: Some(start_ms + 18_000),
            profile_start_ms: Some(start_ms + 18_000),
            profile_end_ms: Some(start_ms + 60_000),
            tpm_check_ms: Some(start_ms + 18_500),
            tpm_ready_ms: Some(start_ms + 40_000),
            desktop_ms: Some(start_ms + 70_000),
            apps: vec![StartupRun {
                command: "Example Capture".into(),
                start_ms: start_ms + 80_000,
                end_ms: Some(start_ms + 82_000),
            }],
            logger_full: false,
            failed_drivers: vec!["exampledrv".into()],
        }
    }

    fn entry(name: &str, exe: &str) -> StartupEntry {
        StartupEntry {
            hive: Hive::CurrentUser,
            list: StartupList::Run,
            name: name.into(),
            exe: exe.into(),
        }
    }

    #[test]
    fn a_boot_carries_its_worked_out_phases() {
        let facts = boot(1_790_000_000_000);
        let dto = BootDto::from(&facts);
        assert_eq!(dto.phases, facts.phases());
        assert_eq!(dto.phases.profile_ms, Some(42_000));
        assert_eq!(dto.phases.from_power_ms, Some(14_000 + 70_000));
        assert_eq!(dto.apps, facts.apps);
        assert_eq!(dto.failed_drivers, ["exampledrv"]);
    }

    #[test]
    fn boots_keep_the_order_they_came_in() {
        let boots = [boot(1_000), boot(2_000), boot(3_000)];
        let dto = area(&boots, None, &[]);
        let starts: Vec<i64> = dto.boots.iter().map(|b| b.kernel_start_ms).collect();
        assert_eq!(starts, [1_000, 2_000, 3_000]);
        assert_eq!(dto.temp, None);
    }

    #[test]
    fn phases_sit_beside_the_boot_fields_in_the_json() {
        let json = serde_json::to_value(BootDto::from(&boot(1_000))).unwrap();
        for field in [
            "kernel_start_ms",
            "boot_type",
            "firmware_ms",
            "to_signin_ms",
        ] {
            assert!(json.get(field).is_some(), "{field}");
        }
        for field in ["profile_ms", "tpm_wait_ms", "desktop_ms", "to_desktop_ms"] {
            assert!(json.get(field).is_some(), "{field}");
        }
        assert!(json["probable_wait"] == false && json.get("from_power_ms").is_some());
    }

    #[test]
    fn a_startup_app_is_addressed_like_its_tweak_row_and_hides_its_path() {
        let off = Value::Bytes(vec![3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let entries = [
            (entry("Example Capture", r"C:\Apps\capture.exe"), None),
            (entry("Vestigium", "vestigium.exe"), Some(off)),
            (entry("Kaspersky Tray", "avp.exe"), None),
        ];
        let dto = area(&[], None, &entries);
        let apps = &dto.startup_apps;
        assert_eq!(apps[0].key, "StartupApp:Example Capture");
        assert!(apps[0].enabled && !apps[0].protected);
        assert!(!apps[1].enabled && !apps[1].protected);
        assert!(apps[2].protected);
        let json = serde_json::to_string(&dto).unwrap();
        assert!(!json.contains("capture.exe") && !json.contains("Apps"));
    }

    #[test]
    fn the_stored_boots_reach_the_page_oldest_first_with_the_temp_count() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vestigium.db");
        let mut store = vestigium_store::Store::open(&path).unwrap();
        for start in [3_000, 1_000, 2_000] {
            store.put_boot(&boot(start)).unwrap();
        }
        let size = |ts_ms, folders| TempSize {
            ts_ms,
            folders,
            files: 5,
            capped: false,
        };
        store.put_temp(&size(10, 213_879)).unwrap();
        store.put_temp(&size(20, 3_558)).unwrap();
        let mut history = History::new(Some(path));
        let (boots, temp) = history.boots(2).unwrap();
        let starts: Vec<i64> = boots.iter().map(|b| b.kernel_start_ms).collect();
        assert_eq!(starts, [2_000, 3_000]);
        assert_eq!(temp.map(|t| t.folders), Some(3_558));
    }

    #[test]
    fn without_a_database_the_page_is_empty() {
        let mut history = History::new(None);
        let (boots, temp) = history.boots(60).unwrap();
        assert!(boots.is_empty() && temp.is_none());
    }
}
