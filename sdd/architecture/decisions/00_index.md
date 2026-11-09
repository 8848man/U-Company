# ADR index

Rows are in ID order. Each ADR starts with its title, a `Status:` line, and a `Date: YYYY-MM-DD` line for the day the decision was accepted; amendments are dated inside the body.

| ID | Decision | Status |
| --- | --- | --- |
| [ADR-0001](ADR-0001.md) | Tauri/Rust/SQLite and platform boundary | Accepted |
| [ADR-0002](ADR-0002.md) | Separate model, runtime, and role through adapters | Accepted |
| [ADR-0003](ADR-0003.md) | Parallel Git worktrees and persisted recovery | Accepted |
| [ADR-0004](ADR-0004.md) | Reuse Pixel Agents Canvas only | Accepted |
| [ADR-0005](ADR-0005.md) | Project registry, goal binding, and native folder choice | Accepted |
| [ADR-0006](ADR-0006.md) | Unified folder adoption and initialization | Accepted; supersedes the separate creation flow in ADR-0005 |
| [ADR-0007](ADR-0007.md) | Direct Windows npm agent entry points and Claude auth probe | Accepted |
| [ADR-0008](ADR-0008.md) | Single implementation task for one artifact | Accepted |
| [ADR-0009](ADR-0009.md) | Persist operational progress instead of model reasoning | Accepted |
| [ADR-0010](ADR-0010.md) | Bounded QA repair loop and per-run token ledger | Accepted |
| [ADR-0011](ADR-0011.md) | Snapshot dirty project contents for approved work | Accepted |
| [ADR-0012](ADR-0012.md) | Goal-owned QA caps and invocation-time model identity | Accepted; supersedes ADR-0010's project-scoped cap |
| [ADR-0013](ADR-0013.md) | Reversible teammate hiding and display-only goal groups | Accepted |
| [ADR-0014](ADR-0014.md) | Goal-scoped dialogue and checkpointed steering | Accepted |
| [ADR-0015](ADR-0015.md) | Provider protocol and process transport boundary | Accepted |
| [ADR-0016](ADR-0016.md) | Ordinary folder registration and isolated copy execution | Accepted; supersedes Git-only requirements in ADR-0003/0006 |
| [ADR-0017](ADR-0017.md) | Saved CLI health and separate team settings | Accepted |
| [ADR-0018](ADR-0018.md) | User-initiated provider login and fixed recovery guidance | Accepted |
| [ADR-0019](ADR-0019.md) | Staged project-scoped SDD import | Accepted |
| [ADR-0020](ADR-0020.md) | Portable project specification and local runtime projection | Accepted |
| [ADR-0021](ADR-0021.md) | One-way portable spec synchronization | Accepted; narrows ADR-0020 apply behavior |
| [ADR-0022](ADR-0022.md) | Explorer-style Windows project folder picker | Accepted |
| [ADR-0023](ADR-0023.md) | Recheck project mode on explicit selection | Accepted |
| [ADR-0024](ADR-0024.md) | Persisted-state recovery in one goal workspace | Accepted |
| [ADR-0025](ADR-0025.md) | Open-work launcher reuses goal conversation | Accepted |
| [ADR-0026](ADR-0026.md) | Conversation-first goal workspace | Accepted |
| [ADR-0027](ADR-0027.md) | Verified selective continuation of revised goals | Accepted |
| [ADR-0028](ADR-0028.md) | One conversation draft with explicit question and instruction sends | Accepted |
| [ADR-0029](ADR-0029.md) | AI-routed conversation and automatic safe continuation | Accepted; supersedes ADR-0028 send choice |
| [ADR-0030](ADR-0030.md) | First-class approved Git bootstrap | Accepted; supersedes ADR-0023 external-only bootstrap |
| [ADR-0031](ADR-0031.md) | Linked user-flow graph as an SDD navigation layer | Accepted |
| [ADR-0032](ADR-0032.md) | Topic folders for independently evolving SDD entities | Accepted |
| [ADR-0033](ADR-0033.md) | Separate v2 frontend over the existing organization engine | Accepted |
| [ADR-0034](ADR-0034.md) | Saved-state attention signals with a native notification adapter | Accepted |
| [ADR-0035](ADR-0035.md) | Goal-bound deliberation with separate transmission grants | Accepted; transmission-grant parts superseded by ADR-0036 |
| [ADR-0036](ADR-0036.md) | Office-character meetings with chair-planned, plan-approved read-only debate | Accepted |
| [ADR-0037](ADR-0037.md) | Bounded meeting packets and explicit recovery after plan approval | Accepted; refines ADR-0036 delivery and recovery |
| [ADR-0038](ADR-0038.md) | Deliverable task contract, checked single assignment and per-deliverable review | Accepted; extends ADR-0003/0008/0012 |
| [ADR-0039](ADR-0039.md) | v3 frontend shell over the same engine | Accepted; follows ADR-0033 |
| [ADR-0040](ADR-0040.md) | Office character sprites from a CC0 pack, as an optional visual skin | Accepted; refines ADR-0004 asset rule |
| [ADR-0041](ADR-0041.md) | Goal squads proposed by the planner, and goal-only external staff from character files | Accepted; extends ADR-0038 |
| [ADR-0042](ADR-0042.md) | UI/UX philosophy: three depths, summary first, control always visible | Accepted; refines content-density long-text rule |
| [ADR-0043](ADR-0043.md) | Goal archive (hide) and permanent delete from 설정 › 백업 삭제 | Accepted; refines DM-04 |
| [ADR-0044](ADR-0044.md) | Consent-gated SDD origin: detect, follow, or bootstrap a project structure | Accepted; narrows the harness-entry rule in SI-02/Security, refines ADR-0021 |
| [ADR-0045](ADR-0045.md) | SDD origin in onboarding and planning: folder first, development goals only | Accepted, implemented (protocol 34); refines ADR-0044 |
| [ADR-0046](ADR-0046.md) | One conversation per goal: plan card, revise, cancel, and an in-app Office window | Accepted (v3); refines ADR-0042, ADR-0045 |
| [ADR-0047](ADR-0047.md) | v3 shell polish: icon rail, compact `지금 할 일`, movable open-work button, Office window coordinates and mini chat | Accepted (v3); refines ADR-0046 |
| [ADR-0048](ADR-0048.md) | One-click `해결 요청` for a blocked goal, and one automatic retry for a malformed plan | Accepted; refines ADR-0038, ADR-0046 |
| [ADR-0049](ADR-0049.md) | Reference material (long text, images) as app-managed files with an index, and a curator role | Accepted; refines ADR-0038 |
| [ADR-0050](ADR-0050.md) | The PM maintains the project's U-SDD | Accepted; refines ADR-0044, ADR-0045 |
| [ADR-0051](ADR-0051.md) | Goal versions read as one conversation | Accepted; refines ADR-0048, ADR-0043, ADR-0046 |
| [ADR-0052](ADR-0052.md) | Prove goal completion with a mock AI and close usability gaps in rounds | Accepted; refines ADR-0002, ADR-0048, ADR-0051 |
| [ADR-0053](ADR-0053.md) | A repair that needs new files asks on the same goal; a repair with nothing to change goes back to QA | Accepted; refines ADR-0048, ADR-0051 |
| [ADR-0054](ADR-0054.md) | Show what approval sets in motion, what a finished goal changed, where goals stop, and how to start | Accepted; refines ADR-0046, ADR-0047 |
| [ADR-0055](ADR-0055.md) | Repeated work in one visible goal conversation | Accepted; refines ADR-0051, ADR-0046 |
| [ADR-0056](ADR-0056.md) | Structured context at each linked goal work-pass boundary | Accepted; refines ADR-0055, ADR-0051 |
| [ADR-0057](ADR-0057.md) | Keep file-name case, pass earlier results forward, and give the goal conversation the QA record | Accepted; refines ADR-0048, ADR-0053, ADR-0056 |
| [ADR-0058](ADR-0058.md) | Ask the CEO for command permissions a teammate needs, inside the goal conversation | Accepted; refines ADR-0002, ADR-0053, ADR-0055 |
| [ADR-0059](ADR-0059.md) | A conversation keeps its original objective: resume it with the project's changes, and close it when later work achieved it | Accepted; refines ADR-0048, ADR-0051, ADR-0055, ADR-0056 |
| [ADR-0060](ADR-0060.md) | Stop the QA loop: criteria the team cannot meet go to the CEO instead of another rework | Accepted; refines ADR-0038, ADR-0048, ADR-0053, ADR-0059 |
| [ADR-0061](ADR-0061.md) | `지금 할 일` is the one place for the CEO's next action: original objective, CEO actions and starting Git | Accepted; refines ADR-0047, ADR-0048, ADR-0059, ADR-0060 |
| [ADR-0062](ADR-0062.md) | v4 application technology and structure presets: app-local selections, immutable plan snapshots, approval boundary | Accepted decision; storage/engine and v4 screen partially implemented; exact AI transmission approval, SDD/API owner checks, build and desktop QA unverified; refines ADR-0046 and ADR-0027 |
| [ADR-0063](ADR-0063.md) | Fewer command cards (read-only commands, project-wide safe grants) and an opt-in per-conversation allow-all mode without push or pull requests | Accepted; refines ADR-0058; allow-all mode (decisions 4, 5, 7) withdrawn 2026-11-10 in favour of ADR-0066 |
| [ADR-0064](ADR-0064.md) | Keep a repairable QA criterion and request a same-goal file fix | Implemented for review; actual provider and Windows app QA pending; refines ADR-0060 and ADR-0053 |
| [ADR-0065](ADR-0065.md) | Dev의 거절된 명령을 PM이 검토하고 기존 권한 재시도 또는 CEO 요청을 선택 | Accepted; refines ADR-0058 |
| [ADR-0066](ADR-0066.md) | 프로젝트 기본·목표 대화별 수동/오토/풀 오토 명령 승인 모드 | Accepted; refines ADR-0058, ADR-0065 |
| [ADR-0067](ADR-0067.md) | PM이 필요한 거절 호출만 골라 CEO에게 요청하고 스크립트 본문은 명령 규칙으로 나누지 않음 | Accepted; refines ADR-0058, ADR-0065, ADR-0066 |
| [ADR-0068](ADR-0068.md) | 승인된 재수정 파일의 재사용 소유권 전환 | Accepted specification; engine implementation and plan/rework/QA runtime verification pending; refines ADR-0027 without widening approval |
