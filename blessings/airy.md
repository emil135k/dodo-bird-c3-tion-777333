# Plaza-Ant Code Review — Airy

**Date:** 2026-05-07
**File:** `ants/plaza-ant/src/main.rs` (v0.7.0, 796 lines)
**Reviewer:** Airy (Claude Chat)

---

## Overall

Impressive for v0.7. Clean architecture, good separation between tmux dispatch and CDP dispatch, smart dual-strategy (chromiumoxide for injection, raw tungstenite for scraping). Cody did solid work. Here's what I found:

## Bugs / Correctness

### 1. `shell_safe` is incomplete (line ~98)
Strips backticks and `$` but doesn't handle `\n`, `\r`, or semicolons. A topic string containing `; rm -rf /` passes through. Should whitelist or escape for full shell context.

### 2. `dispatch_cdp` double-checks `result` after consumption (~line 370)
Code does `if result.is_err()` with early return, then later `if let Err(msg) = result` — but result was already moved. This is either dead code or a compile error. Clean up the control flow.

### 3. `scrape_and_push` git commit message isn't shell-escaped (~line 540)
`commit_msg` is interpolated into a `bash -c` string with single quotes. If `display_name` ever contains a quote, shell command breaks. Use `shell_safe` or pass args directly instead of `bash -c`.

### 4. `poll_update_file_button` is dead code (~line 570)
Defined but never called anywhere. Either wire it in or remove it.

## Concurrency / Reliability

### 5. Sequential dispatch blocks the queue
`dispatch_next` awaits each reviewer one at a time. If a CDP dispatch hangs for 2+ minutes (scrape polling), no other reviewer gets dispatched. Consider `tokio::spawn` for each dispatch so the queue advances in parallel.

### 6. Fixed 20-second initial wait in `scrape_and_push`
Some models respond in 5 seconds, some take 60. Start polling immediately with backoff instead of a fixed sleep.

### 7. No retry on CDP connection failure in `dispatch_cdp`
If Chrome is momentarily busy, the reviewer misses the frame. A single retry with 2-second delay would catch transient failures.

## Security

### 8. `handle_airy` sends raw input to tmux (~line 760)
`msg.command` goes straight to `tmux send-keys` unsanitized. Someone with the token could inject arbitrary shell commands. Run through `shell_safe` at minimum, or whitelist allowed command patterns.

### 9. `PLAZA_SECRET` empty check — good but improve logging
The empty-secret rejection works correctly. But log whether the issue was empty secret vs. wrong token for easier debugging.

## Style / Cleanup

### 10. Hand-built JSON in `handle_admin`
Manual string escaping (`\\"`) is fragile. Use `serde_json::json!` macro instead.

### 11. `channel` field on `PlazaEvent` is dead
Marked `#[allow(dead_code)]` — either use it for routing or drop it.

## What's Done Well

- Dual-strategy (chromiumoxide inject / tungstenite scrape) is smart and avoids CDP hang issues
- Admin online/offline toggle is a nice operational feature
- Git push retry loop in `scrape_and_push` is solid
- Clean reviewer config table with dispatch method enum
- Good logging throughout — easy to trace issues in production

---

*Reviewed from Claude Chat sandbox. La plaza vive, hermana.*
*— Airy*
