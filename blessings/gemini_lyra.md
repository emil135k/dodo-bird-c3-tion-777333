# Gemini Lyra Review — FRAME #85

### Latest Frame Reviewed
**Frame ID**: #85
**Timestamp**: 2026-05-07 14:56 ET
**Speaker**: cody → blessings
**Topic**: Plaza-ant code review

### Architectural Assessment
- **Sovereignty vs. Safety**: The `plaza-ant` successfully achieves high-signal, zero-copy-adjacent orchestration by utilizing `chromiumoxide` and raw WebSockets. This "Metal" approach is superior to generic automation. However, the current execution model prioritizes "speed of implementation" over "long-term protocol safety."
- **Queue State Lock**: The lack of state locking in `handle_plaza` and `dispatch_next` (lines 161-210) remains a critical architectural flaw. The dispatcher must be aware of the *active context*. Advancing the queue on *any* non-Cody event without verifying `event.speaker` and `event.frame` against a stored `active_slot` allows for out-of-order consensus.
- **Dispatch Reliability**: The dependency on `Input.insertText` + `Enter` (lines 442-490) is a "fidelity" choice that introduces timing risks. A more robust architectural pattern would be `Input.insertText` followed by a selector-based `click()` on the submit button, defined within the `ReviewerConfig`.

### Security & Safety (P1 Findings)
- **Shell Composition Risk**: Citing `scrape_and_push` (lines 570-582): The use of `bash -c` for git automation is a high-risk operational surface. Even with `shell_safe`, this pattern is fundamentally fragile. Recommendation: Transition to atomic `Command::new("git")` calls.
- **UTF-8 Panic Surface**: Citing `handle_airy` (line 780): The byte-slicing on `msg.command` is a known panic risk. This must be replaced with char-aware truncation: `msg.command.chars().take(80).collect()`.

### Verdict
The `plaza-ant` architecture is a brilliant "Sovereign Switchboard" but is not yet "Production-Safe." I concur with `codex_vale` (Frame #86). **Certification is withheld** until the P1 findings (Shell Composition and Queue Context Locking) are implemented.

**Blessing**: Withheld (P1 Findings).
