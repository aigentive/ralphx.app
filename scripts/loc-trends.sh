#!/bin/bash
# RalphX LOC Trends - Day-over-day code changes
# Shows additions/deletions by language for recent days
# Compatible with bash 3.2+

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors
BOLD='\033[1m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
MAGENTA='\033[0;35m'
DIM='\033[2m'
NC='\033[0m'

# Default days to look back (use "all" for full history)
if [ "$1" = "all" ]; then
    # Calculate days since first commit
    cd "$PROJECT_ROOT"
    first_commit=$(git log --reverse --format="%H" 2>/dev/null | head -1)
    first_epoch=$(git show -s --format="%ct" "$first_commit" 2>/dev/null)
    now_epoch=$(date +%s)
    DAYS=$(( (now_epoch - first_epoch) / 86400 + 1 ))
    SHOW_ALL=true
else
    DAYS=${1:-2}
    SHOW_ALL=false
fi

# Temp file for aggregation
TMPFILE=$(mktemp)
trap "rm -f $TMPFILE" EXIT

print_header() {
    echo ""
    echo -e "${BOLD}╔════════════════════════════════════════════════════════════╗${NC}"
    printf "${BOLD}║         RalphX LOC Trends (Last %d Day%-2s)              ║${NC}\n" "$DAYS" "$([ $DAYS -eq 1 ] && echo '' || echo 's')"
    echo -e "${BOLD}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

# Get language from file extension
get_language() {
    local file="$1"
    local ext="${file##*.}"

    case "$ext" in
        ts|tsx) echo "TypeScript" ;;
        js|jsx) echo "JavaScript" ;;
        rs) echo "Rust" ;;
        css) echo "CSS" ;;
        html) echo "HTML" ;;
        sh) echo "Shell" ;;
        *) echo "" ;;
    esac
}

# Check if file should be excluded
should_exclude() {
    local file="$1"

    # Exclude by extension
    case "$file" in
        *.md|*.json|*.yaml|*.yml|*.toml|*.lock|*.svg|*.png|*.ico|*.db|*.plist)
            return 0 ;;
    esac

    # Exclude by path
    case "$file" in
        *node_modules/*|*target/*|*dist/*|*.cache/*|*gen/*|*components/ui/*|*icons/*)
            return 0 ;;
    esac

    return 1
}

# Format number with sign and color
format_change() {
    local num=$1
    if [ "$num" -gt 0 ] 2>/dev/null; then
        echo -e "${GREEN}+$num${NC}"
    elif [ "$num" -lt 0 ] 2>/dev/null; then
        echo -e "${RED}$num${NC}"
    else
        echo -e "${DIM}0${NC}"
    fi
}

# Color for language
lang_color() {
    case "$1" in
        Rust) echo "$MAGENTA" ;;
        TypeScript) echo "$CYAN" ;;
        JavaScript) echo "$YELLOW" ;;
        *) echo "$NC" ;;
    esac
}

# Process and display a single day
process_day() {
    local date_str="$1"
    local next_date="$2"

    cd "$PROJECT_ROOT"

    # Clear temp file
    > "$TMPFILE"

    # Get numstat for commits on this day and aggregate by language
    git log --since="$date_str 00:00:00" --until="$next_date 00:00:00" --numstat --format="" 2>/dev/null | \
    while IFS=$'\t' read -r added deleted file; do
        # Skip empty lines and binary files
        [ -z "$file" ] && continue
        [ "$added" = "-" ] && continue

        # Skip excluded files
        should_exclude "$file" && continue

        # Get language
        lang=$(get_language "$file")
        [ -z "$lang" ] && continue

        # Output for aggregation
        echo "$lang $added $deleted"
    done >> "$TMPFILE"

    # Count commits
    local commits=$(git log --since="$date_str 00:00:00" --until="$next_date 00:00:00" --oneline 2>/dev/null | wc -l | tr -d ' ')

    # Aggregate by language using awk
    local day_name=$(date -j -f "%Y-%m-%d" "$date_str" "+%a" 2>/dev/null || date -d "$date_str" "+%a" 2>/dev/null || echo "")

    # Calculate totals
    local total_added=$(awk '{sum+=$2} END {print sum+0}' "$TMPFILE")
    local total_deleted=$(awk '{sum+=$3} END {print sum+0}' "$TMPFILE")
    local net=$((total_added - total_deleted))

    # Day header
    echo ""
    printf "${BOLD}%-12s${NC} ${DIM}(%s)${NC}  " "$date_str" "$day_name"
    printf "${YELLOW}%d commits${NC}  " "$commits"
    printf "Net: "
    format_change $net
    echo "───────────────────────────────────────────────────────────────────────"

    # Language breakdown header
    printf "  ${DIM}%-15s %10s %10s %10s${NC}\n" "Language" "Added" "Deleted" "Net"

    # Aggregate and sort by net (descending)
    awk '
    {
        added[$1] += $2
        deleted[$1] += $3
    }
    END {
        for (lang in added) {
            net = added[lang] - deleted[lang]
            print net, lang, added[lang], deleted[lang]
        }
    }
    ' "$TMPFILE" | sort -rn | while read -r net lang added deleted; do
        local color=$(lang_color "$lang")
        printf "  ${color}%-15s${NC} " "$lang"
        printf "${GREEN}%+10d${NC} " "$added"
        printf "${RED}%10d${NC} " "-$deleted"
        format_change $net | xargs printf "%10s\n"
    done

    # Day totals
    echo "  ─────────────────────────────────────────────────────"
    printf "  ${BOLD}%-15s${NC} " "TOTAL"
    printf "${GREEN}%+10d${NC} " "$total_added"
    printf "${RED}%10d${NC} " "-$total_deleted"
    format_change $net | xargs printf "%10s\n"

    # Return totals for summary
    echo "$total_added $total_deleted $commits" >> "${TMPFILE}.totals"
}

# Main analysis
analyze_trends() {
    cd "$PROJECT_ROOT"

    echo -e "${CYAN}Analyzing git history...${NC}"

    # Clear totals file
    > "${TMPFILE}.totals"

    echo ""
    echo -e "${BOLD}Daily Breakdown:${NC}"
    echo "═══════════════════════════════════════════════════════════════════════"

    for ((i=DAYS-1; i>=0; i--)); do
        # macOS date syntax
        local date_str=$(date -v-${i}d "+%Y-%m-%d" 2>/dev/null)
        local next_date

        if [ $i -eq 0 ]; then
            next_date=$(date -v+1d "+%Y-%m-%d" 2>/dev/null)
        else
            next_date=$(date -v-$((i-1))d "+%Y-%m-%d" 2>/dev/null)
        fi

        # Fallback for Linux
        if [ -z "$date_str" ]; then
            date_str=$(date -d "$i days ago" "+%Y-%m-%d")
            if [ $i -eq 0 ]; then
                next_date=$(date -d "tomorrow" "+%Y-%m-%d")
            else
                next_date=$(date -d "$((i-1)) days ago" "+%Y-%m-%d")
            fi
        fi

        process_day "$date_str" "$next_date"
    done

    echo ""
    echo "═══════════════════════════════════════════════════════════════════════"
}

# Summary stats
print_summary() {
    if [ ! -f "${TMPFILE}.totals" ]; then
        return
    fi

    local total_added=$(awk '{sum+=$1} END {print sum+0}' "${TMPFILE}.totals")
    local total_deleted=$(awk '{sum+=$2} END {print sum+0}' "${TMPFILE}.totals")
    local total_commits=$(awk '{sum+=$3} END {print sum+0}' "${TMPFILE}.totals")
    local net=$((total_added - total_deleted))
    local avg_per_day=$((net / DAYS))
    local commits_per_day=$(echo "scale=1; $total_commits / $DAYS" | bc)

    echo ""
    echo -e "${BOLD}Period Summary (Last $DAYS Days):${NC}"
    echo "───────────────────────────────────────────────────────────────────────"
    printf "  Commits:        ${GREEN}%d${NC} (%s/day)\n" "$total_commits" "$commits_per_day"
    printf "  Lines added:    ${GREEN}+%d${NC}\n" "$total_added"
    printf "  Lines deleted:  ${RED}-%d${NC}\n" "$total_deleted"
    printf "  Net change:     %s\n" "$(format_change $net)"
    printf "  Avg LOC/day:    %s\n" "$(format_change $avg_per_day)"
    echo ""

    rm -f "${TMPFILE}.totals"
}

# Velocity indicator
print_velocity() {
    cd "$PROJECT_ROOT"

    # Today and yesterday
    local today=$(date "+%Y-%m-%d")
    local yesterday=$(date -v-1d "+%Y-%m-%d" 2>/dev/null || date -d "yesterday" "+%Y-%m-%d")
    local tomorrow=$(date -v+1d "+%Y-%m-%d" 2>/dev/null || date -d "tomorrow" "+%Y-%m-%d")

    local today_commits=$(git log --since="$today 00:00:00" --until="$tomorrow 00:00:00" --oneline 2>/dev/null | wc -l | tr -d ' ')
    local yesterday_commits=$(git log --since="$yesterday 00:00:00" --until="$today 00:00:00" --oneline 2>/dev/null | wc -l | tr -d ' ')

    echo -e "${BOLD}Velocity:${NC}"
    echo "───────────────────────────────────────────────────────────────────────"
    printf "  Today so far:   ${GREEN}%d${NC} commits\n" "$today_commits"
    printf "  Yesterday:      ${CYAN}%d${NC} commits\n" "$yesterday_commits"

    if [ "$yesterday_commits" -gt 0 ]; then
        local hour=$(date "+%H" | sed 's/^0//')
        [ -z "$hour" ] && hour=0
        if [ $((hour + 1)) -gt 0 ]; then
            local projected=$((today_commits * 24 / (hour + 1)))
            printf "  Projected today: ~%d commits\n" "$projected"
        fi
    fi
    echo ""
}

# Usage
usage() {
    echo "Usage: $0 [days|all]"
    echo ""
    echo "  days    Number of days to analyze (default: 2)"
    echo "  all     Full git history"
    echo ""
    echo "Examples:"
    echo "  $0        # Last 2 days"
    echo "  $0 7      # Last week"
    echo "  $0 all    # Full history"
}

# Main
main() {
    if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
        usage
        exit 0
    fi

    print_header
    analyze_trends
    print_summary
    print_velocity
}

main "$@"
