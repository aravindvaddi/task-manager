# tm - Task Manager

A CLI task management tool for LLM agent workflows. Organize work into projects, stories, and tasks with DAG dependencies and strict state machine transitions. Designed as the backbone for systems where a dispatcher agent (or human) creates work and worker agents pull actionable tasks.

## Features

- **Project / Story / Task hierarchy** - organize work at multiple levels
- **DAG dependencies** - story-to-story and task-to-task dependencies with cycle detection
- **State machine** - enforced task lifecycle: `pending → running → closed`
- **Actionable task selection** - `tm next` finds tasks ready to work on across unblocked stories
- **JSON output** - all commands output JSON by default, `--pretty` for human-readable
- **Dual binary** - invoke as `task-manager` or `tm`

## Installation

Requires [Rust](https://rustup.rs/) (1.85+ for edition 2024).

```bash
# Clone and install
git clone https://github.com/aravindvaddi/task-manager.git
cd task-manager
./install.sh

# Or manually
cargo install --path .
```

## Quick Start

```bash
# Create a project
tm project create "My Project"

# Create stories
tm story create my-project "Setup infrastructure"
tm story create my-project "Build API"

# Add story dependency (Build API depends on Setup infrastructure)
tm story dep s2 --depends-on s1 --project my-project

# Create tasks in a story
tm task create s1 "Setup database" --project my-project --description "Configure PostgreSQL"
tm task create s1 "Setup CI/CD" --project my-project

# Add task dependency (within same story)
tm task dep t2 --depends-on t1 --project my-project

# Get next actionable task
tm next my-project

# Work on a task
tm task update t1 --project my-project --status running --agent "worker-1"
tm task update t1 --project my-project --status closed --reason successful

# Check project status
tm --pretty project status my-project
```

## Usage

### Projects

```bash
tm project create <name>
tm project list
tm project status <slug>
```

### Stories

Stories are groupings of related tasks within a project. Story status is derived: **closed** when all tasks are closed (and at least one task exists), **open** otherwise.

```bash
tm story create <project-slug> <name>
tm story dep <story-id> --depends-on <story-id> --project <slug>
tm story list <project-slug>
tm story status <story-id> --project <slug>
```

### Tasks

```bash
tm task create <story-id> <name> --project <slug> [--description "..."]
tm task dep <task-id> --depends-on <task-id> --project <slug>
tm task get <task-id> --project <slug>
tm task list <story-id> --project <slug>
tm task update <task-id> --project <slug> --status <status> [--reason <reason>] [--agent <name>]
```

### Next

Returns a random actionable task — one that is `pending`, has all task dependencies closed, and belongs to an unblocked story.

```bash
tm next <project-slug>
```

## Data Model

```
Project
  ├── Stories (DAG - story-to-story dependencies)
  │     └── Tasks (DAG within story - task-to-task dependencies)
  └── Complete when: all stories closed
```

### Task State Machine

```
pending ──→ running ──→ closed(successful)
  │            │
  │            └───────→ closed(not_required)
  │
  └────────────────────→ closed(not_required)
```

- `pending → running` — start working on a task
- `pending → closed(not_required)` — skip a task before starting it
- `running → closed(successful)` — task completed successfully
- `running → closed(not_required)` — task no longer needed after starting
- `pending → closed(successful)` — **not allowed** (can't succeed at work never started)
- `running → pending` — **not allowed** (no going back)
- `closed → anything` — **not allowed** (terminal state)

## Storage

Project data is stored as JSON files in `~/.task-manager/projects/<project-slug>.json`. Writes are atomic (temp file + rename) to prevent corruption.

## Output

All commands output compact JSON by default. Use the `--pretty` global flag for formatted output:

```bash
# Compact JSON (default)
tm project list

# Pretty-printed JSON
tm --pretty project list
```

## License

[MIT](LICENSE)
