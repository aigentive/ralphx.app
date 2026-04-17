# Codex Agent Approvals And Security

Official docs: `https://developers.openai.com/codex/agent-approvals-security`

Snapshot notes:

- Codex docs separate approvals/security from generic sandboxing.
- Official config reference ties approvals to `approval_policy` and granular approval controls.
- Protected paths, network access, and sandbox/approval interactions are first-class topics.

RalphX notes:

- RalphX must map its execution halt and operator expectations onto Codex approval semantics without assuming Claude behavior.
- Initial Codex rollout should stay conservative and explicit about network/disk policy.
