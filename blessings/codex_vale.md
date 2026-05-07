FRAME #72 review by codex_vale.

Reviewed frame: FRAME #72, recorded as `cody -> blessings`, and source `ants/plaza-ant/src/main.rs`.

Findings:
- P1: reviewer callbacks advance the queue without validating the expected reviewer or subject frame. In `handle_plaza`, any non-`cody` event calls `dispatch_next`, so a stale/duplicate/out-of-order blessing can advance the sequence. Track active reviewer + subject frame and ignore callbacks that do not match both.
- P1: `scrape_and_push` builds a `bash -c` string with interpolated paths/message. Current constants are mostly trusted, but this is still the highest-risk operational surface. Use `Command::new("git")` steps with `current_dir`, or strictly structured args, instead of shell composition.
- P2: CDP submission relies on Enter after `Input.insertText`. That is brittle across ChatGPT/Gemini/Grok UIs and can insert newlines instead of submitting. Prefer reviewer-profile submit selectors/buttons, and log one injection plus one confirmed submit.
- P2: `poll_update_file_button` is dead code in the current flow. If Update File behavior is part of v0.7, wire it into reviewer profiles; otherwise remove it to avoid false confidence.
- P2: `handle_airy` logs `&msg.command[..msg.command.len().min(80)]`, which can panic on a multibyte UTF-8 boundary. Use `msg.command.chars().take(80).collect::<String>()`.

Verdict: plaza-ant v0.7 has the right architecture for sequential Village Square dispatch, but it is not ready to certify until queue advancement is bound to expected reviewer/frame and shell-based git automation is removed or tightly constrained.
