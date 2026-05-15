# Codex CLI Vale Full System Review: plaza-ant

Date: 2026-05-14

## Point of View

The proposed direction is correct: plaza-ant should become a JSON-driven observer-dispatcher, not a compiled list of reviewers plus a few runtime toggles. The current source is a useful prototype, but it is not yet the v3 system described by the improvement plan. It can receive filmstrip events, queue reviewers, send prompts through tmux or CDP, and push scraped browser responses, but the operational contract is still hardcoded into `main.rs`.

The core risk is not one individual bug. The risk is that dispatch, prompt construction, browser scraping, git writes, branch selection, and recovery all cross each other without a stable system boundary. The next work should establish those boundaries before adding more automation.

## System Findings

The config layer is missing in practice. The v3 spec describes `PlazaConfig`, `TargetConfig`, `FilmstripConfig`, and JSON-backed reviewers, but I could not find a committed `plaza-ant.json` in this checkout. `REVIEWERS`, `PORT`, `CDP_URL`, tmux session names, tab matching, scrape mode, entry files, Cody session, repo path, branch, commit behavior, selector logic, timeout values, and prompt wording are still compiled into the binary.

The Rust struct design is not flexible enough for the target. `ReviewerConfig` contains static string references and a `DispatchMethod` enum with `Cdp { scrape: bool }`. That works for a prototype, but it is the wrong shape for runtime config. Use owned strings loaded from JSON, validate the config at startup, and model reviewer behavior explicitly as `cli`, `browser_scrape`, and `self_push`. Avoid encoding product behavior in booleans.

Prompt templating is currently the weakest user-facing path. `dispatch_reviewer` builds one shared cookie-cutter prompt for all reviewers, then appends hardcoded CLI or self-push instructions. That does not satisfy Phase 2.1. The template engine should render per-reviewer `prompt_prefix` and `prompt_postfix` against a known context: `repo_path`, `branch`, `tape_file`, `blessings_dir`, `entry_file`, `name`, `display_name`, `github_tape_url`, `github_blessings_url`, `subject_frame`, `topic`, and decoded content. Unknown placeholders should fail fast.

CLI dispatch still corrupts prompts. `shell_safe` strips backticks, dollar signs, semicolons, pipes, ampersands, and newlines before `tmux send-keys`. Since `Command::new("tmux").args(...)` is not shell interpolation, this is over-sanitizing and loses Markdown, code, commands, and structure. Keep the two-step "text first, Enter later" timing, but deliver exact bytes through `tmux load-buffer` / `paste-buffer` or a temporary prompt file.

The browser observer is not yet an observer state machine. It uses fixed sleeps, hardcoded JS selectors, a hardcoded ignored URL fragment, and "last assistant response" scraping. It does not take a pre-dispatch marker, does not verify a new response belongs to the current prompt, does not scan per-reviewer error triggers, and does not return structured success or failure to the queue. Phase 3 should start by making browser dispatch return `DispatchOutcome`, not by adding more selectors inline.

Queue semantics need explicit outcomes. `dispatch_reviewer` returns `bool`, where true means "advance immediately." That hides important cases: sent and waiting for callback, browser scrape succeeded, reviewer skipped, tmux session missing, CDP tab missing, scrape failed, git push failed, and pause required. Use a small enum so queue advancement is controlled by state, not comments.

Git/file handling needs a shared sink. Browser scrape writes raw text directly to the entry file, does not frame it, hardcodes `$HOME/dodo-bird-c3-tion-777333`, ignores commit failure, does not check pull status, and pushes without a configured branch. CLI and self-push reviewers receive different hardcoded instructions. Build one `ReviewSink` that takes a validated target config, writes framed content, runs git commands with checked statuses, and reports a structured result.

Frame handling is not centralized. The improvement plan asks for configured `frame_header`, `frame_footer`, and `frame_counter_file`; the source does not implement those. Plaza-ant should not have multiple ways to write reviews. Whether a response comes from browser scrape, CLI callback, or self-push relay, frame creation should be one code path.

Runtime state is volatile. `queue`, `active_reviewer`, `subject_frame`, and reviewer online/offline status live in memory only. That is acceptable for the first CLI proof, but it is not enough for daily gauntlet use. Persist active cycle state once dispatch is stable, and add admin actions for `pause`, `resume`, `reset`, `reload`, and `status` with clear status fields.

The admin API should use structured JSON responses. Current status output is formatted by hand and only reports reviewer status plus queue length. It should include active reviewer, subject frame, paused flag, last dispatch time, last error, config generation, and pending reviewers. Use `serde::Serialize` instead of string-building JSON.

The hand-written base64 decoder is unnecessary risk. Use the `base64` crate. Decoding filmstrip content is protocol plumbing, not business logic.

## Recommended Build Order

1. Commit the real JSON config and load it at startup. Validate paths, required fields, branch, reviewer names, duplicate entries, and type-specific requirements.
2. Replace `REVIEWERS` and hardcoded operational constants with config. Keep `/admin` online/offline as runtime override on top of config `enabled`.
3. Prove one CLI reviewer end to end with exact prompt preservation, absolute paths, branch-aware git instructions, callback handling, and no browser code in the critical path.
4. Add strict per-reviewer template rendering. Treat missing and unknown placeholders as dispatch errors.
5. Introduce `DispatchOutcome` and make queue advancement depend on explicit results.
6. Build the shared framed `ReviewSink` for write, add, commit, pull/rebase policy, and push.
7. Refactor browser dispatch into a real observer: tab health check, pre-send marker, prompt insertion, send, streaming detection, error trigger scan, scrape selector, response validation, and structured outcome.
8. Persist active cycle state and add admin `pause`, `resume`, `reset`, `reload`, and detailed `status`.
9. Add SIGHUP or bus reload only after config validation and state persistence exist.
10. Defer Postgres/Cognee until the JSON system is stable and boring.

## Parallelizable Work

Config schema and validation can be built in parallel with the CLI prompt transport fix. Browser selector research can happen in parallel, but browser automation should not be wired into the main queue until `DispatchOutcome` and `ReviewSink` exist. Runtime control should wait until the config and queue model settle.

## Verdict

Plaza-ant is pointed at the right architecture, but the next implementation step should be a narrow systems refactor rather than more feature accretion. Establish the config contract, exact prompt transport, explicit dispatch outcomes, and one shared framed write/push sink. Once those are in place, browser observation and hot reload become tractable instead of fragile.
