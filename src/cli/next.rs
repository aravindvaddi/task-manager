use serde_json::json;

use task_manager::DEFAULT_PROJECT_SLUG;

pub fn handle(project_slug: &str, pretty: bool) -> task_manager::error::Result<()> {
    if project_slug == DEFAULT_PROJECT_SLUG {
        task_manager::ensure_default_project()?;
    }

    match task_manager::next_task(project_slug) {
        Ok(Some((story_id, task))) => {
            let value = json!({
                "story_id": story_id,
                "task": {
                    "id": task.id,
                    "name": task.name,
                    "description": task.description,
                    "status": format!("{}", task.status),
                }
            });
            super::output_json(&value, pretty);
            Ok(())
        }
        Ok(None) => {
            let value = json!({
                "task": null,
                "message": "no actionable tasks"
            });
            super::output_json(&value, pretty);
            Ok(())
        }
        Err(e) => Err(e),
    }
}
