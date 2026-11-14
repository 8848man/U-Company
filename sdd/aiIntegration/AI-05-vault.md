# AI-05 — 볼트 MCP 연결

U-Company가 사용자의 Obsidian 볼트에 연결하는 방식이다. 결정 근거는 [ADR-0075](../architecture/decisions/ADR-0075.md), 권한 경계는 [보안](../security.md)이 소유한다. 상태: 명세만 있고 구현 전이다(볼트 과제 U2).

## 연결

- 앱 전역에 볼트 연결 하나를 둔다. 실행 파일과 인자 목록은 CEO가 입력한다. 처음 실행하기 전에 실행 파일·인자·작업 폴더를 보여 주고, 앱 사용자 권한으로 실행되는 코드라는 경고와 함께 CEO의 확인을 받는다.
- 셸 없이 고정 인자로 실행한다. 작업 폴더는 앱 데이터 폴더이고, `VAULT_MCP_`로 시작하는 환경 변수는 넘기지 않는다.
- 전송은 stdio 줄 단위 JSON-RPC 2.0이다. 앱 동작 한 번마다 서버 프로세스를 하나 띄우고 끝나면 닫는다. 제한 시간이 지나면 프로세스 트리를 종료한다.
- `initialize` 응답의 서버 이름이 `obsidian-vault`이고 계약 MAJOR가 1이며, `tools/list`에 `vault_list_projects`, `vault_list_tasks`, `vault_submit_summary`, `vault_request_ingest`, `vault_get_job`이 있어야 기능을 켠다. 모르는 출력 필드와 경고 코드는 무시한다.

## 프로젝트 대응

프로젝트마다 볼트 슬러그를 하나 고른다. 후보는 `vault_list_projects`에서 오고, 폴더 이름과 맞는 슬러그를 먼저 제안한다. 대응은 앱 SQLite에만 저장한다. `PROJECT_NOT_FOUND`면 볼트 프로젝트를 만들지 않고 다시 고르게 한다.

## 과제 가져오기

| 단계 | 동작 |
| --- | --- |
| CEO가 목표 입력 화면에서 `볼트 과제에서 가져오기`를 누름 | `vault_list_tasks(project=<슬러그>, agent="ucompany")` 한 번 |
| 목록 표시 | 과제 문장, 출처 날짜, 담당(`#ucompany` 또는 `공용`). `procedure`는 접힌 참고문 |
| CEO가 과제 하나를 고름 | 문장을 목표 입력란에 채움. 아직 아무것도 보내지 않음 |
| CEO가 목표를 보냄 | 일반 목표 전송([UF-03](../userFlow/UF-03-goal.md)). 과제 원문·줄 번호·출처 날짜를 목표의 `vaultTask`로 저장 |

## 요약 제출과 처리 요청

1. 목표가 `completed` 또는 `cancelled`가 되면 `볼트에 기록` 카드를 보여 준다. 막힌 목표는 목표 메뉴에서 연다.
2. 앱이 저장 기록으로 볼트 세션 요약을 만든다. AI를 부르지 않는다. frontmatter: `date`, `source: u-company`, `model`, `project`, `type: dev`, `status`. 섹션: `목표`, `결정 사항`, `산출물`, `남은 과제`, `이슈`(`새로 발견`·`해결됨`), `작업 방식 회고`, 그리고 `vaultTask`가 있으면 `볼트 과제 갱신`.
3. `볼트 과제 갱신`에는 과제 원문과 CEO가 고른 결과(`완료 | 변경 | 불필요 | 사람 몫`)를 넣는다.
4. CEO는 제출될 전문을 보고 고칠 수 있다. `볼트에 제출`을 누르면 `vault_submit_summary`를 부른다. `SENSITIVE_CONTENT`는 줄 번호를 보여 주고 다시 고치게 한다. `project_match`가 `none`·`ambiguous`면 그대로 알린다.
5. 제출 뒤 `사서 처리 요청`을 누르면 `vault_request_ingest`를 부르고, 카드가 보이는 동안 5초 간격으로 최대 10분 `vault_get_job`을 부른다. `needs_review` 사유는 그대로 보여 준다. 앱은 `retry: true`를 보내지 않는다.
6. 제출 파일명·SHA-256·작업 ID·마지막 상태를 목표별로 저장한다. 재시작 뒤 자동으로 다시 제출하거나 요청하지 않는다.

## 실패

볼트 연결 실패, 계약 불일치, 도구 오류는 볼트 카드 안에서만 보여 준다. 목표 생성·실행·완료는 막지 않는다. 오류 문구는 표시용 자료다.
