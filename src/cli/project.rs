use clap::Subcommand;
use serde_json::json;

use task_manager::graph;
use task_manager::models::TaskStatus;

#[derive(Subcommand)]
pub enum ProjectCommand {
    /// Create a new project
    Create {
        /// Project name
        name: String,
    },
    /// List all projects
    List,
    /// Show project status
    Status {
        /// Project slug
        slug: String,
    },
}

pub fn handle(cmd: ProjectCommand, pretty: bool) -> task_manager::error::Result<()> {
    match cmd {
        ProjectCommand::Create { name } => {
            let project = task_manager::create_project(&name)?;
            let value = json!({
                "slug": project.slug,
                "name": project.name,
                "created_at": project.created_at,
            });
            super::output_json(&value, pretty);
        }

        ProjectCommand::List => {
            let projects = task_manager::list_projects()?;
            let value: Vec<_> = projects
                .iter()
                .map(|p| {
                    json!({
                        "slug": p.slug,
                        "name": p.name,
                        "story_count": p.stories.len(),
                        "created_at": p.created_at,
                    })
                })
                .collect();
            super::output_json(&json!(value), pretty);
        }

        ProjectCommand::Status { slug } => {
            let project = task_manager::get_project(&slug)?;

            let mut total_tasks = 0u64;
            let mut pending = 0u64;
            let mut running = 0u64;
            let mut closed = 0u64;

            let stories: Vec<_> = project
                .stories
                .values()
                .map(|s| {
                    let status = graph::get_story_status(s);
                    let blocked = graph::is_story_blocked(&project, &s.id);
                    for t in s.tasks.values() {
                        total_tasks += 1;
                        match t.status {
                            TaskStatus::Pending => pending += 1,
                            TaskStatus::Running => running += 1,
                            TaskStatus::Closed { .. } => closed += 1,
                        }
                    }
                    json!({
                        "id": s.id,
                        "name": s.name,
                        "status": format!("{status}"),
                        "blocked": blocked,
                        "task_count": s.tasks.len(),
                    })
                })
                .collect();

            let completion_pct = if total_tasks > 0 {
                (closed as f64 / total_tasks as f64) * 100.0
            } else {
                0.0
            };

            let value = json!({
                "slug": project.slug,
                "name": project.name,
                "stories": stories,
                "task_counts": {
                    "total": total_tasks,
                    "pending": pending,
                    "running": running,
                    "closed": closed,
                },
                "completion_percent": (completion_pct * 10.0).round() / 10.0,
                "created_at": project.created_at,
            });
            super::output_json(&value, pretty);
        }
    }
    Ok(())
}
