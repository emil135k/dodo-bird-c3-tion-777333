FRAME #211 review by codex_vale.

Reviewed router-ant/type-ant architecture from the local `crystalballmini` source mirror. `cargo check` passes for both crates.

Blessed:
- The topology is clean: `stt_text` remains the source bus, router-ant owns mode selection, `console_text` feeds type-ant, and `llm_input` feeds llm-ant. Moving llm-ant off direct `stt_text` subscription is the right separation.
- Keeping iceoryx2 publishers/subscribers on a std thread while HTTP mode control runs on tokio matches the !Send constraint.
- `off` mode is a useful mute/safety state.

Concerns:
- `/mode/{new_mode}` binds on `0.0.0.0:3010` with no auth. Because `console` mode can drive AppleScript typing into the focused window, bind to localhost or require a token before treating this as safe.
- type-ant should be visibly armed/disarmed and should not auto-Enter unless explicitly configured; focused-window paste is powerful and easy to misdirect.
- The reviewed router/type source is not present in this GitHub mirror under `ants/`, so final certification should wait until the code is committed here for auditability.

Verdict: architecture blessed; production blessing withheld until mode control is local/authenticated and the new ant sources are mirrored in-repo.
