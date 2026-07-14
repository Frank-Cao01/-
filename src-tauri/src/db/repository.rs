use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::{
    domain::models::{
        AppSettings, BackupManifest, Category, ExportEnvelope, ImportPreview, ImportResult, Note,
        OrganizerResult, SaveNoteInput, SaveNoteResponse, SearchHistoryItem, SearchQuery,
        SearchResult, Tag,
    },
    error::{AppError, AppResult},
};

const INITIAL_MIGRATION: &str = include_str!("../../migrations/0001_initial.sql");

pub struct Database {
    connection: Mutex<Connection>,
    path: PathBuf,
    fts_available: bool,
}

#[derive(Debug)]
struct NoteFlat {
    id: String,
    title: String,
    content: String,
    category_id: Option<String>,
    is_favorite: bool,
    is_todo: bool,
    todo_status: String,
    priority: String,
    due_date: Option<String>,
    completed_at: Option<String>,
    source: String,
    revision: i64,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
    archived_at: Option<String>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut connection = Connection::open(&path)?;
        Self::configure(&connection)?;
        Self::migrate(&mut connection)?;
        let fts_available = connection
            .query_row(
                "SELECT sqlite_compileoption_used('ENABLE_FTS5')",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            == 1;
        let database = Self {
            connection: Mutex::new(connection),
            path,
            fts_available,
        };
        database.ensure_default_settings()?;
        Ok(database)
    }

    #[cfg(test)]
    pub fn in_memory() -> AppResult<Self> {
        let mut connection = Connection::open_in_memory()?;
        Self::configure(&connection)?;
        Self::migrate(&mut connection)?;
        let fts_available = connection
            .query_row(
                "SELECT sqlite_compileoption_used('ENABLE_FTS5')",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            == 1;
        let database = Self {
            connection: Mutex::new(connection),
            path: PathBuf::from(":memory:"),
            fts_available,
        };
        database.ensure_default_settings()?;
        Ok(database)
    }

    fn configure(connection: &Connection) -> AppResult<()> {
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;",
        )?;
        Ok(())
    }

    fn migrate(connection: &mut Connection) -> AppResult<()> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
               version INTEGER PRIMARY KEY NOT NULL,
               description TEXT NOT NULL,
               checksum TEXT NOT NULL,
               applied_at TEXT NOT NULL
             );",
        )?;
        let applied = connection
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .is_some();
        if !applied {
            let transaction = connection.transaction()?;
            transaction.execute_batch(INITIAL_MIGRATION)?;
            let checksum = format!("{:x}", Sha256::digest(INITIAL_MIGRATION.as_bytes()));
            transaction.execute(
                "INSERT INTO schema_migrations(version, description, checksum, applied_at)
                 VALUES(1, 'initial schema', ?1, ?2)",
                params![checksum, Utc::now().to_rfc3339()],
            )?;
            transaction.commit()?;
        }
        Ok(())
    }

    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|_| AppError::Validation("数据库连接状态异常".into()))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn fts_available(&self) -> bool {
        self.fts_available
    }

    fn now() -> String {
        Utc::now().to_rfc3339()
    }

    fn normalize(value: &str) -> String {
        value.trim().to_lowercase()
    }

    fn content_hash(title: &str, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        hasher.update([0]);
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn validate_input(input: &SaveNoteInput) -> AppResult<()> {
        if !["not_started", "in_progress", "completed"].contains(&input.todo_status.as_str()) {
            return Err(AppError::Validation("无效的待办状态".into()));
        }
        if !["low", "normal", "high", "urgent"].contains(&input.priority.as_str()) {
            return Err(AppError::Validation("无效的优先级".into()));
        }
        if !["main", "quick", "import"].contains(&input.source.as_str()) {
            return Err(AppError::Validation("无效的笔记来源".into()));
        }
        if input.title.chars().count() > 500 {
            return Err(AppError::Validation("标题不能超过 500 个字符".into()));
        }
        Ok(())
    }

    fn flat_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NoteFlat> {
        Ok(NoteFlat {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
            category_id: row.get(3)?,
            is_favorite: row.get::<_, i64>(4)? == 1,
            is_todo: row.get::<_, i64>(5)? == 1,
            todo_status: row.get(6)?,
            priority: row.get(7)?,
            due_date: row.get(8)?,
            completed_at: row.get(9)?,
            source: row.get(10)?,
            revision: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
            deleted_at: row.get(14)?,
            archived_at: row.get(15)?,
        })
    }

    fn category_by_id(connection: &Connection, id: &str) -> AppResult<Option<Category>> {
        Ok(connection
            .query_row(
                "SELECT id, name, color, sort_order, created_at, updated_at FROM categories WHERE id = ?1",
                [id],
                |row| {
                    Ok(Category {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        color: row.get(2)?,
                        sort_order: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                },
            )
            .optional()?)
    }

    fn tags_for_note(connection: &Connection, note_id: &str) -> AppResult<Vec<Tag>> {
        let mut statement = connection.prepare(
            "SELECT t.id, t.name, t.created_at, t.updated_at
             FROM tags t
             JOIN note_tags nt ON nt.tag_id = t.id
             WHERE nt.note_id = ?1
             ORDER BY t.name COLLATE NOCASE",
        )?;
        let rows = statement.query_map([note_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn hydrate(connection: &Connection, flat: NoteFlat) -> AppResult<Note> {
        let category = match &flat.category_id {
            Some(id) => Self::category_by_id(connection, id)?,
            None => None,
        };
        let tags = Self::tags_for_note(connection, &flat.id)?;
        Ok(Note {
            id: flat.id,
            title: flat.title,
            content: flat.content,
            category_id: flat.category_id,
            category,
            tags,
            is_favorite: flat.is_favorite,
            is_todo: flat.is_todo,
            todo_status: flat.todo_status,
            priority: flat.priority,
            due_date: flat.due_date,
            completed_at: flat.completed_at,
            source: flat.source,
            revision: flat.revision,
            created_at: flat.created_at,
            updated_at: flat.updated_at,
            deleted_at: flat.deleted_at,
            archived_at: flat.archived_at,
        })
    }

    fn read_note(connection: &Connection, id: &str) -> AppResult<Note> {
        let flat = connection
            .query_row(
                "SELECT id, title, content, category_id, is_favorite, is_todo, todo_status,
                        priority, due_date, completed_at, source, revision, created_at, updated_at,
                        deleted_at, archived_at
                 FROM notes WHERE id = ?1",
                [id],
                Self::flat_from_row,
            )
            .optional()?
            .ok_or(AppError::NotFound)?;
        Self::hydrate(connection, flat)
    }

    pub fn get_note(&self, id: &str) -> AppResult<Note> {
        let connection = self.lock()?;
        Self::read_note(&connection, id)
    }

    pub fn list_notes(&self, view: &str, limit: usize) -> AppResult<Vec<Note>> {
        let connection = self.lock()?;
        Self::list_notes_with_connection(&connection, view, limit)
    }

    fn list_notes_with_connection(
        connection: &Connection,
        view: &str,
        limit: usize,
    ) -> AppResult<Vec<Note>> {
        let condition = match view {
            "today" => "deleted_at IS NULL AND archived_at IS NULL AND date(updated_at, 'localtime') = date('now', 'localtime')",
            "todo" => "deleted_at IS NULL AND archived_at IS NULL AND is_todo = 1",
            "favorite" => "deleted_at IS NULL AND archived_at IS NULL AND is_favorite = 1",
            "archive" => "deleted_at IS NULL AND archived_at IS NOT NULL",
            "trash" => "deleted_at IS NOT NULL",
            _ => "deleted_at IS NULL AND archived_at IS NULL",
        };
        let sql = format!(
            "SELECT id, title, content, category_id, is_favorite, is_todo, todo_status,
                    priority, due_date, completed_at, source, revision, created_at, updated_at,
                    deleted_at, archived_at
             FROM notes WHERE {condition} ORDER BY updated_at DESC LIMIT ?1"
        );
        let mut statement = connection.prepare(&sql)?;
        let flats = statement
            .query_map([limit.min(100_000) as i64], Self::flat_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);
        flats
            .into_iter()
            .map(|flat| Self::hydrate(connection, flat))
            .collect()
    }

    fn ensure_tag(transaction: &Transaction<'_>, name: &str, now: &str) -> AppResult<String> {
        let normalized = Self::normalize(name);
        if normalized.is_empty() {
            return Err(AppError::Validation("标签名称不能为空".into()));
        }
        if let Some(id) = transaction
            .query_row(
                "SELECT id FROM tags WHERE normalized_name = ?1",
                [&normalized],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(id);
        }
        let id = Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO tags(id, name, normalized_name, created_at, updated_at) VALUES(?1, ?2, ?3, ?4, ?4)",
            params![id, name.trim(), normalized, now],
        )?;
        Ok(id)
    }

    fn reindex_note(transaction: &Transaction<'_>, note_id: &str) -> AppResult<()> {
        transaction.execute("DELETE FROM notes_fts WHERE note_id = ?1", [note_id])?;
        let row = transaction
            .query_row(
                "SELECT n.title, n.content, n.created_at, n.updated_at, n.due_date,
                        COALESCE(c.name, ''), n.deleted_at
                 FROM notes n LEFT JOIN categories c ON c.id = n.category_id WHERE n.id = ?1",
                [note_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                },
            )
            .optional()?;
        let Some((title, content, created_at, updated_at, due_date, category, deleted_at)) = row
        else {
            return Ok(());
        };
        if deleted_at.is_some() {
            return Ok(());
        }
        let tags: String = transaction.query_row(
            "SELECT COALESCE(group_concat(t.name, ' '), '')
             FROM tags t JOIN note_tags nt ON nt.tag_id = t.id WHERE nt.note_id = ?1",
            [note_id],
            |row| row.get(0),
        )?;
        let dates = format!(
            "{} {} {}",
            created_at,
            updated_at,
            due_date.unwrap_or_default()
        );
        transaction.execute(
            "INSERT INTO notes_fts(note_id, title, content, tags_text, category_text, dates_text)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![note_id, title, content, tags, category, dates],
        )?;
        Ok(())
    }

    pub fn save_note(&self, input: SaveNoteInput) -> AppResult<SaveNoteResponse> {
        Self::validate_input(&input)?;
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let now = Self::now();
        let hash = Self::content_hash(&input.title, &input.content);

        let (id, response_status) = if let Some(id) = input.id.clone() {
            let (current_revision, current_completed_at) = transaction
                .query_row(
                    "SELECT revision, completed_at FROM notes WHERE id = ?1",
                    [&id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
                )
                .optional()?
                .ok_or(AppError::NotFound)?;
            if input.expected_revision.unwrap_or(current_revision) != current_revision {
                let latest = Self::read_note(&transaction, &id)?;
                return Ok(SaveNoteResponse {
                    status: "conflict".into(),
                    note: latest,
                });
            }
            let completed_at = if input.is_todo && input.todo_status == "completed" {
                current_completed_at.or_else(|| Some(now.clone()))
            } else {
                None
            };
            transaction.execute(
                "UPDATE notes SET title = ?2, content = ?3, category_id = ?4, is_favorite = ?5,
                    is_todo = ?6, todo_status = ?7, priority = ?8, due_date = ?9,
                    completed_at = ?10, source = ?11, revision = revision + 1,
                    content_hash = ?12, updated_at = ?13 WHERE id = ?1",
                params![
                    id,
                    input.title,
                    input.content,
                    input.category_id,
                    input.is_favorite as i64,
                    input.is_todo as i64,
                    input.todo_status,
                    input.priority,
                    input.due_date,
                    completed_at,
                    input.source,
                    hash,
                    now,
                ],
            )?;
            (id, "saved")
        } else {
            let id = Uuid::new_v4().to_string();
            let completed_at = if input.is_todo && input.todo_status == "completed" {
                Some(now.clone())
            } else {
                None
            };
            transaction.execute(
                "INSERT INTO notes(id, title, content, category_id, is_favorite, is_todo,
                    todo_status, priority, due_date, completed_at, source, revision, content_hash,
                    created_at, updated_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, ?12, ?13, ?13)",
                params![
                    id,
                    input.title,
                    input.content,
                    input.category_id,
                    input.is_favorite as i64,
                    input.is_todo as i64,
                    input.todo_status,
                    input.priority,
                    input.due_date,
                    completed_at,
                    input.source,
                    hash,
                    now,
                ],
            )?;
            (id, "created")
        };

        transaction.execute("DELETE FROM note_tags WHERE note_id = ?1", [&id])?;
        let mut normalized_seen = HashSet::new();
        for name in input
            .tag_names
            .iter()
            .map(|name| name.trim())
            .filter(|name| !name.is_empty())
        {
            let normalized = Self::normalize(name);
            if normalized_seen.insert(normalized) {
                let tag_id = Self::ensure_tag(&transaction, name, &now)?;
                transaction.execute(
                    "INSERT OR IGNORE INTO note_tags(note_id, tag_id) VALUES(?1, ?2)",
                    params![id, tag_id],
                )?;
            }
        }
        Self::reindex_note(&transaction, &id)?;
        transaction.commit()?;
        let note = Self::read_note(&connection, &id)?;
        Ok(SaveNoteResponse {
            status: response_status.into(),
            note,
        })
    }

    pub fn soft_delete(&self, id: &str) -> AppResult<()> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "UPDATE notes SET deleted_at = ?2, updated_at = ?2, revision = revision + 1 WHERE id = ?1",
            params![id, Self::now()],
        )?;
        Self::reindex_note(&transaction, id)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn restore_note(&self, id: &str) -> AppResult<()> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "UPDATE notes SET deleted_at = NULL, updated_at = ?2, revision = revision + 1 WHERE id = ?1",
            params![id, Self::now()],
        )?;
        Self::reindex_note(&transaction, id)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn permanently_delete(&self, id: &str) -> AppResult<()> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM notes_fts WHERE note_id = ?1", [id])?;
        transaction.execute(
            "DELETE FROM notes WHERE id = ?1 AND deleted_at IS NOT NULL",
            [id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn set_archived(&self, id: &str, archived: bool) -> AppResult<Note> {
        let connection = self.lock()?;
        connection.execute(
            "UPDATE notes SET archived_at = ?2, updated_at = ?3, revision = revision + 1 WHERE id = ?1",
            params![id, archived.then(Self::now), Self::now()],
        )?;
        Self::read_note(&connection, id)
    }

    pub fn list_categories(&self) -> AppResult<Vec<Category>> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT id, name, color, sort_order, created_at, updated_at
             FROM categories ORDER BY sort_order, name COLLATE NOCASE",
        )?;
        let items = statement
            .query_map([], |row| {
                Ok(Category {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    sort_order: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(items)
    }

    pub fn create_category(&self, name: &str, color: &str) -> AppResult<Category> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Validation("分类名称不能为空".into()));
        }
        let connection = self.lock()?;
        let id = Uuid::new_v4().to_string();
        let now = Self::now();
        let sort_order: i64 = connection.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM categories",
            [],
            |row| row.get(0),
        )?;
        connection.execute(
            "INSERT INTO categories(id, name, normalized_name, color, sort_order, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![id, name, Self::normalize(name), color, sort_order, now],
        )?;
        Self::category_by_id(&connection, &id)?.ok_or(AppError::NotFound)
    }

    pub fn delete_category(&self, id: &str) -> AppResult<()> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let note_ids = {
            let mut statement =
                transaction.prepare("SELECT id FROM notes WHERE category_id = ?1")?;
            let items = statement
                .query_map([id], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            items
        };
        transaction.execute(
            "UPDATE notes SET category_id = NULL WHERE category_id = ?1",
            [id],
        )?;
        transaction.execute("DELETE FROM categories WHERE id = ?1", [id])?;
        for note_id in note_ids {
            Self::reindex_note(&transaction, &note_id)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn list_tags(&self) -> AppResult<Vec<Tag>> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT id, name, created_at, updated_at FROM tags ORDER BY name COLLATE NOCASE",
        )?;
        let items = statement
            .query_map([], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(items)
    }

    pub fn delete_tag(&self, id: &str) -> AppResult<()> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let note_ids = {
            let mut statement =
                transaction.prepare("SELECT note_id FROM note_tags WHERE tag_id = ?1")?;
            let items = statement
                .query_map([id], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            items
        };
        transaction.execute("DELETE FROM tags WHERE id = ?1", [id])?;
        for note_id in note_ids {
            Self::reindex_note(&transaction, &note_id)?;
        }
        transaction.commit()?;
        Ok(())
    }

    fn note_matches_filters(note: &Note, query: &SearchQuery) -> bool {
        let today = chrono::Local::now().date_naive().to_string();
        match query.view.as_deref().unwrap_or("all") {
            "today"
                if note.deleted_at.is_some()
                    || note.archived_at.is_some()
                    || note.updated_at.get(0..10) != Some(today.as_str()) =>
            {
                return false
            }
            "todo" if note.deleted_at.is_some() || note.archived_at.is_some() || !note.is_todo => {
                return false
            }
            "favorite"
                if note.deleted_at.is_some() || note.archived_at.is_some() || !note.is_favorite =>
            {
                return false
            }
            "archive" if note.deleted_at.is_some() || note.archived_at.is_none() => return false,
            "trash" if note.deleted_at.is_none() => return false,
            "all" if note.deleted_at.is_some() || note.archived_at.is_some() => return false,
            _ => {}
        }
        if let Some(category_id) = &query.category_id {
            if note.category_id.as_ref() != Some(category_id) {
                return false;
            }
        }
        if !query.tag_ids.is_empty()
            && !query
                .tag_ids
                .iter()
                .all(|id| note.tags.iter().any(|tag| &tag.id == id))
        {
            return false;
        }
        if let Some(value) = query.is_favorite {
            if note.is_favorite != value {
                return false;
            }
        }
        if let Some(todo_status) = &query.todo_status {
            if !note.is_todo || &note.todo_status != todo_status {
                return false;
            }
        }
        let date = note.updated_at.get(0..10).unwrap_or(&note.updated_at);
        if query
            .date_from
            .as_ref()
            .is_some_and(|from| date < from.as_str())
        {
            return false;
        }
        if query.date_to.as_ref().is_some_and(|to| date > to.as_str()) {
            return false;
        }
        true
    }

    pub fn search(&self, query: SearchQuery) -> AppResult<Vec<SearchResult>> {
        let connection = self.lock()?;
        let terms: Vec<String> = query
            .query
            .split_whitespace()
            .map(str::trim)
            .filter(|term| !term.is_empty())
            .map(str::to_lowercase)
            .collect();
        let view = query.view.as_deref().unwrap_or("all");
        let limit = query.limit.unwrap_or(200).min(500);

        let mut notes = if !terms.is_empty()
            && self.fts_available
            && view != "trash"
            && terms.iter().all(|term| term.chars().count() >= 3)
        {
            let expression = terms
                .iter()
                .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(" AND ");
            let mut statement = connection.prepare(
                "SELECT note_id FROM notes_fts WHERE notes_fts MATCH ?1
                 ORDER BY bm25(notes_fts, 0.0, 10.0, 1.0, 4.0, 3.0, 0.5) LIMIT ?2",
            )?;
            let ids = statement
                .query_map(params![expression, limit as i64 * 3], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            drop(statement);
            ids.into_iter()
                .filter_map(|id| Self::read_note(&connection, &id).ok())
                .collect::<Vec<_>>()
        } else {
            Self::list_notes_with_connection(&connection, view, 2000)?
                .into_iter()
                .filter(|note| {
                    let haystack = format!(
                        "{}\n{}\n{}\n{}\n{}",
                        note.title,
                        note.content,
                        note.tags
                            .iter()
                            .map(|tag| tag.name.as_str())
                            .collect::<Vec<_>>()
                            .join(" "),
                        note.category
                            .as_ref()
                            .map(|item| item.name.as_str())
                            .unwrap_or(""),
                        note.due_date.as_deref().unwrap_or("")
                    )
                    .to_lowercase();
                    terms.iter().all(|term| haystack.contains(term))
                })
                .collect()
        };
        notes.retain(|note| Self::note_matches_filters(note, &query));
        notes.truncate(limit);

        if !query.query.trim().is_empty() {
            let filters_json = serde_json::to_string(&query)?;
            let normalized = query.query.trim().to_lowercase();
            connection.execute(
                "DELETE FROM search_history WHERE normalized_query = ?1 AND filters_json = ?2",
                params![normalized, filters_json],
            )?;
            connection.execute(
                "INSERT INTO search_history(id, query, normalized_query, filters_json, searched_at)
                 VALUES(?1, ?2, ?3, ?4, ?5)",
                params![
                    Uuid::new_v4().to_string(),
                    query.query.trim(),
                    normalized,
                    filters_json,
                    Self::now()
                ],
            )?;
            connection.execute(
                "DELETE FROM search_history WHERE id NOT IN (
                   SELECT id FROM search_history ORDER BY searched_at DESC LIMIT 20
                 )",
                [],
            )?;
        }

        Ok(notes
            .into_iter()
            .map(|note| SearchResult {
                note,
                matched_terms: terms.clone(),
            })
            .collect())
    }

    pub fn search_history(&self) -> AppResult<Vec<SearchHistoryItem>> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT id, query, filters_json, searched_at FROM search_history ORDER BY searched_at DESC LIMIT 20",
        )?;
        let items = statement
            .query_map([], |row| {
                Ok(SearchHistoryItem {
                    id: row.get(0)?,
                    query: row.get(1)?,
                    filters_json: row.get(2)?,
                    searched_at: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(items)
    }

    pub fn clear_search_history(&self) -> AppResult<()> {
        self.lock()?.execute("DELETE FROM search_history", [])?;
        Ok(())
    }

    fn ensure_default_settings(&self) -> AppResult<()> {
        let defaults = [
            ("theme", serde_json::json!("light")),
            ("close_behavior", serde_json::json!("tray")),
            ("hide_on_blur", serde_json::json!(true)),
            ("quick_pinned", serde_json::json!(false)),
            ("autostart", serde_json::json!(false)),
            (
                "shortcut",
                serde_json::json!(if cfg!(target_os = "macos") {
                    "Command+Shift+Space"
                } else {
                    "Ctrl+Shift+Space"
                }),
            ),
            ("quick_active_note_id", serde_json::Value::Null),
            ("organizer_enabled", serde_json::json!(true)),
            ("archive_days", serde_json::json!(90)),
        ];
        let connection = self.lock()?;
        let now = Self::now();
        for (key, value) in defaults {
            connection.execute(
                "INSERT OR IGNORE INTO settings(key, value_json, updated_at) VALUES(?1, ?2, ?3)",
                params![key, value.to_string(), now],
            )?;
        }
        Ok(())
    }

    fn setting_value(connection: &Connection, key: &str) -> AppResult<serde_json::Value> {
        let raw: String = connection.query_row(
            "SELECT value_json FROM settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn get_settings(&self) -> AppResult<AppSettings> {
        let connection = self.lock()?;
        Ok(AppSettings {
            theme: serde_json::from_value(Self::setting_value(&connection, "theme")?)?,
            close_behavior: serde_json::from_value(Self::setting_value(
                &connection,
                "close_behavior",
            )?)?,
            hide_on_blur: serde_json::from_value(Self::setting_value(
                &connection,
                "hide_on_blur",
            )?)?,
            quick_pinned: serde_json::from_value(Self::setting_value(
                &connection,
                "quick_pinned",
            )?)?,
            autostart: serde_json::from_value(Self::setting_value(&connection, "autostart")?)?,
            shortcut: serde_json::from_value(Self::setting_value(&connection, "shortcut")?)?,
            quick_active_note_id: serde_json::from_value(Self::setting_value(
                &connection,
                "quick_active_note_id",
            )?)?,
            organizer_enabled: serde_json::from_value(Self::setting_value(
                &connection,
                "organizer_enabled",
            )?)?,
            archive_days: serde_json::from_value(Self::setting_value(
                &connection,
                "archive_days",
            )?)?,
        })
    }

    pub fn update_settings(&self, settings: &AppSettings) -> AppResult<()> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let now = Self::now();
        let values = [
            ("theme", serde_json::to_value(&settings.theme)?),
            (
                "close_behavior",
                serde_json::to_value(&settings.close_behavior)?,
            ),
            ("hide_on_blur", serde_json::to_value(settings.hide_on_blur)?),
            ("quick_pinned", serde_json::to_value(settings.quick_pinned)?),
            ("autostart", serde_json::to_value(settings.autostart)?),
            ("shortcut", serde_json::to_value(&settings.shortcut)?),
            (
                "quick_active_note_id",
                serde_json::to_value(&settings.quick_active_note_id)?,
            ),
            (
                "organizer_enabled",
                serde_json::to_value(settings.organizer_enabled)?,
            ),
            ("archive_days", serde_json::to_value(settings.archive_days)?),
        ];
        for (key, value) in values {
            transaction.execute(
                "INSERT INTO settings(key, value_json, updated_at) VALUES(?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
                params![key, value.to_string(), now],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// 将规则分析结果写入扩展表，并只返回仍处于待处理状态的建议。
    pub fn persist_organizer_result(
        &self,
        note_id: &str,
        result: &OrganizerResult,
    ) -> AppResult<OrganizerResult> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let now = Self::now();

        transaction.execute("DELETE FROM note_entities WHERE note_id = ?1", [note_id])?;
        for entity in &result.entities {
            transaction.execute(
                "INSERT INTO note_entities(
                    id, note_id, entity_type, value, start_offset, end_offset, created_at
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    Uuid::new_v4().to_string(),
                    note_id,
                    entity.entity_type,
                    entity.value,
                    entity.start_offset as i64,
                    entity.end_offset as i64,
                    now,
                ],
            )?;
        }

        // 已经不再适用的待处理建议应移除；accepted/dismissed 历史保留用于去重。
        transaction.execute(
            "DELETE FROM note_suggestions WHERE note_id = ?1 AND status = 'pending'",
            [note_id],
        )?;
        let mut pending = Vec::new();
        for suggestion in &result.suggestions {
            let payload = serde_json::to_string(suggestion)?;
            let fingerprint = format!(
                "{:x}",
                Sha256::digest(
                    format!(
                        "{}\u{0}{}\u{0}{}",
                        suggestion.kind, suggestion.rule_id, suggestion.value
                    )
                    .as_bytes()
                )
            );
            transaction.execute(
                "INSERT INTO note_suggestions(
                    id, note_id, kind, payload_json, rule_id, confidence, status,
                    fingerprint, created_at, updated_at
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?8, ?8)
                 ON CONFLICT(note_id, fingerprint) DO UPDATE SET
                    payload_json = excluded.payload_json,
                    confidence = excluded.confidence,
                    updated_at = excluded.updated_at",
                params![
                    suggestion.id,
                    note_id,
                    suggestion.kind,
                    payload,
                    suggestion.rule_id,
                    suggestion.confidence,
                    fingerprint,
                    now,
                ],
            )?;
            let (stored_id, status): (String, String) = transaction.query_row(
                "SELECT id, status FROM note_suggestions
                 WHERE note_id = ?1 AND fingerprint = ?2",
                params![note_id, fingerprint],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            if status == "pending" {
                let mut item = suggestion.clone();
                item.id = stored_id;
                pending.push(item);
            }
        }
        transaction.commit()?;
        Ok(OrganizerResult {
            suggestions: pending,
            entities: result.entities.clone(),
        })
    }

    pub fn set_suggestion_status(&self, id: &str, status: &str) -> AppResult<()> {
        if !["accepted", "dismissed"].contains(&status) {
            return Err(AppError::Validation("无效的建议状态".into()));
        }
        let changed = self.lock()?.execute(
            "UPDATE note_suggestions SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, status, Self::now()],
        )?;
        if changed == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    pub fn export_envelope(&self, app_version: &str) -> AppResult<ExportEnvelope> {
        let notes = self.list_notes("all", 100_000)?;
        let mut archived = self.list_notes("archive", 100_000)?;
        let mut trash = self.list_notes("trash", 100_000)?;
        let mut all_notes = notes;
        all_notes.append(&mut archived);
        all_notes.append(&mut trash);
        Ok(ExportEnvelope {
            schema_version: 1,
            app_version: app_version.to_string(),
            exported_at: Self::now(),
            categories: self.list_categories()?,
            tags: self.list_tags()?,
            notes: all_notes,
        })
    }

    pub fn export_json(&self, path: &Path, app_version: &str) -> AppResult<()> {
        let envelope = self.export_envelope(app_version)?;
        fs::write(path, serde_json::to_vec_pretty(&envelope)?)?;
        Ok(())
    }

    fn safe_filename(title: &str, id: &str) -> String {
        let invalid = RegexLike::sanitize(title);
        let base = if invalid.trim().is_empty() {
            "未命名"
        } else {
            invalid.trim()
        };
        let short_id = id.chars().take(8).collect::<String>();
        format!(
            "{}-{}.md",
            base.chars().take(60).collect::<String>(),
            short_id
        )
    }

    pub fn export_markdown(&self, directory: &Path) -> AppResult<usize> {
        fs::create_dir_all(directory)?;
        let mut notes = self.list_notes("all", 100_000)?;
        notes.append(&mut self.list_notes("archive", 100_000)?);
        for note in &notes {
            let frontmatter = format!(
                "---\nid: {}\ncreated_at: {}\nupdated_at: {}\ncategory: {}\ntags: [{}]\nis_todo: {}\ntodo_status: {}\npriority: {}\ndue_date: {}\n---\n\n# {}\n\n{}\n",
                note.id,
                note.created_at,
                note.updated_at,
                note.category.as_ref().map(|item| item.name.as_str()).unwrap_or(""),
                note.tags.iter().map(|tag| format!("\"{}\"", tag.name.replace('"', "\\\""))).collect::<Vec<_>>().join(", "),
                note.is_todo,
                note.todo_status,
                note.priority,
                note.due_date.as_deref().unwrap_or(""),
                note.title,
                note.content,
            );
            fs::write(
                directory.join(Self::safe_filename(&note.title, &note.id)),
                frontmatter,
            )?;
        }
        Ok(notes.len())
    }

    pub fn preview_import(&self, path: &Path) -> AppResult<ImportPreview> {
        let envelope: ExportEnvelope = serde_json::from_slice(&fs::read(path)?)?;
        if envelope.schema_version != 1 {
            return Err(AppError::Validation("不支持的 JSON 数据版本".into()));
        }
        let connection = self.lock()?;
        let mut create = 0;
        let mut skip = 0;
        let mut conflict = 0;
        for note in &envelope.notes {
            let existing: Option<(String, String)> = connection
                .query_row(
                    "SELECT title, content FROM notes WHERE id = ?1",
                    [&note.id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            match existing {
                None => create += 1,
                Some((title, content)) if title == note.title && content == note.content => {
                    skip += 1
                }
                Some(_) => conflict += 1,
            }
        }
        Ok(ImportPreview {
            notes_to_create: create,
            duplicates_to_skip: skip,
            conflicts_as_copies: conflict,
            categories_to_merge: envelope.categories.len(),
            tags_to_merge: envelope.tags.len(),
        })
    }

    pub fn import_json(&self, path: &Path) -> AppResult<ImportResult> {
        let envelope: ExportEnvelope = serde_json::from_slice(&fs::read(path)?)?;
        if envelope.schema_version != 1 {
            return Err(AppError::Validation("不支持的 JSON 数据版本".into()));
        }
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let now = Self::now();
        let mut category_map = HashMap::new();
        for category in &envelope.categories {
            let normalized = Self::normalize(&category.name);
            let local_id = transaction
                .query_row(
                    "SELECT id FROM categories WHERE normalized_name = ?1",
                    [&normalized],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            transaction.execute(
                "INSERT OR IGNORE INTO categories(id, name, normalized_name, color, sort_order, created_at, updated_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![local_id, category.name, normalized, category.color, category.sort_order, now],
            )?;
            category_map.insert(category.id.clone(), local_id);
        }

        let mut result = ImportResult {
            created: 0,
            skipped: 0,
            copied_conflicts: 0,
        };
        for note in &envelope.notes {
            let existing: Option<(String, String)> = transaction
                .query_row(
                    "SELECT title, content FROM notes WHERE id = ?1",
                    [&note.id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            if existing
                .as_ref()
                .is_some_and(|(title, content)| title == &note.title && content == &note.content)
            {
                result.skipped += 1;
                continue;
            }
            let id = if existing.is_some() {
                result.copied_conflicts += 1;
                Uuid::new_v4().to_string()
            } else {
                result.created += 1;
                note.id.clone()
            };
            let category_id = note
                .category_id
                .as_ref()
                .and_then(|id| category_map.get(id))
                .cloned();
            transaction.execute(
                "INSERT INTO notes(id, title, content, category_id, is_favorite, is_todo,
                    todo_status, priority, due_date, completed_at, source, revision, content_hash,
                    created_at, updated_at, deleted_at, archived_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'import', 0, ?11, ?12, ?13, ?14, ?15)",
                params![
                    id,
                    note.title,
                    note.content,
                    category_id,
                    note.is_favorite as i64,
                    note.is_todo as i64,
                    note.todo_status,
                    note.priority,
                    note.due_date,
                    note.completed_at,
                    Self::content_hash(&note.title, &note.content),
                    note.created_at,
                    now,
                    note.deleted_at,
                    note.archived_at,
                ],
            )?;
            for tag in &note.tags {
                let tag_id = Self::ensure_tag(&transaction, &tag.name, &now)?;
                transaction.execute(
                    "INSERT OR IGNORE INTO note_tags(note_id, tag_id) VALUES(?1, ?2)",
                    params![id, tag_id],
                )?;
            }
            Self::reindex_note(&transaction, &id)?;
        }
        transaction.commit()?;
        Ok(result)
    }

    pub fn create_backup(&self, path: &Path, app_version: &str) -> AppResult<BackupManifest> {
        let temp_path = self
            .path
            .with_file_name(format!("shanji-backup-{}.db", Uuid::new_v4()));
        {
            let source = self.lock()?;
            let mut destination = Connection::open(&temp_path)?;
            // SQLite 在线备份 API 在持有一致性快照的同时，不阻塞后续普通读操作。
            let backup = rusqlite::backup::Backup::new(&source, &mut destination)?;
            backup.run_to_completion(5, std::time::Duration::from_millis(100), None)?;
        }
        let database_bytes = fs::read(&temp_path)?;
        let manifest = BackupManifest {
            schema_version: 1,
            app_version: app_version.to_string(),
            created_at: Self::now(),
            database_sha256: format!("{:x}", Sha256::digest(&database_bytes)),
        };
        let output = fs::File::create(path)?;
        let mut archive = ZipWriter::new(output);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        archive.start_file("manifest.json", options)?;
        archive.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
        archive.start_file("shanji.db", options)?;
        archive.write_all(&database_bytes)?;
        archive.finish()?;
        let _ = fs::remove_file(&temp_path);
        Ok(manifest)
    }

    pub fn restore_backup(&self, path: &Path, app_version: &str) -> AppResult<String> {
        let rollback_path = self.path.with_file_name(format!(
            "shanji-pre-restore-{}.sjbackup",
            Utc::now().format("%Y%m%d-%H%M%S")
        ));
        self.create_backup(&rollback_path, app_version)?;

        let file = fs::File::open(path)?;
        let mut archive = ZipArchive::new(file)?;
        let manifest: BackupManifest = {
            let mut entry = archive.by_name("manifest.json")?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            serde_json::from_slice(&bytes)?
        };
        if manifest.schema_version != 1 {
            return Err(AppError::Validation("备份数据库版本不受支持".into()));
        }
        let database_bytes = {
            let mut entry = archive.by_name("shanji.db")?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            bytes
        };
        let checksum = format!("{:x}", Sha256::digest(&database_bytes));
        if checksum != manifest.database_sha256 {
            return Err(AppError::Validation("备份文件校验失败，未执行恢复".into()));
        }

        let temp_path = self
            .path
            .with_file_name(format!("shanji-restore-{}.db", Uuid::new_v4()));
        fs::write(&temp_path, database_bytes)?;
        let source = Connection::open(&temp_path)?;
        {
            let mut destination = self.lock()?;
            let backup = rusqlite::backup::Backup::new(&source, &mut destination)?;
            backup.run_to_completion(5, std::time::Duration::from_millis(100), None)?;
        }
        let _ = fs::remove_file(&temp_path);
        Ok(rollback_path.to_string_lossy().into_owned())
    }

    pub fn category_keyword_profiles(&self) -> AppResult<HashMap<String, Vec<String>>> {
        let connection = self.lock()?;
        let mut profiles: HashMap<String, HashMap<String, usize>> = HashMap::new();
        let mut statement = connection.prepare(
            "SELECT category_id, title, content FROM notes
             WHERE category_id IS NOT NULL AND deleted_at IS NULL",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let (category_id, title, content) = row?;
            let normalized = format!("{title} {content}").to_lowercase();
            let mut tokens = normalized
                .split(|char_: char| !char_.is_alphanumeric())
                .filter(|token| token.chars().count() >= 2)
                .map(str::to_lowercase)
                .collect::<HashSet<_>>();
            let compact: Vec<char> = normalized
                .chars()
                .filter(|char_| char_.is_alphanumeric())
                .collect();
            for size in 2..=4 {
                tokens.extend(
                    compact
                        .windows(size)
                        .map(|window| window.iter().collect::<String>()),
                );
            }
            for token in tokens {
                *profiles
                    .entry(category_id.clone())
                    .or_default()
                    .entry(token)
                    .or_default() += 1;
            }
        }
        Ok(profiles
            .into_iter()
            .map(|(id, counts)| {
                let mut entries: Vec<_> = counts.into_iter().collect();
                entries.sort_by_key(|item| std::cmp::Reverse(item.1));
                (
                    id,
                    entries
                        .into_iter()
                        .take(20)
                        .map(|(token, _)| token)
                        .collect(),
                )
            })
            .collect())
    }

    pub fn other_titles(&self, excluded_id: &str) -> AppResult<Vec<(String, String)>> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT id, title FROM notes WHERE id != ?1 AND deleted_at IS NULL AND title != '' LIMIT 1000",
        )?;
        let items = statement
            .query_map([excluded_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(items)
    }
}

struct RegexLike;

impl RegexLike {
    fn sanitize(value: &str) -> String {
        value
            .chars()
            .map(|char_| match char_ {
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
                char_ if char_.is_control() => '_',
                _ => char_,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_and_saves_note_with_tags() {
        let db = Database::in_memory().unwrap();
        let saved = db
            .save_note(SaveNoteInput {
                title: "测试笔记".into(),
                content: "明天提交方案".into(),
                tag_names: vec!["工作".into(), "工作".into()],
                ..Default::default()
            })
            .unwrap();
        assert_eq!(saved.status, "created");
        assert_eq!(saved.note.tags.len(), 1);
        assert_eq!(db.list_notes("all", 20).unwrap().len(), 1);
    }

    #[test]
    fn detects_revision_conflict() {
        let db = Database::in_memory().unwrap();
        let first = db
            .save_note(SaveNoteInput {
                title: "A".into(),
                ..Default::default()
            })
            .unwrap()
            .note;
        let changed = db
            .save_note(SaveNoteInput {
                id: Some(first.id.clone()),
                expected_revision: Some(first.revision),
                title: "B".into(),
                ..Default::default()
            })
            .unwrap()
            .note;
        let conflict = db
            .save_note(SaveNoteInput {
                id: Some(first.id),
                expected_revision: Some(0),
                title: "C".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(conflict.status, "conflict");
        assert_eq!(conflict.note.title, changed.title);
    }

    #[test]
    fn completed_at_is_not_reset_by_later_edits() {
        let db = Database::in_memory().unwrap();
        let completed = db
            .save_note(SaveNoteInput {
                title: "已完成事项".into(),
                is_todo: true,
                todo_status: "completed".into(),
                ..Default::default()
            })
            .unwrap()
            .note;
        let completed_at = completed.completed_at.clone();
        let edited = db
            .save_note(SaveNoteInput {
                id: Some(completed.id),
                expected_revision: Some(completed.revision),
                title: "补充说明".into(),
                is_todo: true,
                todo_status: "completed".into(),
                ..Default::default()
            })
            .unwrap()
            .note;
        assert_eq!(edited.completed_at, completed_at);
    }

    #[test]
    fn short_chinese_search_uses_fallback() {
        let db = Database::in_memory().unwrap();
        db.save_note(SaveNoteInput {
            title: "项目计划".into(),
            content: "准备发布".into(),
            ..Default::default()
        })
        .unwrap();
        let results = db
            .search(SearchQuery {
                query: "项目".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn quick_window_pin_setting_is_persisted() {
        let db = Database::in_memory().unwrap();
        let mut settings = db.get_settings().unwrap();
        assert!(!settings.quick_pinned);

        settings.quick_pinned = true;
        db.update_settings(&settings).unwrap();

        assert!(db.get_settings().unwrap().quick_pinned);
    }

    #[test]
    fn chinese_trigram_search_uses_fts_when_available() {
        let db = Database::in_memory().unwrap();
        db.save_note(SaveNoteInput {
            title: "项目发布计划".into(),
            content: "准备 Windows 安装包".into(),
            ..Default::default()
        })
        .unwrap();
        let results = db
            .search(SearchQuery {
                query: "发布计划".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn fts_ranking_prefers_title_matches() {
        let db = Database::in_memory().unwrap();
        let title_match = db
            .save_note(SaveNoteInput {
                title: "发布计划".into(),
                content: "简短内容".into(),
                ..Default::default()
            })
            .unwrap()
            .note;
        db.save_note(SaveNoteInput {
            title: "普通记录".into(),
            content: "这里多次提到发布计划、发布计划和发布计划".into(),
            ..Default::default()
        })
        .unwrap();
        let results = db
            .search(SearchQuery {
                query: "发布计划".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            results.first().map(|item| item.note.id.as_str()),
            Some(title_match.id.as_str())
        );
    }

    #[test]
    fn delete_and_restore_round_trip() {
        let db = Database::in_memory().unwrap();
        let note = db
            .save_note(SaveNoteInput {
                title: "恢复我".into(),
                ..Default::default()
            })
            .unwrap()
            .note;
        db.soft_delete(&note.id).unwrap();
        assert_eq!(db.list_notes("trash", 20).unwrap().len(), 1);
        db.restore_note(&note.id).unwrap();
        assert_eq!(db.list_notes("all", 20).unwrap().len(), 1);
    }

    #[test]
    fn backup_and_restore_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let db = Database::open(directory.path().join("shanji.db")).unwrap();
        let original = db
            .save_note(SaveNoteInput {
                title: "备份前".into(),
                ..Default::default()
            })
            .unwrap()
            .note;
        let backup_path = directory.path().join("snapshot.sjbackup");
        db.create_backup(&backup_path, "0.1.0").unwrap();
        let later = db
            .save_note(SaveNoteInput {
                title: "备份后".into(),
                ..Default::default()
            })
            .unwrap()
            .note;

        db.restore_backup(&backup_path, "0.1.0").unwrap();

        assert_eq!(db.get_note(&original.id).unwrap().title, "备份前");
        assert!(matches!(db.get_note(&later.id), Err(AppError::NotFound)));
    }

    #[test]
    fn organizer_suggestion_status_is_persisted() {
        let db = Database::in_memory().unwrap();
        let note = db
            .save_note(SaveNoteInput {
                title: "明天提交方案".into(),
                ..Default::default()
            })
            .unwrap()
            .note;
        let analysis = OrganizerResult {
            suggestions: vec![crate::domain::models::Suggestion {
                id: Uuid::new_v4().to_string(),
                kind: "due_date".into(),
                label: "设置截止日期".into(),
                value: serde_json::json!({ "dueDate": "2026-07-15" }),
                confidence: 0.95,
                rule_id: "date-keyword-v1".into(),
            }],
            entities: vec![],
        };
        let first = db.persist_organizer_result(&note.id, &analysis).unwrap();
        assert_eq!(first.suggestions.len(), 1);
        db.set_suggestion_status(&first.suggestions[0].id, "dismissed")
            .unwrap();

        let repeated = db.persist_organizer_result(&note.id, &analysis).unwrap();
        assert!(repeated.suggestions.is_empty());
    }
}
