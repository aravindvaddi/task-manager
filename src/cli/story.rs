use clap::Subcommand;
use serde_json::json;

use task_manager::graph;

#[derive(Subcommand)]
pub enum StoryCommand {
    /// Create a new story in a project
    Create {
        /// Project slug
        project_slug: String,
        /// Story name
        name: String,
    },
    /// Add a dependency between stories
    Dep {
        /// Story ID (e.g., s1)
        story_id: String,
        /// Story ID this depends on
        #[arg(long)]
        depends_on: String,
        /// Project slug
        #[arg(long)]
        project: String,
    },
    /// List stories in a project
    List {
        /// Project slug
        project_slug: String,
    },
    /// Show story status
    Status {
        /// Story ID (e.g., s1)
        story_id: String,
        /// Project slug
        #[arg(long)]
        project: String,
    },
}

pub fn handle(cmd: StoryCommand, pretty: bool) -> task_manager::error::Result<()> {
    match cmd {
        StoryCommand::Create { project_slug, name } => {
            let (_project, story) = task_manager::create_story(&project_slug, &name)?;
            let value = json!({
                "id": story.id,
                "name": story.name,
                "created_at": story.created_at,
            });
            super::output_json(&value, pretty);
        }

        StoryCommand::Dep {
            story_id,
            depends_on,
            project,
        } => {
            task_manager::add_story_dep(&project, &story_id, &depends_on)?;
            let value = json!({
                "story_id": story_id,
                "depends_on": depends_on,
                "status": "ok",
            });
            super::output_json(&value, pretty);
        }

        StoryCommand::List { project_slug } => {
            let project = task_manager::list_stories(&project_slug)?;
            let stories: Vec<_> = project
                .stories
                .values()
                .map(|s| {
                    let status = graph::get_story_status(s);
                    let blocked = graph::is_story_blocked(&project, &s.id);
                    json!({
                        "id": s.id,
                        "name": s.name,
                        "status": format!("{status}"),
                        "blocked": blocked,
                        "task_count": s.tasks.len(),
                    })
                })
                .collect();
            super::output_json(&json!(stories), pretty);
        }

        StoryCommand::Status {
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

            let status = graph::get_story_status(story);
            let blocked = graph::is_story_blocked(&proj, &story_id);

            // Get dependency story IDs
            let deps: Vec<&str> = proj
                .story_deps
                .iter()
                .filter(|(id, _)| id == &story_id)
                .map(|(_, dep)| dep.as_str())
                .collect();

            let tasks: Vec<_> = story
                .tasks
                .values()
                .map(|t| {
                    json!({
                        "id": t.id,
                        "name": t.name,
                        "status": format!("{}", t.status),
                    })
                })
                .collect();

            let value = json!({
                "id": story.id,
                "name": story.name,
                "status": format!("{status}"),
                "blocked": blocked,
                "depends_on": deps,
                "tasks": tasks,
                "created_at": story.created_at,
            });
            super::output_json(&value, pretty);
        }
    }
    Ok(())
}
