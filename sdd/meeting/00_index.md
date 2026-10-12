# Meeting topic index

This folder owns the intended current contract for a Goal-bound multi-perspective meeting. The [review record](../multiAgentMeeting/00_index.md) explains how the design was reached; it is not an approval. [Security](../security.md) owns transmission and project-write authority, [DM-04](../domain/DM-04-lifecycle.md) owns Goal states, and [AI-03](../aiIntegration/AI-03-adapter.md) owns provider transport. [WORKLOG](../../WORKLOG.md) records what is actually implemented and verified.

| ID | Owner | Facts owned |
| --- | --- | --- |
| ME-01 | [Meeting process](ME-01-process.md) | Office-character roles (chair, participants, evaluator), chair plan and validation, stages, contributions, trace graph, stop/recovery, decision package and handoff |
| ME-02 | [Plan approval and packets](ME-02-packets.md) | Plan approval record and modes, packet bytes, destination fingerprint, reservation checks, dispatch/audit contract |

`belongs_to` binds Meeting to one Goal; `produces` connects a stage to contributions and a decision package; `requires` denotes a prerequisite without authority; `records` preserves a byte-level observation. New entities keep one owner here; global authorization remains in Security.
