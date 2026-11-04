/// Commands the window may call; the capability file grants each one by name.
const COMMANDS: &[&str] = &[
    "timeline",
    "marks",
    "occurrence",
    "live_start",
    "live_stop",
    "agent_status",
    "start_agent",
    "set_paused",
    "set_sensitive",
    "set_autostart",
    "set_settings",
    "settings",
    "about",
    "open_data_folder",
    "clear_history",
    "start_capture",
    "capture_run",
    "import_captures",
    "captures",
    "capture",
    "health",
    "start_scan",
    "dumps",
    "start_dump_scan",
    "delete_dumps",
    "tweaks",
    "apply_tweaks",
    "undo_tweak",
    "tests",
    "disk_detail",
    "open_memory_diagnostic",
    "start_memory_test",
    "start_benchmark",
    "start_surface_scan",
    "start_volume_check",
    "stop_test",
    "test_running",
];

fn main() {
    // The window and its executable embed these icons, so a new icon rebuilds them.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("the Tauri build step failed");
}
