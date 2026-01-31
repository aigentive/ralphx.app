#!/bin/bash
# stream-watch-hygiene.sh - fswatch wrapper for hygiene stream
#
# Runs the hygiene stream once on startup, then watches for file changes
# and re-runs when any backlog files are modified.
# Zero API calls when idle - only runs when work exists.

# Stream configuration
STREAM="hygiene"
MODEL="${RALPH_MODEL:-sonnet}"
WATCH_FILES=("streams/refactor/backlog.md" "streams/polish/backlog.md" "streams/features/backlog.md" "streams/archive/completed.md")
FSWATCH_LATENCY=600  # 10 minutes - batch changes to prevent excessive runs

# Source common functions
source "$(dirname "$0")/stream-watch-common.sh"

# Start the watch loop (does not return)
start_watch_loop
