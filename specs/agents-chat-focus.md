# Agents Chat Focus

## Goal

Agents conversations can create and attach child agent runs, such as ideation and verification. The Agents screen should keep one primary chat surface and let users switch that surface to the relevant child run when they want to interact with it, instead of opening detached modal chats or showing empty artifact tabs.

## Chat Focus Model

The main Agents chat has an explicit focus:

- **Workspace**: the default focus. Shows the agent workspace conversation and composer.
- **Ideation run**: selected from an attached ideation run widget. Shows that ideation session's full chat and composer in the same main chat area.
- **Verification run**: selected from the Verification artifact surface once a verification run exists. Shows that verifier chat in the same main chat area.

Focus changes should not change the selected workspace in the left sidebar. A visible affordance must let the user return to the workspace chat.

## Attached Ideation Runs

Agent workspaces may attach multiple ideation sessions over time.

- The latest attached ideation session is the default attached run for artifact context.
- Clicking an `Open Run` action on a specific ideation widget focuses that specific run, even when it is not the latest.
- A multi-run switcher is optional until real usage proves users need to compare older attached runs frequently.

## Artifact Panel Contract

The right artifact panel is for structured outputs and actions, not duplicate full chats.

Tabs must be data-gated:

- **Plan**: show only when a plan artifact exists.
- **Verification**: show only when a plan exists and verification is possible or has run.
- **Proposals**: show only when a plan exists, because proposals are derived from a plan.
- **Tasks**: show only after the ideation plan has been accepted/converted into execution tasks.

Do not render empty tabs or empty chat containers just because a workspace is in an ideation-related mode.

## Header Shortcut Contract

Agents chat header shortcuts mirror the artifact panel availability. A shortcut must not appear unless the backing artifact tab would appear.

The chat header model/provider chips and stats chip remain independently controllable:

- Workspace/task-style chats may show provider/model and stats depending on surface needs.
- Agents chat header shows stats only.

## Verification Follow-up

When the user opens the Verification artifact tab and a verifier conversation exists, the main Agents chat should focus the verifier conversation while the artifact panel continues to show structured verification state. This keeps the transcript interactive without making the artifact panel a second chat surface.

## Non-Goals

- Do not move structured plan/proposal/task/verification artifacts into the main chat.
- Do not make the artifact panel host a full chat composer.
- Do not pollute the workspace agent conversation with child-run-specific tool calls when a dedicated child run owns those tools.
