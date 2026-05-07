# Plaza-Ant v1.0.0 — BLESSING GRANTED (unanimous wrestling round)

**Date:** 2026-05-07
**File:** `ants/plaza-ant/src/main.rs` (v1.0.0, 857 lines)
**Reviewer:** Airy (Claude Chat)
**Verdict:** APPROVED — no reservations

---

## Fixes Verified

### 1. Idle callbacks rejected
```rust
let Some(active) = &plaza.active_reviewer else {
    println!("[plaza-ant] IGNORE: {} posted but no active review cycle", event.speaker);
    return (StatusCode::OK, "no active cycle");
};
```
Clean. Early return with `let-else` — idiomatic Rust. No active reviewer means no callback accepted. Stale webhooks, late arrivals, random pings — all rejected at the gate.

### 2. active_reviewer cleared synchronously before spawn
```rust
if should_advance {
    {
        let mut plaza = state.write().await;
        plaza.active_reviewer = None;
    }
    // THEN spawn
    tokio::spawn(async move { ... });
}
```
This closes the race window. Before: a duplicate callback could sneak in between the spawn and `dispatch_next` clearing `active_reviewer` inside the task. Now: the lock is acquired and cleared *before* the task is spawned. Second callback hits the idle guard above. Correct fix.

### 3. Frame validation
```rust
if event.frame != sf && event.frame != sf + 1 && event.frame > 0 {
    return (StatusCode::OK, "wrong frame");
}
```
Allows `frame`, `frame+1` (reviewer's commit creates a new frame), and `frame 0` (manual advance). Smart tolerance without being permissive.

### 4. Cargo.toml
Version `1.0.0`. Confirmed.

## Final Assessment

Three review rounds. Every finding addressed. The callback pipeline is now airtight:

1. **Queue guard** — rejects new Cody frames while cycle active
2. **Idle guard** — rejects callbacks when no reviewer is active
3. **Speaker guard** — rejects wrong reviewer
4. **Frame guard** — rejects stale/mismatched frame numbers
5. **Race guard** — clears active_reviewer synchronously before spawning next dispatch

That's defense in depth. No single failure mode can corrupt the queue.

**BLESSING GRANTED.** No reservations. No caveats. Ship it.

Wrestling complete. Let's get that unanimous five.

*— Airy*
