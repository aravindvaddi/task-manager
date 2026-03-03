pub mod error;
pub mod graph;
pub mod models;
pub mod storage;

use chrono::Utc;
use error::{Error, Result};
use models::*;

/// Well-known slug for the default project.
pub const DEFAULT_PROJECT_SLUG: &str = "default";
/// Well-known story ID for the default (never-closing) story.
pub const DEFAULT_STORY_ID: &str = "default";
/// Human-readable name for the default project.
pub const DEFAULT_PROJECT_NAME: &str = "Default";
/// Human-readable name for the default story.
pub const DEFAULT_STORY_NAME: &str = "Default";

// ── Default project ────────────────────────────────────────────────

/// Ensure the default project exists with its default story.
/// Creates both if they don't exist. Returns the project.
pub fn ensure_default_project() -> Result<Project> {
    let project = match storage::load_project(DEFAULT_PROJECT_SLUG) {
        Ok(mut p) => {
            if !p.stories.contains_key(DEFAULT_STORY_ID) {
                let story = Story {
                    id: DEFAULT_STORY_ID.to_string(),
                    name: DEFAULT_STORY_NAME.to_string(),
                    tasks: std::collections::BTreeMap::new(),
                    task_deps: Vec::new(),
                    created_at: Utc::now(),
                };
                p.stories.insert(DEFAULT_STORY_ID.to_string(), story);
                storage::save_project(&p)?;
            }
            p
        }
        Err(Error::ProjectNotFound { .. }) => {
            let mut project = Project::new(
                DEFAULT_PROJECT_NAME.to_string(),
                DEFAULT_PROJECT_SLUG.to_string(),
            );
            let story = Story {
                id: DEFAULT_STORY_ID.to_string(),
                name: DEFAULT_STORY_NAME.to_string(),
                tasks: std::collections::BTreeMap::new(),
                task_deps: Vec::new(),
                created_at: Utc::now(),
            };
            project.stories.insert(DEFAULT_STORY_ID.to_string(), story);
            storage::save_project(&project)?;
            project
        }
        Err(e) => return Err(e),
    };
    Ok(project)
}

// ── Project operations ──────────────────────────────────────────────

pub fn create_project(name: &str) -> Result<Project> {
    let slug = storage::slugify(name);
    if slug.is_empty() {
        return Err(Error::InvalidDependency {
            reason: "project name must contain at least one alphanumeric character".into(),
        });
    }
    if slug == DEFAULT_PROJECT_SLUG {
        return Err(Error::InvalidDependency {
            reason: "'default' is a reserved project name".into(),
        });
    }

    // Check for duplicate
    let path = storage::get_storage_dir()?.join(format!("{slug}.json"));
    if path.exists() {
        return Err(Error::DuplicateProjectName { slug });
    }

    let project = Project::new(name.to_string(), slug);
    storage::save_project(&project)?;
    Ok(project)
}

pub fn list_projects() -> Result<Vec<Project>> {
    let slugs = storage::list_project_slugs()?;
    let mut projects = Vec::new();
    for slug in slugs {
        projects.push(storage::load_project(&slug)?);
    }
    Ok(projects)
}

pub fn get_project(slug: &str) -> Result<Project> {
    storage::load_project(slug)
}

// ── Story operations ────────────────────────────────────────────────

pub fn create_story(project_slug: &str, name: &str) -> Result<(Project, Story)> {
    let mut project = storage::load_project(project_slug)?;
    let id = project.next_story_id();
    let story = Story {
        id: id.clone(),
        name: name.to_string(),
        tasks: std::collections::BTreeMap::new(),
        task_deps: Vec::new(),
        created_at: Utc::now(),
    };
    project.stories.insert(id.clone(), story.clone());
    storage::save_project(&project)?;
    Ok((project, story))
}

pub fn add_story_dep(
    project_slug: &str,
    story_id: &str,
    depends_on: &str,
) -> Result<Project> {
    // The default story cannot have dependencies — it must always be unblocked
    if project_slug == DEFAULT_PROJECT_SLUG && story_id == DEFAULT_STORY_ID {
        return Err(Error::InvalidDependency {
            reason: "the default story cannot have dependencies".into(),
        });
    }

    let mut project = storage::load_project(project_slug)?;

    // Validate both stories exist
    if !project.stories.contains_key(story_id) {
        return Err(Error::StoryNotFound {
            id: story_id.to_string(),
        });
    }
    if !project.stories.contains_key(depends_on) {
        return Err(Error::StoryNotFound {
            id: depends_on.to_string(),
        });
    }

    // Check for duplicate edge
    let edge = (story_id.to_string(), depends_on.to_string());
    if project.story_deps.contains(&edge) {
        return Ok(project); // idempotent
    }

    // Cycle check
    let nodes: Vec<&str> = project.stories.keys().map(|s| s.as_str()).collect();
    graph::can_add_edge(&nodes, &project.story_deps, story_id, depends_on)?;

    project.story_deps.push(edge);
    storage::save_project(&project)?;
    Ok(project)
}

pub fn list_stories(project_slug: &str) -> Result<Project> {
    storage::load_project(project_slug)
}

// ── Task operations ─────────────────────────────────────────────────

pub fn create_task(
    project_slug: &str,
    story_id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<(Project, Task)> {
    let mut project = storage::load_project(project_slug)?;

    // Validate story exists
    if !project.stories.contains_key(story_id) {
        return Err(Error::StoryNotFound {
            id: story_id.to_string(),
        });
    }

    let id = project.next_task_id();
    let now = Utc::now();
    let task = Task {
        id: id.clone(),
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
        status: TaskStatus::Pending,
        agent: None,
        created_at: now,
        updated_at: now,
    };
    project
        .stories
        .get_mut(story_id)
        .unwrap()
        .tasks
        .insert(id.clone(), task.clone());
    storage::save_project(&project)?;
    Ok((project, task))
}

pub fn add_task_dep(
    project_slug: &str,
    task_id: &str,
    depends_on: &str,
) -> Result<Project> {
    let mut project = storage::load_project(project_slug)?;

    // Find which story contains both tasks (must be same story)
    let mut task_story_id = None;
    let mut dep_story_id = None;

    for (sid, story) in &project.stories {
        if story.tasks.contains_key(task_id) {
            task_story_id = Some(sid.clone());
        }
        if story.tasks.contains_key(depends_on) {
            dep_story_id = Some(sid.clone());
        }
    }

    let task_sid = task_story_id.ok_or_else(|| Error::TaskNotFound {
        id: task_id.to_string(),
    })?;
    let dep_sid = dep_story_id.ok_or_else(|| Error::TaskNotFound {
        id: depends_on.to_string(),
    })?;

    if task_sid != dep_sid {
        return Err(Error::InvalidDependency {
            reason: format!(
                "task dependencies must be within the same story (task {task_id} is in {task_sid}, {depends_on} is in {dep_sid})"
            ),
        });
    }

    let story = project.stories.get_mut(&task_sid).unwrap();

    // Check for duplicate edge
    let edge = (task_id.to_string(), depends_on.to_string());
    if story.task_deps.contains(&edge) {
        return Ok(project); // idempotent
    }

    // Cycle check
    let nodes: Vec<&str> = story.tasks.keys().map(|s| s.as_str()).collect();
    graph::can_add_edge(&nodes, &story.task_deps, task_id, depends_on)?;

    story.task_deps.push(edge);
    storage::save_project(&project)?;
    Ok(project)
}

pub fn get_task(project_slug: &str, task_id: &str) -> Result<(Project, String, Task)> {
    let project = storage::load_project(project_slug)?;

    // Find story_id and clone task before moving project
    let found = project
        .stories
        .iter()
        .find_map(|(sid, story)| {
            story.tasks.get(task_id).map(|task| (sid.clone(), task.clone()))
        });

    match found {
        Some((story_id, task)) => Ok((project, story_id, task)),
        None => Err(Error::TaskNotFound {
            id: task_id.to_string(),
        }),
    }
}

pub fn update_task(
    project_slug: &str,
    task_id: &str,
    new_status: Option<TaskStatus>,
    agent: Option<&str>,
) -> Result<(Project, Task)> {
    let mut project = storage::load_project(project_slug)?;

    // Find the task
    let mut found = false;
    for story in project.stories.values_mut() {
        if let Some(task) = story.tasks.get_mut(task_id) {
            if let Some(ref status) = new_status {
                graph::validate_transition(&task.status, status)?;
                task.status = status.clone();
            }
            if let Some(agent_name) = agent {
                task.agent = Some(agent_name.to_string());
            }
            task.updated_at = Utc::now();
            found = true;
            break;
        }
    }

    if !found {
        return Err(Error::TaskNotFound {
            id: task_id.to_string(),
        });
    }

    storage::save_project(&project)?;

    // Return updated task
    let task = project
        .stories
        .values()
        .find_map(|story| story.tasks.get(task_id).cloned())
        .expect("task was just updated, must exist");

    Ok((project, task))
}

pub fn next_task(project_slug: &str) -> Result<Option<(String, Task)>> {
    let project = storage::load_project(project_slug)?;
    let actionable = graph::get_actionable_tasks(&project);

    if actionable.is_empty() {
        return Ok(None);
    }

    // Pick a random actionable task
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..actionable.len());
    let (story, task) = actionable[idx];

    Ok(Some((story.id.clone(), task.clone())))
}
