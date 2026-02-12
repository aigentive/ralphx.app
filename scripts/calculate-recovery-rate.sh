#!/bin/bash
# Calculate Session Recovery Success Rate
#
# Usage: ./scripts/calculate-recovery-rate.sh <log-file>
#
# This script calculates the recovery success rate and provides detailed metrics
# for the session recovery feature.

set -euo pipefail

if [ $# -eq 0 ]; then
    echo "Usage: $0 <log-file>"
    echo "Example: $0 logs/ralphx.log"
    exit 1
fi

LOG_FILE="$1"

if [ ! -f "$LOG_FILE" ]; then
    echo "Error: Log file '$LOG_FILE' not found"
    exit 1
fi

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

echo -e "${BOLD}=== Session Recovery Success Rate Analysis ===${NC}"
echo ""
echo "Log file: $LOG_FILE"
echo ""

# Count events
DETECTED=$(grep -c "event=stale_session_detected" "$LOG_FILE" || echo "0")
SUCCESS=$(grep -c "event=rehydrate_success" "$LOG_FILE" || echo "0")
FAILURE=$(grep -c "event=rehydrate_failure" "$LOG_FILE" || echo "0")

echo -e "${BOLD}Event Counts:${NC}"
echo "  Stale sessions detected: $DETECTED"
echo "  Successful recoveries:   $SUCCESS"
echo "  Failed recoveries:       $FAILURE"
echo ""

# Calculate success rate
TOTAL=$((SUCCESS + FAILURE))

if [ "$TOTAL" -eq 0 ]; then
    echo -e "${YELLOW}No recovery attempts found in log file.${NC}"
    echo ""
    echo "This could mean:"
    echo "  1. No stale sessions occurred during this period"
    echo "  2. Feature flag is not enabled (check ENABLE_SESSION_RECOVERY)"
    echo "  3. Log file doesn't contain recovery events"
    exit 0
fi

SUCCESS_RATE=$(echo "scale=2; $SUCCESS * 100 / $TOTAL" | bc)

echo -e "${BOLD}Success Rate Calculation:${NC}"
echo "  Formula: successes / (successes + failures)"
echo "  Calculation: $SUCCESS / ($SUCCESS + $FAILURE) = $SUCCESS / $TOTAL"
echo ""
echo -e "${BOLD}Success Rate: ${SUCCESS_RATE}%${NC}"
echo ""

# Check against criteria
if (( $(echo "$SUCCESS_RATE >= 95" | bc -l) )); then
    echo -e "${GREEN}✓ SUCCESS RATE MEETS CRITERIA (≥95%)${NC}"
    MEETS_CRITERIA=true
else
    echo -e "${RED}✗ SUCCESS RATE BELOW CRITERIA (<95%)${NC}"
    MEETS_CRITERIA=false
fi
echo ""

# Calculate duration stats
if [ "$SUCCESS" -gt 0 ]; then
    echo -e "${BOLD}Performance Metrics:${NC}"

    # Extract durations
    DURATIONS=$(grep "event=rehydrate_success" "$LOG_FILE" | grep -oE "duration_ms=[0-9]+" | cut -d= -f2)

    # Calculate average
    AVG_DURATION=$(echo "$DURATIONS" | awk '{ sum += $1; n++ } END { if (n > 0) print sum / n; else print "0" }')

    # Calculate median
    MEDIAN_DURATION=$(echo "$DURATIONS" | sort -n | awk '{ a[i++]=$1; } END { if (i % 2 == 1) print a[int(i/2)]; else print (a[i/2-1] + a[i/2])/2; }')

    # Calculate min/max
    MIN_DURATION=$(echo "$DURATIONS" | sort -n | head -1)
    MAX_DURATION=$(echo "$DURATIONS" | sort -n | tail -1)

    echo "  Average duration: ${AVG_DURATION}ms"
    echo "  Median duration:  ${MEDIAN_DURATION}ms"
    echo "  Min duration:     ${MIN_DURATION}ms"
    echo "  Max duration:     ${MAX_DURATION}ms"
    echo ""

    # Check against 5-second target
    if (( $(echo "$AVG_DURATION < 5000" | bc -l) )); then
        echo -e "${GREEN}✓ Average duration meets criteria (<5000ms)${NC}"
    else
        echo -e "${RED}✗ Average duration exceeds target (≥5000ms)${NC}"
    fi
    echo ""
fi

# Token usage analysis
if [ "$SUCCESS" -gt 0 ]; then
    echo -e "${BOLD}Token Usage Analysis:${NC}"

    # Extract token counts
    TOKENS=$(grep "event=rehydrate_success" "$LOG_FILE" | grep -oE "estimated_tokens=[0-9]+" | cut -d= -f2)

    if [ -n "$TOKENS" ]; then
        AVG_TOKENS=$(echo "$TOKENS" | awk '{ sum += $1; n++ } END { if (n > 0) print sum / n; else print "0" }')
        MAX_TOKENS=$(echo "$TOKENS" | sort -n | tail -1)

        echo "  Average tokens: ${AVG_TOKENS}"
        echo "  Max tokens:     ${MAX_TOKENS}"
        echo ""

        if (( $(echo "$MAX_TOKENS > 90000" | bc -l) )); then
            echo -e "${YELLOW}⚠ Some recoveries used >90% of token budget${NC}"
        fi
    fi
fi

# Failure analysis
if [ "$FAILURE" -gt 0 ]; then
    echo -e "${BOLD}Failure Analysis:${NC}"
    echo ""

    # Extract and count unique error messages
    echo "Top error messages:"
    grep "event=rehydrate_failure" "$LOG_FILE" | \
        grep -oE 'error="[^"]+"' | \
        sort | uniq -c | sort -rn | head -5 | \
        awk '{$1=$1; print "  " $1 "x: " substr($0, index($0,$2))}'
    echo ""
fi

# Conversation context analysis
if [ "$DETECTED" -gt 0 ]; then
    echo -e "${BOLD}Context Type Distribution:${NC}"
    grep "event=stale_session_detected" "$LOG_FILE" | \
        grep -oE 'context_type=[^ ]+' | \
        cut -d= -f2 | sort | uniq -c | sort -rn | \
        awk '{print "  " $2 ": " $1 " occurrences"}'
    echo ""
fi

# Summary and recommendations
echo -e "${BOLD}Summary:${NC}"
echo ""

if [ "$MEETS_CRITERIA" = true ]; then
    echo -e "${GREEN}✓ Session recovery feature is performing well${NC}"
    echo ""
    echo "Recommendations:"
    echo "  1. Proceed with production rollout (remove feature flag)"
    echo "  2. Document success metrics in rollout plan"
    echo "  3. Continue monitoring in production"
else
    echo -e "${YELLOW}⚠ Session recovery needs improvement before full rollout${NC}"
    echo ""
    echo "Recommendations:"
    echo "  1. Investigate failure patterns (see Failure Analysis above)"
    echo "  2. Fix identified issues"
    echo "  3. Extend monitoring period after fixes"
    echo "  4. Re-run this analysis after collecting more data"
fi
echo ""

# Export data for further analysis
REPORT_FILE="${LOG_FILE}.recovery-report.txt"
{
    echo "Session Recovery Report - Generated $(date)"
    echo ""
    echo "Detected: $DETECTED"
    echo "Success: $SUCCESS"
    echo "Failure: $FAILURE"
    echo "Total: $TOTAL"
    echo "Success Rate: ${SUCCESS_RATE}%"
    if [ "$SUCCESS" -gt 0 ]; then
        echo "Avg Duration: ${AVG_DURATION}ms"
        echo "Median Duration: ${MEDIAN_DURATION}ms"
    fi
} > "$REPORT_FILE"

echo "Report saved to: $REPORT_FILE"
