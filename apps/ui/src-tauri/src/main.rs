#![windows_subsystem = "windows"]

mod about;
mod agent;
mod capture;
mod commands;
mod history;
mod hwtests;
mod live;
mod tweaks;
mod update;

use tauri::{Manager, RunEvent, WindowEvent};

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(commands::AppState::default())
        .manage(update::UpdateState::default())
        .invoke_handler(tauri::generate_handler![
            commands::timeline,
            commands::marks,
            commands::occurrence,
            commands::episode_detail,
            commands::live_start,
            commands::live_stop,
            commands::agent_status,
            commands::start_agent,
            commands::set_paused,
            commands::set_sensitive,
            commands::set_autostart,
            commands::clear_history,
            commands::set_settings,
            commands::settings,
            commands::start_capture,
            commands::capture_run,
            commands::import_captures,
            commands::captures,
            commands::capture,
            commands::health,
            commands::start_scan,
            commands::dumps,
            commands::start_dump_scan,
            commands::delete_dumps,
            commands::tweaks,
            commands::apply_tweaks,
            commands::undo_tweak,
            commands::tests,
            commands::disk_detail,
            commands::smart_history,
            commands::program_recent,
            commands::open_memory_diagnostic,
            commands::start_memory_test,
            commands::start_benchmark,
            commands::start_surface_scan,
            commands::start_volume_check,
            commands::stop_test,
            commands::test_running,
            about::about,
            about::open_data_folder,
            update::update_prefs,
            update::set_update_prefs,
            update::release_notes,
            update::update_check,
            update::update_install,
            update::update_cancel,
        ])
        .build(tauri::generate_context!())
        .expect("the interface failed to start");
    app.run(|handle, event| {
        // Closing the window ends the process: nothing stays behind in the background.
        if let RunEvent::WindowEvent {
            event: WindowEvent::Destroyed,
            ..
        } = event
        {
            commands::finish_tests(&handle.state::<commands::AppState>());
            handle.exit(0);
        }
    });
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    /// Command names registered with the invoke handler.
    fn registered() -> BTreeSet<String> {
        let source = include_str!("main.rs");
        let (_, rest) = source.split_once("generate_handler![").unwrap();
        let (list, _) = rest.split_once("])").unwrap();
        list.split(',')
            .filter_map(|path| path.trim().rsplit("::").next())
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn every_registered_command_is_in_the_app_manifest() {
        let build = include_str!("../build.rs");
        let (_, rest) = build.split_once("COMMANDS: &[&str] = &[").unwrap();
        let (list, _) = rest.split_once("];").unwrap();
        let manifest: BTreeSet<String> = list
            .split(',')
            .map(|name| name.trim().trim_matches('"').to_owned())
            .filter(|name| !name.is_empty())
            .collect();
        assert_eq!(manifest, registered());
    }

    #[test]
    fn every_registered_command_is_granted_to_the_window() {
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();
        let granted: BTreeSet<String> = capability["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|permission| permission.as_str()?.strip_prefix("allow-"))
            .map(|name| name.replace('-', "_"))
            .collect();
        assert_eq!(granted, registered());
    }

    #[test]
    fn the_app_version_comes_from_the_workspace_only() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert!(
            config.get("version").is_none(),
            "tauri.conf.json must not set its own version"
        );
    }

    #[test]
    fn a_released_version_has_its_changelog_section() {
        let version = env!("CARGO_PKG_VERSION");
        if version == "0.0.0" {
            return;
        }
        let changelog = include_str!("../../../../CHANGELOG.md");
        let heading = format!("## [{version}] - ");
        assert!(
            changelog.contains(&heading),
            "CHANGELOG.md has no '{heading}' section"
        );
    }

    #[test]
    fn the_updater_checks_signed_releases_over_https() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let updater: tauri_plugin_updater::Config =
            serde_json::from_value(config["plugins"]["updater"].clone()).unwrap();
        assert!(!updater.endpoints.is_empty());
        assert!(updater.endpoints.iter().all(|url| url.scheme() == "https"));
        assert!(!updater.pubkey.is_empty());
        assert!(!updater.dangerous_insecure_transport_protocol);
        assert!(!updater.allow_downgrades);
        assert!(updater.require_signed_version);
    }
}
