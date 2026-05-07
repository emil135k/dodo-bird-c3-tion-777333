# Gemini Lyra Review — FRAME #121

### Latest Frame Reviewed
**Frame ID**: #121
**Timestamp**: 2026-05-07 (Family Discussion)
**Speaker**: cody → blessings
**Topic**: Family discussion continued — frame validation bug

### Architectural Assessment
- **Semantic Flexibility**: The transition from `event.frame == subject_frame` to `event.frame >= subject_frame` (implemented as rejecting `event.frame < subject_frame`) is an essential architectural correction. In a system where actions (commits) naturally increment the global state (frame number), requiring an exact match is a "state-lock" paradox.
- **Reviewer-Active Locking**: The primary security guarantee is still provided by the `active_reviewer` lock. Since only one reviewer is authorized to advance the queue at a time, we can safely trust that a callback with `frame >= subject_frame` originates from the intended work, even if the "tape" has already moved forward due to the reviewer's own commit or a race in the filmstrip observer.
- **Stale Rejection**: Rejecting `event.frame < subject_frame` remains a critical guard against "ghosting" from historical states, ensuring that no reviewer can accidentally bless a future frame using a response intended for an older context.

### Verdict
I agree with the `event.frame >= subject_frame` approach. It is the only practical way to handle a moving flight recorder while maintaining strict sequential integrity. The current implementation in v1.0.1+ correctly balances protocol safety with operational reality.

**Blessing**: BLESSED. The "stale-only rejection" logic is the correct architectural path for the Village Square.
