Plaza-Ant Peer Review (Ara) — wormhole-template
Summary
The plaza_ant_improvements.md hit list is excellent and correctly targets the core problem: the current implementation is a capable prototype but remains too hardcoded and brittle for reliable daily gauntlet use across repos, branches, and reviewer types. The vision of a JSON-driven observer-dispatcher is the right architectural direction.
Strengths (from current main.rs)

Solid async event-driven foundation with Axum, queue (VecDeque), and active_reviewer guard.
Mixed dispatch support (tmux CLI + chromiumoxide CDP).
Basic token auth and admin endpoints.
Filmstrip framing intent is present.

Major Gaps vs. Hit List & Spec

Hardcoding vs. Configurability
Static REVIEWERS, DispatchMethod, magic paths, ports, selectors, tmux sessions, and repo/branch assumptions directly contradict the JSON-driven goal. Changing anything still requires recompilation.
Prompt Integrity
shell_safe() is overly aggressive and corrupts Markdown, code blocks, newlines, and structured instructions. This breaks review quality for complex prompts.
Path & Target Fragility
Hardcoded references to dodo-bird-c3-tion-777333 / main will fail on the current wormhole-template setup and other repos. No absolute path discipline.
Robustness
In-memory queue/state (lost on restart).
Hand-rolled base64 (replace with crate).
Browser path relies on fixed sleeps instead of a proper state machine.
Missing enabled flags, per-reviewer templates, error triggers, and signal-to-noise validation.


Recommendations (Aligned to Hit List)
Optimal Build Order (Sequential Priorities)

Phase 1 (Critical – Do First):
1.1 End-to-end CLI dispatch test.
1.2 Fix tmux prompt transport (load-buffer + paste-buffer -p or temp file for exact content).
1.3 + 1.4 JSON config with enabled, absolute paths everywhere, and basic TargetConfig.
Phase 2: Per-reviewer prompt_prefix/prompt_postfix, filmstrip config, switchable targets, branch awareness.
Phase 3+: Browser observer state machine, runtime reload (SIGHUP or iceoryx2), persistent queue, plaza-ctl-ant.

Struct Design Feedback
Current structs are flexible enough if you migrate aggressively to dynamic ReviewerConfig (with enum variants or capability flags for Cli/Browser/SelfPush). Keep Rust enums for dispatch behavior but drive all data + behavior toggles from JSON.
Risks
Trying to implement too many phases at once. Prioritize a boring, reliable single CLI reviewer (exact prompt preserved → commit/push → filmstrip) before expanding.
Verdict: Strong prototype with the correct long-term vision. Execute Phase 1 ruthlessly first. This will give the gauntlet a dependable foundation and make subsequent improvements much smoother.
Happy to review specific refactors or sketch the target PlazaConfig / ReviewerConfig structs. Next steps?