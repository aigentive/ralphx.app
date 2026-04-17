# Codex CLI Command Line Options

Official docs: `https://developers.openai.com/codex/cli/reference`

Snapshot notes:

- The official docs include a full command reference for interactive CLI and `codex exec`.
- The `codex exec` surface includes `--skip-git-repo-check`, repeatable `-c/--config key=value`, a prompt argument or stdin, and a `resume` subcommand.
- The current official docs are materially ahead of the installed local binary.

Local binary snapshot on 2026-04-07:

```text
codex --version -> 0.1.2505172129
brew cask -> codex 0.116.0
```

Observed local `codex --help` flags:

- `-m, --model`
- `-p, --provider`
- `-i, --image`
- `-v, --view`
- `--history`
- `--login`
- `--free`
- `-q, --quiet`
- `-c, --config`
- `-w, --writable-root`
- `-a, --approval-mode`
- `--auto-edit`
- `--full-auto`
- `--no-project-doc`
- `--project-doc`
- `--full-stdout`
- `--notify`
- `--disable-response-storage`
- `--flex-mode`
- `--reasoning`
- `--dangerously-auto-approve-everything`
- `-f, --full-context`

RalphX notes:

- Do not hard-code one CLI reference surface.
- Codex spawn code should expose a capability table derived from runtime detection.
