use serde::{Deserialize, Serialize};

use crate::{
    Guid, Hive, RestorePoint, Setting, SettingChange, SettingResult, StartupList, TweakChange,
    TweakId, Value,
};

/// A `Setting` as the first `TweakChange` layout stored it, before a power value named its scheme.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettingV1 {
    Dword {
        hive: Hive,
        key: String,
        name: String,
    },
    Key {
        hive: Hive,
        key: String,
    },
    Service {
        name: String,
    },
    PowerAc {
        subgroup: Guid,
        setting: Guid,
    },
    ActiveScheme,
    Hibernation,
    StartupApproval {
        hive: Hive,
        list: StartupList,
        name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingChangeV1 {
    pub setting: SettingV1,
    pub from: Value,
    pub to: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingResultV1 {
    pub change: SettingChangeV1,
    pub error: Option<u32>,
}

/// A `TweakChange` of the first layout, read for the rows stored before power schemes were kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakChangeV1 {
    pub tweak: TweakId,
    pub subject: Option<String>,
    pub undo_of: Option<i64>,
    pub results: Vec<SettingResultV1>,
    pub restore_point: Option<RestorePoint>,
}

impl From<SettingV1> for Setting {
    fn from(old: SettingV1) -> Setting {
        match old {
            SettingV1::Dword { hive, key, name } => Setting::Dword { hive, key, name },
            SettingV1::Key { hive, key } => Setting::Key { hive, key },
            SettingV1::Service { name } => Setting::Service { name },
            SettingV1::PowerAc { subgroup, setting } => Setting::PowerAc {
                subgroup,
                setting,
                scheme: None,
            },
            SettingV1::ActiveScheme => Setting::ActiveScheme,
            SettingV1::Hibernation => Setting::Hibernation,
            SettingV1::StartupApproval { hive, list, name } => {
                Setting::StartupApproval { hive, list, name }
            }
        }
    }
}

impl From<TweakChangeV1> for TweakChange {
    fn from(old: TweakChangeV1) -> TweakChange {
        TweakChange {
            tweak: old.tweak,
            subject: old.subject,
            undo_of: old.undo_of,
            results: old
                .results
                .into_iter()
                .map(|r| SettingResult {
                    change: SettingChange {
                        setting: r.change.setting.into(),
                        from: r.change.from,
                        to: r.change.to,
                    },
                    error: r.error,
                })
                .collect(),
            restore_point: old.restore_point,
            journal: None,
        }
    }
}
