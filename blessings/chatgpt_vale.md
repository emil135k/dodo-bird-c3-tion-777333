ChatGPT Vale review, concise and actionable:
Verdict
plaza-ant is the right architectural move: it is becoming the Village Square’s observer-dispatcher, but the current source is still a prototype with too much policy compiled into Rust. The improvements document is pointed at the correct target: JSON-driven reviewers, absolute paths, reliable dispatch, and consistent filmstrip framing.
What I agree with
The highest-priority sequence should be:
1. Prove one CLI reviewer end-to-end2. Move reviewers/paths/templates into JSON3. Preserve prompt fidelity4. Centralize frame writing5. Harden browser observation6. Add runtime reload/control
Do not start with Chrome scraping, Postgres, or runtime control. The boring CLI path must be bulletproof first.
Key findings


REVIEWERS, PORT, CDP_URL, tmux sessions, tab matches, entry files, and repo assumptions are hardcoded in main.rs. That directly conflicts with the JSON-driven objective.


shell_safe() is destructive. It strips backticks, $, pipes, semicolons, ampersands, and newlines. That will corrupt code blocks, Markdown, shell commands, and exact review prompts. For tmux, use tmux load-buffer + paste-buffer, not sanitized send-keys.


Browser scrape currently depends on fixed sleeps and hardcoded selectors. It needs a state machine: pre-dispatch marker, wait for new response, detect streaming completion, scan error triggers, validate signal, then write.


Scraped responses are written directly to the reviewer file, not consistently wrapped through the filmstrip frame system. Frame construction should be one shared function used by CLI, browser, and self-push flows.


Queue state is memory-only. Fine for Phase 1, but daily gauntlet use needs restart-safe state: active reviewer, subject frame, queue position, and last error.


Git operations need a small helper that checks pull, add, commit, and push status. Ignoring commit failure while retrying push can produce false success/failure ambiguity.


Recommended build order
Phase A — Minimal reliable CLI dispatch- Add real plaza-ant.json- Load reviewers from JSON- Add enabled flag- Use absolute paths- Replace shell_safe/send-keys with tmux load-buffer/paste-buffer- Test exactly one CLI reviewer end-to-endPhase B — Config hardening- Per-reviewer prefix/postfix- Strict placeholder validation- Configurable branch/repo/tape/blessings paths- Centralized frame writerPhase C — Browser observer- Configurable tab_match, selectors, error triggers, timeouts- Streaming-complete detection- Scrape validation before commit- Structured result enum, not boolPhase D — Control plane- Persist queue state- SIGHUP or plaza_control reload- plaza-ctl status/pause/resume/reload
Core recommendation
Make the config schema the contract. Let Rust become the executor of that contract.
plaza-ant should not “know” who ChatGPT, Codex, Gemini, Airy, or Ara are in code. It should know reviewer types, transports, paths, templates, state, and outcomes.
Bottom line: approve the improvement plan, but narrow the next milestone. One configured CLI reviewer, exact prompt preserved, review written, commit pushed, queue advanced. Everything else hangs off that spine.