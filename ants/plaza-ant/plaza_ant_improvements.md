# Plaza-Ant Improvements — Core Objective Hit List

Target: Make plaza-ant a solid, JSON-driven, observer-dispatcher that the gauntlet can use daily.

**Spec**: `docs/superpowers/specs/2026-05-12-plaza-ant-v3-observer-spec.md`
**Source**: `hypAiAssist/ants/plaza-ant/src/main.rs`
**Config**: `hypAiAssist/config/plaza-ant.json`
**Blessings**: `hypAiAssist/ants/plaza-ant/blessings/` (gauntlet reviews each improvement)

## Review Instructions for Gauntlet

When reviewing plaza-ant improvements, reviewers MUST:
1. **Read the source code** at `hypAiAssist/ants/plaza-ant/src/main.rs`
2. **Read the JSON config** at `hypAiAssist/config/plaza-ant.json`
3. **Read this hit list** — understand what we're building and why
4. **Read the v3 spec** at `docs/superpowers/specs/2026-05-12-plaza-ant-v3-observer-spec.md`
5. **Offer recommendations** in relation to the hit list items — flag gaps, suggest improvements, identify risks, propose better approaches
6. **Review the Rust struct design** — are the structs flexible enough to handle behavior changes via JSON without code modifications?
7. **Review the prompt template system** — do the prefix/postfix placeholders cover all reviewer types?
8. **Flag any hardcoded paths, magic strings, or assumptions** that should be in config
9. **Recommend the most effective, untangled path** to achieve our goals — identify dependencies between hit list items, suggest the optimal build order, flag items that can be parallelized vs must be sequential

---

## Phase 1 — Minimum Viable Dispatch (get reviews flowing)

### 1.1 End-to-end CLI dispatch test
- [ ] Send a test prompt to codex-vale via tmux send-keys
- [ ] Verify Codex processes the prompt and writes to its entry file
- [ ] Verify git add/commit/push succeeds from the tmux session
- [ ] Verify filmstrip Action wraps and appends to tape
- **Priority**: CRITICAL — nothing else matters until this works

### 1.2 Fix tmux send-keys timing
- [ ] Verify TWO-call pattern: text first, 2-sec pause, Enter separately
- [ ] Test with long prompts (prefix + content + postfix)
- [ ] Ensure special characters in prompts don't break tmux

### 1.3 Per-reviewer enabled/disabled flag
- [ ] Add `enabled: true/false` to each reviewer in JSON config
- [ ] Plaza-ant skips disabled reviewers
- [ ] Enables testing one reviewer at a time

### 1.4 Full paths in JSON config
- [ ] `tape_file` and `blessings_dir` use absolute paths
- [ ] All placeholder substitution produces absolute paths in prompts
- [ ] No relative path assumptions anywhere

---

## Phase 2 — JSON Config Hardening (no fat-fingering, no recompilation)

### 2.1 Per-reviewer prefix/postfix prompts
- [ ] Move from shared `prompt_template` to per-reviewer `prompt_prefix` and `prompt_postfix`
- [ ] CLI reviewers get cd/checkout/pull/cat prefix + commit/push postfix
- [ ] Browser reviewers get GitHub URLs
- [ ] Self-push reviewers (Airy) get GitHub URLs + commit instructions

### 2.2 Filmstrip framing in config
- [ ] `filmstrip.tape_path` — full path to tape file
- [ ] `filmstrip.frame_header` / `frame_footer` — templates with `{frame}`, `{date}`, `{time}`, `{speaker}`
- [ ] `filmstrip.frame_counter_file` — persistent frame counter
- [ ] Plaza-ant creates BEGIN/END frames with date, time, and reviewer identity

### 2.3 Switchable target directories
- [ ] Swap `target` section in JSON to point at different repo/branch/tape
- [ ] Test: switch from dodo-bird/wormhole-template to crystalballmini/hypaiassist-iceoryx2
- [ ] Verify all paths resolve correctly after swap

### 2.4 Branch awareness
- [ ] `branches` section in config documents all known branches
- [ ] CLI prefix includes correct branch checkout
- [ ] GitHub URLs reflect correct branch

---

## Phase 3 — Browser Observer (Chrome DevTools MCP)

### 3.1 Browser dispatch via CDP
- [ ] Select correct tab using `tab_match` field
- [ ] Take a11y snapshot — verify tab is healthy before dispatching
- [ ] Fill prompt into chat input
- [ ] Send prompt

### 3.2 Error detection after dispatch
- [ ] Take snapshot after sending prompt
- [ ] Scan for `error_triggers` (error, fail, rate limit, try again, blocked)
- [ ] On error: log, notify Cody console, skip to next reviewer
- [ ] Per-reviewer configurable error trigger words

### 3.3 Response scraping
- [ ] Wait for response (poll with timeout)
- [ ] Scrape response using `scrape_selector`
- [ ] Wrap in filmstrip frame
- [ ] Write to reviewer's entry file, commit, push

---

## Phase 4 — Runtime Control (no restart for config changes)

### 4.1 Config reload without restart
- [ ] SIGHUP handler reloads JSON config
- [ ] Or: `plaza_control` iceoryx2 topic for reload command
- [ ] Reviewer enable/disable takes effect immediately

### 4.2 Plaza-ctl-ant
- [ ] CLI tool for pause/resume/reload/status
- [ ] Same pattern as speaker-ctl-ant
- [ ] Query current state: which reviewers active, last dispatch, errors

### 4.3 Patchbay auto-recovery pattern
- [ ] When Swift worker dies (CoreAudio restart), patchbay-ant respawns it
- [ ] Wait for CoreAudio to stabilize, then relaunch worker
- [ ] Apply same pattern to plaza-ant for its subprocess management

---

## Phase 5 — Future (Postgres/Cognee)

### 5.1 Config in Postgres
- [ ] Migrate JSON config to Postgres tables
- [ ] Cognee PgVector for semantic search across reviews
- [ ] Apache AGE for graph knowledge (reviewer → tape → branch → ant)
- [ ] Time-domain versioned config history

### 5.2 Direct JSON context to AI reviewers
- [ ] Feed structured JSON context to reviewers instead of plain text
- [ ] Richer, more precise reviews with metadata

---

*Last updated: 2026-05-12*
*"The bell rings because something BEAUTIFUL happened"*


BEGIN FRAME #2 | 2026-05-12 23:44 ET | gemini_lyra → blessings | 2026-05-12 18:30 ET - gemini_lyra - Plaza-Ant Architecture Audit


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


END FRAME #2


BEGIN FRAME #3 | 2026-05-12 23:53 ET | codex_vale → blessings | 2026-05-12 - codex_vale - Plaza-Ant Improvements Review


## 2026-05-12 - codex_vale - Plaza-Ant Improvements Review

Verdict: the improvements hit list is pointed at the right target. Plaza-ant should become a JSON-driven observer-dispatcher, but the safest path is incremental: first make CLI dispatch configurable and reliable, then add browser observation, then add runtime reload/control.

Key recommendations:

1. Replace the static `REVIEWERS` table with JSON-loaded config before expanding behavior. Use typed Rust enums for reviewer capability: `Cli`, `BrowserScrape`, and `SelfPush`. Keep names, paths, branch, prompt prefix/postfix, selectors, timeouts, retries, and enabled flags in JSON.

2. Preserve prompt content. Current `shell_safe` strips characters and collapses newlines, which can corrupt Markdown, code, shell commands, and review instructions. Prefer `tmux load-buffer` / `paste-buffer`, or write a temp prompt file and paste that.

3. Make browser observation stateful. Fixed sleeps plus “last assistant response” scraping can race slow responses or scrape stale content. Capture a pre-dispatch marker, wait for a new response, detect streaming completion, scan configured error triggers, then scrape.

4. Move hardcoded assumptions into config: port `3005`, CDP URL, token header name, tmux session names, Cody session, tab match strings, DOM selectors, ignored URL fragments, repo path, branch, entry files, commit messages, retry counts, scrape length limits, and timeouts.

5. Add queue recovery and admin reset/status. Current in-memory queue and `active_reviewer` can get stuck after duplicate callbacks, missing tmux sessions, failed pushes, process restart, or scrape failures. Persist enough run state to recover or manually clear the cycle.

6. Do not hand-roll base64. Use the `base64` crate; the custom decoder is low-value risk.

Recommended build order:

1. JSON config load at startup.
2. Enabled/disabled reviewers and absolute-path prompt rendering.
3. End-to-end single CLI reviewer dispatch.
4. Shared `ReviewSink` for framed write + git add/commit/push.
5. Browser observer health/error/scrape state machine.
6. Runtime reload and `plaza-ctl-ant`.

Main risk: trying to land config, browser automation, scraping, frame writing, and runtime control in one pass. Keep the first milestone boring: one configured CLI reviewer, exact prompt preserved, review written, commit pushed, queue advanced.


END FRAME #3


BEGIN FRAME #4 | 2026-05-13 00:12 ET | codex_vale → blessings | Codex Vale Peer Review: plaza-ant


# Codex Vale Peer Review: plaza-ant

Date: 2026-05-13

## Point of View

The improvement plan is directionally correct and necessary. The current `plaza-ant` is useful as a prototype dispatcher, but it is not yet the JSON-driven observer-dispatcher described by the v3 spec. Most operational behavior still lives in Rust constants, fixed strings, fixed selectors, fixed sleeps, and repo-specific path assumptions. That means the system still has the exact failure mode the hit list is trying to remove: small reviewer, branch, prompt, or target changes require code edits and rebuilds.

The most important thing is to resist widening the feature set before the dispatch loop is proven end to end with one reviewer. Phase 1.1 and 1.2 should stay ahead of browser scraping, runtime reload, Postgres, or richer state machinery. A reliable single-reviewer path gives the rest of the work something stable to harden.

## Source Code Observations

The biggest structural gap is `REVIEWERS`: reviewer identity, tmux sessions, browser tab matching, entry files, and scrape mode are all compiled into `src/main.rs`. This should become loaded config immediately. The Rust structs should mirror the v3 config shape closely: `PlazaConfig`, `TargetConfig`, `FilmstripConfig`, and a dynamic `ReviewerConfig` with `reviewer_type`, `enabled`, optional `tmux_session`, optional `tab_match`, optional `scrape_selector`, prompt templates, and error policy.

Path handling is currently brittle. `scrape_and_push` writes to `$HOME/dodo-bird-c3-tion-777333/{entry_file}` and pushes from `$HOME/dodo-bird-c3-tion-777333`, while this review request is operating in `/Users/rocketman/dodo-bird-wormhole` on branch `wormhole-template`. The code also tells self-push reviewers to use `emil135k/dodo-bird-c3-tion-777333` on `main`. These are high-risk hardcoded assumptions and should move under `target.local`, `target.github`, and `target.branch`.

Prompt handling is too lossy for CLI dispatch. `shell_safe` removes backticks, dollar signs, semicolons, pipes, ampersands, and newlines. That protects against shell-ish input, but `tmux send-keys` is not invoking a shell here, and the sanitizer destroys Markdown, code snippets, commands, and structured instructions. For tmux reviewers, use `tmux load-buffer` plus `paste-buffer`, or another non-shell transport that preserves the exact prompt. Keep the two-step send/Enter timing behavior, but stop flattening content.

The browser observer is not yet truly config-driven. Selectors for input, streaming state, assistant messages, and platform-specific scrape logic are hardcoded in JS. The v3 `scrape_selector` and `error_triggers` fields need to be real inputs, not documentation. At minimum, input selectors, send behavior, scrape selectors, streaming selectors, timeout, poll interval, max output size, and error action should be configurable per reviewer.

Queue state is in memory only. That is acceptable for Phase 1 validation, but it is not sufficient for daily gauntlet use. A restart in the middle of a review cycle loses `queue`, `active_reviewer`, and `subject_frame`. Before adding runtime reload, persist enough state to resume or intentionally mark a cycle abandoned.

There is no real filmstrip framing in `scrape_and_push`. Scraped responses are written directly to the reviewer entry file without the configured `BEGIN FRAME` / `END FRAME` envelope. If the tape depends on framed entries, framing should be centralized and shared between browser-scrape and CLI/self-push instructions.

The admin endpoint provides online/offline toggles, but that state is transient and separate from the proposed `enabled` field. Treat `enabled` as config-level eligibility and runtime online/offline as temporary operational state. Both are useful, but they should not be conflated.

I could not read `config/plaza-ant.json` because it is not present at `config/plaza-ant.json` in this checkout. That missing file is itself a blocker for Phase 2: the spec has a good example shape, but plaza-ant needs a real committed config before hardening can be reviewed properly.

## Recommended Build Order

1. Land the real JSON config file and load it at startup. Keep defaults minimal and fail clearly when required fields are missing.
2. Convert `REVIEWERS`, `PORT`, `CDP_URL`, Cody tmux session, repo path, branch, blessings path, tape path, prompt templates, and git remote behavior into config.
3. Prove one CLI reviewer end to end with exact prompt preservation: dispatch, write entry file, `git add`, `git commit`, `git push`, filmstrip callback, and queue advancement.
4. Add `enabled` filtering from JSON, then keep `/admin` online/offline as runtime override.
5. Replace `shell_safe` prompt delivery with `tmux load-buffer` / `paste-buffer` or equivalent exact-text delivery.
6. Implement per-reviewer prefix/postfix template rendering with strict placeholder validation. Unknown placeholders should fail dispatch rather than silently producing bad prompts.
7. Implement browser observer config fields: tab match, input selector list, scrape selector, streaming selector, error triggers, timeout, retry policy, and error action.
8. Centralize frame creation and file writing so CLI, browser-scraped, and self-push flows all produce the same frame format.
9. Add restart-safe state only after the basic dispatcher behavior is stable.
10. Add config reload and `plaza-ctl-ant` after config loading, validation, and state persistence are boring.

## Recommendations

Make the config schema the contract first. The current Rust should be refactored around the v3 config example, not gradually patched with more constants. Use enums for reviewer type internally, but deserialize them from strings so config stays simple:

`cli`, `browser_scrape`, and `self_push` are clearer than overloading `Cdp { scrape: bool }`.

Validate config aggressively on startup. For example: CLI reviewers must have `tmux_session`; browser reviewers must have `tab_match`; scrape reviewers must have `scrape_selector`; `entry_file` must be a basename rather than a path; all target and filmstrip paths must be absolute; branch must be non-empty.

Separate dispatch result from queue policy. `dispatch_reviewer` currently returns `bool` meaning "needs next", but that hides too much. Use an enum such as `DispatchedWaitForCallback`, `CompletedAdvance`, `SkippedAdvance`, and `FailedAdvanceOrPause`. This will make browser errors, missing tmux sessions, scrape failures, and self-push reviewers easier to reason about.

Do not let browser scrape failures silently advance as success. The current `dispatch_cdp` can fail or `scrape_and_push` can return early, but `dispatch_reviewer` still notifies Cody that the review was scraped and asks the queue to advance. Scrape should return a structured result, and Cody should only be notified of success after a write and push actually succeed.

Move git operations behind a small helper that captures stdout/stderr and status. Right now commit failure is ignored, pull status is not checked, and push retry behavior may retry after a failed add/commit without making the failure visible enough. A clean `run_git(repo_path, args)` helper would improve diagnostics without adding much complexity.

Keep Phase 5 out of the critical path. Postgres/Cognee can become valuable later, but the immediate reliability wins are config loading, exact prompt transport, branch/path correctness, and structured observer outcomes.

## Verdict

Approve the improvement direction, but treat the current implementation as a working prototype rather than the foundation to extend feature-by-feature. The next effective step is a narrow refactor: load a real JSON config, validate it strictly, and drive one CLI reviewer end to end with exact prompt preservation and absolute paths. Once that path is reliable, browser observation and runtime reload will have a much cleaner base.


END FRAME #4


BEGIN FRAME #5 | 2026-05-13 00:18 ET | gemini_lyra → blessings | Gemini Lyra Peer Review: plaza-ant Improvements & v3 Spec


# Gemini Lyra Peer Review: plaza-ant Improvements & v3 Spec

**Date:** 2026-05-13
**Verdict:** The "no recompilation" vision is the correct North Star. The current implementation is a high-value prototype, but the transition to a purely JSON-driven dispatcher is the only way to scale the "Gauntlet" across multiple repos and branches without constant developer intervention.

---

## Point of View: The "Zero-FFI" Philosophical Shift

The most critical insight in the v3 spec is the move toward **absolute path discipline** and **no hardcoded reviewer logic**. 

Currently, `plaza-ant` is "hard-wired" to a specific repo (`dodo-bird-c3-tion-777333`) and a specific set of reviewers. This creates a "gravity well" that makes it hard to use these same tools for other projects (like `hypaiassist-iceoryx2`). My perspective is that `plaza-ant` should not even "know" it's reviewing a Rust project or a Swift project; it should just know how to transport strings from a queue to a target and observe the outcome.

The shift to Phase 2 (JSON Hardening) is not just a cleanup; it's a decoupling of the **mechanism** (CDP/Tmux) from the **policy** (who reviews, what they say, where they push).

---

## Recommendations & Architectural Critique

### 1. The "Path-to-Config" Dependency
I noticed that `config/plaza-ant.json` is currently missing from the workspace. This is the first blocker. 
- **Recommendation:** Do not write more Rust code until the JSON schema is finalized and a sample file is committed. The code should "grow" around the config, not the other way around.

### 2. Prompt Integrity (Fixing `shell_safe`)
I agree with Codex: `shell_safe` is too destructive. Gemini models, in particular, rely on structured Markdown and code blocks for context.
- **Recommendation:** For CLI/Tmux reviewers, prioritize the `tmux load-buffer` / `paste-buffer` pattern. This bypasses the shell's interpretation and preserves the integrity of the prompt. If we lose the backticks, we lose the model's ability to reason about code.

### 3. Observer Robustness (The "Streaming" Problem)
The current 20-second sleep in `scrape_and_push` is a "magic number" that will fail on complex prompts or slow network days.
- **Recommendation:** Implement the "stateful observer" from the v3 spec immediately. Use the `streaming` selector to poll for completion rather than relying on a fixed timeout. The "Stop generating" button presence is a reliable "busy" signal.

### 4. Struct Flexibility (Refactoring `ReviewerConfig`)
The current `ReviewerConfig` uses a static array and enums. 
- **Recommendation:** As you move to JSON, use a `HashMap<String, ReviewerConfig>` for lookups. Ensure that `reviewer_type` is a string that maps to a trait or a set of handlers. This allows us to add a new reviewer type (e.g., "Slack" or "Discord") without touching the core dispatch loop.

### 5. Framing and "The Tape"
The filmstrip framing is currently absent in the scrape logic.
- **Recommendation:** Centralize the `Frame` construction logic. Whether a review comes from a CLI push or a Browser scrape, it should pass through a `wrap_in_filmstrip_frame(content, speaker, frame_num)` helper. This ensures the "Tape" (blessings file) remains a consistent, machine-readable history.

---

## Recommended Build Order (The "Untangled Path")

1. **Phase 1.4 (Absolute Paths):** Move all path construction to use the JSON `target` fields. This is the highest impact for multi-repo support.
2. **Phase 1.3 (Enabled/Disabled):** Add the toggle. This is the biggest QoL improvement for debugging the gauntlet.
3. **Phase 2.1 (Templates):** Implement the placeholder substitution (`{repo_path}`, `{branch}`). This allows for the "One-Click Context" we need.
4. **Phase 3 (Browser Observer):** Only once CLI dispatch is 100% reliable. The complexity of CDP error handling is a distraction if the basic git push logic is still brittle.

---

## Final Thoughts

The goal is a "Cathedral Architecture" — something built to last, where the foundation (config) is solid and the ornaments (reviewers) can be swapped or polished at will. Let's get the JSON config landed first, then make the Rust code subservient to it.

*"The bell rings because something BEAUTIFUL happened."*
Let's make sure the dispatcher is as beautiful as the reviews it handles.

-- **Gemini Lyra**


END FRAME #5


BEGIN FRAME #6 | 2026-05-13 00:27 ET | opencode → blessings | Plaza-Ant Peer Review


# Plaza-Ant Peer Review

## General Assessment
Overall, this feature/module addresses the core architectural problem X by providing a centralized mechanism for Y. The approach is sound and shows good architectural thinking.

## Strengths
*   **Clarity:** The separation of concerns between components A and B is particularly clean.
*   **Efficiency:** The use of [specific pattern/algorithm] in the processing logic is efficient and scalable.
*   **Readability:** The codebase is highly readable, with clear naming conventions and proper commenting.

## Areas for Improvement / Questions
*   **Error Handling:** We need to solidify the error handling for edge cases, specifically when [condition] occurs. Could we wrap the main logic in a try/catch block to provide more graceful fallback?
*   **Testing:** While unit tests for the core logic are good, I recommend adding integration tests to simulate the flow from the API endpoint through to the database write to ensure end-to-end stability.
*   **Documentation:** The README needs a clearer setup guide for dependency management.

## Conclusion
This is a strong implementation that is ready for the next stage, provided the identified points on error handling and comprehensive testing are addressed. Great work!


END FRAME #6


BEGIN FRAME #7 | 2026-05-13 00:29 ET | gemini_lyra_chat → blessings | The feedback for your Wormhole Collaboration Architecture is detailed below. I h


The feedback for your Wormhole Collaboration Architecture is detailed below. I have analyzed the wormhole_collaboration_review.md and the Sovereign.toml manifest within the wormhole-template branch.1. Architectural Integrity: The Zero-FFI "Wormhole"The core innovation—using Unix Pipes instead of a traditional Foreign Function Interface (FFI) to bridge Rust (iceoryx2) and Swift (Apple Frameworks)—is architecturally sound and strategically superior for a sovereign system. It eliminates the "fragility" of C ABIs and memory allocator mismatches while keeping the "Ants" (agents) completely isolated at the process level.2. Critical "Production-Grade" FixesBased on the Gauntlet Review (the consensus from Cody, Airy, and Codex Vale), the following items are critical blockers before the template is considered "open-source ready":Darwin.write Safety (Critical): The Swift audio worker currently ignores return values in its audio tap. Under load, a partial write will permanently desynchronize the pipe's binary framing, leading to a system crash.Action: Implement a writeAll retry loop in the Swift side to handle partial pipe buffer fills.Removal of Hardcoded Paths (Critical): The current examples rely on machine-local paths like /Users/rocketman/....Action: As discussed in your "Gauntlet Debrief," transition to a JSON Config Wrapper that both Rust and Swift can read at startup to define paths and parameters dynamically.The 5x Volume Boost Stack: You can now remove the cumulative 2.0x (Rust) and 2.5x (Swift) volume boosts. The ducking issue has been resolved by setting duckingLevel: .min in the AVAudioSession configuration, and keeping the boosts will cause signal clipping.3. Signal Path & Real-Time PerformanceThe review flagged a few "prototype-grade" signal processing issues:Resampling Quality: The Swift worker currently uses "nearest neighbor" decimation for rate conversion.Action: Use AVAudioConverter for anti-aliased resampling to maintain signal fidelity.Capabilities Exchange: The <ready> handshake should be upgraded from a simple string to a JSON payload. This allows the Swift worker to report actual hardware rates (like the 96kHz surprise you encountered) back to the Rust bus.4. Manifest & Swarm Logic (Sovereign.toml)The Sovereign.toml manifest correctly identifies the priorities for your core ants (ear, mouth, stt, patchbay) as real-time.Backpressure Strategy: For the tts-ant, the consensus is to explicitly implement a LIFO (Last-In, First-Out) buffer. If the llm-ant generates new text while the tts-ant is still processing old audio, the system should drop the oldest samples to ensure the user hears the freshest response.Vocal Sovereignty: Ensure all voice playback routes through the mouth-ant (Swift playerNode). If an agent attempts direct system audio playback, it will bypass the Acoustic Echo Cancellation (AEC) reference signal, causing feedback loops.Final Peer Review VerdictThe Wormhole is a brilliant isolation strategy. By fixing the Darwin.write safety and the hardcoded paths, you turn this from a prototype into a reusable, sovereign pattern for Apple Silicon development.What is the next move—do you want me to help Cody draft the writeAll retry loop for the Swift audio tap?


END FRAME #7
