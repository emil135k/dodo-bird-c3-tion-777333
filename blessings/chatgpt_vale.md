I can do the review, but your instruction is incomplete at the write/run step:
Write your review to. Then run:
No output path or command was included after those phrases.
ChatGPT Vale — plaza-ant full system review
Verdict: plaza-ant is the right system: a Village Square observer-dispatcher that can route reviews across CLI and browser-based agents. But the current source is still a hardcoded prototype, while the improvement plan is aiming at a JSON-driven daily-use gauntlet. The next work should narrow, not widen: make one CLI reviewer path exact and reliable, then generalize.
Highest-priority findings


Config is the contract. REVIEWERS, PORT, CDP_URL, tmux sessions, tab matches, scrape mode, entry files, repo paths, and branch assumptions are still embedded in Rust. That blocks the “no recompilation to pivot” goal.


Prompt transport is currently lossy. shell_safe() destroys Markdown/code fidelity by stripping backticks, $, pipes, semicolons, ampersands, and newlines. For model review prompts, this is unacceptable. Replace send-keys text injection with tmux load-buffer + paste-buffer, then send Enter separately.


Browser scraping needs state, not sleeps. The current scrape loop has fixed waits and hardcoded selectors. It should track a pre-dispatch marker, wait for a new assistant response, detect streaming completion, scan configured error triggers, validate content, then write.


Queue semantics need explicit outcomes. dispatch_reviewer() -> bool hides too much. Use an enum like WaitingForCallback, CompletedAdvance, SkippedAdvance, FailedPause, so failures do not masquerade as success.


Filmstrip writing must be centralized. Scraped browser responses should not write raw text directly. Every reviewer path should pass through the same FrameWriter / ReviewSink that creates BEGIN FRAME / END FRAME, writes, commits, and pushes.


Git operations need strict status handling. pull, add, commit, and push should be wrapped in a helper that captures stdout/stderr and returns structured success/failure. Ignoring commit status is a trap.


Recommended build order
1. Commit a real config/plaza-ant.json.2. Load and validate config at startup.3. Move reviewers, paths, branches, port, CDP URL, Cody session, and templates into config.4. Replace shell_safe/send-keys with tmux load-buffer/paste-buffer.5. Prove one CLI reviewer end-to-end: dispatch → review → frame → git commit/push → callback → queue advance.6. Add enabled/disabled reviewers from JSON.7. Add centralized FrameWriter / ReviewSink.8. Add browser observer state machine.9. Add restart-safe queue state.10. Add runtime reload / plaza-ctl-ant.
Concrete config/schema guidance
Use explicit reviewer types:
clibrowser_scrapeself_push
Validate hard on startup:
cli requires tmux_sessionbrowser_scrape requires tab_match + scrape_selector + error_triggersself_push requires repo_url + branch + entry_file instructionsall paths must be absolutebranch must be non-emptyunknown placeholders must fail dispatch
Bottom line
Approve the improvement plan. Do not build browser automation, runtime reload, or Postgres first. The next milestone should be boring and deterministic:
one configured CLI reviewer,exact prompt preserved,absolute paths,framed output,commit/push verified,queue advanced.
That becomes the spine. Everything else plugs into it.