use std::path::PathBuf;

use tauri::{Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::{
    db::Database,
    domain::{
        models::{
            AppSettings, BackupManifest, Category, ImportPreview, ImportResult, Note,
            OrganizerResult, SaveNoteInput, SaveNoteResponse, SearchHistoryItem, SearchQuery,
            SearchResult, Tag,
        },
        organizer::{OrganizerContext, OrganizerProvider, RuleOrganizer},
    },
    platform::window::{request_toggle_quick, show_main, show_quick},
};

fn emit_note_changed(app: &tauri::AppHandle, id: &str) {
    let _ = app.emit("note:changed", id);
}

#[tauri::command]
pub fn list_notes(
    db: State<'_, Database>,
    view: String,
    limit: Option<usize>,
) -> Result<Vec<Note>, String> {
    db.list_notes(&view, limit.unwrap_or(500))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_note(db: State<'_, Database>, id: String) -> Result<Note, String> {
    db.get_note(&id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_note(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    input: SaveNoteInput,
) -> Result<SaveNoteResponse, String> {
    let response = db.save_note(input).map_err(|error| error.to_string())?;
    if response.status != "conflict" {
        emit_note_changed(&app, &response.note.id);
    }
    Ok(response)
}

#[tauri::command]
pub fn soft_delete_note(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    id: String,
) -> Result<(), String> {
    db.soft_delete(&id).map_err(|error| error.to_string())?;
    emit_note_changed(&app, &id);
    Ok(())
}

#[tauri::command]
pub fn restore_note(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    id: String,
) -> Result<(), String> {
    db.restore_note(&id).map_err(|error| error.to_string())?;
    emit_note_changed(&app, &id);
    Ok(())
}

#[tauri::command]
pub fn permanently_delete_note(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    id: String,
) -> Result<(), String> {
    db.permanently_delete(&id)
        .map_err(|error| error.to_string())?;
    emit_note_changed(&app, &id);
    Ok(())
}

#[tauri::command]
pub fn set_note_archived(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    id: String,
    archived: bool,
) -> Result<Note, String> {
    let note = db
        .set_archived(&id, archived)
        .map_err(|error| error.to_string())?;
    emit_note_changed(&app, &id);
    Ok(note)
}

#[tauri::command]
pub fn list_categories(db: State<'_, Database>) -> Result<Vec<Category>, String> {
    db.list_categories().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_category(
    db: State<'_, Database>,
    name: String,
    color: String,
) -> Result<Category, String> {
    db.create_category(&name, &color)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_category(db: State<'_, Database>, id: String) -> Result<(), String> {
    db.delete_category(&id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_tags(db: State<'_, Database>) -> Result<Vec<Tag>, String> {
    db.list_tags().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_tag(db: State<'_, Database>, id: String) -> Result<(), String> {
    db.delete_tag(&id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn search_notes(
    db: State<'_, Database>,
    query: SearchQuery,
) -> Result<Vec<SearchResult>, String> {
    db.search(query).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_search_history(db: State<'_, Database>) -> Result<Vec<SearchHistoryItem>, String> {
    db.search_history().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_search_history(db: State<'_, Database>) -> Result<(), String> {
    db.clear_search_history().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_settings(db: State<'_, Database>) -> Result<AppSettings, String> {
    db.get_settings().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_settings(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    settings: AppSettings,
) -> Result<(), String> {
    let previous = db.get_settings().map_err(|error| error.to_string())?;
    if settings.shortcut != previous.shortcut {
        app.global_shortcut()
            .register(settings.shortcut.as_str())
            .map_err(|error| format!("快捷键不可用：{error}"))?;
    }
    if settings.autostart != previous.autostart {
        let manager = app.autolaunch();
        let result = if settings.autostart {
            manager.enable()
        } else {
            manager.disable()
        };
        if let Err(error) = result {
            if settings.shortcut != previous.shortcut {
                let _ = app.global_shortcut().unregister(settings.shortcut.as_str());
            }
            return Err(format!("更新开机启动失败：{error}"));
        }
    }
    db.update_settings(&settings)
        .map_err(|error| error.to_string())?;
    if settings.shortcut != previous.shortcut {
        let _ = app.global_shortcut().unregister(previous.shortcut.as_str());
    }
    app.emit("settings:changed", settings)
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn analyze_note(db: State<'_, Database>, id: String) -> Result<OrganizerResult, String> {
    let note = db.get_note(&id).map_err(|error| error.to_string())?;
    let settings = db.get_settings().map_err(|error| error.to_string())?;
    if !settings.organizer_enabled {
        return Ok(OrganizerResult {
            suggestions: Vec::new(),
            entities: Vec::new(),
        });
    }
    let context = OrganizerContext {
        categories: db.list_categories().map_err(|error| error.to_string())?,
        tags: db.list_tags().map_err(|error| error.to_string())?,
        category_keywords: db
            .category_keyword_profiles()
            .map_err(|error| error.to_string())?,
        other_titles: db.other_titles(&id).map_err(|error| error.to_string())?,
        archive_days: settings.archive_days,
    };
    let result = RuleOrganizer.analyze(&note, &context);
    db.persist_organizer_result(&id, &result)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_organizer_suggestion_status(
    db: State<'_, Database>,
    id: String,
    status: String,
) -> Result<(), String> {
    db.set_suggestion_status(&id, &status)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn export_json(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    path: String,
) -> Result<(), String> {
    db.export_json(
        &PathBuf::from(path),
        &app.package_info().version.to_string(),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn export_markdown(db: State<'_, Database>, directory: String) -> Result<usize, String> {
    db.export_markdown(&PathBuf::from(directory))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn preview_import(db: State<'_, Database>, path: String) -> Result<ImportPreview, String> {
    db.preview_import(&PathBuf::from(path))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn import_json(db: State<'_, Database>, path: String) -> Result<ImportResult, String> {
    db.import_json(&PathBuf::from(path))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_backup(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    path: String,
) -> Result<BackupManifest, String> {
    db.create_backup(
        &PathBuf::from(path),
        &app.package_info().version.to_string(),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn restore_backup(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    path: String,
) -> Result<String, String> {
    db.restore_backup(
        &PathBuf::from(path),
        &app.package_info().version.to_string(),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn show_quick_window(app: tauri::AppHandle, create_new: Option<bool>) -> Result<(), String> {
    show_quick(&app, create_new.unwrap_or(false))
}

#[tauri::command]
pub fn toggle_quick_window(app: tauri::AppHandle) -> Result<(), String> {
    request_toggle_quick(&app)
}

#[tauri::command]
pub fn hide_quick_window(app: tauri::AppHandle) -> Result<(), String> {
    app.get_webview_window("quick")
        .ok_or_else(|| "快速记录窗口不存在".to_string())?
        .hide()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hide_main_window(app: tauri::AppHandle) -> Result<(), String> {
    app.get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?
        .hide()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn show_main_window(app: tauri::AppHandle, note_id: Option<String>) -> Result<(), String> {
    show_main(&app, None)?;
    if let Some(note_id) = note_id {
        app.emit("app:open-note", note_id)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn database_info(db: State<'_, Database>) -> serde_json::Value {
    serde_json::json!({
        "path": db.path().to_string_lossy(),
        "fts5": db.fts_available(),
    })
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}
