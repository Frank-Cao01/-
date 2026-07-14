use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("数据库错误：{0}")]
    Database(#[from] rusqlite::Error),
    #[error("文件操作失败：{0}")]
    Io(#[from] std::io::Error),
    #[error("数据格式错误：{0}")]
    Json(#[from] serde_json::Error),
    #[error("压缩包错误：{0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("{0}")]
    Validation(String),
    #[error("记录不存在")]
    NotFound,
}

pub type AppResult<T> = Result<T, AppError>;
