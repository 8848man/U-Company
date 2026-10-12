/** 32: goal archive and backup purge (ADR-0043); 31: display summaries in snapshots (ADR-0042); 30: teammate appearance; 29: goal squads and external staff. */
export const MIN_MEETING_PROTOCOL = 32;

export function meetingBackendOutdated(protocolVersion: number | undefined): boolean {
  return protocolVersion === undefined || protocolVersion < MIN_MEETING_PROTOCOL;
}
