#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
PROMPT_FILE="${SCRIPT_DIR}/prompts/release-notes-codex-prompt.md"
DEFAULT_MODEL="${RELEASE_NOTES_MODEL:-gpt-5.4}"
DEFAULT_REASONING_EFFORT="${RELEASE_NOTES_REASONING_EFFORT:-xhigh}"
CODEX_RELEASE_NOTES_DEVELOPER_INSTRUCTIONS="For this run, do not read CLAUDE.md, DEVELOPMENT.md, or other fallback project docs unless the prompt explicitly names them. Stay within the provided release context, commit bodies, and commit subjects only."
LOGS_DIR=".artifacts/release-notes/logs"

usage() {
  cat <<'EOF'
Generate draft RalphX release notes with Codex CLI.

Usage:
  ./scripts/generate-release-notes.sh <version> [--from <ref>] [--to <ref>] [--model <model>] [--reasoning-effort <low|medium|high|xhigh>] [--output <file>] [--context-only]

Arguments:
  <version>             Release version without or with leading v (for example 0.2.0 or v0.2.0)

Options:
  --from <ref>          Explicit start ref/tag/commit for the compare range
  --to <ref>            End ref/tag/commit for the compare range (default: HEAD)
  --model <model>       Codex model to use (default: RELEASE_NOTES_MODEL or gpt-5.4)
  --reasoning-effort <level>
                        Codex reasoning effort to use (default: RELEASE_NOTES_REASONING_EFFORT or xhigh)
  --output <file>       Output markdown path (default: release-notes/v<version>.md)
  --context-only        Write the assembled release context instead of invoking Codex
  -h, --help            Show this help

Notes:
  - The generator only sees committed history in the selected git range.
  - Commit any release-affecting changes you want reflected before running it.
EOF
}

die() {
  echo "$*" >&2
  exit 1
}

version=""
from_ref=""
to_ref="HEAD"
model="${DEFAULT_MODEL}"
reasoning_effort="${DEFAULT_REASONING_EFFORT}"
output_path=""
context_only="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --from)
      shift
      [[ $# -gt 0 ]] || die "--from requires a ref"
      from_ref="$1"
      ;;
    --to)
      shift
      [[ $# -gt 0 ]] || die "--to requires a ref"
      to_ref="$1"
      ;;
    --model)
      shift
      [[ $# -gt 0 ]] || die "--model requires a model name"
      model="$1"
      ;;
    --reasoning-effort)
      shift
      [[ $# -gt 0 ]] || die "--reasoning-effort requires low, medium, high, or xhigh"
      reasoning_effort="$1"
      ;;
    --output)
      shift
      [[ $# -gt 0 ]] || die "--output requires a path"
      output_path="$1"
      ;;
    --context-only)
      context_only="true"
      ;;
    -*)
      die "Unknown option: $1"
      ;;
    *)
      if [[ -n "${version}" ]]; then
        die "Version already set to '${version}', unexpected argument '$1'"
      fi
      version="$1"
      ;;
  esac
  shift
done

[[ -n "${version}" ]] || die "Missing required <version> argument"
[[ -f "${PROMPT_FILE}" ]] || die "Missing prompt file: ${PROMPT_FILE}"
[[ "${reasoning_effort}" == "low" || "${reasoning_effort}" == "medium" || "${reasoning_effort}" == "high" || "${reasoning_effort}" == "xhigh" ]] || die "--reasoning-effort must be low, medium, high, or xhigh"

cd "${REPO_ROOT}"

raw_version="${version#v}"
tag="v${raw_version}"

resolve_default_from_ref() {
  local to_ref_local="$1"

  if git rev-parse -q --verify "refs/tags/${tag}" >/dev/null 2>&1; then
    git describe --tags --abbrev=0 "${tag}^" 2>/dev/null || true
    return
  fi

  git describe --tags --abbrev=0 "${to_ref_local}" 2>/dev/null || true
}

if [[ -z "${from_ref}" ]]; then
  from_ref="$(resolve_default_from_ref "${to_ref}")"
  [[ -n "${from_ref}" ]] || die "No prior tag found. Pass --from <ref> for the first release draft."
fi

git rev-parse --verify "${from_ref}^{commit}" >/dev/null 2>&1 || die "Invalid --from ref: ${from_ref}"
git rev-parse --verify "${to_ref}^{commit}" >/dev/null 2>&1 || die "Invalid --to ref: ${to_ref}"

range_spec="${from_ref}..${to_ref}"
commit_count="$(git rev-list --count "${range_spec}")"
[[ "${commit_count}" -gt 0 ]] || die "No commits found in range ${range_spec}"

if [[ -z "${output_path}" ]]; then
  if [[ "${context_only}" == "true" ]]; then
    output_path=".artifacts/release-notes/context-${tag}.md"
  else
    output_path="release-notes/${tag}.md"
  fi
fi

mkdir -p "$(dirname "${output_path}")"
mkdir -p "${LOGS_DIR}"

shortstat="$(git diff --shortstat "${range_spec}" || true)"
[[ -n "${shortstat}" ]] || shortstat="No diff stat available."

commit_log="$(git log --reverse --no-merges --pretty=format:'- %h %s' "${range_spec}")"

raw_commit_bodies="$(
  while IFS= read -r sha; do
    [[ -n "${sha}" ]] || continue
    body="$(git show -s --format=%B "${sha}")"
    [[ -n "$(printf '%s' "${body}" | tr -d '[:space:]')" ]] || continue
    printf -- '--- %s ---\n' "${sha:0:8}"
    printf '%s\n\n' "${body}"
  done < <(git rev-list --reverse "${range_spec}")
)"

reader_guidance=$'- Target multiple audiences at once: public readers, active users, contributors, and maintainers.\n- Favor runtime, UI, workflow, installation, and release outcomes.\n- Keep the tone precise and engineering-literate without drifting into repo-internal maintenance detail.\n- Keep internal config, docs, and scaffolding work out of Highlights unless the commit bodies make the shipped impact explicit.'

tmp_context="$(mktemp)"
tmp_final_input="$(mktemp)"
trap 'rm -f "${tmp_context}" "${tmp_final_input}"' EXIT

output_stem="$(basename "${output_path}" .md)"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
write_log="${LOGS_DIR}/${output_stem}-${timestamp}-generate.log"

{
  printf 'Release metadata:\n'
  printf -- '- Product: RalphX\n'
  printf -- '- Version: %s\n' "${raw_version}"
  printf -- '- Tag: %s\n' "${tag}"
  printf -- '- Compare range: %s\n' "${range_spec}"
  printf -- '- Commit count: %s\n' "${commit_count}"
  printf -- '- Diff stat: %s\n' "${shortstat}"
  printf '\nReader guidance:\n%s\n' "${reader_guidance}"
  printf '\nCommit subjects:\n'
  printf '%s\n' "${commit_log}"
  if [[ -n "${raw_commit_bodies}" ]]; then
    printf '\nRaw commit bodies (primary narrative source):\n%s\n' "${raw_commit_bodies}"
  fi
  printf '\nWriter instructions for this packet:\n'
  printf -- '- Use the raw commit bodies as the primary source of truth.\n'
  printf -- '- Group related bullets into coherent product areas instead of echoing commit subjects line by line.\n'
  printf -- '- Use commit subjects and diff stat only to fill gaps when the raw bodies are sparse.\n'
  printf -- '- Do not assume every `feat:` bullet means a net-new surface; many are expansions of existing behavior.\n'
  printf -- '- Keep internal-only work out of the main Highlights unless the shipped impact is explicit in the commit bodies.\n'
} > "${tmp_context}"

if [[ "${context_only}" == "true" ]]; then
  cp "${tmp_context}" "${output_path}"
  echo "Wrote release-notes context to ${output_path}"
  exit 0
fi

command -v codex >/dev/null 2>&1 || die "codex CLI not found in PATH"

codex_exec_common_args=(
  --model "${model}"
  -c "model_instructions_file=\"${PROMPT_FILE}\""
  -c "model_reasoning_effort=\"${reasoning_effort}\""
  -c 'project_doc_fallback_filenames=[]'
  -c "developer_instructions=\"${CODEX_RELEASE_NOTES_DEVELOPER_INSTRUCTIONS}\""
  --sandbox read-only
  --ephemeral
)

run_codex_with_log() {
  local log_file="$1"
  shift
  local raw_log="${log_file%.log}.raw.log"
  local started_at finished_at session_id exec_events

  started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  if "$@" > "${raw_log}" 2>&1; then
    finished_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    session_id="$(sed -n 's/^session id: //p' "${raw_log}" | head -n 1)"
    exec_events="$(grep -c '^exec$' "${raw_log}" || true)"

    {
      printf 'status: success\n'
      printf 'started_at: %s\n' "${started_at}"
      printf 'finished_at: %s\n' "${finished_at}"
      printf 'session_id: %s\n' "${session_id:-unknown}"
      printf 'model: %s\n' "${model}"
      printf 'reasoning_effort: %s\n' "${reasoning_effort}"
      printf 'exec_events: %s\n' "${exec_events}"
      printf 'raw_log_retained: false\n'
    } > "${log_file}"

    rm -f "${raw_log}"
    return 0
  fi

  finished_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  session_id="$(sed -n 's/^session id: //p' "${raw_log}" | head -n 1)"
  exec_events="$(grep -c '^exec$' "${raw_log}" || true)"

  {
    printf 'status: failure\n'
    printf 'started_at: %s\n' "${started_at}"
    printf 'finished_at: %s\n' "${finished_at}"
    printf 'session_id: %s\n' "${session_id:-unknown}"
    printf 'model: %s\n' "${model}"
    printf 'reasoning_effort: %s\n' "${reasoning_effort}"
    printf 'exec_events: %s\n' "${exec_events}"
    printf 'raw_log_retained: true\n'
    printf 'raw_log_path: %s\n' "${raw_log}"
    printf '\nlast_output_tail:\n'
    tail -n 80 "${raw_log}"
  } > "${log_file}"

  echo "Codex invocation failed. Summary: ${log_file}" >&2
  echo "Full raw log: ${raw_log}" >&2
  return 1
}

cat "${tmp_context}" > "${tmp_final_input}"

echo "Running release-notes writer..."
run_codex_with_log "${write_log}" \
  codex exec \
  "${codex_exec_common_args[@]}" \
  --output-last-message "${output_path}" \
  - < "${tmp_final_input}"

echo "Wrote draft release notes to ${output_path}"
echo "Generation log: ${write_log}"
