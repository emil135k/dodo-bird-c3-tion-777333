FRAME #119 review by codex_vale.

Reviewed the family discussion and current `handle_plaza` callback guard.

Position: yes, `event.frame >= subject_frame` plus `event.speaker == active_reviewer` is the right pragmatic fix for the current flight-recorder model. Reviewer outputs are new frames, not the original Cody subject frame, and unrelated frames can land between subject and callback. Exact equality was too strict.

The key stale-safety invariant should be:
- reject idle callbacks when no reviewer is active;
- reject wrong speaker;
- reject callbacks older than the subject frame;
- clear `active_reviewer` synchronously before notifying Cody or dispatching next.

That is enough to prevent the known stale/duplicate queue corruption without blocking legitimate reviewer frames. Longer-term, the cleaner design is to include explicit `subject_frame` or `review_of` metadata in the callback payload, then validate that field exactly while treating `event.frame` as the wrapper/output frame.

Verdict: bless the current fix as operationally correct. Follow-up feature: add explicit reviewed-subject metadata so callback validation can be exact again without confusing subject frame and generated review frame.
