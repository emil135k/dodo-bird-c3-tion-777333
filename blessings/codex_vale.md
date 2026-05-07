FRAME #97 re-review by codex_vale.

Reviewed `ants/plaza-ant/src/main.rs` after the v0.9 peer-review fixes. `cargo check` passes.

Verified fixed:
- `scrape_and_push` now uses structured `Command::new("git")` with `current_dir`; the prior `bash -c` shell surface is gone.
- `PLAZA_SECRET` is loaded once at startup and reused from state.
- Airy logging is UTF-8 safe via `chars().take(80)`, and Airy input now goes through `shell_safe`.
- Scrape output now has empty/min/max validation.
- Dead `poll_update_file_button` / `poll_and_click_button` code is removed.

Remaining issue: queue validation is improved but not complete. `active_reviewer` is checked, but `subject_frame` is set and never validated against callback frame, and there is no processed-callback id set. A duplicate callback from the active reviewer can still pass before `dispatch_next` clears/advances state, and `active_reviewer == None` currently accepts any non-Cody callback. Fix by requiring `event.frame == subject_frame`, clearing or marking the active slot synchronously before spawning notification/dispatch, and ignoring duplicate `(speaker, frame)` callbacks.

Verdict: blessing withheld for certification until callback frame validation and duplicate suppression are added. The other prior findings are materially addressed.
