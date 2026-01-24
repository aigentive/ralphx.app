#!/bin/bash
# Lint fix script for PostToolUse hook
# Runs linting with auto-fix on TypeScript and Rust files

set -e

# Determine project root (assumes hook is run from project root)
PROJECT_ROOT="${PWD}"

# Check if npm is available and package.json exists
if [ -f "${PROJECT_ROOT}/package.json" ]; then
    # Check if lint:fix script exists
    if npm run --silent 2>/dev/null | grep -q "lint:fix"; then
        echo "Running TypeScript lint fix..."
        npm run lint:fix --silent 2>/dev/null || true
    fi
fi

# Check if Cargo.toml exists for Rust projects
if [ -f "${PROJECT_ROOT}/src-tauri/Cargo.toml" ]; then
    # Run clippy with auto-fix if available
    if command -v cargo &> /dev/null; then
        echo "Running Rust clippy fix..."
        cargo clippy --manifest-path "${PROJECT_ROOT}/src-tauri/Cargo.toml" --fix --allow-dirty --allow-staged 2>/dev/null || true
    fi
fi

echo "Lint fix complete."
