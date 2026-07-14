use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
    pub color: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub category_id: Option<String>,
    pub category: Option<Category>,
    pub tags: Vec<Tag>,
    pub is_favorite: bool,
    pub is_todo: bool,
    pub todo_status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub completed_at: Option<String>,
    pub source: String,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SaveNoteInput {
    pub id: Option<String>,
    pub expected_revision: Option<i64>,
    pub title: String,
    pub content: String,
    pub category_id: Option<String>,
    pub tag_names: Vec<String>,
    pub is_favorite: bool,
    pub is_todo: bool,
    pub todo_status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub source: String,
}

impl Default for SaveNoteInput {
    fn default() -> Self {
        Self {
            id: None,
            expected_revision: None,
            title: String::new(),
            content: String::new(),
            category_id: None,
            tag_names: Vec::new(),
            is_favorite: false,
            is_todo: false,
            todo_status: "not_started".to_string(),
            priority: "normal".to_string(),
            due_date: None,
            source: "main".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveNoteResponse {
    pub status: String,
    pub note: Note,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchQuery {
    pub query: String,
    pub view: Option<String>,
    pub category_id: Option<String>,
    pub tag_ids: Vec<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub is_favorite: Option<bool>,
    pub todo_status: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub note: Note,
    pub matched_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHistoryItem {
    pub id: String,
    pub query: String,
    pub filters_json: String,
    pub searched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: String,
    pub close_behavior: String,
    pub hide_on_blur: bool,
    pub quick_pinned: bool,
    pub autostart: bool,
    pub shortcut: String,
    pub quick_active_note_id: Option<String>,
    pub organizer_enabled: bool,
    pub archive_days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityMatch {
    pub entity_type: String,
    pub value: String,
    pub start_offset: usize,
    pub end_offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub value: serde_json::Value,
    pub confidence: f64,
    pub rule_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizerResult {
    pub suggestions: Vec<Suggestion>,
    pub entities: Vec<EntityMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEnvelope {
    pub schema_version: i64,
    pub app_version: String,
    pub exported_at: String,
    pub categories: Vec<Category>,
    pub tags: Vec<Tag>,
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub notes_to_create: usize,
    pub duplicates_to_skip: usize,
    pub conflicts_as_copies: usize,
    pub categories_to_merge: usize,
    pub tags_to_merge: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub created: usize,
    pub skipped: usize,
    pub copied_conflicts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub schema_version: i64,
    pub app_version: String,
    pub created_at: String,
    pub database_sha256: String,
}
