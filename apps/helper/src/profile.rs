//! The account a dump job works for: its SID, and the crash dump folder of its profile.

use std::path::{Component, Path, PathBuf, Prefix};

use vestigium_collect::{app_dump_dir, configured_dump_folder, profile_image_path};

/// Longest text form of a SID.
const SID_CHARS_MAX: usize = 184;
/// Sub-authorities an account SID has: the domain part and the relative id.
const SUB_AUTHORITIES: std::ops::RangeInclusive<usize> = 5..=15;

/// The SID of a user account, as `S-1-5-21-...` text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserSid(String);

impl UserSid {
    /// The SID this text spells: local, domain or Microsoft account (`S-1-5-21-...`) or Entra
    /// account (`S-1-12-1-...`), in canonical decimal form; nothing else is a user.
    pub fn parse(text: &str) -> Result<UserSid, String> {
        let bad = || format!("{text} is not a user SID");
        if text.len() > SID_CHARS_MAX {
            return Err(bad());
        }
        let mut parts = text.split('-');
        if parts.next() != Some("S") || parts.next() != Some("1") {
            return Err(bad());
        }
        let numbers: Vec<u64> = parts
            .map(|part| {
                let canonical = part == "0" || !part.starts_with('0');
                let digits = !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
                if canonical && digits {
                    part.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .collect::<Option<_>>()
            .ok_or_else(bad)?;
        let [authority, subs @ ..] = numbers.as_slice() else {
            return Err(bad());
        };
        let account = matches!(
            (*authority, subs.first().copied()),
            (5, Some(21)) | (12, Some(1))
        );
        if !account
            || !SUB_AUTHORITIES.contains(&subs.len())
            || subs.iter().any(|&sub| sub > u64::from(u32::MAX))
        {
            return Err(bad());
        }
        Ok(UserSid(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The profile folder Windows lists, kept only when it is a local absolute path with no `.` or
/// `..` and no character a file name cannot hold.
fn profile_folder(listed: &str) -> Result<PathBuf, String> {
    let path = Path::new(listed);
    let mut parts = path.components();
    match parts.next() {
        Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_)) => {}
        _ => return Err(format!("{listed} is not on a local drive")),
    }
    if parts.next() != Some(Component::RootDir) {
        return Err(format!("{listed} is not absolute"));
    }
    let mut count = 0;
    for part in parts {
        match part {
            Component::Normal(name)
                if !name
                    .to_string_lossy()
                    .contains([':', '*', '?', '"', '<', '>', '|']) =>
            {
                count += 1;
            }
            _ => return Err(format!("{listed} has a part a profile folder cannot have")),
        }
    }
    if count == 0 {
        return Err(format!("{listed} is a drive root, not a profile"));
    }
    Ok(path.components().collect())
}

/// The `CrashDumps` folder under the profile that `profile_of` lists for `sid`.
pub fn crash_dumps_of(
    sid: &UserSid,
    profile_of: impl Fn(&str) -> Option<String>,
) -> Result<PathBuf, String> {
    let listed = profile_of(sid.as_str())
        .ok_or_else(|| format!("no profile listed for {}", sid.as_str()))?;
    Ok(profile_folder(&listed)?
        .join("AppData")
        .join("Local")
        .join("CrashDumps"))
}

/// Where the account `user` keeps its app crash dumps: the folder Windows Error Reporting is
/// set to use for everyone, else `CrashDumps` in the account's own profile; without an account,
/// the folder of the user this process runs as.
pub fn app_dump_folder(user: Option<&UserSid>) -> Option<PathBuf> {
    let Some(sid) = user else {
        return app_dump_dir();
    };
    configured_dump_folder().or_else(|| crash_dumps_of(sid, profile_image_path).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SID: &str = "S-1-5-21-1004336348-1177238915-682003330-1001";

    #[test]
    fn a_user_sid_is_accepted_in_its_canonical_form() {
        for good in [
            SID,
            "S-1-12-1-3322310066-1115049823-2331092226-1740421325",
            "S-1-5-21-0-0-0-500",
        ] {
            assert_eq!(UserSid::parse(good).unwrap().as_str(), good);
        }
    }

    #[test]
    fn anything_but_a_user_sid_is_refused() {
        for bad in [
            "",
            "S-1-5-18",
            "S-1-5-32-544",
            "S-1-5-21-1-2-3",
            "S-1-5-21-1-2-3-4-5-6-7-8-9-10-11-12-13-14-15-16",
            "S-1-5-80-1-2-3-4",
            "s-1-5-21-1-2-3-1001",
            "S-2-5-21-1-2-3-1001",
            "S-1-5-21-1-2-3-+1001",
            "S-1-5-21-1-2-3-01001",
            "S-1-5-21-1-2-3-4294967296",
            "S-1-5-21-1-2-3-1001\\..\\x",
            "S-1-5-21-1-2-3-1001 ",
            "S-1-5-21--2-3-1001",
            r"S-1-5-21-1-2-3-1001\..\..\Users\other",
        ] {
            assert!(UserSid::parse(bad).is_err(), "{bad:?}");
        }
        assert!(UserSid::parse(&format!("S-1-5-21{}", "-1".repeat(100))).is_err());
    }

    fn listing(sid: &str, path: &str) -> impl Fn(&str) -> Option<String> {
        let (sid, path) = (sid.to_string(), path.to_string());
        move |asked| (asked == sid).then(|| path.clone())
    }

    #[test]
    fn the_crash_dumps_folder_sits_in_the_listed_profile() {
        let sid = UserSid::parse(SID).unwrap();
        let found = crash_dumps_of(&sid, listing(SID, r"D:\Profiles\ana"));
        assert_eq!(
            found,
            Ok(PathBuf::from(r"D:\Profiles\ana\AppData\Local\CrashDumps"))
        );
    }

    #[test]
    fn a_profile_that_is_not_listed_has_no_folder() {
        let sid = UserSid::parse(SID).unwrap();
        let other = "S-1-5-21-1004336348-1177238915-682003330-1002";
        assert!(crash_dumps_of(&sid, listing(other, r"C:\Users\bia")).is_err());
    }

    #[test]
    fn a_profile_path_that_is_not_local_and_absolute_is_refused() {
        let sid = UserSid::parse(SID).unwrap();
        for bad in [
            r"\\server\share\ana",
            r"\\?\C:\Users\ana",
            r"Users\ana",
            r"C:Users\ana",
            r"\Users\ana",
            r"C:\",
            r"C:\Users\..\Windows",
            r"C:\Users\ana\..",
            r"C:\Users\ana:stream",
            "",
        ] {
            assert!(crash_dumps_of(&sid, listing(SID, bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn windows_lists_the_profile_of_the_user_running_this() {
        let own = vestigium_ipc::current_user_sid().expect("the user's SID");
        let sid = UserSid::parse(&own).expect("an account SID");
        let folder = crash_dumps_of(&sid, profile_image_path).expect("a listed profile");
        assert!(folder.ends_with(r"AppData\Local\CrashDumps"), "{folder:?}");
        assert!(folder.is_absolute());
        let stranger = UserSid::parse("S-1-5-21-1-2-3-4").unwrap();
        assert!(crash_dumps_of(&stranger, profile_image_path).is_err());
    }
}
