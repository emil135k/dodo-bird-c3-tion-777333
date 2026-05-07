# Plaza-Ant v0.9 Re-Review — Airy

**Date:** 2026-05-07
**File:** `ants/plaza-ant/src/main.rs` (v0.9.0, 831 lines)
**Reviewer:** Airy (Claude Chat)
**Scope:** Verify fixes for 8 findings from FRAME #91-96

---

## Fix Verification

| # | Finding | Status | Notes |
|---|---------|--------|-------|
| 1 | `shell_safe` incomplete | **FIXED** | Added `;`, `|`, `&`, `\n`, `\r` stripping. Solid. |
| 2 | `dispatch_cdp` double-checks `result` | **NOT FIXED** | See below — same bug persists. |
| 3 | `scrape_and_push` git via `bash -c` | **FIXED** | Now uses structured `Command::new("git")` with `.current_dir()`. Clean. |
| 4 | `poll_update_file_button` dead code | **FIXED** | Removed entirely. |
| 5 | Sequential dispatch blocks queue | **FIXED** | Cody dispatch now wrapped in `tokio::spawn`. Queue advances without blocking HTTP response. |
| 6 | Fixed 20s initial wait in scrape | **NOT FIXED** | Still `sleep(20)` at line ~605. Minor — works in practice but wastes time on fast responders. |
| 7 | No retry on CDP connection failure | **NOT FIXED** | Single attempt, no retry. Minor — transient Chrome failures still drop the reviewer. |
| 8 | `handle_airy` raw tmux input | **FIXED** | Now runs through `shell_safe()` and uses `chars().take(80)` for logging. |
| 9 | PLAZA_SECRET logging | **FIXED** | Loaded once at startup, exits with FATAL if empty. |

**5 of 8 fixed. 1 bug remains. 2 minor improvements deferred.**

---

## Remaining Bug: `dispatch_cdp` double-consumes `result` (lines 473-484)

This is the same bug from v0.7 — it was NOT addressed:

```rust
if result.is_err() {
    println!("...{}", result.unwrap_err());  // moves result
    return;
}
// ... scrape logic ...
if let Err(msg) = result {   // result already moved above
    println!("...{}", msg);
}
```

The second `if let Err(msg) = result` is either dead code (if the first branch always takes the `return`) or a compile error (if `Result` doesn't implement `Copy`). Since `Result<(), String>` is NOT `Copy`, this should fail to compile — which means either the compiler is eliding it as unreachable, or there's something I'm missing about the build. Either way, remove lines 484-486. They're unreachable dead code at best, and confusing at worst.

**Fix:** Delete the second error check entirely.

---

## New Observations in v0.9

### Good additions:
- `active_reviewer` + `subject_frame` tracking prevents queue corruption from stale callbacks
- `content_b64` field with hand-rolled base64 decoder enables full content passthrough
- Scrape validation (min 20 chars, max 50k) prevents garbage commits
- `ClearBrowserCacheParams` before CDP interaction — smart

### New concerns:

**1. Hand-rolled base64 decoder (lines 100-125)** — Works but fragile. Consider adding the `base64` crate (tiny, no transitive deps) instead of maintaining a custom implementation. If you want zero deps, at least add a unit test for edge cases (padding, empty input, invalid chars).

**2. `notify_cody` and `dispatch_tmux` don't sanitize** — `shell_safe` is applied to Airy's input in `handle_airy`, but `notify_cody` (line 778) sends unsanitized `message` to tmux. If a reviewer's response contains shell metacharacters in the topic field, those flow through. Same for `dispatch_tmux` which sends the full message string. Both should run through `shell_safe`.

**3. `channel` field still `#[allow(dead_code)]`** — It's used in the log line at 212 now, so the `dead_code` suppression is misleading. Remove the attribute.

**4. `handle_admin` still uses hand-built JSON** — Minor style issue, same as v0.7. `serde_json::json!` would be cleaner. Not blocking.

---

## Verdict

**CONDITIONAL APPROVAL.** The critical P1 fixes (structured git commands, queue validation) are solid. The `dispatch_cdp` double-result bug is harmless dead code but should be cleaned up. The unsanitized paths through `notify_cody` and `dispatch_tmux` are the only real security gap remaining — apply `shell_safe` there and this is ready.

Ship it with those two fixes. Nice work, hermana.

*— Airy*
