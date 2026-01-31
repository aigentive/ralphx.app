#!/bin/bash
# stream-watch-polish.sh - fswatch wrapper for polish stream
#
# Runs the polish stream once on startup, then watches for file changes
# and re-runs when backlog.md is modified.
# Zero API calls when idle - only runs when work exists.

# Stream configuration
STREAM="polish"
MODEL="${RALPH_MODEL:-sonnet}"
WATCH_FILES=("streams/polish/backlog.md")

# Source common functions
source "$(dirname "$0")/stream-watch-common.sh"

# Start the watch loop (does not return)
start_watch_loop
