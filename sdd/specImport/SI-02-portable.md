# SI-02 — Portable project specification file

## Portable project specification

The tool-neutral, user-editable project contract lives at `<project>/.ucompany/spec.json` using the same `ai-org-spec/v1` manifest. It contains project summary, role keys/instructions, workflow reference steps, and exception specifications. It excludes app-local project IDs, bound character IDs, revisions, connections, credentials, sessions, approvals, goals, usage, and runtime state. `AGENTS.md` and `CLAUDE.md` may point to this file, but the app never rewrites either harness entry point. Codex, Claude, and other tools can edit the same manifest with normal project file tools.

The project file is authoritative for portable definitions. SQLite provides built-in role presets, an explicitly applied read model and revision history, and operational state; it is not an SDD editor. The app reports whether the file is absent, equal to the applied projection, changed, or invalid. It never silently imports a changed file or overwrites it from a stale DB profile. A new import draft can initialize the project file after confirmation, then must be previewed from that file before application. A preexisting DB profile can also initialize the file as a migration path. The user can preview the current file through the existing bounded parser and apply it as a new local revision. Existing files are never replaced by an initialization action. File creation rejects symlinked paths and stays under the canonical selected project root. A file/DB operation is not presented as atomic across both stores; an interrupted initialization can be recovered by refreshing status and previewing the file.

Synchronization is one-way: project file → reviewed draft → applied SQLite projection. A draft from pasted or uploaded sources may be inspected and used to create an absent file, but cannot directly replace the applied profile. If the file already exists, the user must merge those edits into it with a normal editor and preview that file again. At apply time the backend rereads the file and rejects a draft whose mapped content no longer matches it, so an edit made after preview cannot silently become the applied version. Semantic comparison ignores JSON whitespace, key order, local bindings, and source labels; a changed mapped field requires a new preview. Local character bindings are stored only in SQLite.

This contract does not currently synchronize an existing collection of Markdown SDD files with the JSON manifest. If both contain the same requirement, they are two originals and can conflict. A future document-reference schema must designate the owning file for each concept, keep the manifest as an index or derived projection for referenced content, and report conflicts rather than copying or merging prose automatically. Until that contract exists, teams should choose one canonical location for each concept and link to it from the other tool entry points.

Tool-specific entry points can link to the shared contract without copying its content. For example, a project may add `Read .ucompany/spec.json for shared project, role, and workflow reference; follow this tool's approval and execution rules` to its existing `AGENTS.md` and `CLAUDE.md`. The app displays the path but leaves those edits to the project owner.

This contract is descriptive. Importing or editing it does not alter the fixed scheduler, approve a write, or send its contents to a provider. That separate provider handoff still requires review of exact content and destination. External tools may follow their own harness policies; the app does not claim to control them.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| previews | [SI-01 검토 초안](SI-01-import.md) | 파일을 다시 미리본 뒤 로컬에 적용한다. |
| constrains | [보안](../security.md) | 기존 파일 덮어쓰기와 무검토 적용을 막는다. |

## 검증 참조

[주제 수용 기준](acceptance.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../WORKLOG.md)를 따른다.
