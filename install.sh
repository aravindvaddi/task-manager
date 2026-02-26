#!/usr/bin/env bash
set -euo pipefail

echo "Installing task-manager..."

if ! command -v cargo &>/dev/null; then
    echo "Error: cargo not found. Install Rust first: https://rustup.rs/"
    exit 1
fi

cargo install --path .

echo ""
echo "Installed! Run 'tm --help' or 'task-manager --help' to get started."
