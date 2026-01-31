#!/bin/bash
# stream-watch-features.sh - fswatch wrapper for features stream
#
# Runs the features stream once on startup, then watches for file changes
# and re-runs when backlog.md or manifest.json are modified.
# Zero API calls when idle - only runs when work exists.

# Stream configuration
STREAM="features"
MODEL="${RALPH_MODEL:-opus}"
WATCH_FILES=("streams/features/backlog.md" "specs/manifest.json")

# Source common functions
source "$(dirname "$0")/stream-watch-common.sh"

# Start the watch loop (does not return)
start_watch_loop
