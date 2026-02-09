#!/usr/bin/env bash
# Requires bash 4+ for associative arrays in fallback mode
# RalphX Lines of Code Counter
# Counts source code LOC for frontend (src/) and backend (src-tauri/src/)
# Excludes: docs, package managers, caches, builds, generated files

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
source "$SCRIPT_DIR/milestones.sh"

# Colors
BOLD='\033[1m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Source directories to count
SOURCE_DIRS=(
    "src"                    # Frontend React/TypeScript
    "src-tauri/src"          # Backend Rust
    "src-tauri/tests"        # Backend tests
    "ralphx-plugin"          # Claude plugin
    "tests"                  # Frontend/E2E tests
)

# Exclusion patterns
EXCLUDES=(
    "node_modules"
    "target"
    "dist"
    ".git"
    ".cache"
    "*.lock"
    "*.md"
    "*.json"
    "*.yaml"
    "*.yml"
    "*.toml"
    "*.svg"
    "*.png"
    "*.ico"
    "*.icns"
    "*.db"
    "gen"                    # Tauri generated
    "components/ui"          # shadcn/ui (vendor)
)

print_header() {
    echo ""
    echo -e "${BOLD}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BOLD}║              RalphX Source Code Statistics                 ║${NC}"
    echo -e "${BOLD}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

# Check if cloc is available, offer to install if not
check_cloc() {
    if command -v cloc &> /dev/null; then
        return 0
    fi

    echo -e "${YELLOW}cloc not found. It provides the most accurate counts.${NC}"
    echo ""

    if command -v brew &> /dev/null; then
        echo -e "Install with: ${CYAN}brew install cloc${NC}"
    elif command -v apt-get &> /dev/null; then
        echo -e "Install with: ${CYAN}sudo apt-get install cloc${NC}"
    else
        echo -e "Install from: ${CYAN}https://github.com/AlDanial/cloc${NC}"
    fi

    echo ""
    echo -e "${YELLOW}Falling back to shell-based counter (less accurate)...${NC}"
    echo ""
    return 1
}

# Build find exclusion arguments
build_find_excludes() {
    local excludes=""
    for pattern in "${EXCLUDES[@]}"; do
        excludes="$excludes -name '$pattern' -prune -o"
    done
    echo "$excludes"
}

# Shell-based LOC counter (fallback)
count_loc_shell() {
    echo -e "${CYAN}Counting lines of code...${NC}"
    echo ""

    declare -A lang_loc
    declare -A lang_files
    local total_loc=0
    local total_files=0

    # Define extensions and their languages
    declare -A ext_lang
    ext_lang["ts"]="TypeScript"
    ext_lang["tsx"]="TypeScript/React"
    ext_lang["js"]="JavaScript"
    ext_lang["jsx"]="JavaScript/React"
    ext_lang["rs"]="Rust"
    ext_lang["css"]="CSS"
    ext_lang["html"]="HTML"
    ext_lang["sh"]="Shell"

    cd "$PROJECT_ROOT"

    for dir in "${SOURCE_DIRS[@]}"; do
        if [[ ! -d "$dir" ]]; then
            continue
        fi

        # Find all source files, excluding patterns
        while IFS= read -r -d '' file; do
            # Skip excluded patterns
            skip=false
            for pattern in "${EXCLUDES[@]}"; do
                if [[ "$file" == *"$pattern"* ]]; then
                    skip=true
                    break
                fi
            done
            [[ "$skip" == true ]] && continue

            # Get extension
            ext="${file##*.}"
            lang="${ext_lang[$ext]:-}"
            [[ -z "$lang" ]] && continue

            # Count non-empty, non-comment lines (simplified)
            loc=$(grep -v '^\s*$' "$file" 2>/dev/null | grep -v '^\s*//' | grep -v '^\s*\*' | grep -v '^\s*#' | wc -l | tr -d ' ')

            lang_loc["$lang"]=$((${lang_loc["$lang"]:-0} + loc))
            lang_files["$lang"]=$((${lang_files["$lang"]:-0} + 1))
            total_loc=$((total_loc + loc))
            total_files=$((total_files + 1))

        done < <(find "$dir" -type f \( -name "*.ts" -o -name "*.tsx" -o -name "*.js" -o -name "*.jsx" -o -name "*.rs" -o -name "*.css" -o -name "*.html" -o -name "*.sh" \) -print0 2>/dev/null)
    done

    # Print results
    printf "${BOLD}%-20s %10s %10s %10s${NC}\n" "Language" "Files" "LOC" "Percent"
    echo "─────────────────────────────────────────────────────"

    # Sort by LOC descending
    for lang in $(for k in "${!lang_loc[@]}"; do echo "$k ${lang_loc[$k]}"; done | sort -t' ' -k2 -rn | cut -d' ' -f1); do
        loc=${lang_loc[$lang]}
        files=${lang_files[$lang]}
        if [[ $total_loc -gt 0 ]]; then
            pct=$(echo "scale=1; $loc * 100 / $total_loc" | bc)
        else
            pct="0.0"
        fi

        # Color by language
        case "$lang" in
            *Rust*) color="$MAGENTA" ;;
            *TypeScript*) color="$CYAN" ;;
            *JavaScript*) color="$YELLOW" ;;
            *) color="$NC" ;;
        esac

        printf "${color}%-20s %10d %10d %9s%%${NC}\n" "$lang" "$files" "$loc" "$pct"
    done

    echo "─────────────────────────────────────────────────────"
    printf "${BOLD}%-20s %10d %10d %9s%%${NC}\n" "TOTAL" "$total_files" "$total_loc" "100.0"
}

# cloc-based counter (preferred)
CLOC_TOTAL=0
count_loc_cloc() {
    echo -e "${CYAN}Counting lines of code with cloc...${NC}"
    echo ""

    cd "$PROJECT_ROOT"

    # Build directory list
    local dirs=""
    for dir in "${SOURCE_DIRS[@]}"; do
        if [[ -d "$dir" ]]; then
            dirs="$dirs $dir"
        fi
    done

    # Run cloc with exclusions and capture output
    local cloc_output
    cloc_output=$(cloc $dirs \
        --exclude-dir=node_modules,target,dist,.git,.cache,gen,ui,icons \
        --exclude-ext=md,json,yaml,yml,toml,lock,svg,png,ico,icns,db,plist \
        --exclude-list-file=<(echo -e "src/components/ui\n") \
        --quiet \
        --hide-rate \
        2>/dev/null) || {
            cloc_output=$(cloc $dirs \
                --exclude-dir=node_modules,target,dist,.git,.cache,gen,ui,icons \
                --exclude-ext=md,json,yaml,yml,toml,lock \
                --quiet)
        }

    echo "$cloc_output"

    # Extract total LOC from SUM line
    CLOC_TOTAL=$(echo "$cloc_output" | grep "^SUM:" | awk '{print $NF}')
}

# Summary by area (also calculates total LOC for LOC/day)
TOTAL_SOURCE_LOC=0
print_area_summary() {
    echo ""
    echo -e "${BOLD}Summary by Area:${NC}"
    echo "─────────────────────────────────────────────────────"

    cd "$PROJECT_ROOT"

    local frontend_loc=0 backend_loc=0 plugin_loc=0 tests_loc=0

    # Frontend
    if [[ -d "src" ]]; then
        frontend_loc=$(find src -type f \( -name "*.ts" -o -name "*.tsx" \) \
            ! -path "*/node_modules/*" ! -path "*/components/ui/*" \
            -exec cat {} \; 2>/dev/null | grep -v '^\s*$' | wc -l | tr -d ' ')
        echo -e "${CYAN}Frontend (src/)${NC}: ~$frontend_loc lines"
    fi

    # Backend (excludes: target/, .cache/, gen/, icons/)
    if [[ -d "src-tauri/src" ]]; then
        backend_loc=$(find src-tauri/src src-tauri/tests -type f -name "*.rs" \
            ! -path "*/target/*" ! -path "*/.cache/*" ! -path "*/gen/*" \
            -exec cat {} \; 2>/dev/null | grep -v '^\s*$' | wc -l | tr -d ' ')
        echo -e "${MAGENTA}Backend (src-tauri/)${NC}: ~$backend_loc lines"
    fi

    # Plugin
    if [[ -d "ralphx-plugin" ]]; then
        plugin_loc=$(find ralphx-plugin -type f \( -name "*.ts" -o -name "*.sh" \) \
            ! -path "*/node_modules/*" ! -path "*/.cache/*" \
            -exec cat {} \; 2>/dev/null | grep -v '^\s*$' | wc -l | tr -d ' ')
        echo -e "${YELLOW}Plugin (ralphx-plugin/)${NC}: ~$plugin_loc lines"
    fi

    # Tests
    if [[ -d "tests" ]]; then
        tests_loc=$(find tests -type f -name "*.ts" \
            -exec cat {} \; 2>/dev/null | grep -v '^\s*$' | wc -l | tr -d ' ')
        echo -e "${GREEN}E2E Tests (tests/)${NC}: ~$tests_loc lines"
    fi

    # Calculate total
    TOTAL_SOURCE_LOC=$((frontend_loc + backend_loc + plugin_loc + tests_loc))
    echo "─────────────────────────────────────────────────────"
    echo -e "${BOLD}Total Source LOC:${NC} ~$TOTAL_SOURCE_LOC lines"
}

# What's excluded info
print_exclusions() {
    echo ""
    echo -e "${BOLD}Excluded from counts:${NC}"
    echo "  • Documentation (*.md)"
    echo "  • Package managers (node_modules/, Cargo.lock, package-lock.json)"
    echo "  • Build artifacts (dist/, src-tauri/target/)"
    echo "  • Caches (src-tauri/.cache/, .cache/)"
    echo "  • Generated code (src-tauri/gen/, src/components/ui/)"
    echo "  • Config files (*.json, *.yaml, *.toml)"
    echo "  • Assets (*.svg, *.png, *.ico, icons/)"
    echo "  • Database files (*.db)"
    echo "  • Logs, specs, screenshots directories"
}

# Git statistics (stores days_active for LOC/day calc)
DAYS_ACTIVE=0
print_git_stats() {
    echo ""
    echo -e "${BOLD}Git Statistics:${NC}"
    echo "─────────────────────────────────────────────────────"

    cd "$PROJECT_ROOT"

    # Total commits
    commit_count=$(git rev-list --count HEAD 2>/dev/null || echo "N/A")
    echo -e "  Total commits:     ${GREEN}$commit_count${NC}"

    # First commit
    first_commit=$(git log --reverse --format="%H" 2>/dev/null | head -1)
    if [[ -n "$first_commit" ]]; then
        first_date=$(git show -s --format="%ci" "$first_commit" 2>/dev/null | cut -d' ' -f1)
        first_msg=$(git show -s --format="%s" "$first_commit" 2>/dev/null | head -c 50)
        echo -e "  First commit:      ${CYAN}$first_date${NC}"
        echo -e "  First message:     \"$first_msg\""
    fi

    # Latest commit
    latest_date=$(git log -1 --format="%ci" 2>/dev/null | cut -d' ' -f1)
    latest_msg=$(git log -1 --format="%s" 2>/dev/null | head -c 50)
    echo -e "  Latest commit:     ${CYAN}$latest_date${NC}"
    echo -e "  Latest message:    \"$latest_msg\""

    # Contributors
    contributor_count=$(git log --format='%aN' 2>/dev/null | sort -u | wc -l | tr -d ' ')
    echo -e "  Contributors:      ${GREEN}$contributor_count${NC}"

    # Days active
    if [[ -n "$first_commit" ]]; then
        first_epoch=$(git show -s --format="%ct" "$first_commit" 2>/dev/null)
        now_epoch=$(date +%s)
        DAYS_ACTIVE=$(( (now_epoch - first_epoch) / 86400 ))
        [[ $DAYS_ACTIVE -eq 0 ]] && DAYS_ACTIVE=1  # At least 1 day
        echo -e "  Days active:       ${GREEN}$DAYS_ACTIVE${NC}"

        # Commits per day average
        if [[ "$commit_count" != "N/A" ]]; then
            commits_per_day=$(echo "scale=1; $commit_count / $DAYS_ACTIVE" | bc)
            echo -e "  Commits/day avg:   ${GREEN}$commits_per_day${NC}"
        fi
    fi
}

# Productivity metrics (LOC per day)
print_productivity() {
    local loc_to_use=${CLOC_TOTAL:-$TOTAL_SOURCE_LOC}
    if [[ $DAYS_ACTIVE -gt 0 && $loc_to_use -gt 0 ]]; then
        echo ""
        echo -e "${BOLD}Productivity:${NC}"
        echo "─────────────────────────────────────────────────────"
        loc_per_day=$(echo "scale=0; $loc_to_use / $DAYS_ACTIVE" | bc)
        echo -e "  LOC/day avg:       ${GREEN}$loc_per_day${NC} lines/day (code only)"
    fi
}

# Development era breakdown using milestones
print_era_stats() {
    [[ ${#MILESTONES[@]} -eq 0 ]] && return

    cd "$PROJECT_ROOT"

    echo ""
    echo -e "${BOLD}Development Eras:${NC}"
    echo "─────────────────────────────────────────────────────"

    # Get project start epoch (first commit)
    local first_commit
    first_commit=$(git log --reverse --format="%H" 2>/dev/null | head -1)
    [[ -z "$first_commit" ]] && return

    local project_start_epoch
    project_start_epoch=$(git show -s --format="%ct" "$first_commit" 2>/dev/null)
    local now_epoch
    now_epoch=$(date +%s)

    # Build sorted list of boundary epochs: project_start, milestone1, ..., now
    local boundaries=("$project_start_epoch")
    local labels=()

    for entry in "${MILESTONES[@]}"; do
        parse_milestone "$entry"
        boundaries+=("$MS_EPOCH")
        labels+=("$MS_LABEL")
    done
    boundaries+=("$now_epoch")

    # Print each era
    local i=0
    local total_eras=$(( ${#boundaries[@]} - 1 ))

    while [[ $i -lt $total_eras ]]; do
        local era_start=${boundaries[$i]}
        local era_end=${boundaries[$((i + 1))]}

        # Era label
        local era_label
        if [[ $i -eq 0 && ${#labels[@]} -gt 0 ]]; then
            # Before first milestone — use the label's "from" side
            local first_label="${labels[0]}"
            era_label="${first_label%%->*}"
            era_label=$(echo "$era_label" | sed 's/^ *//;s/ *$//')
        elif [[ $i -gt 0 && $i -le ${#labels[@]} ]]; then
            local label="${labels[$((i - 1))]}"
            # After milestone — use the label's "to" side if it has ->
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

        # Commits in era (using --after/--before with epoch)
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
}

# Main
main() {
    print_header

    if check_cloc; then
        count_loc_cloc
    else
        count_loc_shell
    fi

    print_area_summary
    print_git_stats
    print_productivity
    print_era_stats
    print_exclusions
    echo ""
}

main "$@"
