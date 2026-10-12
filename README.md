# AI Organization OS

Local desktop prototype for a CEO directed, role based AI team. The organization state lives in SQLite; the UI shows goals, task delegation, approvals, connections, and a Canvas office.

## Development

Requirements: Node.js 22, Rust 1.89 or later, Git, and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for the operating system. Windows needs WebView2 and Visual Studio C++ Build Tools. Ubuntu 24.04 needs the Tauri WebKit/GTK development packages. Ubuntu execution has not yet been verified.

On Windows, use one PowerShell window to check prerequisites and start both the Vite server and desktop app. The script uses Node directly, so it also works when the local `npm.cmd` shim is broken. It does not install software. Close the app window to stop the server it started.

```powershell
cd D:\projects\utopia_company
& .\scripts\start-windows.ps1 -CheckOnly
& .\scripts\start-windows.ps1
```

For a separate, persistent manual QA profile, pass `-DataDir 'D:\projects\utopia_company\.run-data\manual-qa-app'`. Follow the [interactive QA runbook](qa/MANUAL-QA.md) for a dedicated Git fixture, expected screens, approval and restart checks, and result recording.

```sh
npm ci
npm run build
cd src-tauri
cargo test --locked
cargo run --locked
```

Set `UTOPIA_DATA_DIR` to a writable directory to use an isolated SQLite database for testing. Without it, the app uses its Tauri app data directory.

On Ubuntu 24.04 x86_64, after the prerequisites are installed, run `bash scripts/verify-ubuntu.sh`. It builds and tests the app without installing system packages. Then launch `cargo run --locked` from `src-tauri` and verify the GUI and goal flow.

In `프로젝트`, choose one folder. An existing Git root is registered unchanged; an empty folder is initialized as a new project. The workspace can hold multiple projects; each CEO goal stays bound to the project where it was created. In `AI 연결`, actual local discoveries are separate from missing tool presets. Registering an available AI assigns it to team members who have no connection; existing assignments stay intact. `다른 AI 연결` opens a dialog, and HTTP model IDs are selected from the server's `/models` response. Office shows draggable pixel characters and role instruction presets. Submit and start a goal from `CEO 데스크`. Missing setup opens a reason dialog, while completed actions appear as short-lived toasts. The team plans tasks, requests project write approval, runs independent implementation tasks in separate worktrees, integrates the changes, and asks QA for a result. A plain HTTP model can serve text roles but cannot edit files.

**Current limits:** a Windows backend acceptance fixture completed two concurrent Codex CLI sessions, file integration, three Node tests, and a read-only Codex QA review. The complete Tauri approval-to-report flow still needs app-level verification. The external process adapter supports protocol version 1 `initialize` and `send_task`, but full event streaming, cancel, and native session resume remain open. Restart recovery can resume an approved plan only before any task starts. See [WORKLOG.md](WORKLOG.md) for current evidence and next steps.

Specs, ADRs, and contributor entry points begin at [sdd/00_index.md](sdd/00_index.md). Pixel Agents attribution is in [third_party/PIXEL_AGENTS_LICENSE.txt](third_party/PIXEL_AGENTS_LICENSE.txt).
