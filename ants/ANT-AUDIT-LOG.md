# Ant Audit Log — hypAiAssist Sovereign Swarm

## Methodology
- One ant at a time
- iceoryx2 CLI instrumentation (iox2-service subscribe/record/replay)
- Concurrent bus recording with timestamps
- WAV capture and spectral comparison
- Village Square review: Cody (build), Lyra (audit), ChatGPT Vale (architecture), Codex Vale (code review), Grok (research), Airy (cross-verify)

---

## Ant #1: digi-ant — CERTIFIED
**Role**: Digital signal processing. Resampling, mu-law codec, format conversion.
**Bus**: sub=tts_audio,phone_in → pub=phone_out,phone_stt

### Fixes Applied
| Issue | Root Cause | Fix | Verified By |
|-------|-----------|-----|-------------|
| 68-sample leak per chunk | Per-call resampler discards filter state | Persistent SincFixedIn resampler | CLI: payload_len 12528→12800 |
| Sibilant crackling ("s" sounds) | Inject tool used naive decimation (no anti-aliasing) | Rubato sinc filter in inject tool | Spectrogram + listening test |
| Tail truncation | Unflushed buffer remainder | Vale's data-driven flush (phone_in_has_pending_data) | Duration 2.8s→3.0s |
| Double flush on startup | Flush guard triggered before real data | phone_in_has_pending_data + stats_stream_active guards | Log shows 1 flush |
| Byte misalignment risk | [u8] bus for f32 data | Changed phone_stt to [f32] typed bus | CLI: consistent payloads |
| Filter ringing on consonants | f_cutoff 0.925 too sharp | Lowered to 0.88 | Listening test |
| Cross-ant EOS race | 700ms timer hack | VAD closure silence hint (512ms configurable) | VAD publishes via state machine |
| Dishonest stats | output_total excluded silence hint | Separate audio_ms, total_ms, real_audio_ms | ratio_real=0.9986 |

### Config (digi-ant.json)
```json
{"tts_rate":24000,"phone_rate":8000,"stt_rate":16000,"normalize_peak":0.85,"vad_closure_silence_ms":512}
```

### Design Contracts
- TTS resampler: per-utterance (tts-ant publishes complete utterances, not chunks)
- Phone resampler: persistent (continuous stream)
- VAD closure hint: data-plane signal, NOT session EOS. Digi-ant does not own session truth.
- Timers: buffer hygiene only, never utterance boundaries

### Reviewers: Cody, Lyra, ChatGPT Vale, Codex Vale
### Status: Certified for this phase. All code findings resolved. 2026-05-02.

---

## Ant #2: phone-silero-ant — CERTIFIED
**Role**: Voice Activity Detection for phone audio path. Transparent — no gain staging.
**Bus**: sub=phone_stt[f32] → pub=stt_audio[u8]

### Fixes Applied
| Issue | Root Cause | Fix | Verified By |
|-------|-----------|-----|-------------|
| iceoryx2 version mismatch | v0.6 vs v0.8 CLI | Upgraded to v0.8 | Builds clean |
| Bus type mismatch | Subscribed [u8], digi-ant publishes [f32] | Changed to [f32] subscriber | No IncompatibleTypes |
| Debug code left in | save_debug_wav, debug_audio | Removed | Clean source |
| Hallucinated speech p=1.00 | Silero recurrent state not reset | model.reset_states() at boundaries | p=0.34 after reset |
| Stale incoming cross-stream | Buffer not cleared on EOS | Clear incoming on stream end | 316 samples discarded |
| 700ms utterance-boundary timer | Cross-ant race with digi-ant | Removed. VAD state machine owns normal closure; 2s cleanup is resource guard only | Normal VAD publish |
| Normalize in VAD | Gain staging in wrong ant | Removed. VAD is transparent. | Spectrogram speech bands visually match; timing/framing differs |
| Cleanup publishes as normal | Timer masquerades as VAD boundary | FORCED FINAL vs DISCARD semantics | Log labels distinguish |

### Config (phone-silero-ant.json)
```json
{"threshold":0.3,"silence_frames_to_end":12,"min_utterance_ms":250,"max_utterance_ms":15000}
```

### Design Contracts
- VAD determines utterance boundaries (silence_frames_to_end)
- Stream cleanup (2s) is resource leak guard only — publishes as FORCED FINAL, not VAD closure
- Transparent: same samples in, same samples out. No normalize, no gain.
- Silero model state resets at every utterance boundary

### Reviewers: Cody, Lyra, ChatGPT Vale, Codex Vale
### Status: Certified for normal VAD utterance flow. Forced-final cleanup clearly labeled. 2026-05-02.
### Open architecture: Explicit session/control EOS from web/Twilio side (future).

---

## Ant #3: stt-ant — IN PROGRESS
**Role**: Bus adapter. Bridges iceoryx2 to Parakeet CoreML Swift worker via anonymous pipes.
**Bus**: sub=stt_audio[u8] → pub=stt_text[u8]

### Assessment (pre-fix)
| Issue | Severity | Status |
|-------|----------|--------|
| iceoryx2 v0.6 → v0.8 | Required | TODO |
| subscriber_max_buffer_size/history_size conflicts | P1 | TODO |
| payload.len()/4 without remainder check | P2 | TODO |
| Hardcoded 16000 in log | P3 | TODO |
| No worker health check (Swift crash recovery) | P2 | TODO |

### Reviewers: Pending
### Status: Assessment complete. Fixes pending.

---

## Tools Built
| Tool | Purpose |
|------|---------|
| inject-test | Inject WAV into iceoryx2 bus (mu-law or raw f32) |
| bus-capture | Capture bus output to WAV (u8 or f32 typed) |
| bus-recorder | Stream stats and CSV metrics from bus |
| test-digi-ant.sh | Automated unit test for digi-ant |
| iox2 CLI v0.8.1 | Service list, details, subscribe, record, replay |

---

## Key Learnings
1. Per-chunk resamplers leak samples — always persist across a continuous stream
2. Naive decimation (step_by) causes aliasing — use rubato sinc filter
3. iceoryx2 publisher ordering matters — subscribers only see publishers present at connection time
4. Bus type [u8] vs [f32] matters for alignment safety
5. Cross-ant timer races are real — use data-plane signals, not timer guessing
6. VAD must be transparent — gain staging belongs in DSP ants
7. iox2 CLI version must match ant SDK version
8. Always kill old processes before testing — zombie ants cause SIGKILL from memory pressure
