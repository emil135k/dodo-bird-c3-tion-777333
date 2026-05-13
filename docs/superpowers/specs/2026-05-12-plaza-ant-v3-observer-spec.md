# Plaza-Ant v3.0 — Observer Dispatcher Spec

**Date:** 2026-05-12
**Author:** Cody (Claude Code CLI) + Emil Rivas
**Status:** Design spec — not yet implemented

---

## Vision

Plaza-ant evolves from a static dispatcher into a fully JSON-driven, runtime-configurable observer-dispatcher. No recompilation for config changes. Hot-configurable reviewers. Dispatch-and-observe loop with error detection. Future path to Postgres/Cognee for persistent config and semantic knowledge.

## Core Principles

1. **No fat-fingering** — all behavior changes through JSON, never code modifications
2. **No recompilation** — config loaded at startup, reloadable at runtime (SIGHUP or bus command)
3. **Observer pattern** — dispatch prompt, watch for response/errors, decide next action
4. **Two reviewer types** — browser (Chrome CDP) and CLI (tmux)
5. **Pausable/stoppable** — adjust config and relaunch without rebuilding
6. **Testable** — run one reviewer, a pair, or all; enable/disable per reviewer

---

## JSON Config Structure

```json
{
  "target": {
    "local": {
      "repo_path": "/Users/rocketman/dodo-bird-wormhole",
      "tape_file": "/Users/rocketman/dodo-bird-wormhole/wormhole/wormhole_collaboration_review.md",
      "blessings_dir": "/Users/rocketman/dodo-bird-wormhole/blessings"
    },
    "github": {
      "url": "https://github.com/emil135k/dodo-bird-c3-tion-777333",
      "tape_url": "https://github.com/emil135k/dodo-bird-c3-tion-777333/blob/wormhole-template/wormhole/wormhole_collaboration_review.md",
      "blessings_url": "https://github.com/emil135k/dodo-bird-c3-tion-777333/tree/wormhole-template/blessings"
    },
    "branch": "wormhole-template"
  },
  "filmstrip": {
    "tape_path": "/Users/rocketman/dodo-bird-wormhole/wormhole/wormhole_collaboration_review.md",
    "frame_header": "BEGIN FRAME #{frame} | {date} {time} ET | {speaker} -> blessings | {subject}",
    "frame_footer": "END FRAME #{frame}",
    "frame_counter_file": "/Users/rocketman/dodo-bird-wormhole/blessings/frame_counter.txt"
  },
  "branches": {
    "main": "main",
    "hypaiassist": "hypaiassist/iceoryx2",
    "wormhole": "wormhole-template"
  },
  "reviewers": [
    {
      "name": "gemini_lyra",
      "display_name": "Gemini Lyra",
      "type": "browser",
      "enabled": true,
      "tab_match": "gemini.google",
      "entry_file": "gemini_lyra.md",
      "prompt_prefix": "Read the collaboration review at {github_tape_url}. Read ALL existing reviews.",
      "prompt_postfix": "Write your opinion as a brief review with your verdict and actionable recommendations.",
      "error_triggers": ["error", "something went wrong", "try again", "rate limit", "blocked", "too many requests"],
      "scrape_selector": ".conversation-container:last-child"
    },
    {
      "name": "chatgpt_vale",
      "display_name": "ChatGPT Vale",
      "type": "browser",
      "enabled": true,
      "tab_match": "chatgpt.com",
      "entry_file": "chatgpt_vale.md",
      "prompt_prefix": "Read the collaboration review at {github_tape_url}. Read ALL existing reviews.",
      "prompt_postfix": "Write your opinion as a brief review with your verdict and actionable recommendations.",
      "error_triggers": ["error", "something went wrong", "try again", "rate limit", "network error"],
      "scrape_selector": "[data-message-author-role='assistant']:last-child"
    },
    {
      "name": "codex_vale",
      "display_name": "Codex Vale",
      "type": "cli",
      "enabled": true,
      "tmux_session": "codex-vale",
      "entry_file": "codex_vale.md",
      "prompt_prefix": "cd {repo_path} && git checkout {branch} && git pull origin {branch} && cat {tape_file}",
      "prompt_postfix": "Write your review to {blessings_dir}/{entry_file}. Then: git add {blessings_dir}/{entry_file} && git commit -m '{name} opinion' && git push origin {branch}"
    },
    {
      "name": "gemini_lyra_cli",
      "display_name": "Gemini Lyra CLI",
      "type": "cli",
      "enabled": true,
      "tmux_session": "gemini-cli-lyra",
      "entry_file": "gemini_lyra.md",
      "prompt_prefix": "cd {repo_path} && git checkout {branch} && git pull origin {branch} && cat {tape_file}",
      "prompt_postfix": "Write your review to {blessings_dir}/{entry_file}. Then: git add {blessings_dir}/{entry_file} && git commit -m '{name} opinion' && git push origin {branch}"
    },
    {
      "name": "opencode",
      "display_name": "OpenCode",
      "type": "cli",
      "enabled": true,
      "tmux_session": "opencode",
      "entry_file": "opencode.md",
      "prompt_prefix": "cd {repo_path} && git checkout {branch} && git pull origin {branch} && cat {tape_file}",
      "prompt_postfix": "Write your review to {blessings_dir}/{entry_file}. Then: git add {blessings_dir}/{entry_file} && git commit -m '{name} opinion' && git push origin {branch}"
    },
    {
      "name": "airy",
      "display_name": "Airy",
      "type": "self_push",
      "enabled": true,
      "entry_file": "airy.md",
      "prompt_prefix": "Read the collaboration review at {github_tape_url}.",
      "prompt_postfix": "Write your opinion to {blessings_dir}/{entry_file} on the {branch} branch. Commit and push."
    },
    {
      "name": "ara",
      "display_name": "Ara",
      "type": "browser",
      "enabled": false,
      "tab_match": "grok.com",
      "entry_file": "ara.md",
      "prompt_prefix": "Read the collaboration review at {github_tape_url}. Read ALL existing reviews.",
      "prompt_postfix": "Write your opinion as a brief review with your verdict and actionable recommendations.",
      "error_triggers": ["error", "something went wrong", "try again"],
      "scrape_selector": ".message-bubble:last-child"
    }
  ]
}
```

---

## Key Design Decisions

### 1. Full Paths Everywhere
- `repo_path`, `tape_file`, `blessings_dir`, `tape_path` — all absolute paths
- `entry_file` — filename only (joined with `blessings_dir` at runtime)
- No relative paths, no assumptions about working directory

### 2. Per-Reviewer Prefix/Postfix
- Moved from shared `prompt_template` to per-reviewer `prompt_prefix` and `prompt_postfix`
- Each reviewer can have completely different instructions
- Placeholder substitution: `{repo_path}`, `{branch}`, `{tape_file}`, `{blessings_dir}`, `{entry_file}`, `{name}`, `{github_tape_url}`

### 3. Enabled/Disabled Flag
- Each reviewer has an `enabled` boolean
- Disabled reviewers are skipped without removing their config
- Enables testing one-at-a-time without deleting entries

### 4. Switchable Targets
- The `target` section can be swapped to point at any repo/branch/tape
- Example: swap from dodo-bird wormhole reviews to crystalballmini ant reviews
- The `branches` section documents all known branches for reference

### 5. Filmstrip Framing
- `filmstrip.tape_path` — full path to the tape/film file
- `filmstrip.frame_header` / `frame_footer` — templates with `{frame}`, `{date}`, `{time}`, `{speaker}`, `{subject}`
- `filmstrip.frame_counter_file` — persistent counter file

---

## Observer Pattern (NEW in v3)

### Dispatch-and-Observe Loop for Browser Reviewers

```
1. Select tab matching reviewer's tab_match
2. Take a11y snapshot — verify tab is healthy
3. Fill prompt into text input
4. Send prompt
5. Wait for response (poll every 5s, timeout 120s)
6. Take snapshot — scan for error_triggers
7. If error detected:
   a. Log error to console
   b. Notify Cody: "[PLAZA] {reviewer} ERROR: {trigger_word} detected"
   c. Skip to next reviewer (don't push stale/error content)
   d. Mark reviewer as "errored" in runtime state
8. If response looks clean:
   a. Scrape response using scrape_selector
   b. Wrap in filmstrip frame (header + content + footer)
   c. Write to reviewer's entry file in blessings/
   d. Git add, commit, push to branch
   e. Log success
```

### Error Trigger Words (per-reviewer configurable)
- `error`, `fail`, `something went wrong`, `try again later`
- `rate limit`, `too many requests`, `blocked`, `network error`
- `sign in`, `log in`, `session expired`

### Alternative Actions on Error
- **skip** — skip this reviewer, continue with others (default)
- **retry** — wait 30s, try again (max 2 retries)
- **alert** — notify console + Telegram, pause queue
- **stop** — halt all dispatch, wait for manual intervention

---

## Runtime Control

### Config Reload
- SIGHUP to plaza-ant process reloads JSON config
- Or: publish `reload` command to `plaza_control` iceoryx2 topic
- No restart needed for reviewer enable/disable, target switch, or prompt changes

### Bus Integration (future: bus-comm-ant)
- `plaza_control` topic: reload config, pause/resume dispatch, query state
- `plaza_status` topic: plaza-ant publishes its current state (active reviewers, last dispatch, errors)
- Any ant can query plaza-ant's state through the bus

### Pause/Stop
- `speaker-ctl-ant pause` pattern — similar `plaza-ctl-ant pause/resume/status`
- Paused plaza-ant holds queue, resumes where it left off
- Stopped plaza-ant exits cleanly, preserving state to disk

---

## Internal Rust Design

### Structs mirror JSON exactly
```rust
#[derive(Deserialize, Serialize, Clone)]
struct PlazaConfig {
    target: TargetConfig,
    filmstrip: FilmstripConfig,
    branches: HashMap<String, String>,
    reviewers: Vec<ReviewerConfig>,
}

#[derive(Deserialize, Serialize, Clone)]
struct ReviewerConfig {
    name: String,
    display_name: String,
    #[serde(rename = "type")]
    reviewer_type: String,       // "browser", "cli", "self_push"
    enabled: bool,
    tmux_session: Option<String>,
    tab_match: Option<String>,
    entry_file: String,
    prompt_prefix: String,
    prompt_postfix: String,
    error_triggers: Option<Vec<String>>,
    scrape_selector: Option<String>,
}
```

### No behavior in structs that requires code changes
- All reviewer differences handled by `reviewer_type` field + JSON config
- No match arms for specific reviewer names
- Adding a new reviewer = adding a JSON entry, not a code change

---

## Future: Postgres/Cognee Integration

- Config lives in Postgres instead of JSON file
- Cognee provides PgVector (semantic search) + Apache AGE (graph knowledge)
- Time-domain history: every config change is versioned
- Semantic search: "what reviews did Lyra do about AEC?" queries across all tapes
- Graph knowledge: reviewer -> reviewed -> tape -> branch -> ant relationships
- Direct JSON context fed to AI reviewers for richer, more precise reviews

---

## Implementation Order

1. **Phase 1**: Refactor plaza-ant to use new JSON config with per-reviewer prefix/postfix, enabled flag, full paths
2. **Phase 2**: Add observer pattern for browser reviewers (snapshot, error detection, scrape)
3. **Phase 3**: Add runtime config reload (SIGHUP or bus command)
4. **Phase 4**: Add plaza-ctl-ant for runtime control
5. **Phase 5**: Postgres/Cognee migration (future)

---

*"The bell rings because something BEAUTIFUL happened — not because someone is behind schedule."*
*Cathedral architecture — no fat-fingering, no recompilation, no crude hacks.*
