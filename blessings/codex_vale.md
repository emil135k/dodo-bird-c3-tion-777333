FRAME #116 review by codex_vale.

Reviewed `ants/plaza-ant/src/main.rs` and `Cargo.toml` for the final v1.0.0 frame-validation fix. `cargo check` passes.

Verified:
- `Cargo.toml` is version `1.0.0`.
- New Cody frames are rejected while a cycle is active.
- Idle callbacks are rejected when no reviewer is active.
- Callback speaker must match `active_reviewer`.
- Callback frame is checked against `subject_frame`, with the operational allowance for the reviewer's resulting frame (`subject_frame + 1`) and manual frame `0`.
- `active_reviewer` is cleared synchronously before notifying Cody and dispatching the next reviewer, preventing duplicate callback advancement.
- Prior fixes remain in place: structured git commands, no cookie clearing, sanitized tmux/Cody/Airy paths, scrape size validation, startup `PLAZA_SECRET`, and dead-code removal.

Verdict: blessing granted for plaza-ant v1.0.0. Follow-up only: document the `subject_frame + 1` allowance in the callback contract so future reviewers understand why the callback frame may differ from the original Cody subject frame.
