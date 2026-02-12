#!/bin/bash
# Session Recovery Monitoring Script
#
# Usage: ./scripts/monitor-session-recovery.sh [log-file]
#
# This script monitors session recovery events and provides statistics.
# If no log file is provided, it reads from stdin (for live monitoring with tail -f).

set -euo pipefail

LOG_FILE="${1:-}"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to extract recovery events
extract_events() {
    local input_source="$1"

    if [ -z "$input_source" ]; then
        # Read from stdin
        grep -E "event=(stale_session_detected|rehydrate_success|rehydrate_failure)"
    else
        # Read from file
        grep -E "event=(stale_session_detected|rehydrate_success|rehydrate_failure)" "$input_source" || true
    fi
}

# Function to count events by type
count_events() {
    local event_type="$1"
    grep -c "event=$event_type" || echo "0"
}

# Function to extract average duration
average_duration() {
    grep "event=rehydrate_success" | \
        grep -oE "duration_ms=[0-9]+" | \
        cut -d= -f2 | \
        awk '{ sum += $1; n++ } END { if (n > 0) print sum / n; else print "0" }'
}

# Function to extract recovery stats
extract_stats() {
    local events="$1"

    local detected=$(echo "$events" | count_events "stale_session_detected")
    local success=$(echo "$events" | count_events "rehydrate_success")
    local failure=$(echo "$events" | count_events "rehydrate_failure")
    local avg_duration=$(echo "$events" | average_duration)

    echo "=== Session Recovery Statistics ==="
    echo ""
    echo -e "${BLUE}Stale Sessions Detected:${NC} $detected"
    echo -e "${GREEN}Successful Recoveries:${NC}   $success"
    echo -e "${RED}Failed Recoveries:${NC}       $failure"

    if [ "$success" -gt 0 ] || [ "$failure" -gt 0 ]; then
        local total=$((success + failure))
        local success_rate=$(echo "scale=2; $success * 100 / $total" | bc)
        echo ""
        echo -e "${YELLOW}Success Rate:${NC} ${success_rate}% (${success}/${total})"

        if [ "$success" -gt 0 ]; then
            echo -e "${YELLOW}Average Duration:${NC} ${avg_duration}ms"
        fi

        echo ""
        if (( $(echo "$success_rate >= 95" | bc -l) )); then
            echo -e "${GREEN}✓ SUCCESS RATE MEETS CRITERIA (>= 95%)${NC}"
        else
            echo -e "${RED}✗ SUCCESS RATE BELOW CRITERIA (< 95%)${NC}"
        fi
    else
        echo ""
        echo -e "${YELLOW}No recovery attempts found${NC}"
    fi
    echo ""
}

# Function to show recent events with details
show_recent_events() {
    local events="$1"
    local limit="${2:-10}"

    echo "=== Recent Events (last $limit) ==="
    echo ""

    echo "$events" | tail -n "$limit" | while IFS= read -r line; do
        if echo "$line" | grep -q "stale_session_detected"; then
            echo -e "${BLUE}[DETECTED]${NC} $line"
        elif echo "$line" | grep -q "rehydrate_success"; then
            echo -e "${GREEN}[SUCCESS]${NC}  $line"
        elif echo "$line" | grep -q "rehydrate_failure"; then
            echo -e "${RED}[FAILURE]${NC}  $line"
        fi
    done
    echo ""
}

# Main execution
main() {
    if [ -z "$LOG_FILE" ]; then
        echo "Reading from stdin... (pipe logs or use tail -f)"
        echo "Press Ctrl+C to stop"
        echo ""

        # Live monitoring mode
        while IFS= read -r line; do
            if echo "$line" | grep -qE "event=(stale_session_detected|rehydrate_success|rehydrate_failure)"; then
                if echo "$line" | grep -q "stale_session_detected"; then
                    echo -e "${BLUE}[DETECTED]${NC} $line"
                elif echo "$line" | grep -q "rehydrate_success"; then
                    echo -e "${GREEN}[SUCCESS]${NC}  $line"
                elif echo "$line" | grep -q "rehydrate_failure"; then
                    echo -e "${RED}[FAILURE]${NC}  $line"
                fi
            fi
        done
    else
        # File analysis mode
        if [ ! -f "$LOG_FILE" ]; then
            echo "Error: Log file '$LOG_FILE' not found"
            exit 1
        fi

        echo "Analyzing log file: $LOG_FILE"
        echo ""

        local events=$(extract_events "$LOG_FILE")

        if [ -z "$events" ]; then
            echo "No session recovery events found in log file."
            exit 0
        fi

        extract_stats "$events"
        show_recent_events "$events" 10
    fi
}

main "$@"
