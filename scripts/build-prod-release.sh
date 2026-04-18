#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Build RalphX production release artifacts without mutating local app data.

Usage:
  ./scripts/build-prod-release.sh [--clean] [--skip-build]

Options:
  --clean       Remove existing release bundle artifacts before building
  --skip-build  Skip the build step and only validate/report artifact paths
  -h, --help    Show this help

This script is production-oriented:
- it never seeds Application Support from the dev DB
- it never copies plugin runtime into Application Support
- it is the correct base entrypoint for CI/release automation

Use ./scripts/build-local-release.sh for internal local release-like workflows.
EOF
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
PREPARE_RUNTIME_SCRIPT="${PROJECT_ROOT}/scripts/prepare-release-runtime.sh"

CLEAN="false"
SKIP_BUILD="false"

for arg in "$@"; do
  case "${arg}" in
    --clean) CLEAN="true" ;;
    --skip-build) SKIP_BUILD="true" ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: ${arg}" >&2
      usage
      exit 1
      ;;
  esac
done

APP_PATH="${PROJECT_ROOT}/src-tauri/target/release/bundle/macos/RalphX.app"
MACOS_BUNDLE_DIR="${PROJECT_ROOT}/src-tauri/target/release/bundle/macos"
DMG_DIR="${PROJECT_ROOT}/src-tauri/target/release/bundle/dmg"
BIN_PATH="${PROJECT_ROOT}/src-tauri/target/release/ralphx"

if [[ "${CLEAN}" == "true" ]]; then
  echo "Cleaning previous release bundle artifacts..."
  rm -rf "${PROJECT_ROOT}/src-tauri/target/release/bundle"
fi

if [[ "${SKIP_BUILD}" != "true" ]]; then
  echo "Building RalphX production release artifacts..."
  "${PREPARE_RUNTIME_SCRIPT}"
  cd "${PROJECT_ROOT}/frontend"
  CI=false npm run tauri build
fi

missing_artifacts="false"

echo ""
echo "Production release artifact summary"
echo "---------------------------------"
echo "Local app data was not modified."
echo "Application Support plugin/runtime sync was not performed."

if [[ -x "${BIN_PATH}" ]]; then
  echo "Binary: ${BIN_PATH}"
fi

if [[ -d "${APP_PATH}" ]]; then
  echo "App bundle: ${APP_PATH}"
else
  echo "App bundle not found: ${APP_PATH}" >&2
  missing_artifacts="true"
fi

if [[ -d "${MACOS_BUNDLE_DIR}" ]]; then
  echo "Updater bundle directory: ${MACOS_BUNDLE_DIR}"
  updater_count=0
  while IFS= read -r updater_path; do
    [[ -n "${updater_path}" ]] || continue
    echo "${updater_path}"
    if [[ ! -f "${updater_path}.sig" ]]; then
      echo "Updater signature not found for: ${updater_path}" >&2
      missing_artifacts="true"
      continue
    fi
    echo "${updater_path}.sig"
    updater_count=$((updater_count + 1))
  done < <(find "${MACOS_BUNDLE_DIR}" -maxdepth 1 -type f -name '*.app.tar.gz' -print)
  if [[ "${updater_count}" -eq 0 ]]; then
    echo "No updater bundles found under: ${MACOS_BUNDLE_DIR}" >&2
    missing_artifacts="true"
  fi
else
  echo "Updater bundle directory not found: ${MACOS_BUNDLE_DIR}" >&2
  missing_artifacts="true"
fi

if [[ -d "${DMG_DIR}" ]]; then
  echo "DMG directory: ${DMG_DIR}"
  dmg_count=0
  while IFS= read -r dmg_path; do
    echo "${dmg_path}"
    dmg_count=$((dmg_count + 1))
  done < <(find "${DMG_DIR}" -maxdepth 1 -type f -name '*.dmg' -print)
  if [[ "${dmg_count}" -eq 0 ]]; then
    echo "No DMG artifacts found under: ${DMG_DIR}" >&2
    missing_artifacts="true"
  fi
else
  echo "DMG directory not found: ${DMG_DIR}" >&2
  missing_artifacts="true"
fi

echo ""
echo "Next step:"
echo "  Validate the packaged app outside the source checkout before publishing."

if [[ "${missing_artifacts}" == "true" ]]; then
  exit 1
fi
