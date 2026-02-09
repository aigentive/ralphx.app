#!/usr/bin/env bash
# Fetch Claude Code memory docs with local cache
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

CACHE_DIR="$(get_project_root)/.claude/cache"
CACHE_FILE="$CACHE_DIR/claude-code-memory-docs.md"
DOCS_URL="https://docs.anthropic.com/en/docs/claude-code/memory"
# Cache valid for 7 days (604800 seconds)
MAX_AGE=604800

ACTION="${1:---read}"  # --read (default), --refresh, --age

mkdir -p "$CACHE_DIR"

# Check cache age
cache_age() {
  if [[ ! -f "$CACHE_FILE" ]]; then
    echo "999999"
    return
  fi
  local now file_mod
  now=$(date +%s)
  # BSD stat (macOS) vs GNU stat (Linux)
  file_mod=$(stat -f %m "$CACHE_FILE" 2>/dev/null || stat -c %Y "$CACHE_FILE" 2>/dev/null)
  echo $(( now - file_mod ))
}

# Fetch fresh docs
fetch_docs() {
  echo "Fetching Claude Code memory docs from $DOCS_URL..." >&2
  if curl -sfL "$DOCS_URL" -o "$CACHE_FILE.tmp" 2>/dev/null; then
    mv "$CACHE_FILE.tmp" "$CACHE_FILE"
    echo "Cached to $CACHE_FILE" >&2
  else
    echo "Warning: Could not fetch docs. Using existing cache if available." >&2
    rm -f "$CACHE_FILE.tmp"
  fi
}

case "$ACTION" in
  --refresh)
    fetch_docs
    [[ -f "$CACHE_FILE" ]] && cat "$CACHE_FILE"
    ;;
  --age)
    age=$(cache_age)
    if [[ $age -gt $MAX_AGE ]]; then
      echo "Cache stale ($age seconds old, max $MAX_AGE). Run with --refresh."
    elif [[ $age -eq 999999 ]]; then
      echo "No cache. Run with --refresh."
    else
      echo "Cache fresh ($age seconds old, max $MAX_AGE)."
    fi
    ;;
  --read|*)
    age=$(cache_age)
    if [[ $age -gt $MAX_AGE ]] || [[ $age -eq 999999 ]]; then
      fetch_docs
    fi
    if [[ -f "$CACHE_FILE" ]]; then
      cat "$CACHE_FILE"
    else
      echo "No docs available. Check network connection." >&2
      exit 1
    fi
    ;;
esac
