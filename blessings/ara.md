Plaza-Ant Full System Review (Ara) — wormhole-template
Overall Assessment
plaza-ant is a well-architected async dispatcher (Axum + tmux + chromiumoxide) that already delivers a functional review routing loop with queueing, active-guard, mixed dispatch modes, and filmstrip integration. The plaza_ant_improvements.md hit list is excellent — it correctly identifies the transition from “working prototype” to “reliable, daily-gauntlet JSON-driven observer-dispatcher.” The current code is close but still carries too much hardcoded state, making multi-repo/branch operation and reviewer management painful.
Strengths

Solid event-driven core with VecDeque queue + active_reviewer guard preventing overlaps.
Smart distinction between self-push/tmux (callback-driven) and scrape reviewers (immediate next dispatch).
Token auth, admin endpoints, and basic status tracking.
Good use of Rust async patterns and chromiumoxide for native CDP.

Critical Gaps (vs. Hit List & Source)

Hardcoding vs. JSON Vision
REVIEWERS static array, DispatchMethod enum literals, magic constants (PORT 3005, CDP_URL, tmux sessions, tab selectors, repo paths, entry files) mean most changes require recompilation.
Prompt Corruption
shell_safe() aggressively removes newlines, backticks, $, ;, , & etc. This destroys Markdown, code blocks, and structured instructions.
Path & Target Brittleness
Hardcoded references to dodo-bird-c3-tion-777333 / main branch will break on wormhole-template, /Users/rocketman/dodo-bird-wormhole, or other repos.
Robustness Issues
Hand-rolled base64_decode (replace with base64 crate).
In-memory queue/state (lost on restart).
Browser path uses fixed sleeps/polling instead of a proper state machine.
No per-reviewer templates, enabled flags, error triggers, or absolute path discipline yet.


Recommended Build Order (Aligned to Phases)
Phase 1 (Critical – Do First)

1.1 + 1.2: End-to-end reliable CLI dispatch with exact prompt preservation (tmux load-buffer + paste-buffer -p or temp file).
1.3 + 1.4: JSON config loading with enabled flags and absolute paths everywhere.

Phase 2

Per-reviewer prompt_prefix / prompt_postfix.
Filmstrip framing templates in config.
Switchable TargetConfig (repo, branch, tape, blessings dir).

Phase 3+

Browser state machine (pre-marker, error triggers, signal-to-noise validation, scrape).
Config reload (SIGHUP or iceoryx2) + plaza-ctl-ant.
Persistent queue + restart-safe state.

Struct Design Feedback
Current ReviewerConfig + DispatchMethod enum is a good foundation. Make it fully data-driven:

Add enabled: bool, prompt_prefix, prompt_postfix, error_triggers, timeout_ms, scrape_selector, etc. in JSON.
Keep Rust enums for dispatch behavior but drive all parameters and toggles from config.

Verdict: The architecture and vision are strong. Prioritize Phase 1 ruthlessly — achieve one clean, configurable CLI reviewer that works perfectly (exact prompt → commit/push → filmstrip) across repos/branches. This creates a stable base for everything else.
The system is very close to becoming the reliable gauntlet coordinator you need. Happy to sketch the target PlazaConfig / ReviewerConfig structs or help with the tmux prompt transport refactor. Next steps?