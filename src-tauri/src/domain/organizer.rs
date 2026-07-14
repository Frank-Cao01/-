use std::collections::{HashMap, HashSet};

use chrono::{Datelike, Duration, Local, NaiveDate};
use regex::Regex;
use serde_json::json;
use uuid::Uuid;

use super::models::{Category, EntityMatch, Note, OrganizerResult, Suggestion, Tag};

#[derive(Debug, Clone, Default)]
pub struct OrganizerContext {
    pub categories: Vec<Category>,
    pub tags: Vec<Tag>,
    pub category_keywords: HashMap<String, Vec<String>>,
    pub other_titles: Vec<(String, String)>,
    pub archive_days: i64,
}

/// 自动整理的抽象边界，未来可增加 AI provider，但 UI 只依赖统一结果。
pub trait OrganizerProvider: Send + Sync {
    fn analyze(&self, note: &Note, context: &OrganizerContext) -> OrganizerResult;
}

#[derive(Default)]
pub struct RuleOrganizer;

impl RuleOrganizer {
    fn suggestion(
        kind: &str,
        label: String,
        value: serde_json::Value,
        confidence: f64,
        rule_id: &str,
    ) -> Suggestion {
        Suggestion {
            id: Uuid::new_v4().to_string(),
            kind: kind.to_string(),
            label,
            value,
            confidence,
            rule_id: rule_id.to_string(),
        }
    }

    fn detect_date(text: &str) -> Option<(NaiveDate, &str, f64)> {
        let today = Local::now().date_naive();
        for (word, offset, confidence) in [
            ("今天", 0, 0.95),
            ("明天", 1, 0.96),
            ("后天", 2, 0.96),
            ("下周", 7, 0.82),
        ] {
            if text.contains(word) {
                return Some((today + Duration::days(offset), word, confidence));
            }
        }

        if text.contains("月底") {
            let (year, month) = if today.month() == 12 {
                (today.year() + 1, 1)
            } else {
                (today.year(), today.month() + 1)
            };
            if let Some(first_next_month) = NaiveDate::from_ymd_opt(year, month, 1) {
                return Some((first_next_month - Duration::days(1), "月底", 0.9));
            }
        }

        let full =
            Regex::new(r"(?P<y>20\d{2})[-/.年](?P<m>\d{1,2})[-/.月](?P<d>\d{1,2})日?").ok()?;
        if let Some(caps) = full.captures(text) {
            let date = NaiveDate::from_ymd_opt(
                caps["y"].parse().ok()?,
                caps["m"].parse().ok()?,
                caps["d"].parse().ok()?,
            )?;
            return Some((date, "明确日期", 0.99));
        }

        let short = Regex::new(r"(?P<m>\d{1,2})月(?P<d>\d{1,2})日").ok()?;
        if let Some(caps) = short.captures(text) {
            let month: u32 = caps["m"].parse().ok()?;
            let day: u32 = caps["d"].parse().ok()?;
            let mut year = today.year();
            let mut date = NaiveDate::from_ymd_opt(year, month, day)?;
            if date < today - Duration::days(30) {
                year += 1;
                date = NaiveDate::from_ymd_opt(year, month, day)?;
            }
            return Some((date, "月日", 0.92));
        }
        None
    }

    fn detect_entities(text: &str) -> Vec<EntityMatch> {
        let patterns = [
            ("url", r"https?://[^\s<>()]+"),
            ("email", r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"),
            (
                "phone",
                r"(?:\+?86[- ]?)?1[3-9]\d{9}|(?:0\d{2,3}[- ]?)?\d{7,8}",
            ),
        ];
        let mut seen = HashSet::new();
        let mut entities = Vec::new();
        for (entity_type, pattern) in patterns {
            if let Ok(regex) = Regex::new(pattern) {
                for matched in regex.find_iter(text) {
                    let key = (matched.start(), matched.end());
                    if seen.insert(key) {
                        entities.push(EntityMatch {
                            entity_type: entity_type.to_string(),
                            value: matched.as_str().to_string(),
                            start_offset: matched.start(),
                            end_offset: matched.end(),
                        });
                    }
                }
            }
        }
        entities.sort_by_key(|entity| entity.start_offset);
        entities
    }

    fn tag_candidates(text: &str, tags: &[Tag]) -> Vec<(String, f64)> {
        let lower = text.to_lowercase();
        let stopwords = [
            "今天", "明天", "后天", "需要", "这个", "那个", "已经", "进行", "完成", "事情", "工作",
            "记录", "一下", "可以", "我们", "你们", "他们",
        ];
        let mut candidates = Vec::new();
        for tag in tags {
            let name = tag.name.trim();
            if name.chars().count() >= 2 && lower.contains(&name.to_lowercase()) {
                candidates.push((name.to_string(), 0.94));
            }
        }

        let chars: Vec<char> = lower
            .chars()
            .filter(|char_| !char_.is_whitespace())
            .collect();
        let mut counts: HashMap<String, usize> = HashMap::new();
        for size in 2..=4 {
            for window in chars.windows(size) {
                if window.iter().all(|char_| char_.is_alphanumeric()) {
                    let token: String = window.iter().collect();
                    if !stopwords.iter().any(|word| token.contains(word)) {
                        *counts.entry(token).or_default() += 1;
                    }
                }
            }
        }
        let mut frequent: Vec<_> = counts
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .collect();
        frequent.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.len().cmp(&a.0.len())));
        for (token, count) in frequent.into_iter().take(3) {
            if !candidates.iter().any(|(name, _)| name == &token) {
                candidates.push((token, (0.58 + count as f64 * 0.06).min(0.82)));
            }
        }
        candidates.truncate(5);
        candidates
    }

    fn title_similarity(left: &str, right: &str) -> f64 {
        fn trigrams(value: &str) -> HashSet<String> {
            let normalized: Vec<char> = value
                .to_lowercase()
                .chars()
                .filter(|char_| char_.is_alphanumeric())
                .collect();
            if normalized.len() < 3 {
                return std::iter::once(normalized.iter().collect()).collect();
            }
            normalized
                .windows(3)
                .map(|window| window.iter().collect())
                .collect()
        }
        let a = trigrams(left);
        let b = trigrams(right);
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let intersection = a.intersection(&b).count() as f64;
        intersection / a.union(&b).count() as f64
    }
}

impl OrganizerProvider for RuleOrganizer {
    fn analyze(&self, note: &Note, context: &OrganizerContext) -> OrganizerResult {
        let text = format!("{}\n{}", note.title, note.content);
        let mut suggestions = Vec::new();

        if note.due_date.is_none() {
            if let Some((date, source, confidence)) = Self::detect_date(&text) {
                suggestions.push(Self::suggestion(
                    "due_date",
                    format!("根据“{source}”设置截止日期为 {date}"),
                    json!({ "dueDate": date.to_string() }),
                    confidence,
                    "date-keyword-v1",
                ));
            }
        }

        let task_words = [
            "待办", "需要", "记得", "提醒", "安排", "跟进", "提交", "处理",
        ];
        let has_task_word = task_words.iter().any(|word| text.contains(word));
        if !note.is_todo && has_task_word {
            suggestions.push(Self::suggestion(
                "todo",
                "这条内容可能是一项待办".to_string(),
                json!({ "isTodo": true, "todoStatus": "not_started" }),
                0.82,
                "todo-keyword-v1",
            ));
        }
        if (note.is_todo || has_task_word)
            && ["已完成", "已处理", "搞定", "完成了"]
                .iter()
                .any(|word| text.contains(word))
        {
            suggestions.push(Self::suggestion(
                "todo_status",
                "内容可能表示事项已完成".to_string(),
                json!({ "todoStatus": "completed" }),
                0.86,
                "todo-complete-v1",
            ));
        }

        for (tag, confidence) in Self::tag_candidates(&text, &context.tags) {
            if !note
                .tags
                .iter()
                .any(|existing| existing.name.eq_ignore_ascii_case(&tag))
            {
                suggestions.push(Self::suggestion(
                    "tag",
                    format!("推荐标签：{tag}"),
                    json!({ "tagName": tag }),
                    confidence,
                    "tag-frequency-v1",
                ));
            }
        }

        if note.category_id.is_none() {
            let lower = text.to_lowercase();
            let mut category_scores: Vec<(String, String, usize)> = context
                .categories
                .iter()
                .map(|category| {
                    let score = context
                        .category_keywords
                        .get(&category.id)
                        .into_iter()
                        .flatten()
                        .filter(|keyword| lower.contains(&keyword.to_lowercase()))
                        .count();
                    (category.id.clone(), category.name.clone(), score)
                })
                .filter(|(_, _, score)| *score > 0)
                .collect();
            category_scores.sort_by_key(|item| std::cmp::Reverse(item.2));
            if let Some((id, name, score)) = category_scores.first() {
                let runner_up = category_scores.get(1).map(|item| item.2).unwrap_or(0);
                if *score >= 2 && *score > runner_up {
                    suggestions.push(Self::suggestion(
                        "category",
                        format!("推荐分类：{name}"),
                        json!({ "categoryId": id, "categoryName": name }),
                        (0.62 + *score as f64 * 0.06).min(0.88),
                        "category-profile-v1",
                    ));
                }
            }
        }

        if !note.title.trim().is_empty() {
            if let Some((id, title, similarity)) = context
                .other_titles
                .iter()
                .map(|(id, title)| (id, title, Self::title_similarity(&note.title, title)))
                .filter(|(_, _, score)| *score >= 0.72)
                .max_by(|a, b| a.2.total_cmp(&b.2))
            {
                suggestions.push(Self::suggestion(
                    "similar_title",
                    format!("标题与“{title}”较为相似"),
                    json!({ "noteId": id, "title": title }),
                    similarity,
                    "similar-title-v1",
                ));
            }
        }

        let archive_days = context.archive_days.max(1);
        if !note.is_favorite
            && (!note.is_todo || note.todo_status == "completed")
            && note.archived_at.is_none()
        {
            if let Ok(updated) = chrono::DateTime::parse_from_rfc3339(&note.updated_at) {
                if Local::now()
                    .signed_duration_since(updated.with_timezone(&Local))
                    .num_days()
                    >= archive_days
                {
                    suggestions.push(Self::suggestion(
                        "archive",
                        format!("超过 {archive_days} 天未修改，建议归档"),
                        json!({ "archive": true }),
                        0.78,
                        "archive-age-v1",
                    ));
                }
            }
        }

        OrganizerResult {
            suggestions,
            entities: Self::detect_entities(&text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(content: &str) -> Note {
        Note {
            id: "note-1".into(),
            title: "".into(),
            content: content.into(),
            category_id: None,
            category: None,
            tags: vec![],
            is_favorite: false,
            is_todo: false,
            todo_status: "not_started".into(),
            priority: "normal".into(),
            due_date: None,
            completed_at: None,
            source: "main".into(),
            revision: 0,
            created_at: Local::now().to_rfc3339(),
            updated_at: Local::now().to_rfc3339(),
            deleted_at: None,
            archived_at: None,
        }
    }

    #[test]
    fn recognizes_todo_date_and_contact() {
        let result = RuleOrganizer.analyze(
            &note("记得明天联系 test@example.com 并提交方案"),
            &OrganizerContext::default(),
        );
        assert!(result
            .suggestions
            .iter()
            .any(|item| item.kind == "due_date"));
        assert!(result.suggestions.iter().any(|item| item.kind == "todo"));
        assert!(result
            .entities
            .iter()
            .any(|item| item.entity_type == "email"));
    }

    #[test]
    fn completion_requires_task_context() {
        let result = RuleOrganizer.analyze(&note("这个阶段已经完成"), &OrganizerContext::default());
        assert!(!result
            .suggestions
            .iter()
            .any(|item| item.kind == "todo_status"));
    }

    #[test]
    fn recommends_category_and_warns_about_similar_title() {
        let mut current = note("项目发布进度需要继续跟进");
        current.title = "Windows 发布计划".into();
        let category = Category {
            id: "category-work".into(),
            name: "工作".into(),
            color: "slate".into(),
            sort_order: 0,
            created_at: Local::now().to_rfc3339(),
            updated_at: Local::now().to_rfc3339(),
        };
        let mut context = OrganizerContext {
            categories: vec![category],
            other_titles: vec![("other-note".into(), "Windows发布计划".into())],
            ..Default::default()
        };
        context
            .category_keywords
            .insert("category-work".into(), vec!["项目".into(), "发布".into()]);
        let result = RuleOrganizer.analyze(&current, &context);
        assert!(result
            .suggestions
            .iter()
            .any(|item| item.kind == "category"));
        assert!(result
            .suggestions
            .iter()
            .any(|item| item.kind == "similar_title"));
    }
}
