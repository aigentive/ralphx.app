#!/bin/bash
# stream-watch-features.sh - fswatch wrapper for features stream
#
# Runs the features stream once on startup, then watches for file changes
# and re-runs when backlog.md or manifest.json are modified.
# Zero API calls when idle - only runs when work exists.

STREAM="features"
DEFAULT_MODEL="opus"
MODEL="${RALPH_MODEL:-$DEFAULT_MODEL}"
WATCH_FILES=("streams/features/backlog.md" "specs/manifest.json")
LOCK_FILE=".stream-${STREAM}-lock"

# Colors for output
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

# Run a cycle with lock protection
run_cycle() {
    local trigger="$1"

    # Check if already running
    if [ -f "$LOCK_FILE" ]; then
        echo -e "${PAD}${BLUE}[$STREAM] Already running, skipping trigger from $trigger${NC}"
        return 0
    fi

    # Acquire lock
    echo $$ > "$LOCK_FILE"

    echo -e "${PAD}${YELLOW}[$STREAM] Starting cycle ($trigger)...${NC}"
    ANTHROPIC_MODEL=$MODEL ./ralph-streams.sh $STREAM 50 </dev/null

    # Release lock
    rm -f "$LOCK_FILE"

    echo ""
    echo -e "${PAD}${GREEN}[$STREAM] IDLE - watching for file changes...${NC}"
}

trap cleanup SIGINT SIGTERM EXIT

# Clean up stale lock from previous run
rm -f "$LOCK_FILE"

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
