use super::models::Note;

/// 后续通知和日历能力的稳定边界；v1 只保留空实现。
#[allow(dead_code)]
pub trait ReminderScheduler: Send + Sync {
    fn schedule(&self, _note: &Note) -> Result<(), String>;
    fn cancel(&self, _note_id: &str) -> Result<(), String>;
}

#[allow(dead_code)]
pub struct NoopReminderScheduler;

impl ReminderScheduler for NoopReminderScheduler {
    fn schedule(&self, _note: &Note) -> Result<(), String> {
        Ok(())
    }

    fn cancel(&self, _note_id: &str) -> Result<(), String> {
        Ok(())
    }
}
