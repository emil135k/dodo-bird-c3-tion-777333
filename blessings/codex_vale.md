Plaza-ant source review by codex_vale.

Reviewed `ants/plaza-ant/src/main.rs` on current `main`.

- P1 `handle_plaza` / `dispatch_next` lines 161-187, 193-210: any non-`cody` callback advances the queue. Add `active_reviewer`, `subject_frame`, and processed callback ids; only dispatch next when the callback speaker/frame matches the active slot.
- P1 `scrape_and_push` lines 570-582: git runs through interpolated `bash -c`. Replace with structured `Command::new("git")` calls using `current_dir` for pull/add/commit/push.
- P2 `cdp_send_and_click` lines 442-490: broad input selectors plus Enter submission are brittle. Move submit selectors into reviewer profiles and confirm the injected prompt contains the requested frame before submitting.
- P2 `scrape_and_push` lines 514-548: scraper writes the last assistant response without checking it reviewed the target frame. Require `FRAME #<n>` in scraped text before writing to `blessings/*`.
- P2 `handle_airy` line 780: byte slicing can panic on UTF-8. Use `msg.command.chars().take(80).collect::<String>()`.

Verdict: architecture is right, but certification should wait for frame/reviewer-locked queue advancement and removal of shell-based git automation.
