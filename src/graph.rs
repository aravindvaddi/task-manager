use std::collections::{HashMap, HashSet};

use crate::error::{Error, Result};
use crate::models::*;

/// Check if adding an edge (from -> depends_on) would create a cycle.
/// Returns Ok(()) if safe, Err(CycleDetected) if it would create a cycle.
pub fn can_add_edge(
    nodes: &[&str],
    edges: &[(String, String)],
    from: &str,
    depends_on: &str,
) -> Result<()> {
    if from == depends_on {
        return Err(Error::CycleDetected);
    }

    // Build adjacency list: id -> [ids it depends on]
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in nodes {
        adj.entry(node).or_default();
    }
    for (id, dep) in edges {
        adj.entry(id.as_str()).or_default().push(dep.as_str());
    }
    // Add the proposed edge
    adj.entry(from).or_default().push(depends_on);

    // DFS cycle detection
    has_cycle(&adj, nodes)
}

fn has_cycle(adj: &HashMap<&str, Vec<&str>>, nodes: &[&str]) -> Result<()> {
    #[derive(Clone, Copy, PartialEq)]
    enum Color {
        White,
        Gray,
        Black,
    }

    let mut color: HashMap<&str, Color> = HashMap::new();
    for node in nodes {
        color.insert(node, Color::White);
    }

    fn dfs<'a>(
        node: &'a str,
        adj: &HashMap<&'a str, Vec<&'a str>>,
        color: &mut HashMap<&'a str, Color>,
    ) -> bool {
        color.insert(node, Color::Gray);
        if let Some(neighbors) = adj.get(node) {
            for &neighbor in neighbors {
                match color.get(neighbor) {
                    Some(Color::Gray) => return true, // cycle
                    Some(Color::White) | None => {
                        if dfs(neighbor, adj, color) {
                            return true;
                        }
                    }
                    Some(Color::Black) => {} // already fully explored
                }
            }
        }
        color.insert(node, Color::Black);
        false
    }

    for node in nodes {
        if color.get(node) == Some(&Color::White) {
            if dfs(node, adj, &mut color) {
                return Err(Error::CycleDetected);
            }
        }
    }

    Ok(())
}

/// Determine if a story is closed: all tasks closed AND at least 1 task exists.
pub fn is_story_closed(story: &Story) -> bool {
    if story.tasks.is_empty() {
        return false;
    }
    story.tasks.values().all(|t| matches!(t.status, TaskStatus::Closed { .. }))
}

/// Get the derived status of a story.
pub fn get_story_status(story: &Story) -> StoryStatus {
    if is_story_closed(story) {
        StoryStatus::Closed
    } else {
        StoryStatus::Open
    }
}

/// Check if a story is blocked by its story-level dependencies.
/// A story is blocked if any of its dependency stories are not closed.
pub fn is_story_blocked(project: &Project, story_id: &str) -> bool {
    let dep_story_ids: Vec<&str> = project
        .story_deps
        .iter()
        .filter(|(id, _)| id == story_id)
        .map(|(_, dep)| dep.as_str())
        .collect();

    for dep_id in dep_story_ids {
        if let Some(dep_story) = project.stories.get(dep_id) {
            if !is_story_closed(dep_story) {
                return true;
            }
        } else {
            // Dependency story doesn't exist — treat as blocked
            return true;
        }
    }
    false
}

/// Get actionable tasks across the project:
/// 1. Find unblocked stories (all story deps are closed)
/// 2. Within those, find tasks that are pending with all task deps closed
pub fn get_actionable_tasks(project: &Project) -> Vec<(&Story, &Task)> {
    let mut actionable = Vec::new();

    for (story_id, story) in &project.stories {
        // Skip closed stories
        if is_story_closed(story) {
            continue;
        }

        // Skip blocked stories
        if is_story_blocked(project, story_id) {
            continue;
        }

        // Find actionable tasks within this story
        let closed_tasks: HashSet<&str> = story
            .tasks
            .values()
            .filter(|t| matches!(t.status, TaskStatus::Closed { .. }))
            .map(|t| t.id.as_str())
            .collect();

        for task in story.tasks.values() {
            if !matches!(task.status, TaskStatus::Pending) {
                continue;
            }

            // Check all task deps are closed
            let all_deps_closed = story
                .task_deps
                .iter()
                .filter(|(id, _)| id == &task.id)
                .all(|(_, dep)| closed_tasks.contains(dep.as_str()));

            if all_deps_closed {
                actionable.push((story, task));
            }
        }
    }

    actionable
}

/// Validate a task state transition.
pub fn validate_transition(
    current: &TaskStatus,
    new_status: &TaskStatus,
) -> Result<()> {
    match (current, new_status) {
        // pending -> running: OK
        (TaskStatus::Pending, TaskStatus::Running) => Ok(()),

        // pending -> closed(not_required): OK
        (TaskStatus::Pending, TaskStatus::Closed { reason: CloseReason::NotRequired }) => Ok(()),

        // pending -> closed(successful): NOT OK
        (TaskStatus::Pending, TaskStatus::Closed { reason: CloseReason::Successful }) => {
            Err(Error::InvalidStateTransition {
                from: "pending".into(),
                to: "closed(successful)".into(),
                reason: "cannot mark a task as successful that was never started".into(),
            })
        }

        // running -> closed(successful): OK
        (TaskStatus::Running, TaskStatus::Closed { reason: CloseReason::Successful }) => Ok(()),

        // running -> closed(not_required): OK
        (TaskStatus::Running, TaskStatus::Closed { reason: CloseReason::NotRequired }) => Ok(()),

        // running -> pending: NOT OK
        (TaskStatus::Running, TaskStatus::Pending) => {
            Err(Error::InvalidStateTransition {
                from: "running".into(),
                to: "pending".into(),
                reason: "cannot revert a running task to pending".into(),
            })
        }

        // closed -> anything: NOT OK
        (TaskStatus::Closed { .. }, _) => {
            Err(Error::InvalidStateTransition {
                from: current.to_string(),
                to: new_status.to_string(),
                reason: "closed is a terminal state".into(),
            })
        }

        // Same state
        _ => Err(Error::InvalidStateTransition {
            from: current.to_string(),
            to: new_status.to_string(),
            reason: "not a valid transition".into(),
        }),
    }
}
