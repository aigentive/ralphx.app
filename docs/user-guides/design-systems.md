# Design Systems User Guide

Design is the RalphX workspace for turning existing product UI into a reviewed, reusable design system. It reads selected projects as source material, stores generated design-system state in RalphX-owned storage, and lets you use the result as a reference contract for future screen and component work.

---

## Quick Reference

| Question | Answer |
|----------|--------|
| How do I start? | Open **Design**, create a design system from one or more source projects, then click **Generate**. |
| What does it generate? | A human styleguide, preview rows, source refs, caveats, and a machine-readable schema. |
| Does it write into my project? | No. Project checkouts are source references only unless a later explicit implementation task changes code. |
| Where is generated state stored? | RalphX-owned app storage and artifact records, not the selected source paths. |
| How do I use it in another project? | Export/import the package, attach it to a project, or reference it in Design/Ideation prompts. |
| How do I fix a row? | Expand the row, click **Needs work**, send feedback, and continue in the same Design chat. |
| How do I accept a row? | Click **Looks good**. Approval is item-local and can reset after source changes. |

---

## Mental Model

A RalphX design system is a reviewed contract, not a package silently installed into a repo.

```
Source project UI
        |
        v
Design source scan
        |
        v
Styleguide rows + preview widgets + schema
        |
        v
Design chat review and feedback
        |
        v
Reference artifacts for future screens/components
```

The generated styleguide is for humans. The schema is for agents. The previews are the inspection surface that helps you decide whether a token, component, layout, or asset rule is usable.

## Core Rules

- Project folders are read-only sources during design-system generation.
- Generated schemas, previews, reports, assets, and exports live in RalphX-owned storage.
- Source paths are metadata and provenance, not output destinations.
- Export/import is explicit.
- Design-generated screens/components are RalphX artifacts first; implementation into a project should happen through a separate approved handoff.

## Create A Design System

1. Open **Design**.
2. Click **New design system**.
3. Pick a primary source project.
4. Add optional reference projects.
5. Scope the scan to useful source paths.
6. Click **Create**.
7. Click **Generate** in the styleguide pane.

Good source scopes are relative paths such as:

```text
frontend/src
frontend/src/components
frontend/src/styles
public
app
src
```

Avoid overly broad roots when the repo is large. A focused UI/style/assets scope gives the source scanner better signal and reduces fallback references.

## Primary And Reference Sources

The primary source project is the main authority for the design system. RalphX uses it for the design conversation, source attribution, and project-level filtering.

Reference sources are supporting evidence. Use them when a product has more than one UI surface, for example:

- app plus marketing site
- admin console plus docs site
- old design system package plus current product repo
- shared component library plus app shell

Reference sources should not override the primary project unless the row makes that source relationship clear.

## Review The Styleguide

After generation, the right pane shows grouped rows:

- **Brand**: visual identity and asset references
- **Colors**: primary, semantic, surface, border, and focus roles
- **Components**: buttons, inputs, composers, cards, and reusable controls
- **Spacing**: radii, density, borders, focus rings, elevation
- **Type**: typography hierarchy, labels, code font, density
- **UI Kit**: workspace and screen surface patterns

Expand one row at a time. The preview widget is the main thing to review. Source refs and confidence explain where the row came from.

Use row state as a work queue:

| State | Meaning | What to do |
|-------|---------|------------|
| `needs_review` | Generated and waiting for review | Inspect preview, then approve or comment |
| `approved` | You accepted this row | Leave it unless source changes |
| `needs_work` | Feedback is open | Let Design regenerate or refine the row |
| `stale` | Source changed after approval | Review the changed row again |
| `source review` | Confidence is low or fallback refs were used | Check source refs before approval |

## Use Design Chat

Each design system has its own Design conversation. Use it to ask questions or request refinements without leaving the styleguide.

Useful prompts:

```text
Explain why the Typography scale row used these three source files.
```

```text
Regenerate Core controls with flatter button radius and stronger focus state.
```

```text
Compare Workspace surfaces against the current Agents view and flag mismatches.
```

```text
Generate a settings screen artifact using this design system.
```

Row feedback from **Needs work** is appended to the same conversation with item id, preview id, and source refs attached.

## Generate Reference Artifacts

From a reviewed row, use **Generate component** or **Generate screen** to create a schema-aligned artifact. The artifact should include:

- source item
- design-system schema version
- rendered preview
- implementation notes
- caveats
- project write status

These artifacts are reference outputs. They let you inspect the proposed UI before any task changes a project checkout.

## Use A Design System In Project Work

There are three common paths.

### 1. Ask Design For A Reference Artifact

Use this when you want a screen or component concept before implementation.

1. Select the design system.
2. Ask Design to generate the screen/component.
3. Review the artifact preview.
4. Approve or refine the artifact.
5. Use the artifact as input to Ideation or a later implementation task.

Example:

```text
Generate a compact settings screen for this app using the current RalphX design system. Keep it as an artifact only.
```

### 2. Reference It During Ideation

Use this when a feature needs to follow an existing visual system.

Example Ideation prompt:

```text
Plan a task to add a notification preferences screen. Use the RalphX Design System as the visual reference. Do not invent new tokens or component shapes unless the design system has no matching pattern.
```

The design system should guide the plan, but implementation still follows the normal task, review, and merge pipeline.

### 3. Export And Import Across Projects

Use export/import when another project should start from an existing design contract.

1. In Design, select the source design system.
2. Click **Export**.
3. When the export result appears, click **Download JSON** to save the package locally.
4. Keep the default redacted package unless full provenance is required.
5. Import the package in the target project.
6. Attach it as a reference or re-analyze against that project.

Imported design systems are not automatically source-grounded for the new project. Re-analyze when you need project-specific confidence.

## Keep It Current

Regenerate a design system when:

- major UI components change
- theme tokens move
- fonts/assets are added
- source paths change
- approvals become stale
- a row used fallback source references

After regeneration, review changed rows instead of re-approving everything blindly.

## Caveats And Source Confidence

A caveat means RalphX could not prove something strongly enough from selected sources. Common examples:

| Caveat | Cause | Fix |
|--------|-------|-----|
| Fallback source references | Selected paths did not include a direct match | Add better UI/style paths and regenerate |
| Missing brand fonts | Font assets were not in selected sources | Add font assets or upload them into RalphX-owned storage |
| Low-confidence visual identity | Logo/icon assets were indirect or absent | Add public assets or brand package as source |
| Sparse components | Component library path was not selected | Add component folders and regenerate |

Treat caveated rows as review-required, not canonical.

## Export Privacy

Default exports should redact absolute local paths and exclude raw source snippets. Include full provenance only when the recipient is allowed to see local source paths and source metadata.

Preferred package contents:

```text
design-system-export.zip
  manifest.json
  schema/design-system.schema.json
  schema/component-patterns.json
  schema/screen-patterns.json
  previews/
  reports/
```

## Troubleshooting

| Symptom | Check |
|---------|-------|
| Generate appears to do nothing | Check for a success banner and the latest app log entry `Generated design styleguide`. |
| Rows have fallback caveats | Narrow or correct selected paths, then regenerate. |
| Typography or Workspace previews look generic | Expand the row after generation; if it still shows only metadata, regenerate and check the preview artifact. |
| Chat is empty | Select a design system row in the sidebar so the Design conversation is scoped to that system. |
| Export/import loses source grounding | Imported packages are references until re-analyzed against the target project. |
| Project files changed unexpectedly | Treat as a bug. Design generation should not write to selected project paths. |

## Related Docs

- [First-class Design feature spec](../../specs/design/first-class-design-feature.md)
- [Design styleguide schema example](../../specs/design/schemas/design-styleguide-view-model.example.json)
- [Getting Started](getting-started.md)
- [Ideation Studio](ideation-studio.md)
