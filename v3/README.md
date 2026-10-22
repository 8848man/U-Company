# Utopia Company v3

v3 is the frontend shell for the first v3 package ([v3 improvement plan](../sdd/v3-improvement-plan.md), [ADR-0038](../sdd/architecture/decisions/ADR-0038.md), [ADR-0039](../sdd/architecture/decisions/ADR-0039.md)). It keeps the v2 five menus (홈, 업무, Office, 토큰, 설정) and the single goal workspace, and runs on the same Tauri/Rust engine, SQLite data, approval and security rules. The engine owns every task state and approval; the screen only reads saved state and sends existing commands.

What changes from v2 is the CEO's minimal flow for the first pilot — three launch-preparation documents from local material:

1. **Home** shows one readiness card only when the team cannot run (with one action), one command box with an optional launch-preparation example, a **결정할 일** inbox where each goal reads 이유 → 마지막 확인 → 담당 → 다음 행동, and goal cards with per-deliverable progress (`산출물 N개 · QA 통과 k`).
2. The **approval dialog** and the **goal workspace** show the same saved plan: deliverable, engine-chosen assignee and reason, files, evidence and acceptance criteria. After QA they show per-criterion verdicts, evidence and unverified items, and a completed goal opens as **산출물 인수 확인**.
3. **Office** shows the same block facts on its goal rail. No new menu, dashboard or run mode is added.

## Start

- Windows PowerShell: `D:\projects\utopia_company\v3\start-windows.ps1`
- Prerequisites only: `D:\projects\utopia_company\v3\start-windows.ps1 -CheckOnly`
- Pilot with separate data: `D:\projects\utopia_company\v3\start-windows.ps1 -DataDir .run-data\v3-pilot` (manual QA-39)
- Sample app with the token-free mock AI: `D:\projects\utopia_company\v3\start-windows.ps1 -DataDir .run-data\sample-mock`, then follow [QA-50](../qa/SAMPLE-APP-FLOW.md)
- Ubuntu 24.04: `bash v3/start-ubuntu.sh` (desktop run not yet verified)
- Build only: `npm run build:v3`

Only one launcher can use Vite port 1420: close the v2 or original app first. Without `-DataDir` the existing data is used, so v2 and v3 show the same projects and goals. Project files change only through the engine's approved actions.

## Source layout

- `src/App.tsx`: v3 state/command controller — a copy of `v2/src/App.tsx` with the v3 Home wired in.
- `src/V3Home.tsx`: readiness, command box with the pilot example, decision inbox, per-deliverable goal cards.
- `src/v3.css`: v3-only styles on top of `v2/src/v2.css`.
- Reused: `v2/src/V2Navigation.tsx`, `V2Work.tsx`, `V2Projects.tsx`, `V2Connections.tsx`, `V2NotificationSettings.tsx`, and shared `../src/` components (`GoalConversationChat`, `DeliverablePanel`, `deliverables.ts`, `OfficeGoalRail`, `MeetingPanel`, Office Canvas).

`src/App.tsx` duplicates the v2 controller. This is recorded migration debt (as v2 recorded for v1): extract the shared commands and dialogs into a typed hook, then let v2 and v3 import it and keep only their Home/navigation differences.

## Interactive QA

Follow [QA-39](../qa/MANUAL-QA.md) with synthetic local material only (for example `notes/idea.md`, `notes/interviews.md` in an empty ordinary folder). Do not use real customer data, web research, posting or outreach in the pilot. Offline tests and mocked screen renders do not prove the real Windows Claude/Codex flow.
