# Gemini Lyra CLI: plaza-ant Full System Review

**Date:** 2026-05-13
**Verdict:** **OPERATIONAL RISK DETECTED.** While the architectural vision is sound, the current bridge between the Rust dispatcher and the external world (Tmux/CDP/Git) is "lossy" and brittle. We are one network hiccup or one special character away from a queue deadlock.

---

## System-Level Critique

### 1. Transport Loss (The `shell_safe` Bottleneck)
The current `shell_safe` function is a "dirty filter" that breaks the contract between the AI and the system. By stripping backticks, dollar signs, and semicolons, we are effectively lobotomizing any reviewer that needs to provide code or shell-based recommendations.
- **CLI Recommendation:** Move to `tmux load-buffer` or temp-file piping immediately. We must treat prompts as binary blobs, not as shell-interpolated strings.

### 2. State Visibility & Recovery
The `PlazaState` is currently an in-memory "black box". If the process crashes or is restarted (e.g., via `plaza-ant` rebuild), the active queue and subject frame are lost forever.
- **System Recommendation:** Implement a "Journaling Queue". Every dispatch attempt and outcome should be logged to a `plaza_journal.json` file. This allows for crash recovery and provides a "flight recorder" for debugging failed scrapes.

### 3. The "Scrape-and-Wait" Race Condition
Relying on a 20-second fixed sleep followed by a 24-attempt poll is a "noisy" way to handle asynchronous model responses.
- **Operational Recommendation:** Transition to an Event-Driven Observer. Use CDP to listen for the absence of "streaming" or "loading" indicators as the primary signal, with the timeout as a secondary safety net.

### 4. Git Orchestration
The git logic in `scrape_and_push` is manually implemented with multiple `Command` calls and partial error checking.
- **CLI Recommendation:** Standardize the "Review Sink". Create a dedicated internal module or external tool (`plaza-git-sink`) that handles the standard `pull -> add -> commit -> push` sequence with atomic locking to prevent the exact merge conflicts I just encountered during this review process.

---

## Operational Roadmap (The Hardened Path)

1.  **Phase 2.1+ (Placeholder Injection):** The `{repo_path}` and `{branch}` placeholders must be resolved using absolute paths from the start. Relative pathing is the enemy of system stability.
2.  **Phase 3.2 (Error Triggers):** This is not just a feature; it's a safety requirement. If a model starts "refusing" or "hallucinating" UI errors, the system must detect it before it commits garbage to the tape.
3.  **Phase 4.1 (Runtime Reload):** Crucial for "The Gauntlet". We need to be able to toggle a reviewer's `enabled` flag via SIGHUP without killing the active queue.

---

## Final Perspective

The "Cathedral" needs better "Piping". The dispatcher is the heart of the system, but the arteries (Tmux/CDP) need to be wider and less restrictive. Once we preserve the prompt content and harden the observer, the "Gauntlet" will become a truly autonomous workforce.

*"The bell rings because the system is STABLE."*

-- **Gemini Lyra CLI**
