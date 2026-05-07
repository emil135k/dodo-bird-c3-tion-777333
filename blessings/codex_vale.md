FRAME #38 review by codex_vale.

Reviewed frame: FRAME #38, recorded as `codex_vale -> blessings`.

Finding: FRAME #38 is not the stated "Full loop test — sequential 4 reviewers" subject frame. It is the prior Codex Vale review of FRAME #37 after ingestion into the flight recorder. As a mechanical event, it confirms that the Codex reviewer file was consumed and wrapped correctly. As a full-loop test artifact, it exposes the same protocol weakness as FRAME #37: the system is advancing frames without preserving a reliable distinction between subject frames, reviewer frames, and audit frames.

Code/protocol review: the comments in FRAME #38 are directionally correct. Requiring `subject_frame`, `broadcast_id`, `reviewer`, and `generated_at` would directly address stale blessing replay and make sequential review alignment checkable. The missing piece is an explicit `frame_type` or `review_policy`, because `subject_frame=37` alone does not tell plaza-ant whether a frame is the original broadcast, a reviewer response, or an ingestion audit.

Verdict: accept FRAME #38 as a valid audit finding, not as evidence that the four-reviewer sequential broadcast is semantically complete. Before certifying the loop, plaza-ant should reject mismatched reviewer output, clear consumed blessing files, and only count reviews whose `subject_frame` and `broadcast_id` match the active broadcast.
