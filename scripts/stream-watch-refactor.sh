#!/bin/bash
# stream-watch-refactor.sh - fswatch wrapper for refactor stream
#
# Runs the refactor stream once on startup, then watches for file changes
# and re-runs when backlog.md is modified.
# Zero API calls when idle - only runs when work exists.

STREAM="refactor"
DEFAULT_MODEL="sonnet"
MODEL="${RALPH_MODEL:-$DEFAULT_MODEL}"
WATCH_FILES=("streams/refactor/backlog.md")
LOCK_FILE=".stream-${STREAM}-lock"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Padding for visual spacing
PAD="  "

cleanup() {
    echo ""
    echo -e "${PAD}${YELLOW}[$STREAM] Shutting down...${NC}"
    # Remove lock file
    rm -f "$LOCK_FILE"
    # Kill ralph-streams processes for this stream first (they have their own cleanup)
    pkill -INT -f "ralph-streams.sh $STREAM" 2>/dev/null || true
    sleep 0.3
    # Force kill if still running
    pkill -9 -f "ralph-streams.sh $STREAM" 2>/dev/null || true
    # Kill all remaining child processes (fswatch)
    pkill -9 -P $$ 2>/dev/null || true
    exit 0
}

# Check if a PID is still running
is_pid_alive() {
    kill -0 "$1" 2>/dev/null
}

# Run a cycle with lock protection
run_cycle() {
    local trigger="$1"

    # Atomic lock acquisition using noclobber
    # This prevents TOCTOU race between check and acquire
    if ! ( set -o noclobber; echo $$ > "$LOCK_FILE" ) 2>/dev/null; then
        echo -e "${PAD}${BLUE}[$STREAM] Already running, skipping trigger from $trigger${NC}"
        return 0
    fi

    echo -e "${PAD}${YELLOW}[$STREAM] Starting cycle ($trigger)...${NC}"
    ANTHROPIC_MODEL=$MODEL ./ralph-streams.sh $STREAM 50 </dev/null

    # Release lock
    rm -f "$LOCK_FILE"

    echo ""
    echo -e "${PAD}${GREEN}[$STREAM] IDLE - watching for file changes...${NC}"
}

trap cleanup SIGINT SIGTERM EXIT

# Clean up stale lock from previous run (only if PID is dead)
if [ -f "$LOCK_FILE" ]; then
    OLD_PID=$(cat "$LOCK_FILE" 2>/dev/null)
    if [ -n "$OLD_PID" ] && is_pid_alive "$OLD_PID"; then
        echo -e "${PAD}${RED}[$STREAM] Another instance is already running (PID $OLD_PID)${NC}"
        echo -e "${PAD}${RED}[$STREAM] Kill it first or wait for it to finish${NC}"
        exit 1
    else
        echo -e "${PAD}${YELLOW}[$STREAM] Removing stale lock (PID $OLD_PID is dead)${NC}"
        rm -f "$LOCK_FILE"
    fi
fi

echo ""
echo -e "${PAD}${GREEN}[$STREAM] Starting with fswatch...${NC}"
echo -e "${PAD}${BLUE}[$STREAM] Model: $MODEL${NC}"
echo -e "${PAD}${BLUE}[$STREAM] Watching: ${WATCH_FILES[*]}${NC}"
echo ""

# Use latency to debounce rapid changes (wait 3s after last change)
fswatch -o -l 3 "${WATCH_FILES[@]}" | while read; do
    echo ""
    run_cycle "file change"
done &

# Give fswatch time to initialize before initial cycle
sleep 0.5

# Initial run (fswatch is already watching, so changes will be caught)
run_cycle "initial"

# After initial cycle, show watching status
echo -e "${PAD}${BLUE}[$STREAM] Watching: ${WATCH_FILES[*]}${NC}"
echo -e "${PAD}${BLUE}[$STREAM] Will auto-start when files change...${NC}"
echo ""

# Wait for background jobs (or be killed via trap)
wait
