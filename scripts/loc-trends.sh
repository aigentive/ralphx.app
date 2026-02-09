#!/bin/bash
# RalphX LOC Trends - Day-over-day code changes
# Shows additions/deletions by language for recent days
# Compatible with bash 3.2+

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Source shared milestones config
source "$SCRIPT_DIR/milestones.sh"

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

# Draw a bar using Unicode blocks
draw_bar() {
    local value=$1
    local max=$2
    local width=${3:-40}
    local color=${4:-$GREEN}

    if [ "$max" -eq 0 ]; then
        echo ""
        return
    fi

    # Calculate bar length
    local bar_len=$((value * width / max))
    [ $bar_len -gt $width ] && bar_len=$width

    # Draw bar
    local bar=""
    for ((j=0; j<bar_len; j++)); do
        bar="${bar}█"
    done

    echo -e "${color}${bar}${NC}"
}

# Print a milestone marker line for charts
print_milestone_marker() {
    local label="$1"
    local width=${2:-60}
    local dashes=""
    for ((d=0; d<width; d++)); do
        dashes="${dashes}-"
    done
    echo -e "       ${YELLOW}★${NC} ${DIM}${dashes}${NC} ${YELLOW}${label}${NC}"
}

# Chart view - visual bar chart
print_chart() {
    cd "$PROJECT_ROOT"

    echo ""
    echo -e "${BOLD}LOC Trend Chart (Net lines per day):${NC}"
    echo ""

    # Collect data first
    local dates=()
    local nets=()
    local commits=()
    local max_net=0

    for ((i=DAYS-1; i>=0; i--)); do
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

        # Get stats for this day
        > "$TMPFILE"
        git log --since="$date_str 00:00:00" --until="$next_date 00:00:00" --numstat --format="" 2>/dev/null | \
        while IFS=$'\t' read -r added deleted file; do
            [ -z "$file" ] && continue
            [ "$added" = "-" ] && continue
            should_exclude "$file" && continue
            lang=$(get_language "$file")
            [ -z "$lang" ] && continue
            echo "$lang $added $deleted"
        done >> "$TMPFILE"

        local day_added=$(awk '{sum+=$2} END {print sum+0}' "$TMPFILE")
        local day_deleted=$(awk '{sum+=$3} END {print sum+0}' "$TMPFILE")
        local day_net=$((day_added - day_deleted))
        local day_commits=$(git log --since="$date_str 00:00:00" --until="$next_date 00:00:00" --oneline 2>/dev/null | wc -l | tr -d ' ')

        dates+=("$date_str")
        nets+=("$day_net")
        commits+=("$day_commits")

        [ $day_net -gt $max_net ] && max_net=$day_net
    done

    # Draw chart
    local bar_width=45

    for ((i=0; i<${#dates[@]}; i++)); do
        local date="${dates[$i]}"
        local net="${nets[$i]}"
        local commit="${commits[$i]}"
        local short_date=$(echo "$date" | cut -c6-)  # MM-DD

        # Check for milestone on current day
        local ms_label
        ms_label=$(milestone_label_for_date "$date" 2>/dev/null) && \
            print_milestone_marker "$ms_label"

        # Color based on magnitude
        local color=$GREEN
        [ $net -lt 5000 ] && color=$YELLOW
        [ $net -lt 1000 ] && color=$DIM

        printf "${BOLD}%s${NC} %6d │ " "$short_date" "$net"
        draw_bar $net $max_net $bar_width "$color"
    done

    echo ""

    # Commits chart
    echo -e "${BOLD}Commits per day:${NC}"
    echo ""

    local max_commits=0
    for c in "${commits[@]}"; do
        [ $c -gt $max_commits ] && max_commits=$c
    done

    for ((i=0; i<${#dates[@]}; i++)); do
        local date="${dates[$i]}"
        local commit="${commits[$i]}"
        local short_date=$(echo "$date" | cut -c6-)

        # Check for milestone on current day
        local ms_label
        ms_label=$(milestone_label_for_date "$date" 2>/dev/null) && \
            print_milestone_marker "$ms_label"

        printf "${BOLD}%s${NC} %6d │ " "$short_date" "$commit"
        draw_bar $commit $max_commits $bar_width "$CYAN"
    done

    echo ""
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

        # Check for milestone on this date — print highlighted banner
        if date_has_milestone "$date_str"; then
            # Find the matching entry for full details
            for _ms_entry in "${MILESTONES[@]}"; do
                parse_milestone "$_ms_entry"
                if [ "$MS_DATE" = "$date_str" ]; then
                    break
                fi
            done
            echo ""
            echo -e "${YELLOW}╔══════════════════════════════════════════════════════════════════════════╗${NC}"
            printf "${YELLOW}║  ★  %-67s ║${NC}\n" "$MS_LABEL"
            printf "${YELLOW}║     %-67s ║${NC}\n" "$MS_DESC"
            printf "${YELLOW}║     %-67s ║${NC}\n" "$MS_DATE $MS_TIME"
            echo -e "${YELLOW}╚══════════════════════════════════════════════════════════════════════════╝${NC}"
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

# Era comparison — before/after stats for each milestone in the analyzed range
print_era_comparison() {
    [[ ${#MILESTONES[@]} -eq 0 ]] && return

    cd "$PROJECT_ROOT"

    # Get project start epoch (first commit)
    local first_commit
    first_commit=$(git log --reverse --format="%H" 2>/dev/null | head -1)
    [[ -z "$first_commit" ]] && return

    local project_start_epoch
    project_start_epoch=$(git show -s --format="%ct" "$first_commit" 2>/dev/null)
    local now_epoch
    now_epoch=$(date +%s)

    # Compute the epoch for the start of the analyzed range (DAYS ago at 00:00)
    local range_start_epoch
    if [[ "$(uname)" == "Darwin" ]]; then
        range_start_epoch=$(date -v-${DAYS}d -j -f "%Y-%m-%d %H:%M:%S" "$(date "+%Y-%m-%d") 00:00:00" "+%s" 2>/dev/null)
    else
        range_start_epoch=$(date -d "$DAYS days ago 00:00:00" "+%s" 2>/dev/null)
    fi
    [[ -z "$range_start_epoch" ]] && range_start_epoch=$((now_epoch - DAYS * 86400))

    # Clamp project start to range start
    local effective_start=$range_start_epoch
    [[ $project_start_epoch -gt $effective_start ]] && effective_start=$project_start_epoch

    # Build sorted boundary list: effective_start, milestones (within range), now
    local boundaries=("$effective_start")
    local labels=()

    for entry in "${MILESTONES[@]}"; do
        parse_milestone "$entry"
        # Only include milestones within the analyzed range
        if [[ $MS_EPOCH -gt $effective_start && $MS_EPOCH -lt $now_epoch ]]; then
            boundaries+=("$MS_EPOCH")
            labels+=("$MS_LABEL")
        fi
    done
    boundaries+=("$now_epoch")

    # Need at least one milestone in range to show comparison
    [[ ${#labels[@]} -eq 0 ]] && return

    echo ""
    echo -e "${BOLD}Era Comparison:${NC}"
    echo "───────────────────────────────────────────────────────────────────────"

    local i=0
    local total_eras=$(( ${#boundaries[@]} - 1 ))

    while [[ $i -lt $total_eras ]]; do
        local era_start=${boundaries[$i]}
        local era_end=${boundaries[$((i + 1))]}

        # Era label
        local era_label
        if [[ $i -eq 0 && ${#labels[@]} -gt 0 ]]; then
            local first_label="${labels[0]}"
            era_label="${first_label%%->*}"
            era_label=$(echo "$era_label" | sed 's/^ *//;s/ *$//')
        elif [[ $i -gt 0 && $i -le ${#labels[@]} ]]; then
            local label="${labels[$((i - 1))]}"
            if [[ "$label" == *"->"* ]]; then
                era_label="${label##*->}"
                era_label=$(echo "$era_label" | sed 's/^ *//;s/ *$//')
            else
                era_label="$label"
            fi
        else
            era_label="Era $((i + 1))"
        fi

        # Days in era
        local era_days=$(( (era_end - era_start) / 86400 ))
        [[ $era_days -eq 0 ]] && era_days=1

        # Commits in era
        local era_commits
        era_commits=$(git rev-list --count --after="$era_start" --before="$era_end" HEAD 2>/dev/null || echo "0")

        # Commits per day
        local era_cpd
        era_cpd=$(echo "scale=1; $era_commits / $era_days" | bc)

        # Date range for display
        local start_date end_date
        if [[ "$(uname)" == "Darwin" ]]; then
            start_date=$(date -r "$era_start" "+%Y-%m-%d" 2>/dev/null)
            end_date=$(date -r "$era_end" "+%Y-%m-%d" 2>/dev/null)
        else
            start_date=$(date -d "@$era_start" "+%Y-%m-%d" 2>/dev/null)
            end_date=$(date -d "@$era_end" "+%Y-%m-%d" 2>/dev/null)
        fi

        printf "  ${CYAN}%-25s${NC} %3d days  %5s commits  %5s/day  (%s → %s)\n" \
            "$era_label" "$era_days" "$era_commits" "$era_cpd" "$start_date" "$end_date"

        i=$((i + 1))
    done

    echo ""
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
    echo "Usage: $0 [-c|--chart] [days|all]"
    echo ""
    echo "Options:"
    echo "  -c, --chart   Show only chart (no detailed breakdown)"
    echo ""
    echo "  days          Number of days to analyze (default: 2)"
    echo "  all           Full git history"
    echo ""
    echo "Examples:"
    echo "  $0            # Last 2 days with chart + details"
    echo "  $0 -c all     # Full history, chart only"
    echo "  $0 7          # Last week"
}

# Main
main() {
    local chart_only=false
    local arg="$1"

    if [ "$arg" = "-h" ] || [ "$arg" = "--help" ]; then
        usage
        exit 0
    fi

    if [ "$arg" = "-c" ] || [ "$arg" = "--chart" ]; then
        chart_only=true
        arg="$2"
    fi

    # Re-parse DAYS if needed
    if [ -n "$arg" ]; then
        if [ "$arg" = "all" ]; then
            cd "$PROJECT_ROOT"
            first_commit=$(git log --reverse --format="%H" 2>/dev/null | head -1)
            first_epoch=$(git show -s --format="%ct" "$first_commit" 2>/dev/null)
            now_epoch=$(date +%s)
            DAYS=$(( (now_epoch - first_epoch) / 86400 + 1 ))
        else
            DAYS="$arg"
        fi
    fi

    print_header

    if $chart_only; then
        print_chart
        print_summary
        print_era_comparison
    else
        print_chart
        analyze_trends
        print_summary
        print_era_comparison
        print_velocity
    fi
}

main "$@"
