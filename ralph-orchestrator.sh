#!/bin/bash

# Ralph Wiggum Autonomous Orchestrator
# ====================================
# Runs all streams sequentially in round-robin fashion.
# Each stream has its own model configuration for cost optimization.
#
# Usage: ./ralph-orchestrator.sh
#
# Override models per stream:
#   MODEL_FEATURES=sonnet MODEL_REFACTOR=opus ./ralph-orchestrator.sh
#
# Default configuration:
#   - features: opus (most critical - PRD tasks + P0 gaps)
#   - refactor: sonnet (P1 large splits)
#   - polish: sonnet (P2/P3 cleanup)
#   - verify: sonnet (gap detection)
#   - hygiene: sonnet (backlog maintenance)

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# ===================================
# Per-stream model configuration
# Override via environment variables
# ===================================
MODEL_FEATURES=${MODEL_FEATURES:-opus}    # Features = opus (most critical)
MODEL_REFACTOR=${MODEL_REFACTOR:-sonnet}  # Refactor = sonnet (cost savings)
MODEL_POLISH=${MODEL_POLISH:-sonnet}      # Polish = sonnet (cost savings)
MODEL_VERIFY=${MODEL_VERIFY:-sonnet}      # Verify = sonnet (cost savings)
MODEL_HYGIENE=${MODEL_HYGIENE:-sonnet}    # Hygiene = sonnet (cost savings)

# ===================================
# Per-stream iteration counts
# ===================================
ITER_FEATURES=${ITER_FEATURES:-5}   # Most iterations - primary work
ITER_REFACTOR=${ITER_REFACTOR:-2}   # Large refactors
ITER_POLISH=${ITER_POLISH:-2}       # Small cleanups
ITER_VERIFY=${ITER_VERIFY:-1}       # Gap detection
ITER_HYGIENE=${ITER_HYGIENE:-1}     # Backlog maintenance

# Pause between cycles (seconds)
CYCLE_PAUSE=${CYCLE_PAUSE:-10}

# Track cycle count
CYCLE=0

# Cleanup function
cleanup() {
  echo ""
  echo -e "${YELLOW}Orchestrator interrupted. Cleaning up...${NC}"
  # Kill any running ralph-streams.sh processes
  pkill -P $$ 2>/dev/null || true
  exit 0
}

trap cleanup EXIT INT TERM

# Print banner
echo -e "${MAGENTA}========================================${NC}"
echo -e "${MAGENTA}   Ralph Wiggum Stream Orchestrator${NC}"
echo -e "${MAGENTA}========================================${NC}"
echo ""
echo -e "Model configuration:"
echo -e "  features: ${GREEN}$MODEL_FEATURES${NC} ($ITER_FEATURES iterations)"
echo -e "  refactor: ${GREEN}$MODEL_REFACTOR${NC} ($ITER_REFACTOR iterations)"
echo -e "  polish:   ${GREEN}$MODEL_POLISH${NC} ($ITER_POLISH iterations)"
echo -e "  verify:   ${GREEN}$MODEL_VERIFY${NC} ($ITER_VERIFY iterations)"
echo -e "  hygiene:  ${GREEN}$MODEL_HYGIENE${NC} ($ITER_HYGIENE iterations)"
echo ""
echo -e "Cycle pause: ${GREEN}${CYCLE_PAUSE}s${NC}"
echo ""
echo -e "${YELLOW}Starting in 3 seconds... Press Ctrl+C to abort${NC}"
sleep 3
echo ""

# Main orchestration loop
while true; do
  CYCLE=$((CYCLE + 1))

  echo -e "${MAGENTA}========================================${NC}"
  echo -e "${MAGENTA}   Cycle $CYCLE${NC}"
  echo -e "${MAGENTA}========================================${NC}"
  echo ""

  # --- Features Stream ---
  # Primary work: PRD tasks and P0 critical gap fixes
  echo -e "${BLUE}--- Features Stream (${MODEL_FEATURES}, ${ITER_FEATURES} iterations) ---${NC}"
  ANTHROPIC_MODEL=$MODEL_FEATURES ./ralph-streams.sh features $ITER_FEATURES || {
    echo -e "${YELLOW}Features stream completed or max iterations reached${NC}"
  }
  echo ""

  # --- Refactor Stream ---
  # P1 items: large file splits, architectural refactors
  echo -e "${BLUE}--- Refactor Stream (${MODEL_REFACTOR}, ${ITER_REFACTOR} iterations) ---${NC}"
  ANTHROPIC_MODEL=$MODEL_REFACTOR ./ralph-streams.sh refactor $ITER_REFACTOR || {
    echo -e "${YELLOW}Refactor stream completed or max iterations reached${NC}"
  }
  echo ""

  # --- Polish Stream ---
  # P2/P3 items: type fixes, lint fixes, small extractions
  echo -e "${BLUE}--- Polish Stream (${MODEL_POLISH}, ${ITER_POLISH} iterations) ---${NC}"
  ANTHROPIC_MODEL=$MODEL_POLISH ./ralph-streams.sh polish $ITER_POLISH || {
    echo -e "${YELLOW}Polish stream completed or max iterations reached${NC}"
  }
  echo ""

  # --- Verify Stream ---
  # Gap detection: scan completed phases, produce P0 items
  echo -e "${BLUE}--- Verify Stream (${MODEL_VERIFY}, ${ITER_VERIFY} iterations) ---${NC}"
  ANTHROPIC_MODEL=$MODEL_VERIFY ./ralph-streams.sh verify $ITER_VERIFY || {
    echo -e "${YELLOW}Verify stream completed or max iterations reached${NC}"
  }
  echo ""

  # --- Hygiene Stream ---
  # Backlog maintenance: archive, refill, validate
  echo -e "${BLUE}--- Hygiene Stream (${MODEL_HYGIENE}, ${ITER_HYGIENE} iterations) ---${NC}"
  ANTHROPIC_MODEL=$MODEL_HYGIENE ./ralph-streams.sh hygiene $ITER_HYGIENE || {
    echo -e "${YELLOW}Hygiene stream completed or max iterations reached${NC}"
  }
  echo ""

  # Pause between cycles
  echo -e "${YELLOW}Cycle $CYCLE complete. Pausing ${CYCLE_PAUSE}s before next cycle...${NC}"
  echo ""
  sleep $CYCLE_PAUSE
done
