use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("project not found: {slug}")]
    ProjectNotFound { slug: String },

    #[error("story not found: {id}")]
    StoryNotFound { id: String },

    #[error("task not found: {id}")]
    TaskNotFound { id: String },

    #[error("cycle detected: adding this dependency would create a circular dependency")]
    CycleDetected,

    #[error("duplicate project name: a project with slug '{slug}' already exists")]
    DuplicateProjectName { slug: String },

    #[error("invalid state transition: cannot move from {from} to {to}: {reason}")]
    InvalidStateTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("invalid dependency: {reason}")]
    InvalidDependency { reason: String },

    #[error("story has no tasks: {id}")]
    StoryHasNoTasks { id: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
