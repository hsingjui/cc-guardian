use thiserror::Error;

#[derive(Error, Debug)]
pub enum CheckpointError {
    #[error("Git repository not found")]
    RepositoryNotFound,

    #[error("Git operation failed: {0}")]
    GitOperationFailed(#[from] git2::Error),

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("Checkpoint not found: {0}")]
    CheckpointNotFound(String),

    #[error("Invalid checkpoint hash: {0}")]
    InvalidHash(String),

    #[error("Invalid date format: {0}")]
    InvalidDateFormat(String),

    #[error("Repository has uncommitted changes")]
    UncommittedChanges,

    #[error("No changes to commit")]
    NoChangesToCommit,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Operation cancelled by user")]
    UserCancelled,

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Dialoguer error: {0}")]
    DialoguerError(#[from] dialoguer::Error),
}

pub type Result<T> = std::result::Result<T, CheckpointError>;
