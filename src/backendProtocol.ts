/** 43: CEO pause/cancel commands (ADR-0073); earlier versions cover meeting and goal capabilities. */
export const MIN_MEETING_PROTOCOL = 43;

export function meetingBackendOutdated(protocolVersion: number | undefined): boolean {
  return protocolVersion === undefined || protocolVersion < MIN_MEETING_PROTOCOL;
}
