#!/bin/bash

# find-debug-logs.sh — Search Claude Code debug logs by date, agent name, or ideation session
# Usage: find-debug-logs.sh [options]
#
# Modes:
#   Content search (default): greps debug log file contents
#   DB cross-reference (-s):  queries RalphX DB to map agent names -> debug file UUIDs
#
# Options:
#   -d, --date DATE          Search for logs from specific date (YYYY-MM-DD)
#   -a, --agent NAME         Search for agent name (partial match)
#   -k, --keywords WORDS     Search for keywords (comma-separated, any match)
#   -t, --time HH:MM         Filter by file birth time (local, prefix match)
#   -s, --session TITLE      DB mode: find debug logs for agents in an ideation session (title search)
#   -v, --verbose            Show context + sample matches
#   --db PATH                Path to RalphX DB (default: src-tauri/ralphx.db)
#   -h, --help               Show this help message
#
# Examples:
#   find-debug-logs.sh -d 2026-02-24 -t 12:13        # Files born on date near time
#   find-debug-logs.sh -s "Block plan acceptance"     # DB lookup by ideation title
#   find-debug-logs.sh -a "frontend-researcher" -v    # Content grep for agent name

DEBUG_DIR="$HOME/.claude/debug"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DB_PATH="${SCRIPT_DIR}/../src-tauri/ralphx.db"
DATE=""
AGENT_NAME=""
KEYWORDS=""
TIME_FILTER=""
SESSION_TITLE=""
VERBOSE=0

show_help() {
  sed -n '3,25p' "$0" | sed 's/^# //'
}

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    -d|--date) DATE="$2"; shift 2 ;;
    -a|--agent) AGENT_NAME="$2"; shift 2 ;;
    -k|--keywords) KEYWORDS="$2"; shift 2 ;;
    -t|--time) TIME_FILTER="$2"; shift 2 ;;
    -s|--session) SESSION_TITLE="$2"; shift 2 ;;
    -v|--verbose) VERBOSE=1; shift ;;
    --db) DB_PATH="$2"; shift 2 ;;
    -h|--help) show_help; exit 0 ;;
    *) echo "Error: Unknown option $1"; show_help; exit 1 ;;
  esac
done

# ── DB cross-reference mode ──────────────────────────────────────────────────
if [ -n "$SESSION_TITLE" ]; then
  if [ ! -f "$DB_PATH" ]; then
    echo "Error: RalphX DB not found at $DB_PATH (use --db to override)"
    exit 1
  fi

  echo "Searching ideation sessions for: $SESSION_TITLE"
  echo ""

  # Find matching ideation sessions
  SESSIONS=$(sqlite3 "$DB_PATH" "SELECT id, title, created_at FROM ideation_sessions WHERE title LIKE '%${SESSION_TITLE}%' ORDER BY created_at DESC LIMIT 10;")
  if [ -z "$SESSIONS" ]; then
    echo "No ideation sessions matching '$SESSION_TITLE'"
    exit 1
  fi

  echo "Matching ideation sessions:"
  echo "$SESSIONS" | while IFS='|' read -r sid stitle screated; do
    echo "  $sid  $screated  $stitle"
  done
  echo ""

  # For each session, find team_sessions with teammate spawn data
  echo "$SESSIONS" | while IFS='|' read -r sid stitle screated; do
    TEAM_DATA=$(sqlite3 "$DB_PATH" "SELECT id, team_name, teammate_json, created_at, disbanded_at FROM team_sessions WHERE context_id = '$sid' ORDER BY created_at DESC LIMIT 5;")
    if [ -z "$TEAM_DATA" ]; then
      echo "  No team sessions found for ideation $sid"
      continue
    fi

    echo "Team sessions for: $stitle"
    echo "$TEAM_DATA" | while IFS='|' read -r tid tname tjson tcreated tdisbanded; do
      echo "  Team: $tname (created: $tcreated, disbanded: $tdisbanded)"

      # Parse teammate_json to extract names and spawn times
      # Uses python3 for reliable JSON parsing
      echo "$tjson" | python3 -c "
import json, sys, subprocess, os, re
from datetime import datetime, timezone, timedelta

data = json.load(sys.stdin)
debug_dir = os.path.expanduser('~/.claude/debug')

if not data:
    print('    No teammates in this team session')
    sys.exit(0)

# Get all debug file birth times
files = {}
try:
    result = subprocess.run(
        ['stat', '-f', '%SB %N'] + [
            os.path.join(debug_dir, f)
            for f in os.listdir(debug_dir) if f.endswith('.txt')
        ],
        capture_output=True, text=True, timeout=10
    )
    for line in result.stdout.strip().split('\n'):
        if not line:
            continue
        # Parse: 'Mon DD HH:MM:SS YYYY /path/to/file.txt'
        parts = line.rsplit(' ', 1)
        if len(parts) == 2:
            timestr, filepath = parts
            uuid = os.path.basename(filepath).replace('.txt', '')
            files[uuid] = timestr
except Exception as e:
    print(f'    Warning: could not stat debug files: {e}')

# Build candidate list: (uuid, file_utc_datetime) for all debug files
candidates = []
for uuid in files:
    fpath = os.path.join(debug_dir, uuid + '.txt')
    try:
        with open(fpath, 'r') as f:
            first_line = f.readline()
        m = re.search(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+)Z', first_line)
        if m:
            file_dt = datetime.fromisoformat(m.group(1)).replace(tzinfo=timezone.utc)
            candidates.append((uuid, file_dt))
    except:
        pass

# Sort teammates by spawn time for deterministic matching
sorted_mates = sorted(data, key=lambda m: m.get('spawned_at', ''))
used_uuids = set()

for mate in sorted_mates:
    name = mate.get('name', '?')
    model = mate.get('model', '?')
    role = mate.get('role', '?')
    spawned = mate.get('spawned_at', '?')
    status = mate.get('status', '?')
    cost = mate.get('cost', {})
    tokens = cost.get('input_tokens', 0) + cost.get('output_tokens', 0)
    parent_session = mate.get('parent_session_id', '')

    # Find closest unused debug file by birth time
    best_match = None
    best_delta = None
    try:
        spawn_dt = datetime.fromisoformat(spawned.replace('+00:00', '+00:00'))
        for uuid, file_dt in candidates:
            if uuid in used_uuids:
                continue
            delta = abs((file_dt - spawn_dt).total_seconds())
            if delta < 5 and (best_delta is None or delta < best_delta):
                best_delta = delta
                best_match = uuid
    except:
        pass

    if best_match:
        used_uuids.add(best_match)

    match_str = best_match + '.txt' if best_match else '(no match)'
    delta_str = f' ({best_delta:.1f}s delta)' if best_delta is not None else ''
    size_str = ''
    if best_match:
        fpath = os.path.join(debug_dir, best_match + '.txt')
        try:
            sz = os.path.getsize(fpath)
            if sz > 1048576:
                size_str = f' [{sz/1048576:.1f}MB]'
            elif sz > 1024:
                size_str = f' [{sz/1024:.0f}KB]'
            else:
                size_str = f' [{sz}B]'
        except:
            pass

    # Find conversation JSONL in subagents directory
    conv_file = '(not found)'
    conv_size = ''
    if parent_session:
        projects_dir = os.path.expanduser('~/.claude/projects')
        if os.path.isdir(projects_dir):
            for proj in os.listdir(projects_dir):
                sess_dir = os.path.join(projects_dir, proj, parent_session, 'subagents')
                if os.path.isdir(sess_dir):
                    # Search for JSONL files containing this agent's name
                    for jf in sorted(os.listdir(sess_dir)):
                        if not jf.endswith('.jsonl'):
                            continue
                        jpath = os.path.join(sess_dir, jf)
                        try:
                            with open(jpath, 'r') as fj:
                                head = fj.read(4096)
                            if name in head or role in head:
                                sz = os.path.getsize(jpath)
                                if sz > 1048576:
                                    conv_size = f' [{sz/1048576:.1f}MB]'
                                elif sz > 1024:
                                    conv_size = f' [{sz/1024:.0f}KB]'
                                else:
                                    conv_size = f' [{sz}B]'
                                conv_file = jpath
                                break
                        except:
                            pass

    print(f'    {name} ({model}, {status}, {tokens} tokens)')
    print(f'      Spawned:  {spawned}')
    print(f'      Debug:    {match_str}{delta_str}{size_str}')
    print(f'      Convo:    {conv_file}{conv_size}')
" 2>&1
      echo ""
    done
  done

  # Also check for agent_runs linked to the ideation conversation
  if [ "$VERBOSE" -eq 1 ]; then
    echo "Agent runs for this ideation session:"
    sqlite3 "$DB_PATH" "
      SELECT ar.id, ar.status, ar.started_at, ar.completed_at
      FROM agent_runs ar
      JOIN chat_conversations cc ON ar.conversation_id = cc.id
      WHERE cc.context_id = (
        SELECT id FROM ideation_sessions WHERE title LIKE '%${SESSION_TITLE}%' ORDER BY created_at DESC LIMIT 1
      )
      ORDER BY ar.started_at DESC LIMIT 10;
    " | while IFS='|' read -r arid arstatus arstart arcomplete; do
      echo "  $arid  $arstatus  $arstart -> $arcomplete"
    done
    echo ""
  fi

  exit 0
fi

# ── Content search mode (original) ──────────────────────────────────────────

# Validate at least one criterion
if [ -z "$DATE" ] && [ -z "$AGENT_NAME" ] && [ -z "$KEYWORDS" ] && [ -z "$TIME_FILTER" ]; then
  echo "Error: Specify at least one search criterion (-d, -a, -k, -t, or -s)"
  show_help
  exit 1
fi

# If only date + time filter (no content search), use birth-time mode
if [ -n "$DATE" ] && [ -z "$AGENT_NAME" ] && [ -z "$KEYWORDS" ]; then
  # Convert YYYY-MM-DD to month abbreviation + day for stat output matching
  _MONTH_NUM=$(echo "$DATE" | cut -d'-' -f2)
  _DAY_NUM=$(echo "$DATE" | cut -d'-' -f3 | sed 's/^0/ /')
  case "$_MONTH_NUM" in
    01) _MONTH_ABR="Jan";; 02) _MONTH_ABR="Feb";; 03) _MONTH_ABR="Mar";;
    04) _MONTH_ABR="Apr";; 05) _MONTH_ABR="May";; 06) _MONTH_ABR="Jun";;
    07) _MONTH_ABR="Jul";; 08) _MONTH_ABR="Aug";; 09) _MONTH_ABR="Sep";;
    10) _MONTH_ABR="Oct";; 11) _MONTH_ABR="Nov";; 12) _MONTH_ABR="Dec";;
  esac
  _DATE_PATTERN="${_MONTH_ABR} ${_DAY_NUM}"

  echo "Searching debug logs by file birth time..."
  echo "   Date: $DATE ($_DATE_PATTERN)"
  [ -n "$TIME_FILTER" ] && echo "   Time: $TIME_FILTER"
  echo ""

  stat -f "%SB %N" "$DEBUG_DIR"/*.txt 2>/dev/null | grep "$_DATE_PATTERN" | while IFS= read -r line; do
    # Filter by time if specified
    if [ -n "$TIME_FILTER" ] && ! echo "$line" | grep -q "$TIME_FILTER" 2>/dev/null; then
      continue
    fi
    filepath=$(echo "$line" | awk '{print $NF}')
    filename=$(basename "$filepath")
    birthtime=$(echo "$line" | sed "s| $filepath||")
    size=$(ls -lh "$filepath" 2>/dev/null | awk '{print $5}')
    lines=$(wc -l < "$filepath" 2>/dev/null | xargs)
    printf "  %-40s  %7s  %5s lines  Born: %s\n" "$filename" "$size" "$lines" "$birthtime"
  done | sort

  exit 0
fi

# Build grep patterns
declare -a PATTERNS

[ -n "$AGENT_NAME" ] && PATTERNS+=("$AGENT_NAME")

if [ -n "$KEYWORDS" ]; then
  IFS=',' read -ra KEYWORD_ARRAY <<< "$KEYWORDS"
  for kw in "${KEYWORD_ARRAY[@]}"; do
    PATTERNS+=("$(echo "$kw" | xargs)")
  done
fi

# Fallback: if only date given with keywords/agent, add date to patterns
[ -n "$DATE" ] && [ ${#PATTERNS[@]} -eq 0 ] && PATTERNS+=("$DATE")

# Create regex pattern
if [ ${#PATTERNS[@]} -eq 1 ]; then
  PATTERN="${PATTERNS[0]}"
  GREP_OPTS="-i"
else
  # OR pattern for multiple criteria
  PATTERN=$(printf '%s\|' "${PATTERNS[@]}" | sed 's/\\|$//')
  GREP_OPTS="-iE"
fi

# Pre-filter files by birth date if -d provided
declare -a FILE_LIST
if [ -n "$DATE" ]; then
  # Map date to month abbreviation for stat output matching
  MONTH_NUM=$(echo "$DATE" | cut -d'-' -f2)
  DAY_NUM=$(echo "$DATE" | cut -d'-' -f3 | sed 's/^0//')
  case "$MONTH_NUM" in
    01) MONTH_ABR="Jan";; 02) MONTH_ABR="Feb";; 03) MONTH_ABR="Mar";;
    04) MONTH_ABR="Apr";; 05) MONTH_ABR="May";; 06) MONTH_ABR="Jun";;
    07) MONTH_ABR="Jul";; 08) MONTH_ABR="Aug";; 09) MONTH_ABR="Sep";;
    10) MONTH_ABR="Oct";; 11) MONTH_ABR="Nov";; 12) MONTH_ABR="Dec";;
  esac

  while IFS= read -r line; do
    filepath=$(echo "$line" | awk '{print $NF}')
    if [ -n "$TIME_FILTER" ] && ! echo "$line" | grep -q "$TIME_FILTER"; then
      continue
    fi
    FILE_LIST+=("$filepath")
  done < <(stat -f "%SB %N" "$DEBUG_DIR"/*.txt 2>/dev/null | grep "$MONTH_ABR" | grep " $DAY_NUM ")
else
  for f in "$DEBUG_DIR"/*.txt; do
    [ -f "$f" ] && FILE_LIST+=("$f")
  done
fi

# Search and display results
echo "Searching debug logs..."
[ -n "$DATE" ] && echo "   Date: $DATE"
[ -n "$TIME_FILTER" ] && echo "   Time: $TIME_FILTER"
[ -n "$AGENT_NAME" ] && echo "   Agent: $AGENT_NAME"
[ -n "$KEYWORDS" ] && echo "   Keywords: $KEYWORDS"
echo ""

MATCHES=()
for file in "${FILE_LIST[@]}"; do
  if [ -f "$file" ]; then
    count=$(grep $GREP_OPTS "$PATTERN" "$file" 2>/dev/null | wc -l | xargs)
    if [ "$count" -gt 0 ]; then
      filename=$(basename "$file")
      size=$(ls -lh "$file" | awk '{print $5}')
      birthtime=$(stat -f "%SB" "$file" 2>/dev/null)
      MATCHES+=("$filename:$size:$birthtime:$count")
      printf "  %-40s  %7s  Born: %-24s  (%d matches)\n" "$filename" "$size" "$birthtime" "$count"
    fi
  fi
done

if [ ${#MATCHES[@]} -eq 0 ]; then
  echo "No matching debug logs found"
  exit 1
fi

echo ""
echo "Found ${#MATCHES[@]} matching file(s)"

# Show sample if verbose
if [ "$VERBOSE" -eq 1 ] && [ ${#MATCHES[@]} -gt 0 ]; then
  IFS=':' read -r first_file rest <<< "${MATCHES[0]}"
  echo ""
  echo "Sample from: $first_file (first 5 matches)"
  echo "  ---"
  grep $GREP_OPTS "$PATTERN" "$DEBUG_DIR/$first_file" 2>/dev/null | head -5 | sed 's/^/  /'
  echo ""
fi

echo ""
echo "Tip: Use -v/--verbose to see sample lines"
echo "     Use -s 'session title' for DB cross-reference mode"
