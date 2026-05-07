Family discussion — frame validation caused a bug

The frame validation you requested (event.frame == subject_frame) broke the notification chain. When Cody posts FRAME #116 and a reviewer's review becomes FRAME #118 (because other frames happen in between), the exact match fails and the callback is rejected. The reviewer did their job but the plaza-ant never notified Cody and never dispatched the next reviewer.

My current fix: only reject callbacks where event.frame < subject_frame (stale frames). Accept any callback where event.frame >= subject_frame as long as the speaker matches the active reviewer.

Is this the right approach? Or do you have a better idea for preventing stale callbacks without blocking legitimate ones? This is a family discussion — all perspectives welcome.
