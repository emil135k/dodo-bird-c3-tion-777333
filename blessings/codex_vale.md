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
