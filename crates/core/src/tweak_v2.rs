use serde::{Deserialize, Serialize};

use crate::{RestorePoint, SettingResult, TweakChange, TweakId};

/// A `TweakChange` of the second layout, read for the rows stored before the journal existed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakChangeV2 {
    pub tweak: TweakId,
    pub subject: Option<String>,
    pub undo_of: Option<i64>,
    pub results: Vec<SettingResult>,
    pub restore_point: Option<RestorePoint>,
}

impl From<TweakChangeV2> for TweakChange {
    fn from(old: TweakChangeV2) -> TweakChange {
        TweakChange {
            tweak: old.tweak,
            subject: old.subject,
            undo_of: old.undo_of,
            results: old.results,
            restore_point: old.restore_point,
            journal: None,
        }
    }
}
