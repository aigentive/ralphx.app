#!/bin/bash
# stream-watch-verify.sh - fswatch wrapper for verify stream
#
# Runs the verify stream once on startup, then watches for file changes
# and re-runs when manifest.json or PRD files are modified.
# Zero API calls when idle - only runs when work exists.

# Stream configuration
STREAM="verify"
MODEL="${RALPH_MODEL:-sonnet}"
WATCH_FILES=("specs/manifest.json" "specs/phases")

# Source common functions
source "$(dirname "$0")/stream-watch-common.sh"

# Start the watch loop (does not return)
start_watch_loop
