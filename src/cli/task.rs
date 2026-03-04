use clap::Subcommand;
use serde_json::json;

use task_manager::models::{CloseReason, TaskStatus};
use task_manager::DEFAULT_PROJECT_SLUG;

#[derive(Subcommand)]
pub enum TaskCommand {
    /// Create a new task in a story
    Create {
        /// Task name
        name: String,
        /// Project slug (defaults to "default")
        #[arg(long)]
        project: Option<String>,
        /// Story ID (e.g., s1). Defaults to "default" story in default project.
        #[arg(long)]
        story: Option<String>,
        /// Task description
        #[arg(long)]
        description: Option<String>,
    },
    /// Add a dependency between tasks (same story only)
    Dep {
        /// Task ID (e.g., t1)
        task_id: String,
        /// Task ID this depends on
        #[arg(long)]
        depends_on: String,
        /// Project slug
        #[arg(long, default_value = DEFAULT_PROJECT_SLUG)]
        project: String,
    },
    /// Get full task details
    Get {
        /// Task ID (e.g., t1)
        task_id: String,
        /// Project slug
        #[arg(long, default_value = DEFAULT_PROJECT_SLUG)]
        project: String,
    },
    /// List tasks in a story
    List {
        /// Story ID (e.g., s1). Defaults to "default".
        #[arg(default_value = task_manager::DEFAULT_STORY_ID)]
        story_id: String,
        /// Project slug
        #[arg(long, default_value = DEFAULT_PROJECT_SLUG)]
        project: String,
    },
    /// Update a task's status and/or agent
    Update {
        /// Task ID (e.g., t1)
        task_id: String,
        /// Project slug
        #[arg(long, default_value = DEFAULT_PROJECT_SLUG)]
        project: String,
        /// New status: running, closed
        #[arg(long)]
        status: Option<String>,
        /// Close reason: successful, not_required (required when status=closed)
        #[arg(long)]
        reason: Option<String>,
        /// Agent name
        #[arg(long)]
        agent: Option<String>,
    },
}

fn parse_status(status: &str, reason: Option<&str>) -> task_manager::error::Result<TaskStatus> {
    match status {
        "running" => Ok(TaskStatus::Running),
        "pending" => Ok(TaskStatus::Pending),
        "closed" => {
            let reason_str = reason.ok_or_else(|| task_manager::error::Error::InvalidStateTransition {
                from: String::new(),
                to: "closed".into(),
                reason: "--reason is required when setting status to closed (successful or not_required)".into(),
            })?;
            match reason_str {
                "successful" => Ok(TaskStatus::Closed {
                    reason: CloseReason::Successful,
                }),
                "not_required" => Ok(TaskStatus::Closed {
                    reason: CloseReason::NotRequired,
                }),
                other => Err(task_manager::error::Error::InvalidStateTransition {
                    from: String::new(),
                    to: format!("closed({other})"),
                    reason: "reason must be 'successful' or 'not_required'".into(),
                }),
            }
        }
        other => Err(task_manager::error::Error::InvalidStateTransition {
            from: String::new(),
            to: other.into(),
            reason: "status must be 'pending', 'running', or 'closed'".into(),
        }),
    }
}

/// Ensure the default project exists if the given slug is the default.
fn ensure_default_if_needed(project: &str) -> task_manager::error::Result<()> {
    if project == DEFAULT_PROJECT_SLUG {
        task_manager::ensure_default_project()?;
    }
    Ok(())
}

pub fn handle(cmd: TaskCommand, pretty: bool) -> task_manager::error::Result<()> {
    match cmd {
        TaskCommand::Create {
            name,
            project,
            story,
            description,
        } => {
            let (project_slug, story_id) = match (project, story) {
                (Some(p), Some(s)) => (p, s),
                (Some(p), None) => {
                    if p == DEFAULT_PROJECT_SLUG {
                        task_manager::ensure_default_project()?;
                        (p, task_manager::DEFAULT_STORY_ID.to_string())
                    } else {
                        return Err(task_manager::error::Error::InvalidDependency {
                            reason: "--story is required when --project is specified (unless project is 'default')".into(),
                        });
                    }
                }
                (None, None) => {
                    task_manager::ensure_default_project()?;
                    (
                        DEFAULT_PROJECT_SLUG.to_string(),
                        task_manager::DEFAULT_STORY_ID.to_string(),
                    )
                }
                (None, Some(_)) => {
                    return Err(task_manager::error::Error::InvalidDependency {
                        reason: "--project is required when --story is specified".into(),
                    });
                }
            };

            let (_project, task) =
                task_manager::create_task(&project_slug, &story_id, &name, description.as_deref())?;
            let value = json!({
                "id": task.id,
                "name": task.name,
                "description": task.description,
                "status": format!("{}", task.status),
                "created_at": task.created_at,
            });
            super::output_json(&value, pretty);
        }

        TaskCommand::Dep {
            task_id,
            depends_on,
            project,
        } => {
            ensure_default_if_needed(&project)?;
            task_manager::add_task_dep(&project, &task_id, &depends_on)?;
            let value = json!({
                "task_id": task_id,
                "depends_on": depends_on,
                "status": "ok",
            });
            super::output_json(&value, pretty);
        }

        TaskCommand::Get { task_id, project } => {
            ensure_default_if_needed(&project)?;
            let (_proj, story_id, task) = task_manager::get_task(&project, &task_id)?;
            let value = json!({
                "id": task.id,
                "story_id": story_id,
                "name": task.name,
                "description": task.description,
                "status": format!("{}", task.status),
                "agent": task.agent,
                "created_at": task.created_at,
                "updated_at": task.updated_at,
            });
            super::output_json(&value, pretty);
        }

        TaskCommand::List {
            story_id,
            project,
        } => {
            ensure_default_if_needed(&project)?;
            let proj = task_manager::get_project(&project)?;
            let story = proj
                .stories
                .get(&story_id)
                .ok_or_else(|| task_manager::error::Error::StoryNotFound {
                    id: story_id.clone(),
                })?;

            let tasks: Vec<_> = story
                .tasks
                .values()
                .map(|t| {
                    json!({
                        "id": t.id,
                        "name": t.name,
                        "status": format!("{}", t.status),
                        "agent": t.agent,
                    })
                })
                .collect();
            super::output_json(&json!(tasks), pretty);
        }

        TaskCommand::Update {
            task_id,
            project,
            status,
            reason,
            agent,
        } => {
            ensure_default_if_needed(&project)?;
            let new_status = if let Some(ref s) = status {
                Some(parse_status(s, reason.as_deref())?)
            } else {
                None
            };

            let (_proj, task) =
                task_manager::update_task(&project, &task_id, new_status, agent.as_deref())?;

            let value = json!({
                "id": task.id,
                "name": task.name,
                "status": format!("{}", task.status),
                "agent": task.agent,
                "updated_at": task.updated_at,
            });
            super::output_json(&value, pretty);
        }
    }
    Ok(())
}
