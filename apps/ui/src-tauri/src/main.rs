#![windows_subsystem = "windows"]

use tauri::{Manager, RunEvent, WindowEvent};

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .build(tauri::generate_context!())
        .expect("the interface failed to start");
    app.run(|handle, event| {
        // Closing the window ends the process: nothing stays behind in the background.
        if let RunEvent::WindowEvent {
            event: WindowEvent::Destroyed,
            ..
        } = event
        {
            handle.exit(0);
        }
    });
}
