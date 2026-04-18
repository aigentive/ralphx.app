# GPT-5.4 Prompting Notes

This note captures the OpenAI guidance that matters most for RalphX when writing prompts for `gpt-5.4` and Codex CLI flows.

## Scope

- Focus: prompt authoring and instruction layering for GPT-5.4/Codex use in RalphX
- Source policy: official OpenAI sources only
- Goal: preserve one repo-local reference for future prompt work without relying on memory

## What We Confirmed

### 1. GPT-5.4 has a public system card

Confirmed sources:
- GPT-5.4 system card page: `https://openai.com/index/gpt-5-4-thinking-system-card/`
- GPT-5.4 system card PDF: `https://deploymentsafety.openai.com/gpt-5-4-thinking/gpt-5-4-thinking.pdf`

What matters for RalphX:
- The GPT-5.4 system card is real and should be treated as an authoritative model-behavior and safety reference.
- It is not a prompt-formatting guide.
- It does contain instruction-hierarchy and developer-message context relevant to steerability and computer-use behavior.

Relevant model-behavior findings:
- GPT-5.4 is designed for professional work and long-horizon tasks with tools, software environments, and complex workflows.
- GPT-5.4 behavior is steerable through developer messages.
- The system card describes the model following configurable developer-provided confirmation policy in line with instruction hierarchy.

Supporting sources:
- GPT-5.4 launch post, lines 42-50 and 124-127: `https://openai.com/index/introducing-gpt-5-4/`
- GPT-5.4 system card PDF, around the computer-use confirmation-policy section: `https://deploymentsafety.openai.com/gpt-5-4-thinking/gpt-5-4-thinking.pdf`

### 2. The XML-like prompt recommendation does not come from the GPT-5.4 system card

What we confirmed:
- We did not find GPT-5.4 system-card guidance saying “use XML syntax”.
- The XML-like structure recommendation comes from OpenAI’s GPT-5 coding guidance, not from the GPT-5.4 system card itself.

RalphX conclusion:
- It is valid to use XML-like structure for GPT-5.4 prompts.
- We should cite the GPT-5 coding cheat sheet for that recommendation, not the system card.

Supporting source:
- GPT-5 for Coding cheat sheet: `https://cdn.openai.com/API/docs/gpt-5-for-coding-cheatsheet.pdf`

### 3. Codex instruction layering matters and should be used correctly

What OpenAI says:
- Codex reads the `instructions` field from `model_instructions_file` when configured.
- `developer_instructions` is a separate developer-layer message.

RalphX conclusion:
- If we have a reusable prompt contract, prefer passing it through `model_instructions_file`.
- Keep lightweight operational guardrails in `developer_instructions`.
- Pass task-specific facts as the user payload instead of concatenating all instruction layers into the same blob.

Supporting source:
- OpenAI, *Unrolling the Codex agent loop*: `https://openai.com/index/unrolling-the-codex-agent-loop/`

## Prompting Guidance We Should Follow

### Be precise and avoid conflicting instructions

OpenAI guidance:
- GPT-5 models follow instructions strongly and can struggle when instructions are vague or conflicting.

RalphX implication:
- Keep prompt rules crisp and non-overlapping.
- Avoid telling the model to do two things that compete, such as “be exhaustive” and “be concise” without context.
- Avoid keeping old/dead prompt rules around after the workflow changes.

### Use XML-like structure when the prompt has multiple instruction blocks

OpenAI guidance:
- GPT-5 works well with XML-like syntax for structured instructions.

RalphX implication:
- Prefer sectioned prompt files for long-lived prompt contracts.
- Use stable blocks such as:
  - `<task>`
  - `<style_goals>`
  - `<output_format>`
  - `<source_of_truth>`
  - `<writing_rules>`
- Do not use XML-like structure as ornament; use it only when it clarifies hierarchy.

### Avoid overly forceful language unless it is truly necessary

OpenAI guidance:
- GPT-5 can overreact to very forceful instructions and become overly eager or overly thorough.

RalphX implication:
- Avoid prompt lines like “be EXTREMELY THOROUGH” or “get the FULL picture” unless the task truly requires it.
- Prefer concrete operational guidance over emotional intensity.
- If the model is over-gathering context, tighten the scope instead of just yelling at it harder.

### Control eagerness explicitly

OpenAI guidance:
- GPT-5 agents can be very eager in context gathering and tool use.

RalphX implication:
- For agentic flows, tell the model what it should treat as the primary source of truth.
- Tell it what is secondary context.
- Bound the work shape instead of relying on generic “be smart” instructions.

## RalphX-Specific Recommendations

### For reusable prompt contracts

- Put the stable prompt contract in a dedicated file.
- Pass that contract through `model_instructions_file` when using Codex CLI config overrides.
- Keep role separation intact:
  - system-style prompt file = stable contract
  - developer instructions = run guardrails
  - user payload = concrete task facts

### For release-note generation prompts

- Primary source of truth: raw commit bodies
- Secondary context: commit subjects and diff stat
- Do not infer shipped behavior from file names alone
- Use a balanced public/dev-community tone:
  - technically precise
  - not apologetic
  - not marketing fluff
- Prefer concrete visible examples in bullets
- Use short SHA citations when the supporting commit is known

### For Codex/GPT-5.4 prompt maintenance

- Remove stale prompt rules when the workflow changes
- Avoid duplicated instruction layers that say the same thing in different words
- Keep sections small and semantically distinct
- If the model starts producing self-undermining phrasing, tighten the prompt with explicit negative examples rather than adding more generic prose

## What Not To Assume

- Do not assume the GPT-5.4 system card is the right source for prompt-formatting advice.
- Do not assume all official GPT-5 guidance is specific to GPT-5.4; some is general GPT-5 guidance that still applies to GPT-5.4.
- Do not assume every strong prompt should become longer; GPT-5.4 benefits more from clarity than from instruction bulk.

## Current RalphX Application

Current adopted practice for the release-notes generator:
- XML-like prompt structure
- prompt file passed via `model_instructions_file`
- lightweight `developer_instructions` guardrail
- release context passed separately as the user payload

This is the pattern to prefer for future reusable Codex/GPT-5.4 prompt surfaces unless a more specific official OpenAI recommendation supersedes it.
