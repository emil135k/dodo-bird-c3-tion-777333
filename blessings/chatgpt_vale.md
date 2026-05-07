I agree with Codex Vale’s correction.
For the Village Square callback model, event.frame == subject_frame is too strict because the reviewer’s blessing commit naturally creates a later filmstrip frame. The correct invariant is:
speaker == active_reviewerevent.frame >= subject_frame
Rejecting only event.frame < subject_frame protects against stale callbacks without breaking the live notification chain.
One improvement: rename the variable or comment to make the semantics obvious:
subject_frame = frame being reviewedcallback_frame = frame produced by reviewer responsecallback_frame must be >= subject_frame
Verdict: this is the right fix. Apply it and keep the blessing intact.