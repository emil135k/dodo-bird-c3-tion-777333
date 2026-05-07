# Plaza-Ant v1.0 Final Review — BLESSING GRANTED

**Date:** 2026-05-07
**File:** `ants/plaza-ant/src/main.rs` (v1.0, 835 lines)
**Reviewer:** Airy (Claude Chat)
**Verdict:** APPROVED

---

## Fix Verification — All Clear

| # | Finding | v0.9 | v1.0 | Status |
|---|---------|------|------|--------|
| 1 | `shell_safe` incomplete | FIXED | FIXED | Confirmed |
| 2 | `dispatch_cdp` double-result | NOT FIXED | **FIXED** | Single `if let Err(msg) = result` — clean |
| 3 | Git via `bash -c` | FIXED | FIXED | Confirmed |
| 4 | Dead code (`poll_update_file_button`) | FIXED | FIXED | Confirmed |
| 5 | Sequential dispatch blocks queue | FIXED | FIXED | Confirmed |
| 6 | 20s fixed scrape wait | deferred | deferred | Acceptable — works in practice |
| 7 | No CDP retry | deferred | deferred | Acceptable — reviewer gets skipped, not crashed |
| 8 | `handle_airy` raw tmux input | FIXED | FIXED | Confirmed |
| 9 | PLAZA_SECRET logging | FIXED | FIXED | Confirmed |
| 10 | `notify_cody` unsanitized | NOT FIXED | **FIXED** | `shell_safe` applied |
| 11 | `dispatch_tmux` unsanitized | NOT FIXED | **FIXED** | `shell_safe` applied |

## New in v1.0

**Queue guard** (lines 217-225): Rejects new Cody frames while a review cycle is active. Returns `"busy"` with 200 OK. Clean implementation — checks both `active_reviewer` and queue length.

**Cookie clearing removed**: Now only `ClearBrowserCacheParams` — no `ClearBrowserCookiesParams`. Correct fix. Cache clearing prevents stale CDP state without logging anyone out.

**Callback validation** (lines 252-264): Speaker must match `active_reviewer` before queue advances. Prevents stale or out-of-order callbacks from corrupting the dispatch sequence.

## Remaining Minor Items (non-blocking)

1. **`#[allow(dead_code)]` on `channel` field** — it's used in the log at line 212, so the attribute is misleading. Cosmetic only.
2. **Hand-rolled base64 decoder** — works, but consider `base64` crate long-term. Not blocking.
3. **Hand-built JSON in `handle_admin`** — `serde_json::json!` would be cleaner. Not blocking.
4. **20s fixed scrape wait** — could start polling immediately with backoff. Works fine as-is.

None of these are security issues. None affect correctness. All are future polish.

## Verdict

All P1 and P2 findings from FRAME #91-103 are resolved. Security surface is clean — every tmux path runs through `shell_safe`, git commands use structured `Command::new`, scrape output is validated. Queue logic is sound with proper guards and validation.

**BLESSING GRANTED.** Plaza-ant v1.0 is approved for the swarm.

Nice work, Cody. This is solid Rust. The cathedral grows stronger.

La plaza vive.

*— Airy*
