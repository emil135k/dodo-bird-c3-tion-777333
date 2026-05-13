BEGIN FRAME #1 | 2026-05-12 22:30 ET | codex_vale -> Cody | plaza-ant peer review

## Verdict

The v3 direction is right: plaza-ant should become a JSON-driven observer-dispatcher. The current source is useful as a working prototype, but reviewer behavior is still encoded in Rust constants, match arms, hardcoded paths, selectors, tmux sessions, branch assumptions, and scrape logic. That means operational changes still require code edits and redeploys.

## Struct Design

Use the spec's `PlazaConfig` / `ReviewerConfig` shape, but make `reviewer_type` an enum instead of a free string:

- `Cli { tmux_session }`
- `Browser { tab_match, scrape_selector, error_triggers, on_error }`
- `SelfPush { tab_match }`

Keep reviewer identity, prompt templates, enabled flags, target paths, branch, filmstrip paths, selectors, timeout/retry values, and error triggers in JSON. Code should dispatch by capability, not by reviewer name. Adding a reviewer should be a config-only operation.

## Observer Pattern

The browser observer should be a state machine:

1. find tab
2. health snapshot / input present
3. inject prompt
4. observe streaming / errors / timeout
5. scrape final response
6. validate length/content
7. write frame
8. git add/commit/push
9. advance queue

Do not rely only on fixed sleeps. The current `20s + polling` scrape can race slow responses and can scrape stale content. Track pre-dispatch last-message identity or text hash, then wait until a new assistant response completes.

## Untangled Path

Build order should be:

1. Load JSON config at startup and replace `REVIEWERS`, `PORT`, `CDP_URL`, repo path, branch, and tmux session constants.
2. Add `enabled` and full-path prompt rendering for CLI reviewers only. Prove one-reviewer dispatch end to end.
3. Move filmstrip writing and git commands behind one `ReviewSink` function that takes config paths.
4. Add browser observer health/error detection.
5. Add scraping and framed writes.
6. Add reload/status controls after the stable config model exists.

This avoids mixing config refactor, CDP behavior, scraping, and runtime control in one change.

## Hardcoded Values To Move

Move these into JSON config: port `3005`, CDP URL `http://localhost:9222`, tmux sessions, Cody session name, tab match strings, selectors, ignored URL fragments like `codex/cloud`, repo path, branch, entry paths, commit messages, prompt text, scrape timeouts, retry counts, minimum/maximum scrape lengths, token header name, and reviewer online defaults.

Also replace the manual `base64_decode` with the `base64` crate. The current decoder is easy to get subtly wrong and is not worth owning.

## Risks And Gaps

The biggest risk is queue correctness. Duplicate callbacks, stale browser scrapes, failed git pushes, or a missing tmux session can leave `active_reviewer` inconsistent. Persist enough run state to recover or expose a manual reset/status command.

The second risk is prompt corruption. `shell_safe` removes characters and collapses newlines, which can damage source snippets and instructions. Prefer `tmux load-buffer` / `paste-buffer` or a temp file paste path for CLI dispatch.

The third risk is browser brittleness. DOM selectors and error words must be per-reviewer config, and the observer should log the matched trigger/selector so failures are diagnosable.

## Recommendation

Do not rewrite plaza-ant all at once. First make current CLI dispatch config-driven and testable. Then add observer behavior behind explicit `BrowserReviewer` config. Once those two are stable, runtime reload and plaza-ctl will be straightforward instead of compounding the current hardcoded assumptions.

END FRAME #1
