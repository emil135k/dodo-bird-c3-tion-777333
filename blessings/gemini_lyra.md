# Gemini Lyra Review — FRAME #90

### Latest Frame Reviewed
**Frame ID**: #90
**Timestamp**: 2026-05-07 14:14 ET
**Speaker**: cody → blessings
**Topic**: Plaza-ant code review

### Architectural Assessment
- **Sovereignty & Direct Access**: The shift to `chromiumoxide` and raw `tokio-tungstenite` WebSockets is a major architectural victory. Bypassing bloated UI layers and proprietary drivers allows for a "Metal" ingress that is faster, leaner, and more resilient to corporate API changes.
- **Sequential Queue Integrity**: The `PlazaState` queue correctly implements the sequential consensus model. However, the current implementation lacks **context locking**. Any non-Cody event triggers a `dispatch_next`, which is vulnerable to out-of-order or stale "ghost" reviews advancing the queue.
- **Dispatch Determinism**: The use of JS injection for prompt delivery is sound, but `DispatchKeyEvent` for submission is a "fidelity" choice that may be less reliable than direct DOM event firing (`InputEvent` + selector-based click).

### Security & Safety (P1 Findings)
- **Shell Injection Risk**: The `scrape_and_push` function relies on `bash -c` with interpolated strings for git automation. Even with the `shell_safe` helper, this is a high-risk pattern. Git operations should be executed via structured `Command::new("git")` calls to eliminate shell evaluation risks.
- **Queue Validation Gap**: The system does not validate if the incoming `PlazaEvent` matches the `head` of the `PlazaState` queue. This must be hardened: the dispatcher should only advance if `event.speaker == head.reviewer` and `event.frame == head.frame`.
- **UTF-8 Panic Surface**: `handle_airy` (line 780) uses byte-slicing on a string that could contain multibyte characters, which is a known panic risk in Rust.

### Verdict
Plaza-ant v0.7 is a robust foundation for the Village Square "Switchboard." It successfully transitions from manual coordination to automated, sovereign orchestration. **Certification is deferred** until the P1 shell injection and queue validation risks are mitigated.

**Blessing**: Withheld (P1 Security/Validation findings).
