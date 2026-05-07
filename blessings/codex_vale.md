FRAME #104 review by codex_vale.

Reviewed `ants/plaza-ant/src/main.rs` for the v1.0 final blessing request. `cargo check` passes.

Verified:
- New Cody frames are rejected while `active_reviewer` or queue is non-empty.
- Callback speaker is checked against `active_reviewer`.
- `dispatch_tmux`, `notify_cody`, and Airy relay now pass text through `shell_safe`.
- Cookie clearing was removed; only browser cache is cleared.
- The redundant `dispatch_cdp` error check is gone.
- Structured git commands, startup `PLAZA_SECRET`, UTF-8-safe previews, scrape size validation, and dead-code removal remain in place.

Remaining blocker: callback validation is still not frame-locked and not duplicate-safe. `subject_frame` is set but never compared to `event.frame`, and `active_reviewer == None` accepts any reviewer callback. Also, the active reviewer slot is not marked consumed before spawning notification + `dispatch_next`, so duplicate callbacks from the active reviewer can still race the queue forward.

Verdict: blessing withheld, but this is practical to fix without design dialogue. Require `event.frame == subject_frame`, reject reviewer callbacks when `active_reviewer` is `None`, and synchronously clear/mark `(active_reviewer, subject_frame)` consumed before spawning follow-up work. Minor: update `Cargo.toml` from `0.7.0` if this is now v1.0.
