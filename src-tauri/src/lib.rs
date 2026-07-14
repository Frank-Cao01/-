mod commands;
mod db;
mod domain;
mod error;
mod platform;

use db::Database;
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = platform::window::request_toggle_quick(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = platform::window::show_main(app, None);
        }))
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            let database = Database::open(data_dir.join("shanji.db"))
                .map_err(|error| Box::<dyn std::error::Error>::from(error.to_string()))?;
            let settings = database
                .get_settings()
                .map_err(|error| Box::<dyn std::error::Error>::from(error.to_string()))?;
            app.manage(database);

            if let Err(error) = app.global_shortcut().register(settings.shortcut.as_str()) {
                log::warn!("无法注册全局快捷键 {}: {}", settings.shortcut, error);
            }
            platform::tray::setup(app)?;

            if std::env::args().any(|arg| arg == "--background") {
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.hide();
                }
            } else {
                let _ = platform::window::show_main(app.handle(), None);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } if window.label() == "quick" => {
                api.prevent_close();
                let _ = window.app_handle().emit("quick:toggle-request", ());
            }
            WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                api.prevent_close();
                let close_to_tray = window
                    .app_handle()
                    .try_state::<Database>()
                    .map(|database| {
                        database
                            .get_settings()
                            .map(|settings| settings.close_behavior == "tray")
                            .unwrap_or(true)
                    })
                    .unwrap_or(true);
                let event = if close_to_tray {
                    "main:hide-request"
                } else {
                    "app:quit-request"
                };
                let _ = window.app_handle().emit(event, ());
            }
            WindowEvent::Focused(false) if window.label() == "quick" => {
                if let Some(database) = window.app_handle().try_state::<Database>() {
                    if database
                        .get_settings()
                        .map(|settings| settings.hide_on_blur && !settings.quick_pinned)
                        .unwrap_or(true)
                    {
                        let _ = window.app_handle().emit("quick:toggle-request", ());
                    }
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_notes,
            commands::get_note,
            commands::save_note,
            commands::soft_delete_note,
            commands::restore_note,
            commands::permanently_delete_note,
            commands::set_note_archived,
            commands::list_categories,
            commands::create_category,
            commands::delete_category,
            commands::list_tags,
            commands::delete_tag,
            commands::search_notes,
            commands::get_search_history,
            commands::clear_search_history,
            commands::get_settings,
            commands::update_settings,
            commands::analyze_note,
            commands::set_organizer_suggestion_status,
            commands::export_json,
            commands::export_markdown,
            commands::preview_import,
            commands::import_json,
            commands::create_backup,
            commands::restore_backup,
            commands::show_quick_window,
            commands::toggle_quick_window,
            commands::hide_quick_window,
            commands::hide_main_window,
            commands::show_main_window,
            commands::database_info,
            commands::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
