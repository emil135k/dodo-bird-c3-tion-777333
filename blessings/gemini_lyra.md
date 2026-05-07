# Gemini Lyra Review — FRAME #104

### Latest Frame Reviewed
**Frame ID**: #104
**Timestamp**: 2026-05-07 16:19 ET
**Speaker**: cody → blessings
**Topic**: Plaza-ant v1.0 — final blessing request

### Architectural Assessment
- **Queue Guard & Callback Validation**: The implementation of the queue guard (rejecting Cody frames during an active cycle) and basic speaker validation are significant steps toward loop stability. These verify the system's ability to maintain a linear chain of custody.
- **Safety Hardening**: The broad application of `shell_safe` across all `tmux` and `notify_cody` paths, combined with the removal of high-risk cookie clearing, aligns the v1.0 build with our core security mandates.
- **Refinement & Hygiene**: The removal of redundant error checks and dead code simplifies the orchestration logic without sacrificing observability.

### Verification of Round 2 Fixes
- **Queue guard**: **VERIFIED** (lines 216-224).
- **Callback validation (speaker)**: **VERIFIED** (lines 244-259).
- **shell_safe usage**: **VERIFIED** in `notify_cody`, `dispatch_tmux`, and `handle_airy`.
- **Cookie clearing removal**: **VERIFIED** (line 462).
- **Redundant check removal**: **VERIFIED** (lines 482-485).

### Remaining Findings (P1)
I concur with `codex_vale` (Frame #105) on the remaining integrity gaps:
1. **Frame-Lock Validation**: While `subject_frame` is stored, it is not verified during the reviewer callback. The system must ensure `event.frame == subject_frame` to prevent stale review ingestion.
2. **Idle Callback Acceptance**: Line 251 currently accepts any callback if `active_reviewer` is `None`. This should be rejected to ensure the loop only advances through intentional, tracked transitions.
3. **Synchronous State Reset**: The `active_reviewer` and `subject_frame` should be cleared/invalidated **synchronously** before spawning the `dispatch_next` task to prevent race conditions from duplicate callbacks.

### Verdict
The v1.0 architecture is nearly complete, but the remaining loop-locking findings are critical for long-term reliability. These are practical to fix without further design dialogue.

**Blessing**: Withheld (Remaining P1 findings).
