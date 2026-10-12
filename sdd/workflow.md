# Development workflow

For each change:

1. Read this index, the relevant code, then the owning spec and applicable ADRs. Load only the documents that affect the change.
2. Determine whether the spec already covers the desired behavior, needs updating, or conflicts with code. Resolve a conflict before implementation.
3. Update the owning spec first. Create an ADR before code when a choice crosses ownership areas, is costly to reverse, has viable alternatives, or its reason is not evident from code.
4. Review the spec and ADR against the product goal and security boundary.
5. Implement in a focused step. Keep structural refactors separate from new behavior and validate each step.
6. Run mechanical checks, behavior tests, cross-area checks, then update the owning documents and index. Record any drift and its resolution.
7. Update WORKLOG.md with evidence and the next action. Record a release only after a real deployment.
8. Open a pull request for the milestone following the [PR workflow](pr-workflow.md): one milestone per branch, a plain-language title and a three-depth summary.

For UI work, read the [UI/UX 구현 철학](ux-principles.md) first and design each screen at one of its three depths; check the result against its screen checklist before recording the milestone. Inspect existing components and their call sites before adding a screen. Reuse an existing component when its behavior and ownership fit. Keep data loading and commands in a controller, and give replaceable screen components explicit typed inputs and callbacks. A menu or panel should be replaceable without copying backend commands, approval rules, or persistence logic. If an existing component mixes those concerns, extract the smallest useful boundary before changing its presentation. Record any deliberate duplication and its removal path in WORKLOG.

When a change alters what the CEO does next, a visible decision, or a recovery branch, update the affected [user-flow files](userFlow/00_index.md) and their typed links in step 3. Keep business rules in their owning Spec; the flow graph describes navigation and observed state, not a second approval or lifecycle policy.

Apply the [SDD authoring principle](authoring.md) when adding or reshaping a topic. Split it into an indexed folder only when independent entities and their relationships meet that decision test. Preserve one rule owner and validate links after a move.

An ADR is a historical rationale and is never rewritten to change an accepted decision; supersede it. A spec is the mutable source of truth for intended current behavior.
