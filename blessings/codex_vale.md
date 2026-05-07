FRAME #85 review by codex_vale.

Reviewed frame: FRAME #85, recorded as `cody -> blessings`, and source `ants/plaza-ant/src/main.rs`.

Findings:
- P1 `handle_plaza` / `dispatch_next` lines 161-187, 193-210: any non-`cody` callback advances the queue. There is no active reviewer, subject frame, or duplicate callback validation. A stale `chatgpt_vale`/`ara` event can dispatch the next reviewer for the wrong frame. Store `active_reviewer`, `subject_frame`, and processed callback ids; only advance when `event.speaker` and `event.frame` match the active slot.
- P1 `scrape_and_push` lines 570-582: git automation is executed through `bash -c` with interpolated values. Even if today’s reviewer constants are trusted, this is an unnecessary shell surface in the component that writes and pushes. Replace with structured `Command::new("git")` calls using `current_dir`, and handle pull/add/commit/push as separate checked steps.
- P2 `cdp_send_and_click` lines 442-490: prompt injection uses broad input selectors plus Enter key submission. This can hit the wrong editable area or insert a newline instead of submitting. Move submit behavior into reviewer profiles with explicit selectors and verify that the submitted prompt includes `FRAME #85` before moving on.
- P2 `scrape_and_push` lines 514-548: scraping always takes the last assistant response and does not verify it mentions the target frame. Require a `FRAME #<n>` marker in scraped text before writing the blessing file.
- P2 `handle_airy` line 780: `&msg.command[..msg.command.len().min(80)]` can panic on a UTF-8 boundary. Use a char-safe preview helper.

Verdict: plaza-ant is a sound dispatcher/router design, but code review certification should remain blocked until queue advancement is frame/reviewer locked and shell-based git push is removed.
