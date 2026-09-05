use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "type", content = "details")]
pub enum AppError {
    #[error("Executable not found: {0}")]
    ExecutableNotFound(String),

    #[error("Profile directory error: {0}")]
    ProfileDirectoryError(String),

    #[error("Process conflict: Profile '{running_profile}' is currently running (PID {pid})")]
    ProcessConflict { running_profile: String, pid: u32 },

    #[error("Account is disabled: {0}")]
    AccountDisabled(String),

    #[error("Unsupported execution surface '{surface}' for platform '{platform}'")]
    UnsupportedExecutionSurface { platform: String, surface: String },

    #[error("Process termination error: {0}")]
    ProcessTerminationFailed(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Launch failed: {0}")]
    LaunchFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Database corrupted: {0}")]
    DatabaseCorrupt(String),

    #[error("Database migration failed: {0}")]
    MigrationFailed(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Account not found: {0}")]
    NotFound(String),

    #[error("Workspace directory not found: {0}")]
    WorkspaceDirectoryNotFound(String),

    #[error("Browser error: {0}")]
    BrowserError(String),

    #[error("Browser profile is locked: {0}")]
    BrowserProfileLocked(String),

    #[error("Profile directory is missing: {0}")]
    ProfileMissing(String),

    #[error("Profile is locked: {0}")]
    ProfileLocked(String),

    #[error("Security violation detected: {0}")]
    SecurityViolation(String),

    #[error("Partial deletion failure: {0}")]
    PartialDeleteError(String),

    #[error("Operation cancelled by user")]
    Cancelled,
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::IoError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
