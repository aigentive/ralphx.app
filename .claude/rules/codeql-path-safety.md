> **Maintainer note:** Keep this file compact. Prefer one-line rules, links to source docs, and explicit non-negotiables over prose.

# CodeQL Path Safety

| Rule | Detail |
|---|---|
| Validate before path sinks | Before `read`, `write`, `remove`, `rename`, `symlink`, `Command::current_dir`, or process/plugin launch paths, validate any path influenced by env vars, project settings, HTTP/MCP payloads, DB state, agent metadata, or repo contents. |
| Tests are scanned too | Test-only `read`, `write`, `create_dir_all`, `remove_*`, and temp-path helpers can block PRs. Apply the same path-sink rules in tests as production. |
| No env roots at sinks | Do not feed paths rooted in `HOME`, `TMPDIR`, `RALPHX_*`, request env, or other env vars into filesystem sinks. Resolve through an app-owned helper (`runtime_log_paths`, Tauri/dirs APIs) and keep env-derived values out of sink paths. |
| Prefer containment helpers | Join untrusted relative paths only through helpers that reject `RootDir`, `Prefix`, `ParentDir`, empty components, separators in single components, and `..`; canonicalize the trusted parent and assert the sink parent stays under it. |
| Hash or enum path components | User/runtime strings such as task ids, branch names, modes, agent names, and filenames must not be raw path components. Use a fixed enum mapping or a hash-derived component before joining. |
| Do not rely on construction provenance | Even if a value “should” come from canonical config, validate again at the filesystem sink; CodeQL tracks taint across helper layers and needs sink-local proof. |
| Keep runtime roots distinct | RalphX-owned runtime/plugin/cache roots may be outside the active project, but child paths under them still need containment checks. |
| Test traversal failures | Add focused tests for `../`, absolute paths, symlink escapes where relevant, and the accepted normal path. |
| No string sanitizing shortcuts | Do not strip `../`; reject unsafe input. |
| If adding a sink, search alerts first | Before committing path-heavy changes, run `rg "std::fs::|fs\\.|File::|OpenOptions|current_dir|remove_"` on touched files and verify each sink uses the project’s safe helper. |
