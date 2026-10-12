# External SDD import topic index

외부 명세를 앱에 가져오는 단계와 프로젝트 폴더의 휴대 가능한 원본 파일을 구분한다. [보안](../security.md)은 외부 텍스트의 신뢰 경계, [도메인](../domain/00_index.md)은 적용된 프로필의 프로젝트 소속을 소유한다.

| ID | 원본 | 소유하는 사실 |
| --- | --- | --- |
| SI-01 | [소스·초안·로컬 프로필](SI-01-import.md) | JSON/Markdown 입력, 예외 보존, 미리보기, 바인딩, SQLite 적용·복구 |
| SI-02 | [프로젝트 공통 파일](SI-02-portable.md) | `.ucompany/spec.json`의 원본성, 생성·변경 감지·단방향 동기화 |

관계 `previews`는 원본을 검토 가능한 초안으로 읽는 것이고, `projects_to`는 검토한 파일을 로컬 읽기 모델에 적용하는 것이다. `constrains`는 보안·프로젝트 경계의 제한을 뜻한다. SI-01의 임의 업로드 초안은 SI-02의 기존 파일을 덮거나 바로 적용하지 않는다. 새 파일을 만든 뒤 다시 파일을 미리봐야 한다. [수용 기준](acceptance.md)이 두 경로의 차이를 검사한다. [사용자 여정](../userFlow/UF-07-spec-import.md)은 실제 화면 순서다.
