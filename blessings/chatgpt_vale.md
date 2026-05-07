Plaza-ant is solid in concept, but a few source-level issues need tightening:


handle_plaza() clears plaza.queue whenever event.speaker == "cody". A new Cody frame arriving mid-dispatch can silently drop pending reviewers from the prior frame. Add per-frame queues or reject/serialize overlapping Cody frames.


dispatch_cdp() checks result.is_err() and returns, then later has a second if let Err(msg) = result; the second check is unreachable/redundant. Simplify the control flow.


scrape_and_push() builds a shell command with interpolated dir, file, and msg. commit_msg is currently derived from config, but this should still use Command arguments or stronger escaping. Avoid bash -c for git plumbing.


scrape_and_push() writes scraped text directly to the blessing file. Add minimum validation: non-empty, max size, frame number present, and no obvious prompt echo before committing.


raw_cdp_evaluate() always uses CDP message id: 1. Fine for fresh one-shot sockets, but fragile if reused later. Generate monotonically increasing IDs or keep the one-shot invariant documented.


poll_update_file_button() / poll_and_click_button() appear unused in the current scrape path. Remove dead code or wire it intentionally; otherwise it obscures the actual reviewer-return mechanism.


shell_safe() strips backticks and $ but only protects tmux prompt content, not the later git shell command. Rename it to tmux_shell_safe or avoid implying global shell safety.


Verdict: architecture is good, but queue idempotency and shell-command hardening are the two highest-priority fixes before trusting this as durable orchestration infrastructure.