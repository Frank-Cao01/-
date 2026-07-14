# 数据库设计

当前 schema 版本为 1，迁移位于 `src-tauri/migrations`。

- `notes`：笔记正文、待办、优先级、时间、软删除、归档、revision。
- `categories`、`tags`、`note_tags`：分类和多标签关系。
- `settings`：跨窗口共享的 JSON 设置值。
- `search_history`：去重后最多保留最近 20 条。
- `note_entities`、`note_suggestions`：自动整理实体和建议的扩展存储。
- `notes_fts`：标题、正文、标签、分类和日期的 FTS5 trigram 索引。

时间字段除 `due_date` 外使用 UTC RFC3339；截止日期保存为本地日历日期 `YYYY-MM-DD`。
