BEGIN FRAME #2 | 2026-05-12 23:15 ET | gemini_lyra -> Cody | Plaza-ant v3 Architecture Review

**Verdict:** The transition to a JSON-driven observer-dispatcher is an architectural necessity. The current reliance on hardcoded Rust structs (REVIEWERS) creates a "recompile-to-pivot" friction that is incompatible with the project's goal of decoupling.

### 1. Rust Struct Design & Flexibility
The current `DispatchMethod` enum and `REVIEWERS` constant are too rigid. I agree with Codex Vale: move this to a dynamic registry populated from JSON. 
- **Action:** Define a `ReviewerCapability` struct that captures dispatch (CLI/Browser), scraping rules, and identity. 
- **Recommendation:** Do not use string-matching for reviewer names in the dispatch logic. Use the capability traits defined in JSON.

### 2. Observer Pattern & Error Detection
"Blind dispatch" is the primary source of queue stalls. The observer pattern must be reactive, not just passive.
- **Action:** The browser observer should detect "Reviewer Fatigue" (e.g., repeating the same refusal text) in addition to explicit error words.
- **Validation:** Scraped responses must be validated for "Signal-to-Noise" ratio (e.g., minimum length and absence of error trigger words) *before* being written to the blessings tape.

### 3. Path & Config Normalization (The Untangled Path)
I disagree with the implementation order if it skips path normalization.
- **Action:** Prioritize Phase 1.4 (Full Paths) and Phase 2.1 (Prefix/Postfix) first. 
- **Reasoning:** CLI and Browser dispatch both fail if they don't know the absolute `repo_path` or the correct `branch`. Fix the environment assumptions before adding complex CDP observation.

### 4. Hardcoded Paths & Magic Strings
- **Config Candidates:** Axum Port (`3005`), CDP URL (`localhost:9222`), the `PLAZA_SECRET` name, and the `HOME`-relative path construction.
- **Protocol:** The `x-plaza-token` header name should be configurable.

### 5. Risks & Gaps
- **Atomic Operations:** Scraping followed by `git push` is not atomic. If the push fails, the local file is updated but the repo is out of sync. Use a "Write-Staging" area and only commit if the content passes validation.
- **Tmux Corruption:** I strongly agree with Codex Vale on `shell_safe` risks. Large prompts with complex Markdown or source code will break `send-keys`. 
- **State Persistence:** The in-memory `VecDeque` queue is lost on crash. For a system managing multiple long-running AI reviews, a "Restart-Safe Queue" (simple JSON file on disk) is required.

### 6. Alignment with Codex Vale
- **Agree:** Move to the `base64` crate immediately. 
- **Agree:** The observer must be a state machine, not a series of `sleep` calls.
- **Clarification:** I prioritize "Vocal Sovereignty" (Ensuring all AI output follows the same pipe/tape protocol) as a requirement for any new reviewer added to the JSON config.

**Overall Rating:** ⭐️⭐️⭐️⭐️ (Architecturally sound vision; needs path discipline and state persistence to reach production-grade.)

END FRAME #2
