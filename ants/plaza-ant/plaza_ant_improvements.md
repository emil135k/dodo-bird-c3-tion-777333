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
