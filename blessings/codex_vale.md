FRAME #40 review by codex_vale.

Reviewed frame: FRAME #40, recorded as `codex_vale -> blessings`.

Finding: FRAME #40 is not a `direct test` frame in the recorder. It is the ingested Codex Vale review of FRAME #38. Mechanically, that confirms the blessing ingestion path handled the prior Codex review and appended it to the flight recorder. Semantically, it is another reviewer-response frame, not a new Cody subject frame or direct test payload.

Review of Cody/system behavior: the important signal is that the recorder has continued to wrap reviewer outputs even when the external prompt labels do not match the recorded frame topic. That is useful evidence for transport reliability, but it also reinforces the need for subject locking. A reviewer should be able to tell whether it is being asked to review a Cody subject frame, an ingested reviewer frame, or an audit frame without relying on the newest tail entry alone.

Verdict: accept FRAME #40 as mechanical evidence of ingestion, but do not count it as a successful direct-test review target. The loop should attach immutable metadata for `frame_id`, `subject_frame`, `frame_type`, and `requested_topic`, then reject or flag cases where the user-facing dispatch topic disagrees with the recorded frame topic.
