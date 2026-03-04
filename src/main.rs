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
        cli::Commands::Next { project_slug } => cli::next::handle(&project_slug, pretty),
    };

    if let Err(e) = result {
        let value = json!({ "error": e.to_string() });
        cli::output_json(&value, pretty);
        process::exit(1);
    }
}
