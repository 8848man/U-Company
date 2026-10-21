# ADR index

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
| [ADR-0050](ADR-0050.md) | The PM maintains the project's U-SDD | Accepted; refines ADR-0044, ADR-0045 |
| [ADR-0049](ADR-0049.md) | Reference material (long text, images) as app-managed files with an index, and a curator role | Accepted; refines ADR-0038 |
| [ADR-0048](ADR-0048.md) | One-click `해결 요청` for a blocked goal, and one automatic retry for a malformed plan | Accepted; refines ADR-0038, ADR-0046 |
| [ADR-0047](ADR-0047.md) | v3 shell polish: icon rail, compact `지금 할 일`, movable open-work button, Office window coordinates and mini chat | Accepted (v3); refines ADR-0046 |
