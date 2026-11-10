use vestigium_collect::{DEEP_CAPTURE_SESSION, ProcessSnapshotter, session_exists, stop_session};

/// Image names of the helper, installed and development.
const HELPER_IMAGES: [&str; 2] = ["Vestigium.Helper.exe", "vestigium-helper.exe"];

/// Whether the deep capture's kernel session is left over: it exists and no helper is running.
fn should_stop_orphan(session_exists: bool, helper_running: bool) -> bool {
    session_exists && !helper_running
}

fn is_helper(image: &str) -> bool {
    HELPER_IMAGES
        .iter()
        .any(|name| image.eq_ignore_ascii_case(name))
}

/// Stops the deep capture's kernel session when no helper is running to own it; a stop that
/// is refused or fails is ignored.
pub fn release_orphaned_capture() {
    if !session_exists(DEEP_CAPTURE_SESSION) {
        return;
    }
    let helper_running = ProcessSnapshotter::with_capacity(1 << 20)
        .snapshot()
        .map_or(true, |all| all.iter().any(|p| is_helper(&p.name)));
    if should_stop_orphan(true, helper_running) {
        let _ = stop_session(DEEP_CAPTURE_SESSION);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_session_with_no_helper_is_an_orphan() {
        assert!(should_stop_orphan(true, false));
        assert!(!should_stop_orphan(true, true));
        assert!(!should_stop_orphan(false, false));
        assert!(!should_stop_orphan(false, true));
    }

    #[test]
    fn the_helper_is_known_by_either_image_name() {
        assert!(is_helper("Vestigium.Helper.exe"));
        assert!(is_helper("vestigium-helper.EXE"));
        assert!(!is_helper("Vestigium.exe"));
        assert!(!is_helper("helper.exe"));
    }

    #[test]
    fn a_session_that_does_not_exist_is_left_alone() {
        assert!(!session_exists("Vestigium Test Absent Session"));
    }
}
