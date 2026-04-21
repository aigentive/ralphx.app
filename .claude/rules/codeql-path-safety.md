# CodeQL Path Safety

| Rule | Detail |
|---|---|
| Validate before path sinks | Before `read`, `write`, `remove`, `rename`, `symlink`, `Command::current_dir`, or process/plugin launch paths, validate any path influenced by env vars, project settings, HTTP/MCP payloads, DB state, agent metadata, or repo contents. |
| Prefer containment helpers | Join untrusted relative paths only through helpers that reject `RootDir`, `Prefix`, `ParentDir`, empty components, separators in single components, and `..`; canonicalize the trusted parent and assert the sink parent stays under it. |
| Do not rely on construction provenance | Even if a value “should” come from canonical config, validate again at the filesystem sink; CodeQL tracks taint across helper layers and needs sink-local proof. |
| Keep runtime roots distinct | RalphX-owned runtime/plugin/cache roots may be outside the active project, but child paths under them still need containment checks. |
| Test traversal failures | Add focused tests for `../`, absolute paths, symlink escapes where relevant, and the accepted normal path. |
| No string sanitizing shortcuts | Do not strip `../`; reject unsafe input. |
