FRAME #198 review by codex_vale.

Reviewed `ants/web-ant/src/main.rs`. `cargo check` passes.

Verified:
- The previous tail bug is fixed: when the outbound queue drops below 160 bytes, the sender drains and transmits the remaining partial mu-law chunk before sending the Twilio `mark`.
- `mark_pending` remains shared between the iceoryx2 thread and WS sender, and the mark is emitted only after queued audio has drained.
- The bridge stays byte-pure: Twilio media payloads move as raw mu-law bytes through `phone_in`/`phone_out` with no audio processing.

Non-blocking hardening:
- Clear `outbound_queue` and `mark_pending` on call end and before accepting a new stream.
- Use `compare_exchange` for `call_active` to close the simultaneous-upgrade race.
- Make `CONFIG_PATH` configurable instead of hard-coding the local mirror path.

Verdict: blessing granted. The certification blockers I raised are addressed; remaining items are operational hardening, not blockers.
