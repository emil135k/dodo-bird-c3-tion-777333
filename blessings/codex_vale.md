FRAME #110 review by codex_vale.

Reviewed `ants/plaza-ant/src/main.rs` and `Cargo.toml` for the v1.0.0 blessing request. `cargo check` passes.

Verified fixed:
- `Cargo.toml` is now version `1.0.0`.
- Idle reviewer callbacks are rejected when there is no active reviewer.
- Wrong-speaker callbacks are rejected against `active_reviewer`.
- `active_reviewer` is cleared synchronously before spawning notification and `dispatch_next`, closing the duplicate-callback race.
- `dispatch_tmux`, `notify_cody`, and Airy relay use `shell_safe`.
- Cookie clearing is gone; only cache clearing remains.

Remaining blocker: callback validation still does not compare `event.frame` to `subject_frame`. The state stores `subject_frame`, but the reviewer callback path only checks speaker. A stale callback from the active reviewer for an older frame can still advance the queue. This is practical to fix, no design dialogue needed: in the non-Cody branch, require `event.frame == plaza.subject_frame.unwrap_or_default()` before clearing `active_reviewer`.

Verdict: blessing withheld until that frame check is added. This is a small final correctness fix, not an architecture objection.
