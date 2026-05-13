Plaza-Ant Peer Review (Ara) — wormhole-template
Summary
plaza-ant is a solid event-driven dispatcher built with Axum + tmux + chromiumoxide. It already handles queueing, active-reviewer guards, token auth, and mixed dispatch (tmux vs CDP). The plaza_ant_improvements.md hit list is excellent and correctly identifies the main gap: the system is still too hardcoded for reliable daily gauntlet use across repos/branches.
Strengths (Current Code)

Good async architecture with VecDeque queue + active_reviewer guard to prevent overlaps.
Clean separation of CLI (tmux) and browser (CDP) paths.
Token-protected endpoints + basic admin controls.
Self-documenting with helpful logging.

Critical Issues

Hardcoding vs JSON Goal
REVIEWERS static array, DispatchMethod enum, magic constants (PORT, CDP_URL, paths, selectors, tmux sessions) directly contradict the JSON-driven vision. Changing reviewers, branches, or targets requires recompilation.
Prompt Corruption
shell_safe() aggressively strips newlines, backticks, $, ;, , & etc. This destroys Markdown, code blocks, and structured instructions — fatal for quality reviews.
Brittle Paths & Assumptions
Hardcoded repo paths (dodo-bird-c3-tion-777333, main branch) will break on wormhole-template or other repos. No absolute path discipline.
Other Risks
Hand-rolled base64 decoder (use base64 crate).
In-memory queue/state (lost on restart).
Browser scraping uses fixed sleeps/polling instead of robust state machine.
No config reload or persistent queue.


Recommendations Aligned with Hit List
Immediate Priority (Phase 1)

Implement full JSON config loading (PlazaConfig, TargetConfig, FilmstripConfig, ReviewerConfig[] with enabled, prompt_prefix, prompt_postfix, dispatch_type, etc.).
Fix prompt transport: use tmux load-buffer + paste-buffer -p (or temp file) for exact content preservation.
Enforce absolute paths everywhere.

Phase 2

Per-reviewer templates + filmstrip framing in config.
Switchable target (repo/branch).
Proper error detection + signal-to-noise validation for scraping.

Build Order Suggestion

JSON config + absolute paths + enabled flags.
Reliable single CLI reviewer end-to-end (exact prompt, commit/push, filmstrip).
Shared ReviewSink for framed writes.
Browser state machine.
Runtime reload + plaza-ctl-ant.

Verdict: The vision is correct and the current prototype is a strong starting point. Ruthlessly complete Phase 1 first — one clean, fully configurable CLI reviewer that works perfectly across repos. This will unblock everything else and make the gauntlet far more stable.
The structs are flexible enough once you migrate from static REVIEWERS to dynamic JSON-loaded configs. Happy to sketch the new ReviewerConfig or dispatch refactors.