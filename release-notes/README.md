# Release Notes

This directory holds curated release notes that the release workflow will use automatically when a matching file exists.

Naming convention:

- `release-notes/v0.2.0.md`
- `release-notes/v0.2.1.md`

Typical flow:

1. Generate a draft with:
   - `./scripts/generate-release-notes.sh 0.2.0 --model gpt-5.4 --reasoning-effort xhigh`
2. Review and edit `release-notes/v0.2.0.md`
3. Commit it before tagging if you want the workflow-created draft release to use it automatically

Notes:

- Codex generation logs are written to `.artifacts/release-notes/logs/`
- The full release sequence lives in `docs/release-process.md`
