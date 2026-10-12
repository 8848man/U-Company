# Office current-goal visibility plan

Status: Partially implemented. Office now shows a compact active-goal rail with phase graphs, assigned teammates, QA counts, approval/blocked review actions, and an entry to goal conversation. The richer expanded goal workspace, last confirmed event, exact approval scope, and full keyboard/narrow-layout acceptance remain planned. This document is not evidence of live desktop QA.

## User need

The CEO should see every goal currently moving through the team, who is involved, and which decision comes next without opening each character. Office already receives the persisted goals, tasks, approvals, events, and agents used by the board; reuse those selectors and the existing phase graph.

## One-screen layout

Place a compact `진행 중인 목표` rail above or beside the Office room. Show active and approval-waiting goals first, then blocked goals; completed goals live behind a separate history control. Keep the pixel room and selected-member activity visible in the same desktop viewport. At narrower widths, stack the rail above the room and retain the fixed app header/sidebar.

Each goal card shows: deterministic short goal title with full original text on expansion; status and last confirmed update; the six existing phase segments and completed/total file-task count (never an invented percent); active and assigned teammate portraits/names derived from stage ownership and `Task.assigneeId`; a distinct CEO approval badge; and a short next action. If grouped on the board, show the work-group label while keeping a separate card per goal.

Expanding a card reveals the original goal text, task list with assignee and status, approved or pending decision scope, latest safe operational event, QA rework count/limit, and links to the same board approval dialog and teammate activity view. Avoid raw model reasoning, arbitrary stdout, prompts, and credentials. A character click continues to open that character's activity, while a goal click selects a goal; neither action changes execution state.

## Implementation and acceptance

Reuse `phaseState`, `GoalGraph`, `memberActivity`, and the saved snapshot. Extract a shared goal-card selector only if the Office and board would otherwise duplicate nontrivial status logic. Add no model call and no new progress percentage. Verify two simultaneous goals with different developers, one approval-waiting goal, a blocked goal, a group of related goals, absent runtime events, keyboard expansion, and narrow layout. Office and board must agree on assignees, task counts, and status after restart.
