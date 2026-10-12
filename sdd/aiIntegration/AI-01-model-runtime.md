# AI-01 — Model and runtime identity

## Separate model and runtime

A **model** is an inference identity with provider, model ID, availability, and verified capabilities. A **runtime** runs agent sessions and may support tools, file access, approval, streaming, cancellation, and resumption. Role assignment stores both references. A discovered model without a suitable runtime may be used for text-only roles; code editing requires an agent runtime or a connected existing harness.

## 관계와 소유 경계

| 관계 | 대상 | 의미 |
| --- | --- | --- |
| selects | [AI-02 연결 상태](AI-02-discovery.md) | 사용 가능한 모델·런타임을 능력과 상태로 배정한다. |
| invokes | [AI-03 어댑터](AI-03-adapter.md) | 실제 실행은 선택한 런타임 계약을 따른다. |

## 검증 참조

[검증 기준](../validation.md), [수동 QA](../../qa/MANUAL-QA.md). 구현 증거와 미완료 항목은 [WORKLOG](../../WORKLOG.md)를 따른다.
