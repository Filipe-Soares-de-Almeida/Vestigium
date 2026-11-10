use vestigium_ipc::current_user_sid;
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::HSTRING;

use crate::app::StartError;

/// Proof that this process is the only watcher of its user in this session; released on drop.
#[derive(Debug)]
pub struct Instance {
    mutex: HANDLE,
}

impl Instance {
    /// Takes the per-user guard, or fails with `AlreadyRunning` when another watcher holds it.
    pub fn acquire() -> Result<Instance, StartError> {
        let sid = current_user_sid().map_err(|err| StartError::Failed(err.to_string()))?;
        Instance::named(&format!(r"Local\Vestigium-{sid}"))
    }

    /// Takes the guard called `name`; the mutex only has to exist, nobody waits on or owns it.
    pub fn named(name: &str) -> Result<Instance, StartError> {
        // SAFETY: an unowned mutex with default security and a name that outlives the call.
        let mutex = unsafe { CreateMutexW(None, false, &HSTRING::from(name)) }
            .map_err(|err| StartError::Failed(err.to_string()))?;
        // SAFETY: read straight after the call, before anything else can change it.
        let existed = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let instance = Instance { mutex };
        if existed {
            return Err(StartError::AlreadyRunning);
        }
        Ok(instance)
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        // SAFETY: the handle came from `CreateMutexW` and is closed exactly once.
        let _ = unsafe { CloseHandle(self.mutex) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique(tag: &str) -> String {
        format!(r"Local\Vestigium-test-{}-{tag}", std::process::id())
    }

    #[test]
    fn second_guard_is_refused_while_the_first_lives() {
        let name = unique("held");
        let _first = Instance::named(&name).unwrap();

        let second = Instance::named(&name);

        assert!(matches!(second, Err(StartError::AlreadyRunning)));
    }

    #[test]
    fn the_guard_is_free_after_the_first_drops() {
        let name = unique("freed");
        drop(Instance::named(&name).unwrap());

        assert!(Instance::named(&name).is_ok());
    }
}
