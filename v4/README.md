# Utopia Company v4

v4 reuses the v3 controller, its five menus, and the existing Tauri engine. The typed `techSettings` and `planCard` slots in `v3/src/App.tsx` add the application preset UI without copying commands or approval handling. Open **설정 > 앱 기술·구조** from the main navigation's **설정** menu; this tab sits beside **프로젝트** and is also opened by **설정 > 앱 기술·구조 열기** on a goal's plan card. The preset screen appears only in this dedicated tab. v4 invokes the engine's project scoped preset and immutable plan snapshot commands.

## Commands

All commands run from the repository root after `npm install` (or `npm ci`). v4 is the default app ([ADR-0069](../sdd/architecture/decisions/ADR-0069.md)): `src-tauri/tauri.conf.json` serves `v4/dist`, so `npm run tauri dev`, `npm run dev` and `npm run build` all use v4. On Windows use `v4/start-windows.ps1` (or `scripts/start-windows.ps1`, which forwards to it); on Ubuntu `bash v4/start-ubuntu.sh`. `scripts/build-installer-windows.ps1` builds the NSIS installer.

| Command | What it does |
| --- | --- |
| `npm run build:v4` | Type checks `v4/tsconfig.json`, then builds the `v4/index.html` → `v4/src/main.tsx` entry into `v4/dist`. |
| `npm test -- v4/src/PresetSettings.test.tsx` | Runs the focused v4 screen flow tests. |
| `npm run dev:v4` | Serves v4 with Vite at `http://127.0.0.1:1420` (browser only; engine commands need the desktop app). |
| `npm run tauri:v4` | Opens the desktop app in development mode. Tauri runs `npm run dev:v4` and loads `http://127.0.0.1:1420`. |
| `npm run tauri:v4:build` | Runs `npm run build:v4`, then compiles a release desktop binary that loads `../v4/dist` (relative to `src-tauri`). `--no-bundle` skips installers; the binary is in `src-tauri/target/release`. |

Port 1420 is strict, so stop any other `npm run dev`, `dev:v4` or Tauri dev session first. These commands only build and run locally; they do not deploy or publish anything.

## Checking the screen locally

1. `npm run build:v4`, then check that `v4/dist/index.html` and `v4/dist/assets` exist.
2. `npm test -- v4/src/PresetSettings.test.tsx`.
3. `npm run tauri:v4`. Open **설정 > 앱 기술·구조**. With no project, the tab shows `프로젝트를 먼저 선택하세요`, a **프로젝트 선택** button, and the built-in preset outline; no save or AI send is offered. Click **프로젝트 선택** to reach **설정 > 프로젝트**, choose a disposable project there, then click the sibling **앱 기술·구조** tab to return. Check that only that project's choices load. Switch to a second project and check that the earlier preview, draft and send approval are gone. The **프로젝트** tab no longer shows the preset screen.
   The settings tabs and the 초급·중급·고급 tabs take ←/→ (and Home/End) to move; Tab reaches the options and actions in order. In a narrow window the summary stacks in one column and the save button stays at the bottom of the preview.
4. Follow the desktop acceptance path in [the validation spec](../sdd/validation.md) (v4 application preset acceptance). Record the build and test exit codes, the provider, the observed screen states and any file or approval changes in the owning worklog round. If dependencies, the desktop toolchain or a test provider are missing, mark desktop acceptance unverified and name what is missing.

## Settings screen

The screen follows [UX principles](../sdd/ux-principles.md) in three parts:

1. **Summary first:** the project, the saved choice per stage by name (or `선택 안 함`), the current plan's revision, approval state and fixed choices (with `재계획 필요` if they differ), and one **다음 행동** line. A pending AI send approval appears right below it.
2. **Stage choice:** the 초급·중급·고급 tabs, `선택 안 함` plus three built-in presets (the first marked 권장 시작점), inactive user and AI drafts, deterministic validation questions, and **선택 전 차이·영향 보기**. Only a preview without 충돌 or 질문 필요 enables an explicit **이 프로젝트의 선택으로 저장**.
3. **Folded details:** **직접 프리셋 만들기 (AI 또는 양식)** and **규격과 근거 자세히** (IDs, versions, decisions, constraints and validation).

Choosing an option, switching a stage or settings tab, or opening the screen from a plan card saves nothing and sends nothing. The panel is keyed by project ID, and loaded state, previews, plans and AI requests from another project are discarded. The plan snapshot is never edited here; a changed selection shows `재계획 필요` and applies only through a new plan and approval. Numeric targets stay `미검증` until measured.

The goal's plan card (**이 계획에 고정된 기술·구조 결정**) has a **설정 > 앱 기술·구조 열기** button that opens this tab. The card's fixed values, fingerprint and pending approval stay as they were.

AI drafts are a separate two-step flow; they don't use `ask_goal` or the goal conversation:

1. After the CEO writes a requirement, **보낼 내용과 목적지 확인** calls `preview_tech_preset_ai_draft` for the enabled `cto` agent (or the first enabled agent). Nothing is sent at this step. The screen shows the exact payload and the destination: agent, connection, kind, model, and executable or address.
2. **이 내용과 목적지로 전송 승인** calls `approve_tech_preset_ai_draft` with the payload and destination hashes, and the engine sends one request only if both still match. **전송 거절** calls `refuse_tech_preset_ai_draft`. Editing the requirement or switching projects discards the preview.
3. While the request is sending, the screen polls `get_tech_presets`. The result arrives as an inactive AI draft that has to be previewed and saved like any other draft.

**직접 초안 양식 만들기** fills a common schema JSON template with a `U-<STAGE>-<time>` ID without calling a model. **초안 검사·보관** saves it through `save_tech_preset_draft` as an inactive user draft.
