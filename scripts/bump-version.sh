#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMMON_FILE="${SCRIPT_DIR}/release-analysis-common.sh"

source "${COMMON_FILE}"

VERSION="${1:-}"

if [[ -z "${VERSION}" ]]; then
  VERSION="$(release_analysis_try_read_selected_version || true)"
fi

if [[ -z "${VERSION}" ]]; then
  echo "Usage: ./scripts/bump-version.sh <version>"
  echo "Example: ./scripts/bump-version.sh 0.2.0"
  echo "Tip: run ./scripts/propose-release.sh and accept the proposal to populate ${RELEASE_ANALYSIS_VERSION_FILE}."
  exit 1
fi

VERSION="$(release_analysis_normalize_version "${VERSION}")"

if [[ $# -eq 0 ]]; then
  echo "Using stored release version ${VERSION} from ${RELEASE_ANALYSIS_VERSION_FILE}"
fi

echo "Bumping version to $VERSION..."

# Update package.json
npm --prefix frontend version $VERSION --no-git-tag-version

# Update Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" src-tauri/Cargo.toml

# Update tauri.conf.json
cd src-tauri
cat tauri.conf.json | jq ".version = \"$VERSION\"" > tauri.conf.json.tmp
mv tauri.conf.json.tmp tauri.conf.json
cd ..

echo "Version updated to $VERSION"
echo ""
echo "To release:"
echo "  git add frontend/package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json"
echo "  git commit -m 'chore: bump version to $VERSION'"
echo "  git tag v$VERSION"
echo "  git push origin main --tags"
