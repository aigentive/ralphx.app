@prd.md @activity.md

We are building the project according to the PRD in this repo.

First read activity.md to see what was recently accomplished.

## Start the Application

<!-- START_COMMAND_PLACEHOLDER: Update this section after creating your PRD -->
Start the site locally. Use the appropriate command for your tech stack:
- `npm run dev` (for Next.js, Vite, or similar)
- `pnpm dev` (if using pnpm)
- `bun dev` (if using Bun)
- `python3 -m http.server 8000 --bind 127.0.0.1` (for static HTML)

If the port is taken, try another port.
<!-- END_START_COMMAND_PLACEHOLDER -->

## Work on Tasks

Open prd.md and find the single highest priority task where `"passes": false`.

Work on exactly ONE task:
1. Implement the change according to the task steps
2. Run any available checks:
   - `npm run lint` (if available)
   - `npm run typecheck` (if available)
   - `npm run build` (if available)

## Log Progress

Append a dated progress entry to activity.md describing:
- What you changed
- What commands you ran
- Any issues encountered and how you resolved them

## Update Task Status

When the task is confirmed working, update that task's `"passes"` field in prd.md from `false` to `true`.

## Commit Changes

Make one git commit for that task only with a clear, descriptive message:
```
git add .
git commit -m "feat: [brief description of what was implemented]"
```

Do NOT run `git init`, do NOT change git remotes, and do NOT push.

## Important Rules

- ONLY work on a SINGLE task per iteration
- Always log your progress in activity.md
- Always commit after completing a task

## Completion

When ALL tasks have `"passes": true`, output:

<promise>COMPLETE</promise>
