#!/bin/bash
set -e

VERSION=$1

if [ -z "$VERSION" ]; then
  echo "Usage: ./scripts/bump-version.sh <version>"
  echo "Example: ./scripts/bump-version.sh 0.2.0"
  exit 1
fi

echo "Bumping version to $VERSION..."

# Update package.json
npm version $VERSION --no-git-tag-version

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
echo "  git add -A && git commit -m 'chore: bump version to $VERSION'"
echo "  git tag v$VERSION"
echo "  git push origin main --tags"
