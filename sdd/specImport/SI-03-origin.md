# SI-03 — SDD 구조 감지와 부트스트랩

## 목적과 범위

선택한 프로젝트가 이미 스펙 주도 작업 규칙을 갖고 있으면 그 규칙을 따르고, 없으면 U 컴퍼니 SDD 구조를 프로젝트 파일로 만든다. 어느 AI 툴로 열어도 같은 구조를 찾을 수 있게 하는 것이 목적이다. 결정 이유는 [ADR-0044](../architecture/decisions/ADR-0044.md)가 기록한다.

## 원칙

1. **확인 전에는 바꾸지 않는다.** 감지는 읽기 전용이다. 파일 쓰기는 아래 세 동작뿐이고, 각각 CEO가 미리보기를 보고 누른 확인 하나로만 허용된다.
2. **통제와 발견을 나눈다.** 단계 진행과 승인은 엔진과 [Security](../security.md)가 통제한다. 어느 툴로 들어와도 구조를 찾는 일은 파일 경로(`AGENTS.md` → `.ucompany/sdd/00_index.md` → `.ucompany/sdd/graph.json`)가 맡는다.
3. **앱은 자기 영역만 쓴다.** `.ucompany/` 아래와 `AGENTS.md`의 관리 구간만 쓴다. 관리 구간 밖의 내용은 바꾸지 않는다.
4. **토큰은 정적 분석으로 아낀다.** 분류는 파일 이름·위치·짧은 키워드 목록으로 한다. AI 호출은 없다.
5. **상태는 파일에 둔다.** 진행 중인 기능과 다음 작업은 `.ucompany/sdd/`에 남아 다른 툴이 이어받는다.

## 감지

`get_sdd_origin_status`는 정규화한 프로젝트 루트를 깊이 4까지 읽는다. 심볼릭 링크는 따라가지 않고, `.git`·`node_modules`·`vendor`·`dist`·`build`·`target`·가상환경 폴더는 건너뛴다.

| 신호 | 분류 | 정책 |
| --- | --- | --- |
| `.ucompany/sdd/` | `continue` | P12 |
| `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `.cursorrules`, `.github/copilot-instructions.md`, `.cursor/rules/` 중 스펙 절차 키워드가 있는 것 | `existing` | P2, P4 |
| 문서가 들어 있는 `specs/`, `.specify/`, `openspec/`, `.kiro/specs/` | `existing` | P2 |
| 위 신호가 하위 폴더에만 있음 | `existing`, 범위 = 그 하위 폴더. 루트에는 아무 동작도 제안하지 않음 | P3 |
| 내용은 있지만 스펙 절차 키워드가 없는 지침 파일만 있음 | `ask` | P6, P7 |
| 공백뿐인 지침 파일 | 없는 것으로 봄 | P6 |
| 아무 신호 없음 | `bootstrap` | P10 |

같은 범위에 `AGENTS.md`와 다른 툴 지침이 함께 규칙을 담고 있으면 `AGENTS.md`를 따르고 그 사실을 표시한다(P5). `.ucompany/spec.json`([SI-02](SI-02-portable.md))은 역할·업무 흐름 매니페스트이므로 구조 신호로 보지 않는다. 감지 결과에는 AI가 나중에 읽을 만한 애매한 후보(내용이 짧은 지침, 절차 키워드가 없는 지침, 같은 범위의 여러 지침)도 이유와 함께 담는다. 이 목록을 언제 AI에게 보일지는 온보딩 설계에서 정한다.

## 동작과 쓰기 계약

| 동작 | 가능한 분류 | 하는 일 |
| --- | --- | --- |
| `bootstrap` (구조 잡기) | `bootstrap` | `.ucompany/sdd/` 뼈대, 관리 구간만 있는 `AGENTS.md`, 한 줄짜리 `CLAUDE.md`(`@AGENTS.md`)를 만든다. 비어 있는 기존 진입 파일은 채운다. |
| `add_section` (절차 추가) | `ask` | 기존 `AGENTS.md` 끝에 관리 구간을 붙이고 `.ucompany/sdd/`를 만든다. `CLAUDE.md`가 없거나 비어 있으면 한 줄 연결을 만들고, 내용이 있는데 `AGENTS.md`를 부르지 않으면 그 뒤에 `@AGENTS.md` 한 줄만 담은 관리 구간을 붙인다. 기존 내용은 바이트 그대로 앞에 남는다. |
| `convert` (U 컴퍼니 구조로 바꾸기) | `existing`, `ask` (루트 범위만) | 루트의 모든 지침 파일을 `.ucompany/legacy/<UTC 시각>/`에 먼저 복사한다. 그다음 `AGENTS.md` 원문은 관리 구간 밖에 두고 관리 구간을 더하고, `CLAUDE.md`는 한 줄 연결로 바꾸고, `.ucompany/sdd/`를 만든다. 다른 툴 지침과 외부 SDD 폴더는 백업만 하고 그대로 둔다. 아무것도 지우지 않는다. |

`plan_sdd_origin`은 동작별로 만들 파일과 바꿀 파일, 바뀌는 방식(`create`·`append`·`replace`·`backup`)과 그 대상들의 내용 digest를 돌려준다. `apply_sdd_origin`은 `confirmed = true`와 그 digest가 있을 때만 실행한다. 실행 직전에 계획을 다시 만들어 digest가 다르면 거절한다. 모든 경로는 정규화한 프로젝트 루트 안에 있어야 하고, 쓰는 경로나 그 상위 폴더가 심볼릭 링크이면 거절한다. 512 KiB를 넘는 지침 파일은 내용을 읽지 않고 기존 규칙으로만 표시하며, 그 파일이 걸린 구조 동작은 거절한다. 새 파일은 `create_new`로 만들어 경쟁 중인 다른 쓰기를 덮지 않는다. 이미 `.ucompany/sdd/`가 있으면 세 동작 모두 거절한다(P12).

등록, 감지, 미리보기, 명세 가져오기는 프로젝트 파일을 바꾸지 않는다. 세 동작은 [Security](../security.md)의 "기존 하네스 수정은 CEO 승인 필요"에 해당하는 별도 승인이며, 목표 실행의 프로젝트 쓰기 승인과 섞지 않는다.

## 관리 구간

```text
<!-- U-SDD:START -->
## U 컴퍼니 SDD
이 프로젝트는 U 컴퍼니 SDD를 사용합니다.
1. 작업 전에 `.ucompany/sdd/00_index.md`를 읽으세요. 진행 중인 기능, 다음 작업, 먼저 읽을 파일이 적혀 있습니다.
2. 전부 읽지 말고 `.ucompany/sdd/graph.json`에서 지금 작업과 연결된 문서만 여세요.
3. 단계(스펙 → 계획 → 작업 → 구현)를 넘어가기 전에 사용자에게 확인받으세요.
4. `.ucompany/sdd/`는 PM(계획 리드)이 관리합니다. U 컴퍼니 팀에서 다른 업무를 맡았다면 SDD 파일은 읽기만 하세요. 바꾸면 결과를 합칠 때 거절됩니다.
5. PM은 계획이 승인되면 기능·요구사항·계획·작업 목록을, QA를 통과하면 작업 체크박스·결정·`handoff.md`·`00_index.md`·`graph.json`을 갱신합니다.
이 구간은 U 컴퍼니 앱이 관리합니다. 구간 밖의 내용은 앱이 바꾸지 않습니다.
<!-- U-SDD:END -->
```

3번 문장이 단계 관문(P14)을 앱 밖의 툴에 전달한다. 앱 안에서는 기존 승인 단계가 같은 역할을 한다. 4·5번은 앱 안의 관리 주체를 정한다([ADR-0050](../architecture/decisions/ADR-0050.md)). 예전 문구("작업 후 직접 갱신")로 만든 프로젝트는 PM의 SDD 단계가 돌 때 앱이 이 구간만 새 문구로 바꾼다.

## PM의 SDD 관리 (ADR-0050)

`.ucompany/sdd/`가 있는 프로젝트에서 CTO와 PM은 색인, 헌법, handoff, focus 기능의 다음 작업을 읽고 시작한다. 계획 업무는 SDD 파일을 바꿀 파일로 적을 수 없다. 계획이 승인되면 PM이 격리 작업 폴더에서 `features/<기능>/`(feature·requirements·plan·tasks)과 색인·graph·handoff를 쓰고, QA를 통과하면 작업 체크박스·결정·handoff·색인을 갱신한다. 두 단계 모두 `.ucompany/sdd/` 밖을 바꾸면 아무것도 반영하지 않는다. SDD 단계가 실패해도 목표는 멈추지 않고 대화에 한 줄로 알린다. 헌법이 비어 있으면 PM이 `(초안, CEO 확인 필요)` 표시와 함께 초안을 쓴다.

## 구조 (v0)

```text
AGENTS.md                      관리 구간 → .ucompany/sdd/00_index.md
CLAUDE.md                      @AGENTS.md
.ucompany/
  spec.json                    SI-02 매니페스트 (이 동작이 만들거나 바꾸지 않음)
  sdd/
    00_index.md                ID·관계 어휘·읽는 순서의 원본
    graph.json                 노드(id·종류·파일#앵커)와 관계의 기계 판독 색인
    constitution.md            CON-1 원칙
    handoff.md                 현재 상태와 막힌 점
    features/<기능>/           feature.md, requirements.md(## REQ-###), plan.md, tasks.md(- [ ] T-### … (implements: REQ-###))
    decisions/                 ADR-####.md
  legacy/<UTC 시각>/           convert 전 원문 백업
```

노드 종류는 `constitution`, `feature`, `requirement`, `plan`, `task`, `decision`, `component`이고 관계는 `governed_by`(기능→헌법), `has_requirement`(기능→요구사항), `planned_by`(기능→계획), `implements`(작업→요구사항), `decided_by`(계획→결정), `touches`(작업→코드)다. `00_index.md`가 원본이고 `graph.json`은 파생 색인이다. 둘이 다르면 Markdown 색인이 이긴다. 앱은 산문을 동기화하지 않는다. 부트스트랩 직후에는 헌법 노드 하나만 있고, 기능 노드는 AI 툴이나 사람이 만든다.

## 툴 연결

| 툴 | 상태 | 기본으로 읽는 파일 | 동작이 만드는 연결 |
| --- | --- | --- | --- |
| Claude Code | 실측 대상 | `CLAUDE.md` | `@AGENTS.md` 한 줄 |
| GPT (Codex CLI) | 실측 대상 | `AGENTS.md` | 없음 |
| Gemini CLI, Cursor, GitHub Copilot | placeholder, 미검증 | 미확인 | 만들지 않음 |

## 계획 단계 질문 (ADR-0045)

[ADR-0045](../architecture/decisions/ADR-0045.md)가 미뤄 둔 질문을 정했고, 같은 날 구현했다(백엔드 프로토콜 34). 위 쓰기 계약은 바뀌지 않는다. 계획 카드는 같은 미리보기·digest·확인을 거쳐 같은 세 동작을 부른다.

1. 폴더 먼저: 등록·감지·세 동작은 AI 연결 없이 된다. 튜토리얼은 같은 단계를 다시 보여준다.
2. 개발 목표만: CTO 분석이 목표마다 `development`·`other`를 판단하고, `development`일 때만 구조를 제안한다. 판단이 없거나 불분명하면 `other`로 보고 아무것도 제안하지 않는다.
3. 계획 승인 카드에서 구조 잡기와 부족한 AI 연결을 함께 묻는다. 답은 따로 기록하고, 계획 승인과 구조 쓰기 확인은 서로를 대신하지 않는다.
4. 애매한 후보는 첫 개발 목표의 CTO 분석에서 목록에 있는 파일만 읽고, 판단은 카드에 조언으로만 보인다.
5. 화면 문구의 원본은 구현(`src/sddOrigin.ts`)이다.
6. 확인 창은 덧붙이거나 바꾸는 진입 파일의 전후를 보여준다. "이 폴더는 묻지 않기"는 프로젝트별 앱 설정이고 프로젝트 파일에 남기지 않는다.
7. 구조 v0는 고정한다. 바꿀 때는 D1~D3 실측 근거와 새 ADR이 필요하다.

구현 계약:

- CTO 분석 직전에 정적 감지를 한 번 더 하고, 분석 프롬프트에 `업무:` 줄을 요구한다. 구조를 제안할 수 있고 애매한 후보가 있으면 그 경로와 이유만 넘기고 `구조 조언:` 한 줄을 요구한다. 엔진은 분석 뒤 `goal_origin` 기록(업무 종류, 제안 여부, 감지 분류, 조언, CEO 답)을 남긴다. 다시 실행해도 이미 받은 답은 지우지 않는다.
- 제안 조건(`offer`): 업무가 `development`이고, 범위가 루트이고, 분류가 `continue`가 아니고, 할 수 있는 동작이 있고, 프로젝트가 "묻지 않기"가 아닐 때만.
- 답: `apply_sdd_origin`에 `goalId`를 주면 쓰기 전에 그 질문이 열려 있는지(같은 프로젝트, 제안됨, 개발 목표, 미응답) 확인하고, 성공 후 `applied:<동작>`을 기록한다. `decide_goal_origin`은 `declined` 또는 `muted`를 기록하고 파일은 바꾸지 않는다. `muted`는 그 프로젝트의 "묻지 않기"를 켠다. `set_sdd_origin_mute`는 설정 카드에서 되돌린다. 세 명령 모두 계획 승인 상태를 읽거나 바꾸지 않는다.
- 미리보기: `plan_sdd_origin`의 `append`·`replace` 항목은 전후 내용(`before`, `after`, 각각 6,000자까지 표시용)을 함께 돌려준다. 실제 쓰기는 자르지 않는다.
- 연결 질문은 엔진이 이미 계산하는 스쿼드 부족(`shortfall`)을 그대로 보여주고 AI 연결 화면으로 안내한다. 연결을 저장하는 일은 기존 연결 기록이 남긴다.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| previews | [SI-01 검토 초안](SI-01-import.md) | `existing` 분류에서 찾은 파일을 기존 미리보기로 연다. |
| references | [SI-02 공통 파일](SI-02-portable.md) | 같은 `.ucompany/` 폴더를 쓰지만 `spec.json`은 만들거나 바꾸지 않는다. |
| refined_by | [ADR-0045](../architecture/decisions/ADR-0045.md) | 온보딩 순서, 개발 목표 한정, 계획 카드, 미리보기, v0 고정. |
| asked_in | [UF-05 범위 승인](../userFlow/UF-05-approval.md) | 개발 목표의 계획 승인 카드에서 구조와 AI 연결을 함께 묻는다. |
| constrains | [보안](../security.md) | 확인·digest·경로·심볼릭 링크 검사와 하네스 수정 승인. |

## 검증 참조

[주제 수용 기준](acceptance.md), [Validation](../validation.md)의 SDD 구조 항목, 오프라인 하네스 [`tests/sdd-origin/`](../../tests/sdd-origin/README.md), [QA-43·QA-44](../../qa/MANUAL-QA.md). 정책 번호(P#)와 화면 문구 초안은 하네스 README의 정책 대응표를 따른다.
