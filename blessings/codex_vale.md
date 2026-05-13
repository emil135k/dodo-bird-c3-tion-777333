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
