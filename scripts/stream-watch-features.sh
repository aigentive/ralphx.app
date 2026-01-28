#!/bin/bash
# stream-watch-features.sh - fswatch wrapper for features stream
#
# Runs the features stream once on startup, then watches for file changes
# and re-runs when backlog.md or manifest.json are modified.
# Zero API calls when idle - only runs when work exists.

STREAM="features"
MODEL="opus"
WATCH_FILES="streams/features/backlog.md specs/manifest.json"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}[$STREAM] Starting with fswatch...${NC}"
echo -e "${BLUE}[$STREAM] Model: $MODEL${NC}"
echo -e "${BLUE}[$STREAM] Watching: $WATCH_FILES${NC}"
echo ""

# Initial run
echo -e "${YELLOW}[$STREAM] Running initial cycle...${NC}"
ANTHROPIC_MODEL=$MODEL ./ralph-streams.sh $STREAM 50

# Watch for changes and re-run
echo ""
echo -e "${GREEN}[$STREAM] IDLE - watching for file changes...${NC}"
echo -e "${BLUE}[$STREAM] Watching: $WATCH_FILES${NC}"
echo -e "${BLUE}[$STREAM] Will auto-start when files change...${NC}"
echo ""

fswatch -o $WATCH_FILES | while read; do
    echo ""
    echo -e "${YELLOW}[$STREAM] File change detected, starting cycle...${NC}"
    ANTHROPIC_MODEL=$MODEL ./ralph-streams.sh $STREAM 50
    echo ""
    echo -e "${GREEN}[$STREAM] IDLE - watching for file changes...${NC}"
done
