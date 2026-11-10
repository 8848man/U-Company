# GC-04 — Blocked-goal recovery

## Blocked-goal recovery

`no_changes` 복구 카드([ADR-0072](../architecture/decisions/ADR-0072.md))는 변경할 파일이 없었다는 저장된 사실, 원본 파일이 실제로 바뀌었는지 확인 가능한 통합 기록, 권장 다음 행동을 쉬운 한국어로 보여 준다. `충족 처리`는 엔진이 선언 파일에서 근거를 다시 확인한 경우에만 해당 작업을 완료 상태로 옮기고 남은 승인된 계획을 이어간다. 독립 QA가 목표의 원래 산출물별 수용 기준을 판정하며, 실패하면 기존 재작업으로 간다. 근거가 없으면 완료 처리를 허용하지 않고 필요한 확인 자료를 표시한다. `재계획`은 저장된 변경 없음 사실을 다음 계획자에게 넘기고 새 목표 버전과 승인 카드로 간다. 같은 목표 계보의 같은 오류 종류·파일 집합 차단이 두 번이면 자동 새 버전 버튼 대신 CEO의 충족 처리 또는 직접 지시를 보여 준다. 영어 내부 오류는 상세 기록에서만 볼 수 있다.

If the selected ordinary-folder goal's conversation is about Git (a blocked Git initialization or first-commit task, records that mention `.git` or commits on a blocked goal, a CEO action or stuck criterion about Git, commits or pushing), `지금 할 일` offers the app's Git bootstrap scope preview; the backend allows it for any execution goal of that project while no goal of the project runs or waits for approval ([ADR-0061](../architecture/decisions/ADR-0061.md)). A model may point to this fixed operation, but its prose, commands, author identity, email, and proposed file list are not execution input. The CEO selects existing files, enters commit author details and message, and approves the exact local operation in the app. The backend verifies the root and file fingerprints again, initializes `main`, stages only those paths, normalizes those paths in the Git index according to attributes, creates one initial commit, and records the result. If the blocked task's declared file set exactly matches the commit and every other task is complete, the engine records file proof and starts independent read-only QA. Only a QA pass completes the goal. Otherwise the app shows the resulting commit and a state-safe next step without rerunning the failed metadata-only agent task.

Every blocked-goal entry from CEO, Office, projects, and the board opens the same goal workspace. Its decision card shows the persisted failure report as evidence, the stopped stage, and a concrete next action. A selected task appears in its own compact context message; the last confirmed event remains in progress details. A generic reason-only dialog is not the recovery destination. Provider failures may offer navigation to AI Connections; navigation never silently logs in, changes a model, or retries execution.

Recovery choices follow persisted state, not wording in an AI report: a pending project-write approval opens its exact approval; a pre-write analysis/planning failure with no tasks or approvals can rerun the same goal after the CEO checks its configuration; a blocked approved plan whose unfinished tasks were never started (approval checkpoint, or a failed pre-call claim such as an unusable connection or a busy teammate) can resume through the engine's guarded command, which re-runs every claim check; a denied approval is recorded and never re-requested automatically; other blocked execution/integration/QA failures offer a CEO-authored direction change and linked new goal version. The last choice explicitly says that it starts a fresh plan and approval for new file work and does not resume an old provider session or silently integrate an isolated worktree. Existing guidance awaiting review takes precedence over another recovery proposal. Unsupported or ambiguous state must never display a misleading one-click resume.

The decision card states, in this order, the saved reason, the last confirmed fact (latest operational event), the current owner (a teammate or the CEO) and the one available next action. The actionable label on goal cards, the workspace's next-action summary and Office's goal rail use the same recovery classification and the same four facts. Failed command responses leave the workspace open with the refreshed saved state and an actionable error. Approval and provider authentication retain their separate explicit user actions under [Security](../security.md).

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| reads | [DM-04 수명주기](../domain/DM-04-lifecycle.md) | 저장된 단계와 업무·승인 상태가 선택을 결정한다. |
| routes_to | [GC-02 지침](GC-02-guidance.md) | 나중 단계의 실패는 새 목표 버전을 검토한다. |
| constrains | [보안](../security.md) | 오류 문구나 AI 답변을 권한으로 해석하지 않는다. |

## 검증 참조

[주제 수용 기준](acceptance.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../worklog/00_index.md)를 따른다.
