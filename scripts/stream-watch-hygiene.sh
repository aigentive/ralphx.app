#!/bin/bash
# stream-watch-hygiene.sh - fswatch wrapper for hygiene stream
#
# Watches backlog files with a 10-minute delay to batch changes.
# This prevents hygiene from running after every single commit while still
# reacting to accumulated changes automatically.

STREAM="hygiene"
MODEL="sonnet"
WATCH_FILES=("streams/refactor/backlog.md" "streams/polish/backlog.md" "streams/features/backlog.md" "streams/archive/completed.md")

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

cleanup() {
    echo ""
    echo -e "${YELLOW}[$STREAM] Shutting down...${NC}"
    # Kill ralph-streams processes for this stream first (they have their own cleanup)
    pkill -INT -f "ralph-streams.sh $STREAM" 2>/dev/null || true
    sleep 0.3
    # Force kill if still running
    pkill -9 -f "ralph-streams.sh $STREAM" 2>/dev/null || true
    # Kill all remaining child processes (fswatch)
    pkill -9 -P $$ 2>/dev/null || true
    exit 0
}

trap cleanup SIGINT SIGTERM EXIT

echo -e "${GREEN}[$STREAM] Starting with fswatch...${NC}"
echo -e "${BLUE}[$STREAM] Model: $MODEL${NC}"
echo -e "${BLUE}[$STREAM] Watching: ${WATCH_FILES[*]}${NC}"
echo ""

# Use 10-minute latency to batch changes and prevent excessive runs
fswatch -o -l 600 "${WATCH_FILES[@]}" | while read; do
    echo ""
    echo -e "${YELLOW}[$STREAM] File change detected, starting cycle...${NC}"
    ANTHROPIC_MODEL=$MODEL ./ralph-streams.sh $STREAM 50 </dev/null
    echo ""
    echo -e "${GREEN}[$STREAM] IDLE - watching for file changes...${NC}"
done &

# Give fswatch time to initialize before initial cycle
sleep 0.5

# Initial run (fswatch is already watching, so changes will be caught)
echo -e "${YELLOW}[$STREAM] Running initial cycle...${NC}"
ANTHROPIC_MODEL=$MODEL ./ralph-streams.sh $STREAM 50

# After initial cycle, show idle status and wait for background jobs
echo ""
echo -e "${GREEN}[$STREAM] IDLE - watching for file changes...${NC}"
echo -e "${BLUE}[$STREAM] Watching: ${WATCH_FILES[*]}${NC}"
echo -e "${BLUE}[$STREAM] Will auto-start when files change...${NC}"
echo ""

# Wait for background jobs (or be killed via trap)
wait
