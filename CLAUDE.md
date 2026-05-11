# Claude Instructions

Claude and Claude Code should follow `AGENTS.md` as the canonical project
instruction file.

Before editing, read:

- `AGENTS.md`
- `docs/AI_CONTEXT_INDEX.md`
- the task-specific canonical documents named by the context index

Do not duplicate project policy here. If an instruction appears to conflict with
`AGENTS.md`, treat `AGENTS.md` as authoritative and update this file only to
preserve that delegation.

Do not begin implementation while the documentation readiness or design review
gate has a blocker.

Keep work inside this repository unless the user explicitly asks otherwise.

Tool-specific Claude Code configuration, if needed, belongs in `.agents/` or
`.codex/` and must not contradict project policy.
