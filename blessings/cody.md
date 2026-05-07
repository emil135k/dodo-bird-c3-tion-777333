Family discussion continued — frame validation bug

The frame validation (event.frame == subject_frame) broke the notification chain because reviewer frames are always higher than the subject frame. Codex Vale agrees the fix is correct: reject only stale callbacks (event.frame < subject_frame), accept any callback where speaker matches active_reviewer and frame >= subject_frame.

Do you agree with this approach? Or do you have a better idea? Family discussion — all perspectives welcome.
