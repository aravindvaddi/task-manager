use std::fs;
use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::models::Project;

/// Slugify a project name: lowercase, replace non-alphanumeric with hyphens,
/// collapse multiple hyphens, trim leading/trailing hyphens.
pub fn slugify(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();

    // Collapse multiple hyphens and trim
    let mut result = String::new();
    let mut prev_hyphen = false;
    for c in slug.chars() {
        if c == '-' {
            if !prev_hyphen && !result.is_empty() {
                result.push('-');
            }
            prev_hyphen = true;
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }

    // Trim trailing hyphen
    if result.ends_with('-') {
        result.pop();
    }

    result
}

/// Get the storage directory, creating it if needed.
pub fn get_storage_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "could not determine home directory",
        ))
    })?;
    let dir = home.join(".task-manager").join("projects");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn project_path(slug: &str) -> Result<PathBuf> {
    Ok(get_storage_dir()?.join(format!("{slug}.json")))
}

/// Load a project from disk.
pub fn load_project(slug: &str) -> Result<Project> {
    let path = project_path(slug)?;
    if !path.exists() {
        return Err(Error::ProjectNotFound {
            slug: slug.to_string(),
        });
    }
    let data = fs::read_to_string(&path)?;
    let project: Project = serde_json::from_str(&data)?;
    Ok(project)
}

/// Save a project to disk using atomic write (temp file + rename).
pub fn save_project(project: &Project) -> Result<()> {
    let path = project_path(&project.slug)?;
    let dir = path.parent().unwrap();

    // Write to temp file first
    let tmp_path = dir.join(format!(".{}.tmp", project.slug));
    let data = serde_json::to_string_pretty(project)?;
    fs::write(&tmp_path, &data)?;

    // Atomic rename
    fs::rename(&tmp_path, &path)?;
    Ok(())
}

/// List all project slugs from the storage directory.
pub fn list_project_slugs() -> Result<Vec<String>> {
    let dir = get_storage_dir()?;
    let mut slugs = Vec::new();

    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                // Skip temp files
                if !stem.starts_with('.') {
                    slugs.push(stem.to_string());
                }
            }
        }
    }

    slugs.sort();
    Ok(slugs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("My Cool Project"), "my-cool-project");
        assert_eq!(slugify("hello world!"), "hello-world");
        assert_eq!(slugify("  spaces  "), "spaces");
        assert_eq!(slugify("UPPER"), "upper");
        assert_eq!(slugify("a--b"), "a-b");
        assert_eq!(slugify("test_project"), "test-project");
    }
}
