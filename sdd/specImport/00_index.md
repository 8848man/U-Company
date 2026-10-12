# External SDD import topic index

외부 명세를 앱에 가져오는 단계와 프로젝트 폴더의 휴대 가능한 원본 파일을 구분한다. [보안](../security.md)은 외부 텍스트의 신뢰 경계, [도메인](../domain/00_index.md)은 적용된 프로필의 프로젝트 소속을 소유한다.

| ID | 원본 | 소유하는 사실 |
| --- | --- | --- |
| SI-01 | [소스·초안·로컬 프로필](SI-01-import.md) | JSON/Markdown 입력, 예외 보존, 미리보기, 바인딩, SQLite 적용·복구 |
| SI-02 | [프로젝트 공통 파일](SI-02-portable.md) | `.ucompany/spec.json`의 원본성, 생성·변경 감지·단방향 동기화 |
| SI-03 | [SDD 구조 감지와 부트스트랩](SI-03-origin.md) | 기존 작업 규칙의 정적 감지, 확인으로만 허용되는 구조 만들기·절차 추가·백업 후 바꾸기, `.ucompany/sdd/` 구조와 `AGENTS.md` 관리 구간 |

관계 `previews`는 원본을 검토 가능한 초안으로 읽는 것이고, `projects_to`는 검토한 파일을 로컬 읽기 모델에 적용하는 것이다. `constrains`는 보안·프로젝트 경계의 제한을 뜻한다. `references`는 같은 폴더나 파일을 읽지만 규칙을 물려받지 않는다는 뜻이다. SI-01의 임의 업로드 초안은 SI-02의 기존 파일을 덮거나 바로 적용하지 않는다. 새 파일을 만든 뒤 다시 파일을 미리봐야 한다. SI-03의 감지는 SI-01 미리보기를 제안할 뿐 적용하지 않고, SI-02의 `spec.json`을 만들거나 바꾸지 않는다. [수용 기준](acceptance.md)이 세 경로의 차이를 검사한다. [사용자 여정](../userFlow/UF-07-spec-import.md)은 실제 화면 순서다.
