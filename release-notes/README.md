# Release Notes

This directory holds curated release notes that the release workflow will use automatically when a matching file exists.

Naming convention:

- `release-notes/v0.2.0.md`
- `release-notes/v0.2.1.md`

Typical flow:

1. Prefer the guided wrapper:
   - `./scripts/release.sh`
2. Review the proposal when prompted, accept it to continue, then review and edit the generated `release-notes/vX.Y.Z.md`
3. Commit it before tagging if you want the workflow-created draft release to use it automatically

Notes:

- Release proposals default to `.artifacts/release-notes/proposal-from-v<current-version>.md`
- Accepted release versions are stored in `.artifacts/release-notes/.version`
- `./scripts/propose-release.sh`, `./scripts/bump-version.sh`, and `./scripts/generate-release-notes.sh` still work as standalone lower-level steps
- Codex generation logs are written to `.artifacts/release-notes/logs/`
- The full release sequence lives in `docs/release-process.md`
