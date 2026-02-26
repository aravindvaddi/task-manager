mod cli;

use clap::Parser;
use serde_json::json;
use std::process;

fn main() {
    let args = cli::Cli::parse();
    let pretty = args.pretty;

    let result = match args.command {
        cli::Commands::Project { command } => cli::project::handle(command, pretty),
        cli::Commands::Story { command } => cli::story::handle(command, pretty),
        cli::Commands::Task { command } => cli::task::handle(command, pretty),
        cli::Commands::Next { project_slug } => {
            match task_manager::next_task(&project_slug) {
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
                    cli::output_json(&value, pretty);
                    Ok(())
                }
                Ok(None) => {
                    let value = json!({
                        "task": null,
                        "message": "no actionable tasks"
                    });
                    cli::output_json(&value, pretty);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
    };

    if let Err(e) = result {
        let value = json!({ "error": e.to_string() });
        cli::output_json(&value, pretty);
        process::exit(1);
    }
}
