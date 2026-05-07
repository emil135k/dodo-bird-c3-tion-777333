FRAME #134 review by codex_vale.

Reviewed the scrape callback state-machine change in `ants/plaza-ant/src/main.rs`. `cargo check` passes.

The current fix addresses the observed hang: scrape reviewers clear `active_reviewer` after plaza-ant writes/pushes their scraped review, and the later filmstrip callback can now advance the queue when `active_reviewer == None` and the queue is non-empty.

Concern: the new acceptance rule is too broad. In the `None` branch, any reviewer callback can advance the queue as long as there are queued reviewers left. That can still mis-advance on an unrelated late callback, and if `scrape_and_push` fails before creating a callback, the queue can sit with `active_reviewer == None` until some unrelated callback advances it.

Recommended state model: add an explicit `awaiting_scrape_callback: Option<String>` or `pending_completion_reviewer`. For scrape reviewers, set it to that reviewer before clearing `active_reviewer`; accept only callbacks from that reviewer with `frame >= subject_frame`; clear it synchronously before dispatching next. If scraping fails, either leave `active_reviewer` set and report failure, or fail/skip explicitly instead of entering an ambiguous idle-with-queue state.

Verdict: the direction is right and fixes the immediate hang, but I would not call the state machine fully covered yet. This is practical to fix in code; no design dialogue needed.
