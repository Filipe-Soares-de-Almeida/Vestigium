//! The tweak journal as the elevated helper opens it: the folders are made here, with an access
//! list that lets only SYSTEM and Administrators write, and are checked before every use.

use std::path::Path;

use vestigium_collect::{
    DiskFolder, FolderJournal, FolderProtection, WindowsProtection, journal_dir, open_journal,
};
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::Win32::Storage::FileSystem::CreateDirectoryW;
use windows::core::{HSTRING, w};

/// Opens the journal of this machine for writing, making `<ProgramData>\Vestigium\tweaks` when it
/// does not exist. A folder that exists but is not protected is refused, never repaired.
pub fn open() -> Result<FolderJournal<DiskFolder>, String> {
    let tweaks = journal_dir().ok_or("the ProgramData folder is unknown")?;
    open_in(&tweaks, create_protected, &mut WindowsProtection)
}

/// Makes `tweaks` and its parent with `create`, then opens the journal when `protection` accepts both.
pub fn open_in(
    tweaks: &Path,
    create: impl Fn(&Path) -> Result<(), String>,
    protection: &mut impl FolderProtection,
) -> Result<FolderJournal<DiskFolder>, String> {
    let parent = tweaks.parent().ok_or("the journal folder has no parent")?;
    create(parent)?;
    create(tweaks)?;
    open_journal(tweaks, protection)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "the journal folder is missing".into())
}

/// Creates one folder that only SYSTEM and Administrators may write, with no inherited access;
/// Users may read and list it. A folder that already exists is left as it is.
fn create_protected(path: &Path) -> Result<(), String> {
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // SAFETY: the SDDL is a literal, the out pointer a local freed below.
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            w!("D:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1200a9;;;BU)"),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
    }
    .map_err(|e| e.to_string())?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: false.into(),
    };
    let name = HSTRING::from(path.as_os_str());
    // SAFETY: a NUL-terminated name and a descriptor that both outlive the call.
    let made = unsafe { CreateDirectoryW(&name, Some(&attributes)) };
    // SAFETY: the descriptor was allocated by the conversion for LocalFree.
    unsafe {
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
    }
    match made {
        Ok(()) => Ok(()),
        Err(err) if err.code() == ERROR_ALREADY_EXISTS.to_hresult() => Ok(()),
        Err(err) => Err(format!("{}: {err}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use vestigium_collect::JournalError;

    use super::*;

    /// A check that accepts or refuses every folder it is shown.
    struct Fixed(bool);

    impl FolderProtection for Fixed {
        fn verify(&mut self, _dir: &Path) -> Result<Option<std::fs::File>, JournalError> {
            if self.0 {
                Ok(None)
            } else {
                Err(JournalError::Untrusted("users may write".into()))
            }
        }
    }

    fn make(path: &Path) -> Result<(), String> {
        std::fs::create_dir_all(path).map_err(|e| e.to_string())
    }

    #[test]
    fn the_folders_are_made_and_a_protected_one_opens() {
        let dir = tempfile::tempdir().unwrap();
        let tweaks = dir.path().join("Vestigium").join("tweaks");
        assert!(open_in(&tweaks, make, &mut Fixed(true)).is_ok());
        assert!(tweaks.is_dir());
        assert!(
            open_in(&tweaks, make, &mut Fixed(true)).is_ok(),
            "an existing folder is fine"
        );
    }

    #[test]
    fn a_folder_that_is_not_protected_is_refused_not_repaired() {
        let dir = tempfile::tempdir().unwrap();
        let tweaks = dir.path().join("Vestigium").join("tweaks");
        std::fs::create_dir_all(&tweaks).unwrap();
        let refused = open_in(&tweaks, make, &mut Fixed(false));
        assert!(refused.is_err_and(|why| why.contains("not protected")));
    }

    #[test]
    fn a_folder_that_cannot_be_made_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let tweaks = dir.path().join("Vestigium").join("tweaks");
        let refused = open_in(&tweaks, |_| Err("denied".into()), &mut Fixed(true));
        assert_eq!(refused.err().as_deref(), Some("denied"));
    }

    #[test]
    fn a_folder_made_here_by_a_user_is_not_accepted_as_protected() {
        let dir = tempfile::tempdir().unwrap();
        let tweaks = dir.path().join("Vestigium").join("tweaks");
        let refused = open_in(&tweaks, make, &mut WindowsProtection);
        assert!(
            refused.is_err_and(|why| why.contains("not protected")),
            "owned and writable by the user running the test"
        );
    }

    /// Run elevated: `cargo test -p vestigium-helper --lib -- --ignored a_folder_made_by_the_helper`.
    #[test]
    #[ignore = "needs an elevated process"]
    fn a_folder_made_by_the_helper_is_accepted_and_users_cannot_write_in_it() {
        let dir = tempfile::tempdir().unwrap();
        let tweaks = dir.path().join("Vestigium").join("tweaks");
        let mut journal = open_in(&tweaks, create_protected, &mut WindowsProtection).unwrap();
        let entry = vestigium_core::JournalEntry {
            ts_ms: 1,
            tweak: vestigium_core::TweakId::Mpo,
            subject: None,
            undoes: None,
            settings: Vec::new(),
        };
        let id = vestigium_core::TweakJournal::record(&mut journal, &entry).unwrap();
        assert_eq!(
            vestigium_core::TweakJournal::entry(&journal, id).unwrap(),
            Some(entry)
        );
        let file = std::fs::File::open(&tweaks).unwrap();
        let report = vestigium_collect::report_of(&file).unwrap();
        assert!(
            report.aces.iter().all(|ace| matches!(ace,
                vestigium_collect::Ace::Allow { sid, mask }
                    if sid != "S-1-5-32-545" || mask & 0x0004_0006 == 0)),
            "{report:?}"
        );
    }
}
