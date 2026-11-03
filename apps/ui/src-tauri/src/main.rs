#![windows_subsystem = "windows"]

mod about;
mod agent;
mod capture;
mod commands;
mod history;
mod hwtests;
mod live;
mod tweaks;

use tauri::{Manager, RunEvent, WindowEvent};

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::timeline,
            commands::marks,
            commands::occurrence,
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
            commands::open_memory_diagnostic,
            commands::start_memory_test,
            commands::start_benchmark,
            commands::start_surface_scan,
            commands::start_volume_check,
            commands::stop_test,
            commands::test_running,
            about::about,
            about::open_data_folder,
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
