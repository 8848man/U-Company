# Utopia Company v2

v2 applies the five-destination UX in [UX-01](../sdd/uxSimplification/UX-01-navigation.md) and [UX-02](../sdd/uxSimplification/UX-02-surfaces.md). It shares the existing Tauri/Rust engine, SQLite data, security policy, and reusable React components. The original UI and launcher remain available for comparison.

## Start

- Windows PowerShell: `D:\projects\utopia_company\v2\start-windows.ps1`
- Prerequisites only: `D:\projects\utopia_company\v2\start-windows.ps1 -CheckOnly`
- Separate test data: `D:\projects\utopia_company\v2\start-windows.ps1 -DataDir .run-data\v2-qa`
- Meeting QA-38 (isolated `.run-data\meeting-qa` data + debug-only offline meeting fixture): double-click `v2\start-meeting-qa.cmd`, or run `D:\projects\utopia_company\v2\start-meeting-qa.ps1`. Add `-CheckOnly` to check prerequisites only.
- Ubuntu 24.04: `bash v2/start-ubuntu.sh` (system libraries from the root project setup are required; desktop run is not yet verified).
- Windows installer build for native notifications: `D:\projects\utopia_company\v2\build-installer-windows.ps1` (NSIS output under `src-tauri\target\release\bundle\nsis`).

Only one launcher can use Vite port 1420. Close the original app and its launcher before starting v2. The Windows launcher uses the existing data unless `-DataDir` is provided. Project files are changed only by the existing approved engine actions.

The Windows installer uses the normal app data location when launched from its shortcut. For notification QA without mixing existing data, open PowerShell, set `$env:UTOPIA_DATA_DIR = 'D:\projects\utopia_company\.run-data\v2-notification-qa'`, then launch the **installed** `ai-organization-os.exe` from its actual installation folder in that same PowerShell session. Close the app before clearing that session variable. Keep the installed-app test separate from the development launcher.

## Source layout

- `src/App.tsx`: v2 state/command controller and retained approval/goal dialogs.
- `src/V2Navigation.tsx`: five-menu shell and global project switch.
- `src/V2Home.tsx`: compact CEO goal and decision surface.
- `src/V2Work.tsx`: goal-centered progress and selected-goal details.
- `src/V2Projects.tsx`: project registration and on-demand detail.
- `src/V2Connections.tsx`: saved connections first, discovery on demand.
- `src/V2NotificationSettings.tsx`: separate optional system notification and app sound controls.
- Existing `../src/` components: Office Canvas/member panel, team settings, goal conversation, token analytics, folder picker, SDD import, density controls.

V2 intentionally reuses the backend contract. `App.tsx` still contains a copy of the original controller/dialog wiring. This is migration debt: the next structural slice should extract shared commands and dialogs into a typed hook/module, then have both frontends import it. The v2 screen components already use typed props and callbacks so they can be replaced independently.

## Interactive QA

1. Use `-DataDir .run-data\v2-qa` for a disposable data set. Confirm the sidebar shows 홈, 업무, Office, 토큰, 설정, and the project switch opens registered projects plus 폴더 추가.
2. In 설정 > 프로젝트, register an existing nonempty folder. Switch projects from the sidebar and confirm goals follow the selected project. Check the QA default control and its confirmation.
3. In 설정 > AI 연결, confirm saved connections appear before discovery. Open AI 추가, rescan, register a real CLI, inspect login/status actions, and close the dialog without registering. In 설정 > 팀, check bulk assignment and hidden-member management.
4. From 홈, submit a goal, open it, start team execution, and confirm a blocked start explains why. Confirm a pending approval is shown near the top and opens the exact scope dialog. Reject or defer it without losing the goal.
5. In 업무, compare goal graphs, select one goal, expand only its tasks and real events, change that goal's QA limit with confirmation, and open its conversation. Verify the goal remains selected after closing the conversation.
6. In Office, select a character, inspect individual activity/settings, drag it, and use 팀 전체 설정. In 토큰, compare project and goal usage. Switch projects and return through 열린 작업.
7. Restart v2 with the same test data and confirm projects, goals, team settings, approvals, and token records persist. Test a narrow window and keyboard focus. Repeat core flows on Ubuntu before marking cross-platform UI complete.
8. In 설정 > 알림, enable sound and use its test button. Enable system notifications and test from an **installed** Windows build. Create a new approval and a blocked goal, then repeat refresh/restart: each new transition should alert once, startup backlog should remain quiet, and notification delivery must never approve or resume work. Deny OS permission and verify the explanation. Windows may play an OS-configured notification sound independently of the app sound switch.

Build: `npx tsc --noEmit -p v2/tsconfig.json` and `npx vite build --config v2/vite.config.ts` from the repository root. These checks do not replace desktop QA.
