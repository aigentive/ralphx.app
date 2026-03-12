# API Versioning — RalphX External MCP

## Current Version

The external MCP server is at API version **1.0.0**. This version string is returned in MCP server capabilities during session initialization:

```json
{
  "name": "ralphx-external-mcp",
  "version": "1.0.0"
}
```

## Tool Name Convention: v1_ Prefix

All 33 tools are prefixed with `v1_` to encode the API version directly in the tool name. This is a deliberate MCP-compatible versioning strategy: MCP has no native concept of API versions, so the prefix makes version visible to agents listing tools.

```
v1_list_projects
v1_get_project_status
v1_start_ideation
... (all 33 tools)
```

**Rationale:** When v2 tools are introduced, both `v1_list_projects` and `v2_list_projects` can coexist in the same server. An agent targeting v1 tools continues to work without modification.

## Deprecation Policy

When a tool is superseded by a v2 equivalent:

1. The v1 tool's description gains a `[DEPRECATED]` prefix and a migration note:
   ```
   [DEPRECATED: use v2_list_projects instead] List projects accessible to this API key
   ```
2. A `deprecated: true` field is added to the tool definition metadata.
3. The v1 tool is maintained for **two full API versions** (i.e., until v3 ships).
4. Deprecated tools emit a warning in server logs on each call.

## Breaking vs Non-Breaking Changes

**Non-breaking changes** (allowed without version bump):
- Adding optional input parameters with defined defaults
- Adding new fields to response objects
- Adding new tools (new v1_ names)
- Fixing response body bugs where old behavior was clearly wrong

**Breaking changes** (require v2_ prefix on affected tools):
- Removing input parameters
- Renaming input parameters
- Changing the type or semantics of existing parameters
- Removing fields from response objects that callers may depend on
- Changing error codes or error shapes
- Changing the permission level required for a tool

## v1 to v2 Migration Pattern

When a tool requires a breaking change, the migration follows this sequence:

1. Implement `v2_<tool_name>` with the new schema.
2. Mark `v1_<tool_name>` as deprecated (description prefix + metadata).
3. Announce in release notes with a migration guide showing input/output diff.
4. Maintain v1 for at least one full minor release cycle before removal.
5. Remove v1 tool only when v3 ships.

### Example: hypothetical v1 → v2 migration for v1_list_projects

**v1 response** (current):
```json
[{ "id": "proj-1", "name": "MyProject" }]
```

**v2 response** (hypothetical):
```json
{
  "projects": [{ "id": "proj-1", "name": "MyProject", "created_at": "2026-01-01T00:00:00Z" }],
  "total": 1
}
```

Migration guide would show the wrapper object change and instruct callers to access `.projects` instead of the root array.

## Schema Stability Guarantees

Within v1:
- All input parameter names and types are stable.
- Response object fields will not be removed.
- New optional response fields may be added at any time.
- Error codes (`error` string field in JSON error responses) are stable.

Agents should tolerate unknown fields in responses — this is safe under the non-breaking rules above.
