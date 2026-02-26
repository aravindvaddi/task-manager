use clap::Subcommand;
use serde_json::json;

use task_manager::models::{CloseReason, TaskStatus};

#[derive(Subcommand)]
pub enum TaskCommand {
    /// Create a new task in a story
    Create {
        /// Story ID (e.g., s1)
        story_id: String,
        /// Task name
        name: String,
        /// Project slug
        #[arg(long)]
        project: String,
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
        #[arg(long)]
        project: String,
    },
    /// Get full task details
    Get {
        /// Task ID (e.g., t1)
        task_id: String,
        /// Project slug
        #[arg(long)]
        project: String,
    },
    /// List tasks in a story
    List {
        /// Story ID (e.g., s1)
        story_id: String,
        /// Project slug
        #[arg(long)]
        project: String,
    },
    /// Update a task's status and/or agent
    Update {
        /// Task ID (e.g., t1)
        task_id: String,
        /// Project slug
        #[arg(long)]
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

pub fn handle(cmd: TaskCommand, pretty: bool) -> task_manager::error::Result<()> {
    match cmd {
        TaskCommand::Create {
            story_id,
            name,
            project,
            description,
        } => {
            let (_project, task) =
                task_manager::create_task(&project, &story_id, &name, description.as_deref())?;
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
            task_manager::add_task_dep(&project, &task_id, &depends_on)?;
            let value = json!({
                "task_id": task_id,
                "depends_on": depends_on,
                "status": "ok",
            });
            super::output_json(&value, pretty);
        }

        TaskCommand::Get { task_id, project } => {
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
