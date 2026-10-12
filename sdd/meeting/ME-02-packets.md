# ME-02 — Meeting plan approval, packets and transmission records

## Plan approval

A `MeetingPlanApproval` binds one meeting to an exact plan revision and digest, the mode (`plan` or `per_stage`), a status (`approved`, `revoked`, `expired`, `rejected`) and its decision time. The plan digest covers the plan fields (member IDs and stances, selected source paths, protocol, round and call cap). Frozen source hashes and destination fingerprints are stored separately in the meeting and packets and checked at their dispatch gates. The approval command must present the revision and digest the CEO saw; it is refused when the plan changed, still has errors, or a member became busy or ineligible. Restart expires an approval; revocation ends it under the same store transaction that reserves dispatch.

## Exact packet

The engine assembles each provider payload as immutable bytes with a stable packet ID, stage, member, plan digest, source snapshot IDs, byte length, destination connection fingerprint and SHA-256, and keeps each payload, measured as the quoted Windows command-line argument the CLI receives, at or below 30,000 UTF-16 units (Windows refuses command lines over 32,767; the rest is reserved for the executable path and fixed arguments). Only the proposing packet carries source text; its real length and conservative upper bounds for every later stage are checked at plan validation. Any oversized packet blocks before a call and is never truncated. The `planning` packet to the chair is recorded with approval mode `start` because the CEO's `회의 시작` click authorized it. Packets of later stages are derived only from an approved plan: in `plan` mode their transmission approval is recorded automatically with mode `plan` and the plan digest; in `per_stage` mode the CEO decides each packet (approve or deny) and a stage card may show several packets with independent choices.

Before a proposal call the engine checks the current source files unless the CEO chose the approved frozen snapshot after a change; later stages use the frozen snapshot and show a change notice. Before each call it checks the member's current connection fingerprint. Then one store transaction reserves the call: the meeting stage equals the packet stage, the plan approval is `approved` with the packet's plan digest, the packet is approved under the current template and dispatch contract with matching bytes, digest, revision and destination fingerprint, the destination's read-only profile matches the one recorded at approval, and the call cap is not exceeded. Stop, revocation and restart use the same lock; whichever commits first decides whether the call started. A started call cannot be recalled. A failed call may be retried only by an explicit CEO command within the approved plan and call cap. An ambiguous start becomes `outcome_unknown` and requires a separate duplicate-send confirmation for manual retry; it is never re-sent automatically. A denied packet is never sent or fabricated as a contribution.

Each attempt records start/finish time, duration, command-line length, member and connection name, model, fingerprint, read-only profile, outcome, and reported or estimated usage. Usage is attributed to the real member ID with the meeting ID. On start the engine expires packets and approvals of older templates, dispatch contracts or meeting models.

## Relations

## Approved delivery and recovery refinement (MAM-014)

Prompt transport is a provider-specific `argument` or `stdin` profile. Argument delivery remains the default. Claude and Codex may switch only after separate synthetic Windows provider-harness checks; the common provider entry point means execution goals use the same verified choice. Argument packets use quoted whole-command length and verified stdin packets use the 30,000-unit packet budget. Every stage's worst-case size must fit at plan approval. The engine never shortens content to fit.

A format-repair packet identifies its invalid predecessor, stage/member and bounded violation list. It consumes a planned spare call and does not reinclude the raw answer. `retry_meeting_packet` creates a fresh revision for one failed or invalid member/stage under a still-approved plan and available cap. An uncertain predecessor also requires the CEO's duplicate-send acknowledgement. Stop and revocation retain transactional precedence over new reservations.

Source snapshots, rather than current source files, supply all debate packets. A source change detected before proposal dispatch pauses for CEO choice; after proposals start it only produces a notice. Connection fingerprint version 2 covers kind, executable, endpoint, arguments and model, excluding transient capabilities/health. Earlier saved slots may be checked against the old algorithm, without changing the approved destination. Normalized CRLF/LF and consecutive whitespace may be used to verify a quotation; reported offsets map back to original source bytes.

| Relation | Target | Meaning |
| --- | --- | --- |
| `requires` | [ME-01 stages](ME-01-process.md) | Packet stage follows the approved plan |
| `records` | [AI-04 usage](../aiIntegration/AI-04-telemetry.md) | Every attempted call has reported or estimated usage provenance |
| `constrained_by` | [Security](../security.md) | CEO approval and actual scope govern dispatch |
