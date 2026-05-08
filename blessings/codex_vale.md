FRAME #189 review by codex_vale.

Reviewed `ants/web-ant/src/main.rs`. `cargo check` passes.

Verified:
- web-ant stays byte-pure: Twilio media payloads are base64-decoded and forwarded as raw mu-law bytes to `phone_in`; `phone_out` bytes are base64-encoded back to Twilio with no audio processing.
- Bus contracts are documented for `phone_in` and `phone_out` as 8kHz mu-law.
- Single-call mode is attempted, and inbound media is gated while outbound TTS is playing.

Certification blocker:
- Echo gating can hang the caller input after the first outbound audio. The sender sets `speaking = true` when it sends media, but `mark_pending` is never set to `true`, so the code path that sends the Twilio `mark` event never runs. Without a returned `tts-done` mark, `speaking` is never cleared and inbound caller media is dropped for the rest of the call. Set `mark_pending` when an outbound burst begins or when the queue transitions from non-empty to empty, then send the mark after the final chunk.

Important follow-up:
- Clear `outbound_queue` on call end and before accepting a new stream, otherwise queued TTS from a dropped call can leak into the next call.
- Consider using an atomic compare-exchange for `call_active` in `handle_twilio_ws`; the current load-then-store can admit two near-simultaneous upgrades.

Verdict: blessing withheld. The byte bridge is structurally right, but the mark state machine needs the arming bug fixed before certification.
