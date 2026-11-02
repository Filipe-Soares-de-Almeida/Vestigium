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
