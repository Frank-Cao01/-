use crate::db::Database;
use tauri::{Emitter, Manager, PhysicalPosition, WebviewWindow};

pub fn position_on_cursor_monitor(
    app: &tauri::AppHandle,
    window: &WebviewWindow,
) -> Result<(), String> {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|position| {
            app.monitor_from_point(position.x, position.y)
                .ok()
                .flatten()
        })
        .or_else(|| {
            app.get_webview_window("main")
                .and_then(|main| main.current_monitor().ok().flatten())
        })
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or_else(|| "无法获取当前显示器".to_string())?;

    let area = monitor.work_area();
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let x = area.position.x + ((area.size.width as i32 - size.width as i32) / 2);
    let y = area.position.y + ((area.size.height as i32 - size.height as i32) / 2);
    let max_x = area.position.x + area.size.width as i32 - size.width as i32;
    let max_y = area.position.y + area.size.height as i32 - size.height as i32;
    window
        .set_position(PhysicalPosition::new(
            x.clamp(area.position.x, max_x.max(area.position.x)),
            y.clamp(area.position.y, max_y.max(area.position.y)),
        ))
        .map_err(|error| error.to_string())
}

pub fn show_quick(app: &tauri::AppHandle, create_new: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("quick")
        .ok_or_else(|| "快速记录窗口不存在".to_string())?;
    if create_new {
        app.emit("quick:new", ())
            .map_err(|error| error.to_string())?;
    }
    // 固定后保留用户拖动得到的位置；取消固定时仍在鼠标所在屏幕居中。
    let quick_pinned = app
        .try_state::<Database>()
        .and_then(|database| database.get_settings().ok())
        .map(|settings| settings.quick_pinned)
        .unwrap_or(false);
    if !quick_pinned {
        position_on_cursor_monitor(app, &window)?;
    }
    let _ = window.unminimize();
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    app.emit("quick:focus-editor", ())
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn request_toggle_quick(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("quick")
        .ok_or_else(|| "快速记录窗口不存在".to_string())?;
    if window.is_visible().unwrap_or(false) {
        app.emit("quick:toggle-request", ())
            .map_err(|error| error.to_string())?;
    } else {
        show_quick(app, false)?;
    }
    Ok(())
}

pub fn show_main(app: &tauri::AppHandle, route: Option<&str>) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    let _ = window.unminimize();
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    if let Some(route) = route {
        app.emit("app:navigate", route)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}
