//! The tweak journal: a ProgramData folder only administrators write, where the helper records each
//! change before writing it and the watcher and window read it.

use std::ffi::c_void;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use vestigium_core::{JournalEntry, TWEAK_JOURNAL_VERSION, TweakJournal};
use windows::Win32::Foundation::{
    ERROR_ALREADY_EXISTS, ERROR_FILE_EXISTS, HANDLE, HLOCAL, LocalFree,
};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, GetSecurityInfo, SE_FILE_OBJECT,
};
use windows::Win32::Security::{
    ACE_HEADER, ACL, ACL_SIZE_INFORMATION, AclSizeInformation, DACL_SECURITY_INFORMATION, GetAce,
    GetAclInformation, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_READ, FILE_SHARE_WRITE, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};
use windows::Win32::UI::Shell::{FOLDERID_ProgramData, KF_FLAG_DEFAULT, SHGetKnownFolderPath};
use windows::core::{HSTRING, PWSTR};

/// Longest entry file the journal reads; a real one is a few hundred bytes.
const MOST_ENTRY_BYTES: u64 = 1 << 20;
/// How many ids are tried before giving up when other writers keep taking the next one.
const ID_TRIES: u32 = 16;

/// Why a journal could not be opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JournalError {
    /// The folder is not protected: someone other than an administrator could have written in it.
    Untrusted(String),
    /// The folder or a file in it could not be read or written.
    Io(String),
}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JournalError::Untrusted(why) => write!(f, "the journal folder is not protected: {why}"),
            JournalError::Io(why) => f.write_str(why),
        }
    }
}

/// Where the journal's entries are kept; one file per entry, named by its id.
pub trait JournalFolder {
    /// The ids of the entries held.
    fn ids(&self) -> Result<Vec<u64>, String>;
    /// The bytes of one entry, or `None` when there is none.
    fn read(&self, id: u64) -> Result<Option<Vec<u8>>, String>;
    /// Stores `bytes` under `id` and puts them on disk; `false` when the id is taken.
    fn create(&mut self, id: u64, bytes: &[u8]) -> Result<bool, String>;
}

/// A journal over a folder: entries are a version byte and the entry, written whole.
pub struct FolderJournal<F> {
    folder: F,
    /// Handles that keep the checked folders from being renamed or replaced while in use.
    _held: Vec<File>,
}

impl<F: JournalFolder> FolderJournal<F> {
    pub fn new(folder: F) -> Self {
        FolderJournal {
            folder,
            _held: Vec::new(),
        }
    }
}

impl<F: JournalFolder> TweakJournal for FolderJournal<F> {
    fn entry(&self, id: u64) -> Result<Option<JournalEntry>, String> {
        let Some(bytes) = self.folder.read(id)? else {
            return Ok(None);
        };
        Ok(match bytes.split_first() {
            Some((&version, body)) if version == TWEAK_JOURNAL_VERSION => {
                postcard::from_bytes(body).ok()
            }
            _ => None,
        })
    }

    fn record(&mut self, entry: &JournalEntry) -> Result<u64, String> {
        let mut bytes = vec![TWEAK_JOURNAL_VERSION];
        bytes.extend(postcard::to_allocvec(entry).map_err(|e| e.to_string())?);
        let first = self
            .folder
            .ids()?
            .into_iter()
            .max()
            .map_or(1, |last| last + 1);
        for id in first..first + u64::from(ID_TRIES) {
            if self.folder.create(id, &bytes)? {
                return Ok(id);
            }
        }
        Err("no free journal id".into())
    }
}

/// The entries as files of a folder on disk.
pub struct DiskFolder {
    dir: PathBuf,
}

impl DiskFolder {
    fn path(&self, id: u64) -> PathBuf {
        self.dir.join(format!("{id}.entry"))
    }
}

impl JournalFolder for DiskFolder {
    fn ids(&self) -> Result<Vec<u64>, String> {
        let mut ids = Vec::new();
        for item in fs::read_dir(&self.dir).map_err(|e| e.to_string())? {
            let name = item.map_err(|e| e.to_string())?.file_name();
            let id = name
                .to_str()
                .and_then(|name| name.strip_suffix(".entry"))
                .and_then(|stem| stem.parse::<u64>().ok());
            ids.extend(id);
        }
        Ok(ids)
    }

    fn read(&self, id: u64) -> Result<Option<Vec<u8>>, String> {
        let file = match File::open(self.path(id)) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.to_string()),
        };
        let mut bytes = Vec::new();
        file.take(MOST_ENTRY_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Ok((bytes.len() as u64 <= MOST_ENTRY_BYTES).then_some(bytes))
    }

    fn create(&mut self, id: u64, bytes: &[u8]) -> Result<bool, String> {
        let target = self.path(id);
        if target.exists() {
            return Ok(false);
        }
        let temp = self.dir.join(format!("{id}.entry.tmp"));
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => return Ok(false),
            Err(err) => return Err(err.to_string()),
        };
        let written = file.write_all(bytes).and_then(|()| file.sync_all());
        drop(file);
        if let Err(err) = written {
            let _ = fs::remove_file(&temp);
            return Err(err.to_string());
        }
        let from = HSTRING::from(temp.as_os_str());
        let to = HSTRING::from(target.as_os_str());
        // SAFETY: two NUL-terminated strings that outlive the call; no replace flag, so a taken id fails.
        let moved = unsafe { MoveFileExW(&from, &to, MOVEFILE_WRITE_THROUGH) };
        match moved {
            Ok(()) => Ok(true),
            Err(err) => {
                let _ = fs::remove_file(&temp);
                let taken = err.code() == ERROR_ALREADY_EXISTS.to_hresult()
                    || err.code() == ERROR_FILE_EXISTS.to_hresult();
                if taken {
                    Ok(false)
                } else {
                    Err(err.to_string())
                }
            }
        }
    }
}

/// Decides whether a folder may be trusted to hold only what administrators wrote.
pub trait FolderProtection {
    /// Checks the folder; the file it returns keeps the folder in place while it is held.
    fn verify(&mut self, dir: &Path) -> Result<Option<File>, JournalError>;
}

/// Opens the journal in `tweaks`, after `protection` accepts it and its parent folder: `Ok(None)`
/// when there is no such folder, so nothing was ever recorded.
pub fn open_journal(
    tweaks: &Path,
    protection: &mut impl FolderProtection,
) -> Result<Option<FolderJournal<DiskFolder>>, JournalError> {
    match fs::symlink_metadata(tweaks) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return Err(JournalError::Untrusted("it is not a folder".into())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(JournalError::Io(err.to_string())),
    }
    let parent = tweaks
        .parent()
        .ok_or_else(|| JournalError::Untrusted("it has no parent folder".into()))?;
    let mut journal = FolderJournal::new(DiskFolder {
        dir: tweaks.to_path_buf(),
    });
    journal._held.extend(protection.verify(parent)?);
    journal._held.extend(protection.verify(tweaks)?);
    Ok(Some(journal))
}

/// `<ProgramData>\Vestigium\tweaks`, from the known folder and not from the environment, which a
/// user can change for the processes it starts.
pub fn journal_dir() -> Option<PathBuf> {
    // SAFETY: the returned string is read once and freed with CoTaskMemFree.
    unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_ProgramData, KF_FLAG_DEFAULT, None).ok()?;
        let base = path.to_string().ok();
        windows::Win32::System::Com::CoTaskMemFree(Some(path.0 as *const c_void));
        Some(PathBuf::from(base?).join("Vestigium").join("tweaks"))
    }
}

/// The journal of this machine, opened the first time an entry is asked for. A folder that is
/// missing or not protected holds nothing; one that cannot be read is an error.
#[derive(Default)]
pub struct KnownJournal {
    opened: OnceLock<Result<Option<FolderJournal<DiskFolder>>, JournalError>>,
}

impl KnownJournal {
    pub fn new() -> Self {
        KnownJournal::default()
    }
}

impl TweakJournal for KnownJournal {
    fn entry(&self, id: u64) -> Result<Option<JournalEntry>, String> {
        let opened = self.opened.get_or_init(|| match journal_dir() {
            Some(dir) => open_journal(&dir, &mut WindowsProtection),
            None => Ok(None),
        });
        match opened {
            Ok(Some(journal)) => journal.entry(id),
            Ok(None) | Err(JournalError::Untrusted(_)) => Ok(None),
            Err(JournalError::Io(why)) => Err(why.clone()),
        }
    }

    fn record(&mut self, _entry: &JournalEntry) -> Result<u64, String> {
        Err("only the elevated helper writes the journal".into())
    }
}

/// Well-known accounts that may own and write the journal folder: SYSTEM, Administrators and
/// TrustedInstaller, and CREATOR OWNER.
const TRUSTED: [&str; 4] = [
    // CREATOR OWNER names the owner of each file made inside, which is an administrator here.
    "S-1-3-0",
    "S-1-5-18",
    "S-1-5-32-544",
    "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464",
];
/// Access bits that let an account add, change, delete or take over what a folder holds.
const WRITES: u32 = 0x0000_0002 // FILE_WRITE_DATA, FILE_ADD_FILE
    | 0x0000_0004 // FILE_APPEND_DATA, FILE_ADD_SUBDIRECTORY
    | 0x0000_0010 // FILE_WRITE_EA
    | 0x0000_0040 // FILE_DELETE_CHILD
    | 0x0000_0100 // FILE_WRITE_ATTRIBUTES
    | 0x0001_0000 // DELETE
    | 0x0004_0000 // WRITE_DAC
    | 0x0008_0000 // WRITE_OWNER
    | 0x4000_0000 // GENERIC_WRITE
    | 0x1000_0000; // GENERIC_ALL

/// One entry of a folder's access list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ace {
    Allow {
        sid: String,
        mask: u32,
    },
    /// An allowing entry of a kind this check does not read.
    Unread,
}

/// A folder's owner and the entries that allow access.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AclReport {
    pub owner: String,
    pub aces: Vec<Ace>,
}

/// Accepts a folder only when an administrator owns it and nobody else is allowed to write in it.
pub fn judge(report: &AclReport) -> Result<(), String> {
    if !TRUSTED.contains(&report.owner.as_str()) {
        return Err(format!("it is owned by {}", report.owner));
    }
    for ace in &report.aces {
        match ace {
            Ace::Allow { sid, mask } if mask & WRITES != 0 && !TRUSTED.contains(&sid.as_str()) => {
                return Err(format!("{sid} may write in it"));
            }
            Ace::Allow { .. } => {}
            Ace::Unread => return Err("it has an access entry that cannot be read".into()),
        }
    }
    Ok(())
}

/// Reads the real access list of a folder and judges it.
#[derive(Default)]
pub struct WindowsProtection;

impl FolderProtection for WindowsProtection {
    fn verify(&mut self, dir: &Path) -> Result<Option<File>, JournalError> {
        let held = OpenOptions::new()
            .access_mode(0x0002_0000) // READ_CONTROL
            .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(dir)
            .map_err(|e| JournalError::Io(format!("{}: {e}", dir.display())))?;
        let meta = held
            .metadata()
            .map_err(|e| JournalError::Io(e.to_string()))?;
        if !meta.is_dir() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(JournalError::Untrusted(format!(
                "{} is a link",
                dir.display()
            )));
        }
        let report = report_of(&held).map_err(JournalError::Io)?;
        judge(&report).map_err(JournalError::Untrusted)?;
        Ok(Some(held))
    }
}

/// The owner and the allowing entries of an open file or folder.
pub fn report_of(file: &File) -> Result<AclReport, String> {
    let handle = HANDLE(file.as_raw_handle());
    let mut owner = PSID::default();
    let mut dacl: *mut ACL = std::ptr::null_mut();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // SAFETY: the out pointers are locals; the descriptor is freed below once the report is built.
    let status = unsafe {
        GetSecurityInfo(
            handle,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            Some(&mut owner),
            None,
            Some(&mut dacl),
            None,
            Some(&mut descriptor),
        )
    };
    if status.is_err() || owner.0.is_null() {
        return Err(format!("the access list could not be read ({})", status.0));
    }
    // SAFETY: `owner` and `dacl` point into the descriptor, which lives until it is freed here.
    let report = unsafe { read_report(owner, dacl) };
    // SAFETY: the descriptor was allocated by GetSecurityInfo for LocalFree.
    unsafe {
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
    }
    report
}

/// # Safety
/// `owner` is a valid SID and `dacl` a valid ACL or null, both alive for the call.
unsafe fn read_report(owner: PSID, dacl: *const ACL) -> Result<AclReport, String> {
    // SAFETY: the caller guarantees `owner` is a valid SID.
    let owner = unsafe { sid_text(owner)? };
    if dacl.is_null() {
        // A folder with no access list lets everyone in.
        return Ok(AclReport {
            owner,
            aces: vec![Ace::Allow {
                sid: "S-1-1-0".into(),
                mask: 0x1000_0000,
            }],
        });
    }
    let mut size = ACL_SIZE_INFORMATION::default();
    // SAFETY: `dacl` is a valid ACL and `size` is the structure this class fills.
    unsafe {
        GetAclInformation(
            dacl,
            std::ptr::from_mut(&mut size).cast::<c_void>(),
            std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    }
    .map_err(|e| e.to_string())?;
    let mut aces = Vec::new();
    for index in 0..size.AceCount {
        let mut raw: *mut c_void = std::ptr::null_mut();
        // SAFETY: `index` is below the ACL's entry count.
        unsafe { GetAce(dacl, index, &mut raw) }.map_err(|e| e.to_string())?;
        // SAFETY: GetAce returned a pointer to an entry inside the ACL.
        let header = unsafe { &*raw.cast::<ACE_HEADER>() };
        match header.AceType {
            0 => {
                // An ACCESS_ALLOWED_ACE is its header, a mask, then the SID.
                let after = size_of::<ACE_HEADER>();
                // SAFETY: an allowing entry holds a mask and a SID after its header.
                let (mask, sid) = unsafe {
                    let base = raw.cast::<u8>();
                    (
                        base.add(after).cast::<u32>().read_unaligned(),
                        PSID(base.add(after + 4).cast::<c_void>()),
                    )
                };
                // SAFETY: the SID inside the entry is valid while the ACL is.
                let sid = unsafe { sid_text(sid)? };
                aces.push(Ace::Allow { sid, mask });
            }
            // Allowing entries with an object type or a condition.
            5 | 9 | 11 | 15 | 19 => aces.push(Ace::Unread),
            // Denying, logging and label entries grant nothing.
            _ => {}
        }
    }
    Ok(AclReport { owner, aces })
}

/// # Safety
/// `sid` is a valid SID.
unsafe fn sid_text(sid: PSID) -> Result<String, String> {
    let mut text = PWSTR::null();
    // SAFETY: the caller guarantees `sid`; the string it allocates is freed below.
    unsafe { ConvertSidToStringSidW(sid, &mut text) }.map_err(|e| e.to_string())?;
    // SAFETY: the conversion returned a NUL-terminated string.
    let owned = unsafe { text.to_string() }.map_err(|e| e.to_string());
    // SAFETY: the string was allocated by the conversion for LocalFree.
    unsafe {
        let _ = LocalFree(Some(HLOCAL(text.0.cast())));
    }
    owned
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use vestigium_core::{IntentSetting, TweakId, Value};

    use super::*;

    /// A folder held in memory; `taken` ids appear free to `ids` but refuse `create`.
    #[derive(Clone, Default)]
    struct Fake {
        files: Rc<RefCell<BTreeMap<u64, Vec<u8>>>>,
        taken: Rc<RefCell<Vec<u64>>>,
    }

    impl JournalFolder for Fake {
        fn ids(&self) -> Result<Vec<u64>, String> {
            Ok(self.files.borrow().keys().copied().collect())
        }

        fn read(&self, id: u64) -> Result<Option<Vec<u8>>, String> {
            Ok(self.files.borrow().get(&id).cloned())
        }

        fn create(&mut self, id: u64, bytes: &[u8]) -> Result<bool, String> {
            if self.taken.borrow().contains(&id) || self.files.borrow().contains_key(&id) {
                return Ok(false);
            }
            self.files.borrow_mut().insert(id, bytes.to_vec());
            Ok(true)
        }
    }

    fn entry(ts_ms: i64) -> JournalEntry {
        JournalEntry {
            ts_ms,
            tweak: TweakId::Mpo,
            subject: None,
            undoes: None,
            settings: vec![IntentSetting {
                setting: TweakId::Mpo.settings()[0].clone(),
                before: Some(Value::Absent),
                to: Value::Dword(5),
            }],
        }
    }

    #[test]
    fn an_entry_is_read_back_under_the_id_it_got() {
        let mut journal = FolderJournal::new(Fake::default());
        let first = journal.record(&entry(1)).unwrap();
        let second = journal.record(&entry(2)).unwrap();
        assert_ne!(first, second, "ids never repeat");
        assert_eq!(journal.entry(first).unwrap(), Some(entry(1)));
        assert_eq!(journal.entry(second).unwrap(), Some(entry(2)));
        assert_eq!(journal.entry(second + 1).unwrap(), None);
    }

    #[test]
    fn a_taken_id_is_skipped() {
        let fake = Fake::default();
        fake.taken.borrow_mut().extend([1, 2]);
        let mut journal = FolderJournal::new(fake);
        assert_eq!(journal.record(&entry(1)).unwrap(), 3);
    }

    #[test]
    fn a_file_of_another_version_or_garbled_holds_no_entry() {
        let fake = Fake::default();
        let mut journal = FolderJournal::new(fake.clone());
        let id = journal.record(&entry(1)).unwrap();
        fake.files.borrow_mut().get_mut(&id).unwrap()[0] = TWEAK_JOURNAL_VERSION + 1;
        assert_eq!(journal.entry(id).unwrap(), None);
        fake.files
            .borrow_mut()
            .insert(id, vec![TWEAK_JOURNAL_VERSION, 0xff]);
        assert_eq!(journal.entry(id).unwrap(), None);
    }

    /// A check that accepts or refuses every folder it is shown.
    struct Fixed(bool);

    impl FolderProtection for Fixed {
        fn verify(&mut self, _dir: &Path) -> Result<Option<File>, JournalError> {
            if self.0 {
                Ok(None)
            } else {
                Err(JournalError::Untrusted("users may write".into()))
            }
        }
    }

    #[test]
    fn a_folder_is_opened_only_when_the_check_accepts_it() {
        let dir = tempfile::tempdir().unwrap();
        let tweaks = dir.path().join("Vestigium").join("tweaks");
        assert!(
            matches!(open_journal(&tweaks, &mut Fixed(true)), Ok(None)),
            "no folder means nothing was recorded"
        );
        fs::create_dir_all(&tweaks).unwrap();
        assert!(matches!(
            open_journal(&tweaks, &mut Fixed(true)),
            Ok(Some(_))
        ));
        assert!(matches!(
            open_journal(&tweaks, &mut Fixed(false)),
            Err(JournalError::Untrusted(_))
        ));
    }

    #[test]
    fn entries_written_to_disk_are_read_back_and_never_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let tweaks = dir.path().join("Vestigium").join("tweaks");
        fs::create_dir_all(&tweaks).unwrap();
        let mut journal = open_journal(&tweaks, &mut Fixed(true)).unwrap().unwrap();
        let id = journal.record(&entry(7)).unwrap();
        assert_eq!(journal.entry(id).unwrap(), Some(entry(7)));
        assert!(!tweaks.join(format!("{id}.entry.tmp")).exists());
        let mut disk = DiskFolder {
            dir: tweaks.clone(),
        };
        assert!(
            !disk.create(id, b"other").unwrap(),
            "an id is never replaced"
        );
        assert_eq!(journal.record(&entry(8)).unwrap(), id + 1);
        fs::write(tweaks.join("x.entry"), b"junk").unwrap();
        assert_eq!(disk.ids().unwrap().len(), 2, "only numbered entries count");
    }

    fn allow(sid: &str, mask: u32) -> Ace {
        Ace::Allow {
            sid: sid.into(),
            mask,
        }
    }

    const READ: u32 = 0x0012_00a9;

    #[test]
    fn a_folder_only_administrators_write_is_accepted() {
        let report = AclReport {
            owner: "S-1-5-32-544".into(),
            aces: vec![
                allow("S-1-5-18", 0x001f_01ff),
                allow("S-1-5-32-544", 0x001f_01ff),
                allow("S-1-5-32-545", READ),
            ],
        };
        assert_eq!(judge(&report), Ok(()));
    }

    #[test]
    fn a_folder_that_others_own_or_write_is_refused() {
        let admin = |aces| AclReport {
            owner: "S-1-5-18".into(),
            aces,
        };
        let user = "S-1-5-21-1-2-3-1001";
        for (name, report) in [
            (
                "owned by a user",
                AclReport {
                    owner: user.into(),
                    aces: vec![allow("S-1-5-18", 0x001f_01ff)],
                },
            ),
            (
                "users add files",
                admin(vec![allow("S-1-5-32-545", READ | 0x2)]),
            ),
            (
                "users append",
                admin(vec![allow("S-1-5-32-545", READ | 0x4)]),
            ),
            (
                "users delete",
                admin(vec![allow("S-1-5-32-545", READ | 0x1_0000)]),
            ),
            ("users change the list", admin(vec![allow(user, 0x4_0000)])),
            (
                "everyone writes",
                admin(vec![allow("S-1-1-0", 0x4000_0000)]),
            ),
            ("an unread entry", admin(vec![Ace::Unread])),
        ] {
            assert!(judge(&report).is_err(), "{name}");
        }
    }

    #[test]
    fn the_real_access_list_of_a_folder_a_user_writes_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut real = WindowsProtection;
        assert!(matches!(
            real.verify(dir.path()),
            Err(JournalError::Untrusted(_))
        ));
        assert!(matches!(
            real.verify(&dir.path().join("missing")),
            Err(JournalError::Io(_))
        ));
    }

    #[test]
    fn the_real_access_list_of_the_system_folder_is_accepted() {
        let system = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32");
        let held = WindowsProtection.verify(&system).unwrap();
        assert!(held.is_some());
    }

    #[test]
    fn the_known_folder_is_under_program_data() {
        let dir = journal_dir().unwrap();
        assert!(dir.ends_with(r"Vestigium\tweaks"), "{dir:?}");
        assert!(dir.is_absolute());
    }
}
