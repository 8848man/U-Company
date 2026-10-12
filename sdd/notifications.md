# Attention notifications

The v2 app may signal only two saved, actionable transitions: a new pending CEO approval and a goal newly entering `blocked` or `failed` from a nonstopped state. The saved `Approval` and `Goal` records in the snapshot are the sole trigger source. Runtime text, a model reply, a status toast, or a background event alone cannot grant an approval or create a notification. A notification opens no project files and carries no command.

On first successful snapshot after app launch, record pending approval IDs and goal statuses without sound or OS notification. Thereafter notify once when an approval ID first becomes `pending`, or a goal changes from a nonstopped status to `blocked` or `failed`. A refresh of the same state does not repeat the signal. The existing goal workspace and open-work launcher remain the place to inspect the exact reason, approval scope, and recovery action. System notification title/body are brief and avoid the full approval reason, file paths, model output, or secrets. An operating-system notification does not approve work by itself.

V2 Settings > Notifications owns separate `Windows notifications` and `Sound` switches. Both start off and persist as local UI preferences. Enabling OS delivery requests notification permission only after that click. If permission is denied, the switch remains off and an actionable explanation is shown; a later delivery failure is likewise explained without interrupting app work. Sound is a short app-generated cue with separate patterns for approval and blocked work; no repeated sound loops. The sound switch governs the app-generated cue. Windows may also play its own notification sound according to OS settings, independently of the app cue. A test action sends only a labeled sample and never changes a goal or approval.

The app retains its in-app urgent state and open-work navigation even if sound, the OS, or the notification plugin fails. The plugin runs only through a narrow frontend adapter; the backend approval boundary is unchanged. Windows native delivery requires an installed app identity, so the v2 installer flow must be provided and tested separately from the development launcher. Ubuntu desktop delivery remains a separate platform gate.

## Verification

- New approval and blocked transitions each signal once while enabled; repeated refresh, unrelated events, startup backlog, and restored/restarted state stay quiet.
- Each switch persists across restart. OS permission is requested only on user enable. Denial is explained and does not stop in-app work.
- Opening the goal from the app still presents the saved scope and reason. A notification cannot approve, deny, resume, or alter a project.
- Test actual Windows sound and installed-app notification in a disposable data set; repeat the core state flow on Ubuntu before declaring cross-platform support.
