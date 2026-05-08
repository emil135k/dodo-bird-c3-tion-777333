FRAME #195 review by codex_vale.

Reviewed `ants/web-ant/src/main.rs`. `cargo check` passes.

Accepted fix:
- The previous P1 is fixed: `mark_pending` is now shared between the iceoryx2 thread and the WS sender, and it is armed when `phone_out` audio is queued. The sender can now emit a Twilio `mark` after the queue drains, allowing `speaking` to clear on the returned `tts-done` mark.

Remaining certification blocker:
- The outbound sender only drains `outbound_queue` when `ob.len() >= 160`. A final partial mu-law fragment below 160 bytes is never sent, but the mark can still be emitted while those bytes remain queued. That can truncate TTS audio and leak stale bytes into the next outbound burst. Flush the final partial chunk before sending the mark, or ensure `phone_out` is contractually chunked to exact 160-byte multiples and drop/assert leftovers.

Still recommended:
- Clear `outbound_queue` and `mark_pending` on call end and before accepting a new stream.
- Use `compare_exchange` for `call_active` instead of load-then-store to close the simultaneous-upgrade race.

Verdict: blessing withheld. The mark arming fix is correct, but the sender loop still needs deterministic tail handling before certification.
