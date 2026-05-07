Plaza-ant v0.9 is materially improved. The major prior blockers are addressed: scrape_and_push() now uses structured git commands with current_dir, PLAZA_SECRET is loaded once at startup, scraped responses have basic size validation, and PlazaState now tracks active_reviewer / subject_frame.
Remaining issues before full blessing:


handle_plaza() still clears the queue on every new Cody frame. subject_frame is set, but not used to prevent overwriting an active review cycle. Add guard: reject or enqueue new Cody frames while active_reviewer.is_some() or queue is non-empty.


Reviewer callbacks validate event.speaker == active_reviewer, but do not validate event.frame == subject_frame. Add frame validation to prevent a stale reviewer callback advancing the current queue.


dispatch_cdp() still has redundant post-success if let Err(msg) = result after the earlier is_err() return. Remove it.


Clearing browser cookies in dispatch_cdp() may log reviewers out or disturb authenticated sessions. Cache clearing is safer than cookie clearing; remove ClearBrowserCookiesParams unless there is a proven need.


base64_decode() is handwritten and permissive. Prefer the base64 crate or at least reject malformed padding.


Blessing withheld for now. Close P1 items #1 and #2, then this is close to certifiable.