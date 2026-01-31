#!/bin/bash
# stream-watch-visual-qa.sh - fswatch wrapper for visual-qa stream
#
# Runs the visual-qa stream once on startup, then watches for file changes
# and re-runs when manifest.md or backlog.md are modified.
# Zero API calls when idle - only runs when work exists.

# Stream configuration
STREAM="visual-qa"
MODEL="${RALPH_MODEL:-sonnet}"
WATCH_FILES=("streams/visual-qa/manifest.md" "streams/visual-qa/backlog.md")

# Source common functions
source "$(dirname "$0")/stream-watch-common.sh"

# Start the watch loop (does not return)
start_watch_loop
