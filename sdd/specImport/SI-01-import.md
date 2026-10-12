# SI-01 — External source, draft, and local profile

## Purpose and scope

Users may bring existing SDD, agent role files, and harness instructions into a registered project without modifying that project's files or an installed harness. The app offers a documented `ai-org-spec/v1` JSON manifest and a best-effort Markdown importer. Import creates a reviewable draft first. Applying it creates a versioned, project-scoped profile; it never changes an existing goal, approval, connection, local executable, or workspace-wide agent record.

## Import contract

Inputs are 1–20 UTF-8 text files or pasted sources, no more than 512 KiB together. File names are labels, not paths the backend opens. The JSON manifest contains `schemaVersion: 1`, optional `project.summary`, `agents` (`key`, `role`, `instructions`), `workflow` (`title`, `description`), and `exceptions` (`title`, `content`, `scope`). `AGENTS.md`, `CLAUDE.md`, and arbitrary Markdown headings that cannot be safely mapped are preserved as source-attributed exception specifications. Markdown recognizes explicit project, role/agent, and workflow headings; ambiguous text remains an exception. The parser makes no AI call, does not execute instructions, and never silently discards unmapped sections. A preview shows mapped items, exceptions, warnings, source names, current version, and the target project.

The importer rejects a bounded set of obvious API tokens, bearer headers, and private keys before staging; this is a safety check, not a complete secret detector. The UI asks users to remove secrets from the source copy before import.

An agent specification is a placeholder until bound to an existing active team character. Import does not create an AI connection or infer one from document prose. A workflow specification is structured reference guidance; it does not alter the engine's fixed stage order, file ownership validation, sandbox, or CEO approval policy. Exception specifications are labeled lower-trust reference material and cannot grant approval or broaden project scope. Existing harness files stay in place and remain independently usable. The first import slice stores and displays these specifications locally; sending their contents to an AI provider requires a separate review of the exact text and destination.

## Apply and recovery

The backend stores a draft and applies it in one SQLite transaction after validating the project ID, bindings, base revision, and current project file. A concurrent profile or file change requires a new preview. Each applied version is retained as an operational snapshot for inspection and recovery, not as an editable SDD original. Source names and bounded text stay inside local SQLite; event messages contain counts only. The active profile is included in the snapshot. Existing goals, prompt composition, and in-flight processes do not change in this slice.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| previews | [SI-02 공통 파일](SI-02-portable.md) | 적용 전 현재 프로젝트 파일을 다시 읽는다. |
| projects_to | [DM-01 프로젝트 프로필](../domain/DM-01-workspace-team.md) | 검토한 파일만 프로젝트별 로컬 읽기 모델에 적용한다. |
| constrains | [보안](../security.md) | 외부 문서의 지침을 실행하지 않는다. |

## 검증 참조

[주제 수용 기준](acceptance.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../WORKLOG.md)를 따른다.
