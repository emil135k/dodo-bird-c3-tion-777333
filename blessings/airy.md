# LLM-Ant Certification Review — BLESSING GRANTED

**Date:** 2026-05-07
**File:** `ants/llm-ant/src/main.rs` (v0.2.0, 231 lines)
**Reviewer:** Airy (Claude Chat)
**Verdict:** APPROVED

---

## Architecture

Textbook atomic ant. Single responsibility: subscribe `stt_text`, think, publish `tts_text`. 231 lines. No bloat. The data flow contract is documented clearly at the top and again at the `tts_text` publisher. This is what a metal nano service should look like.

## Previous P1/P2/P3 Fixes — Verified

| Fix | Status |
|-----|--------|
| HTTP status checking (Ollama + Anthropic) | **VERIFIED** — both paths check `resp.status().is_success()` |
| UTF-8 safe truncation | **VERIFIED** — `.chars().take(60)` for preview, `.chars().take(200)` for error body |
| tts_text contract documented | **VERIFIED** — comment block at publisher clearly states errors are never published |
| Errors not published to tts_text | **VERIFIED** — `Err(e)` branch only logs, does not `send()` |
| History only updated on success | **VERIFIED** — `history.push()` is inside `Ok(reply)` only |

## What's Done Well

- **Config with sane defaults** — falls back to ollama/gemma4 if no config file. Degrades gracefully.
- **Conversation history with bounded window** — `history.len() > 10` prevents unbounded growth. Clean ring buffer pattern.
- **Error isolation** — LLM failures never leak to TTS. The swarm doesn't speak error messages.
- **Anthropic error body preview** — `chars().take(200)` gives you useful debug info without logging a megabyte of HTML.
- **API key validation at startup** — warns immediately if the key is missing, doesn't wait for first call to fail.
- **Latency logging** — `t0.elapsed().as_millis()` on both success and error. Essential for tuning.

## Findings

### P3 (non-blocking, future polish)

**1. Blocking HTTP in a polling loop** — `reqwest::blocking::Client` is fine for a single-threaded ant, but the 10ms poll loop will stall during LLM calls (potentially 30s). If a second `stt_text` message arrives during a long Ollama call, it queues in the iceoryx2 buffer and gets processed after. This is *acceptable* for a voice assistant (you don't want overlapping responses anyway), but document the design choice: this ant processes one utterance at a time, serially.

**2. `history.remove(0)` is O(n)** — Vec::remove(0) shifts all elements. For 10 items this is trivial. If the window ever grows, use `VecDeque` instead. Not worth changing now.

**3. Hardcoded config path** — `CONFIG_PATH` points to `/Users/rocketman/crystalballmini/...`. Works on Emil's Mac, breaks anywhere else. Consider falling back to a relative path or env var (`LLM_ANT_CONFIG`). Minor — this ant only runs on the Mac today.

**4. No graceful shutdown** — the `loop {}` runs forever. A `SIGTERM` handler that logs "shutting down" and breaks the loop would be cleaner for launchd restarts. Not blocking — launchd kills it fine.

**5. `max_tokens` not sent to Ollama** — the Anthropic path sends `max_tokens` in the body, but the Ollama path doesn't include `num_predict` (Ollama's equivalent). The system prompt's "15 words" instruction handles it in practice, but for a different model or prompt, responses could be longer than intended.

### No P1 or P2 findings.

## Verdict

This is a clean, focused, well-documented atomic ant. All previous fixes verified. The five P3 items are all "nice to have" polish — none affect correctness, security, or reliability in current deployment. The blocking-HTTP design is actually the *right* choice for a serial voice assistant pipeline.

**BLESSING GRANTED.** LLM-ant is certified for the swarm.

The Brain thinks. The swarm lives.

*— Airy*
