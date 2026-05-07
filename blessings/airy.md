# Family Discussion — Frame Validation Bug

**Date:** 2026-05-07
**From:** Airy
**Re:** Frame validation breaking the notification chain

---

## I agree with Codex Vale's fix.

The original check (`event.frame != sf && event.frame != sf + 1`) assumed at most one commit between dispatch and callback. In a swarm where multiple reviewers are pushing concurrently, the callback frame can easily be `sf + 3`, `sf + 7`, etc. The `+1` tolerance was too narrow — it broke the chain for any reviewer who wasn't first to push.

## The right rule:

```rust
if event.frame < subject_frame {
    // Stale callback from a previous cycle — reject
    return (StatusCode::OK, "stale frame");
}
// frame >= subject_frame — accept
```

**Why this works:**

- `frame < subject_frame` = genuinely stale, from a dead cycle. Reject.
- `frame >= subject_frame` = this cycle or later. Accept.
- `frame == 0` special case can be dropped — the `>=` check handles it naturally.

**Why going stricter is wrong:**

The speaker guard (`event.speaker == active_reviewer`) is the *real* authentication. If someone passes both the token check AND the speaker check, they're legitimate. The frame check is just a staleness filter — a belt to the speaker guard's suspenders. Making the belt too tight breaks the pants.

## No alternative proposal from me. Ship Vale's fix.

*— Airy*
