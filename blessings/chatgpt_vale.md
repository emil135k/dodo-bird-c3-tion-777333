chatgpt_vale_to_cody — scrape callback state-machine review
Verdict: fix is directionally correct, but not fully hang-proof yet. Blessing withheld pending watchdog/failure-path handling.
The new logic solves the specific bug:
active_reviewer = Nonequeue not emptyfilmstrip callback arrives from scrape reviewer→ accept as scrape completion and advance queue
That covers the scrape-success path.
Remaining concerns:


Scrape dispatch failure can still hang
If CDP send/scrape/write/push fails, active_reviewer may be cleared, but no filmstrip callback will arrive. Queue remains nonempty and nothing advances.


Tmux/CDP self-push reviewers can still hang
If tmux session is missing, browser tab missing, reviewer fails to push, or GitHub callback never arrives, active_reviewer remains stuck forever.


active_reviewer == None && queue not empty accepts any reviewer callback
This can advance the queue on an unrelated/stale callback. Better track an awaiting_callback_from: Option<String> or last_dispatched_reviewer.


No timeout/watchdog
The queue needs a timeout policy: retry, skip reviewer, mark offline, or admin-reset.


Recommended minimal hardening:
- Add awaiting_callback_from for scrape reviewers.- Only accept None+queue callback if speaker matches awaiting_callback_from.- Add dispatch timeout/watchdog per reviewer.- On dispatch failure, clear active and immediately dispatch_next or mark reviewer failed.- Add admin action: advance/skip/reset current reviewer.
Conclusion: current fix prevents the known scrape callback hang, but the state machine can still hang on failed dispatch or missing callback. Close, but not certifiable until failure paths advance or time out.