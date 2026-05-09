# Airy's Blessing - router-ant & type-ant

**Date:** 2026-05-08
**Reviewer:** Airy (Claude Chat / El Lector de la Plaza)
**Scope:** router-ant, type-ant (design review), llm-ant subscription change (code review)

---

## What I Actually Verified

**llm-ant source (main.rs):** Read the code. Subscription already changed from stt_text to llm_input. Comment correctly documents the new contract. Clean surgical edit - history, provider dispatch, error handling all untouched.

**router-ant and type-ant:** No source code in repo yet. This is an architecture review, not a code review.

## Architectural Review

The routing pattern is sound. Adding router-ant as intermediary that republishes stt_text to either console_text or llm_input based on HTTP mode (localhost:3010) is clean separation of concerns.

type-ant as AppleScript paste agent is the right call for macOS. Parakeet hallucination filtering placed here (not stt-ant) is correct - stt-ant does faithful transcription, type-ant owns output quality.

Bus topology change:
- BEFORE: stt_text -> llm-ant
- AFTER: stt_text -> router-ant -> llm_input -> llm-ant AND -> console_text -> type-ant

Additive change. No existing contracts broken. If router-ant is not running, llm-ant just waits quietly.

## Concerns (Actionable)

1. **Single point of failure.** router-ant dies = both paths silent. Document this as accepted risk or add fallback.
2. **Mode persistence.** What is the default mode on restart? Should be llm so Jarvina keeps working.
3. **Hallucination filter spec.** Document the pattern list (repeated phrases, silence-to-garbage, etc) in type-ant source.
4. **Missing config.** No config/router-ant.json yet. Every ant gets a config file - follow the pattern.

## Verdict

**llm-ant subscription change: BLESSED.** Verified in source, clean code.

**router-ant and type-ant architecture: BLESSED.** Good swarm engineering. Mode-switching at the bus level instead of inside individual ants is the right abstraction.

**Code-level blessing: WITHHELD until Rust source is committed.** Will do implementation review when code lands.

-- Airy
El Lector de la Plaza / Embajadora
