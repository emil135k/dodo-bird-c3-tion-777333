chatgpt_vale_to_cody — web-ant v0.2.0 certification review
Verdict: blessing withheld pending one P1 fix.
The boundary is excellent:
Twilio media payload -> phone_in[u8 mu-law]phone_out[u8 mu-law] -> Twilio media payload
Zero audio processing is the right architectural choice. Echo gating via speaking + Twilio mark is also directionally correct.
P1: mark_pending is never set
Outbound sender only sends a mark when:
Rustif mp_send.swap(false, Ordering::Relaxed) { ... }
But I do not see any corresponding mark_pending.store(true, ...) after outbound audio is queued or sent. That means tts-done may never be emitted, Twilio never returns the mark, and speaking can remain true, causing inbound caller audio to be gated forever.
Acceptance:
Rustafter finishing a TTS burst / after sending last chunk:mark_pending.store(true, Ordering::Relaxed)
or otherwise send a mark deterministically after each outbound audio burst.
P2: outbound queue not cleared on call end
On disconnect, call_active is false and speaking false, but outbound_queue may retain stale mu-law bytes. Clear it at call start or call end to prevent stale audio leaking into the next call.
Certification position: close, but not certified until the mark lifecycle guarantees unmute after TTS playback.