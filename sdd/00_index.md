# AI Organization OS — SDD index

This index is the entry point for a new contributor or AI agent. Read the affected code before assuming a spec describes the current implementation. Specs state intended current behavior; ADRs preserve why a significant choice was made.

| Concern | Canonical document |
| --- | --- |
| **UI/UX philosophy every screen follows** (three depths, summary first, control always visible, plain words) | [UI/UX 구현 철학](ux-principles.md) |
| Product goals and MVP capabilities | [Product](product.md) |
| Proposed v3 cross-functional customer goal and pilot | [v3 improvement plan](v3-improvement-plan.md) |
| Proposed v2 multi-perspective AI meeting: cross-review record (proposal, critiques, responses, decisions) | [Multi-agent meeting review folder](multiAgentMeeting/00_index.md) |
| Goal-bound multi-perspective meeting: intended process and packets | [Meeting topic](meeting/00_index.md) |
| SDD authorship, ownership, and topic-folder criterion | [Authoring principle](authoring.md) |
| User journey graph and flow navigation | [User flow index](userFlow/00_index.md) |
| Proposed whole-app UX simplification | [UX simplification plan](uxSimplification/00_index.md) |
| Organization, tasks, lifecycle | [Domain](domain/00_index.md) |
| Work progress UI and observable evidence | [일의 진행 UI 표시 정책](progress-visibility.md) |
| User attention sounds and OS notifications | [Attention notifications](notifications.md) |
| Repeated lists and long content | [정보 밀도 UI 정책](content-density.md) |
| Goal conversation and CEO steering | [Goal conversation](goalConversation/00_index.md) |
| Goal-centered improvement delivery | [Workflow improvement plan](workflow-improvement-plan.md) |
| Planned Office current-goal layout | [Office goal visibility plan](office-goal-visibility-plan.md) |
| AI discovery and adapter contract | [Integrations](aiIntegration/00_index.md) |
| External SDD, team, workflow, and exception import; SDD origin detection and consent-gated structure bootstrap | [External SDD import](specImport/00_index.md) |
| Provider/runtime failure investigation | [Investigation note](provider-runtime-investigation.md) |
| Persistence and operating-system boundary | [Architecture](architecture.md) |
| Privileges, approvals, trust boundaries | [Security](security.md) |
| Acceptance and regression checks | [Validation](validation.md) |
| Implementation lifecycle | [Workflow](workflow.md) |
| Branches, pull request titles and summaries | [PR workflow](pr-workflow.md) |
| Architecture rationale | [ADR index](architecture/decisions/00_index.md) |
| Current implementation progress | [Worklog](../WORKLOG.md) |
| Source and verification map | [Implementation map](implementation-map.md) |

Only the canonical document defines each fact. Other documents link to it. Add a document when a real new ownership area emerges; update this index at the same time.

Specs describe the target behavior. `WORKLOG.md` records which acceptance criteria are implemented and actually verified; an unchecked item is not a delivered capability.
