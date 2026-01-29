#!/bin/bash
# stream-watch-verify.sh - fswatch wrapper for verify stream
#
# Runs the verify stream once on startup, then watches for file changes
# and re-runs when manifest.json or PRD files are modified.
# Zero API calls when idle - only runs when work exists.

STREAM="verify"
MODEL="sonnet"
WATCH_FILES=("specs/manifest.json" "specs/phases")

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

# Start fswatch FIRST to avoid race condition with initial cycle
# Use latency to debounce rapid changes (wait 3s after last change)
fswatch -o -l 3 "${WATCH_FILES[@]}" | while read; do
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

# After initial cycle, show idle status and wait for fswatch
echo ""
echo -e "${GREEN}[$STREAM] IDLE - watching for file changes...${NC}"
echo -e "${BLUE}[$STREAM] Watching: ${WATCH_FILES[*]}${NC}"
echo -e "${BLUE}[$STREAM] Will auto-start when files change...${NC}"
echo ""

# Wait for background jobs (or be killed via trap)
wait
