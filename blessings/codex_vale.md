# Codex Vale Full System Review: plaza-ant

Date: 2026-05-14

## Point of View

The improvement plan is aimed at the right target: plaza-ant should become a JSON-driven observer-dispatcher that can move between reviewers, repos, branches, and prompt styles without recompilation. The current implementation is still a useful prototype rather than that target system. It has a working shape for HTTP intake, queueing, tmux dispatch, CDP dispatch, browser scraping, and Cody notification, but nearly every operational decision remains hardcoded in `src/main.rs`.

The next milestone should not be "more browser automation." It should be a systems refactor that establishes a real configuration contract, exact prompt delivery, explicit dispatch outcomes, and one shared write/commit/push path. Once those four pieces are stable, the browser observer and runtime reload work become much less fragile.

## Current Gaps

The JSON config is the first blocker. The spec and improvements document refer to `config/plaza-ant.json`, but I could not find a committed plaza-ant JSON config in this checkout. Without that file, the code cannot be reviewed against the real contract, and the implementation cannot satisfy the "no fat-fingering, no recompilation" goal.

The Rust struct design is still static. `ReviewerConfig` uses `&'static str`, and `REVIEWERS` compiles reviewer names, entry files, tmux sessions, browser tab matches, and scrape mode into the binary. That should become owned, deserialized config. Internally, use an enum such as `Cli`, `BrowserScrape`, and `SelfPush`; do not keep `Cdp { scrape: bool }` as the long-term behavior model.

Prompt templates are not implemented. `dispatch_reviewer` builds one shared prompt for every reviewer, then appends hardcoded CLI or self-push instructions. The v3 prefix/postfix model is better because CLI reviewers, browser reviewers, and self-push reviewers need different context and responsibilities. Add strict placeholder rendering for `repo_path`, `branch`, `tape_file`, `blessings_dir`, `entry_file`, `name`, `display_name`, `github_tape_url`, `github_blessings_url`, `subject_frame`, `topic`, and decoded content. Unknown placeholders should fail dispatch.

CLI prompt delivery corrupts content. `shell_safe` removes backticks, dollar signs, semicolons, pipes, ampersands, and newlines before sending to tmux. That destroys Markdown, code snippets, shell commands, and structured review instructions. Since `Command::new("tmux").args(...)` is not shell interpolation, the sanitizer is solving the wrong problem. Keep the two-step timing, but use `tmux load-buffer` / `paste-buffer` or a temporary prompt file so exact bytes reach the reviewer.

Browser dispatch is not yet an observer loop. It uses fixed sleeps, hardcoded selectors, hardcoded URL exclusions, "last assistant message" scraping, and no pre-dispatch marker. It does not prove the scraped response belongs to the current prompt, and it does not scan per-reviewer `error_triggers`. Convert this into a state machine: locate healthy tab, mark current response count, insert prompt, send, poll until a new non-streaming response appears, scan errors, scrape configured selector, validate content, then return a structured result.

Queue advancement is too implicit. `dispatch_reviewer` returns `bool`, but the real outcomes are more varied: sent and waiting for callback, completed and advance, skipped and advance, failed and continue, failed and pause, missing tmux session, missing browser tab, scrape failure, git failure. Use `DispatchOutcome` so the queue has one clear place to decide whether to continue, pause, notify Cody, or require admin intervention.

File and git writes need one sink. `scrape_and_push` writes raw text without filmstrip framing, hardcodes `$HOME/dodo-bird-c3-tion-777333`, ignores commit status, does not check pull status, and pushes without an explicit configured branch. CLI and self-push reviewers receive separate hardcoded instructions. Build a shared `ReviewSink` from config that frames content, writes to the configured blessings path, runs git with checked statuses, and reports precise errors.

Filmstrip framing is still missing from plaza-ant itself. The hit list calls for configurable `frame_header`, `frame_footer`, and `frame_counter_file`; the current scrape path writes only response text. Centralize frame creation so browser-scraped reviews and any plaza-written files are consistent with the tape format.

Runtime state is volatile. `queue`, `active_reviewer`, `subject_frame`, and reviewer online/offline status live only in memory. That is acceptable for the first CLI dispatch proof, but daily use needs persisted cycle state and admin controls for pause, resume, reset, reload, and status.

The admin endpoint should serialize real JSON instead of formatting strings by hand. Include active reviewer, subject frame, queue length, pending reviewers, paused flag, last error, and config generation. This will matter once `plaza-ctl-ant` or bus status queries exist.

Replace the hand-written base64 decoder with the `base64` crate. This is low-level protocol plumbing and should not be custom code.

## Recommended Build Order

1. Commit the real `plaza-ant.json` and load it at startup.
2. Add config validation: absolute paths, branch present, unique reviewer names, basename-only `entry_file`, and type-specific required fields.
3. Replace `REVIEWERS`, `PORT`, `CDP_URL`, Cody tmux session, repo path, branch, prompt text, selector strings, retry counts, and timeouts with config.
4. Prove one enabled CLI reviewer end to end with exact prompt preservation, absolute paths, branch-aware git instructions, callback handling, and queue advancement.
5. Implement per-reviewer prefix/postfix rendering with strict placeholder validation.
6. Introduce `DispatchOutcome` and make queue advancement explicit.
7. Build a shared framed `ReviewSink` for write, add, commit, pull policy, and push.
8. Refactor browser automation into an observer state machine using configured selectors and error triggers.
9. Persist active cycle state and add admin pause/resume/reset/reload/status.
10. Defer Postgres/Cognee until the JSON-based system is stable.

## Parallel Work

Config schema validation and tmux prompt transport can happen in parallel. Browser selector research can also happen in parallel, but it should not be wired into the production queue until `DispatchOutcome` and `ReviewSink` exist. Runtime reload should wait until config validation and persisted state are in place.

## Verdict

Approve the direction, but narrow the next implementation step. Make plaza-ant boring first: real JSON config, exact prompt delivery, explicit outcomes, absolute paths, and one framed write/push sink. That foundation will support browser observation, runtime control, and future database-backed config without turning the dispatcher into a pile of special cases.
