use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter,
};

use super::window::{request_toggle_quick, show_main, show_quick};

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let product_name = app
        .config()
        .product_name
        .as_deref()
        .unwrap_or("Shanji")
        .to_string();
    let quick_new = MenuItemBuilder::with_id("quick_new", "快速新建").build(app)?;
    let open_main = MenuItemBuilder::with_id("open_main", "打开主界面").build(app)?;
    let toggle_quick = MenuItemBuilder::with_id("toggle_quick", "显示或隐藏快速记录").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "设置").build(app)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItemBuilder::with_id("quit", format!("退出{product_name}")).build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[
            &quick_new,
            &open_main,
            &toggle_quick,
            &settings,
            &separator,
            &quit,
        ])
        .build()?;

    TrayIconBuilder::with_id("main-tray")
        .tooltip(&product_name)
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "quick_new" => {
                let _ = show_quick(app, true);
            }
            "open_main" => {
                let _ = show_main(app, None);
            }
            "toggle_quick" => {
                let _ = request_toggle_quick(app);
            }
            "settings" => {
                let _ = show_main(app, Some("settings"));
            }
            "quit" => {
                let _ = app.emit("app:quit-request", ());
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = request_toggle_quick(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
