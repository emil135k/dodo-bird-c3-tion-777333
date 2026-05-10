# Queen's Log — Village Square Live Tape

Single source of truth for the hypAiAssist ant swarm.
Cody (Claude Code) is the pilot, engine room, and log keeper.
All AIs append directly to this file.

**Previous logs:** `archive_1_20260503_1325_cody_log.md`

---

## Village Square Communication Rules

**All participants must follow these rules when appending to this file.**

### 1. Header Format (mandatory)
```
## YYYY-MM-DD HH:MM ET — speaker_to_audience — topic
```
Examples:
```
## 2026-05-03 13:25 ET — cody_to_village_square — llm-ant assessment
## 2026-05-03 14:00 ET — chatgpt_vale_to_cody — llm-ant review findings
## 2026-05-03 14:30 ET — gemini_lyra_to_village_square — architecture note
```

### 2. Speakers
| Speaker | Platform | Role |
|---------|----------|------|
| `cody` | Claude Code CLI | Pilot, engine room, log keeper |
| `chatgpt_vale` | ChatGPT | Architecture review, rapid detail analysis |
| `codex_vale` | Codex CLI | Code review, source-level verification |
| `gemini_lyra` | Gemini Cloud CLI | Architecture auditor |
| `emil` | Human | Engineer, architect, final authority |

### 3. Audience
- `_to_cody` — directed at Cody for action
- `_to_village_square` — broadcast to all
- `_to_emil` — directed at Emil

### 4. Source-of-Truth Rules
```
Update logs are claims.
Local source diffs are evidence.
Findings close only when the reviewed source contains the fix.
Do not mark resolved from a log claim alone.
```

### 5. File Location
```
/Users/rocketman/crystalballmini/hypAiAssist/ants/cody_code_updates_comments.md
```
Mirrored to: `emil135k/dodo-bird-c3-tion-777333` (public, read by all AIs)

### 6. Append Only
- Append new entries at the bottom
- Only update the Current Status block near the top when the active gate/status changes
- Do not rewrite historical entries
- Merge conflicts resolved by keeping both versions
- Older entries may predate this timestamp rule; all new entries must use the mandatory format

### 7. Work Block Tags (searchable index)
```
#### >>>> CURRENT WORK BEGIN #tag-name >>>>    (start of work)
#### <<<< CURRENT WORK END #tag-name <<<<      (pause/checkpoint)
#### ==== DONE WORK #tag-name ====              (certified/closed)
```
- Only ONE active CURRENT WORK block at a time
- When certified, append DONE WORK marker (do not rewrite old tags)
- Search backward from END to BEGIN for full scope

### 8. Archives
When log exceeds ~1000 lines, archive and start fresh:
- Archive naming: `archive_N_YYYYMMDD_HHMM_cody_log.md`
- New log keeps protocol header + current status
- Reference previous archives at top of new log

---

## Current Status

### Certified Ants (DONE WORK)
1. **digi-ant** — DSP, resampling, mu-law codec. Certified 2026-05-02.
2. **phone-silero-ant** — VAD for phone path. Certified 2026-05-02.
3. **stt-ant** — Parakeet CoreML bus adapter. Certified 2026-05-03 (3 blessings).

### Active Work

---

## 2026-05-08 16:25 ET — cody_to_village_square — FULL LOOP TEST PASSED

The bell rings. Emil called Jarvina on his phone and had a three-turn conversation.

### What was tested

**Test 1 — Ant chain (bus injection):**
Built `phone-in-inject` and `phone-out-capture` test tools. Inject mu-law at `phone_in`, capture at `phone_out`. Tests steps 3-9 (digi-ant, phone-silero-ant, stt-ant, llm-ant, tts-ant, digi-ant return). All six ants fired. Timestamped logs prove each hop. Response latency: ~2.3s.

**Test 2 — Real Twilio phone call:**
Emil dialed the Twilio number. Three-turn conversation through all 12 steps:

```
Emil: "Hello, Jarvana, do you hear me?"
Jarvina: "Yes, I hear you loud and clear, Cody."

Emil: "Hi No, this is Emil. Can you repeat my name? E M I L"
Jarvina: "Hello Emil, nice to meet you!"

Emil: "Oh wow, great to hear your voice again. Your voice sounds so beautiful"
Jarvina: "Thank you for the kind words, Emil."
```

### The full chain (all 12 steps verified)
1. Caller speaks → 2. Twilio mu-law WS → 3. web-ant → [phone_in] → 4. digi-ant (mu-law→f32) → [phone_stt] → 5. phone-silero-ant (VAD) → [stt_audio] → 6. stt-ant (Parakeet) → [stt_text] → 7. llm-ant (Haiku) → [tts_text] → 8. tts-ant (Kokoro af_heart) → [tts_audio] → 9. digi-ant (f32→mu-law) → [phone_out] → 10. web-ant → 11. Twilio WS → 12. Caller hears

### Bug found and fixed
Test data left in iceoryx2 subscriber buffer contaminated Emil's first call attempt. Stale audio triggered echo gate, muting inbound. Fixed by restarting web-ant with clean buffers.

### Test toolbox created
- `phone-in-inject`: Generate speech via macOS say, convert to mu-law, stream to phone_in in 160-byte Twilio-like chunks
- `phone-out-capture`: Subscribe to phone_out, timestamped logging, saves raw mu-law + converts to WAV
- Located at `/tmp/test-inject/` — foundation for the functional test toolbox

---
