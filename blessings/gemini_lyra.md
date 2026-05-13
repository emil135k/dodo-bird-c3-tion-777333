## 2026-05-12 18:30 ET - gemini_lyra - Plaza-Ant Architecture Audit

**Verdict:** The transition to a JSON-driven observer-dispatcher is an architectural necessity. The current reliance on hardcoded Rust structs (REVIEWERS) and path assumptions creates significant "recompile-to-pivot" friction.

### Architectural Critique

1. **Decouple Reviewer Logic from Source:**
   - The current `REVIEWERS` constant and `DispatchMethod` enum are too rigid. Adding or disabling a reviewer should never require a Rust recompilation.
   - **Recommendation:** Implement a dynamic registry that maps JSON capability strings (e.g., "browser_scrape", "tmux_dispatch") to internal handler functions.

2. **Path Normalization is Critical:**
   - The current code constructs paths like `{HOME}/dodo-bird-c3-tion-777333/`. This is fragile. 
   - **Recommendation:** Adopt Phase 1.4 from the hit list immediately. Every path (repo, blessings, tape) must be an absolute path defined in JSON.

3. **Observer Robustness:**
   - The "scrape-and-push" loop currently uses fixed sleeps and simple polling.
   - **Recommendation:** Implement a proper state machine for the observer. It should detect "Reviewer Fatigue" (repetitive refusals) or session timeouts in addition to basic error triggers.
   - **Validation:** Scraped content must pass a "Signal-to-Noise" check (minimum length, absence of UI noise) before being committed.

4. **Security & Data Integrity:**
   - `shell_safe` is a dangerous bottleneck for complex prompts.
   - **Recommendation:** Move from `send-keys` to `load-buffer` / `paste-buffer` or temp file injection for CLI reviewers to preserve Markdown and code snippet integrity.
   - **Integrity:** The in-memory `VecDeque` queue is lost on crash. Move to a "Restart-Safe Queue" backed by a simple JSON file on disk.

### Specific Recommendations for Hit List Phase 1/2

- **Phase 1.4 (Paths):** This is the highest priority. Without absolute path discipline, multi-repo support is impossible.
- **Phase 2.1 (Templates):** Transitioning to per-reviewer templates will allow the "Gauntlet" to provide distinct instructions to different model types (e.g., reminding local models about specific file access constraints).

**Audit Status:** Architecturally sound vision. The shift to "Zero-FFI" at the protocol level (JSON/Pipes) must be mirrored in the configuration layer (JSON/Runtime).

