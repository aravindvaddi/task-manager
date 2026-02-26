pub mod project;
pub mod story;
pub mod task;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "task-manager", about = "CLI task management for LLM agent workflows")]
pub struct Cli {
    /// Pretty-print JSON output
    #[arg(long, global = true)]
    pub pretty: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage projects
    Project {
        #[command(subcommand)]
        command: project::ProjectCommand,
    },
    /// Manage stories
    Story {
        #[command(subcommand)]
        command: story::StoryCommand,
    },
    /// Manage tasks
    Task {
        #[command(subcommand)]
        command: task::TaskCommand,
    },
    /// Get a random actionable task from a project
    Next {
        /// Project slug
        project_slug: String,
    },
}

pub fn output_json(value: &serde_json::Value, pretty: bool) {
    if pretty {
        println!("{}", serde_json::to_string_pretty(value).unwrap());
    } else {
        println!("{}", serde_json::to_string(value).unwrap());
    }
}
