# Voice STT Integrity Architecture

Date: 2026-05-20

## Context

The Atomic Ant voice path is now functional enough for live dictation, but the session exposed a serious integrity problem: Parakeet can append fluent words that Emil did not say when Silero publishes audio with poor end boundaries.

This is worse than ordinary transcription error. A wrong word is tolerable. A whole invented phrase is not tolerable, especially when the future Jacob's Lattice/Postgres memory layer will categorize conversations, architectural decisions, code changes, and agent collaboration history.

Raw voice transcript must not be treated as canonical truth.

## Current Pipeline

```text
microphone
  -> patchbay-ant
  -> stt_raw
  -> silero-ant
  -> stt_audio
  -> stt-ant
  -> parakeet-worker
  -> stt_text
  -> router-ant
  -> console_text / llm_input / memory path
```

## What We Found

The problem was not simply "Parakeet is bad."

Observed behavior:

- Silero sometimes published audio segments with trailing speech-like material.
- Parakeet decoded the bounded audio it received.
- When the tail contained acoustic junk or speech-like residue, Parakeet could produce a fluent continuation.
- Dotted hallucination tails are easy to detect.
- Fluent hallucinated continuations are dangerous because they look like real language.

Example failure class:

```text
Emil stops at: "... at you"
System emits extra fluent words after that point.
```

This is a boundary and provenance problem before it is a language problem.

## Changes Made During Investigation

### silero-ant

Silero was tightened toward real utterance ownership:

- Added sustained speech start gating.
- Added configurable `start_threshold`.
- Added configurable `start_speech_frames`.
- Kept a lower continuation threshold for softer speech.
- Added VAD-end based tail trimming instead of trimming by last loud sample.
- Kept low-energy publish gating to avoid feeding obvious junk to Parakeet.

The important principle:

```text
Silero owns speech boundaries.
Parakeet should receive only bounded utterance audio.
```

### stt-ant

`stt-ant` was tightened around the `stt_text` contract:

- Added sanitation for known Parakeet dotted/uppercase hallucination tails.
- Added optional debug WAV dumping through `STT_DEBUG_DUMP_DIR`.
- The debug dump writes the exact `stt_audio` segment sent to Parakeet.

This is forensic tooling inside the ant, not an external workaround.

## Current Risk

The system is improved but not final.

Remaining risk:

- Fluent hallucinations cannot be reliably removed with text filters.
- If Parakeet invents normal-looking words, sanitation cannot know user intent.
- VAD logs showed cases where Silero stayed at high speech probability after Emil believed he stopped.
- We still need audio evidence for bad utterances to determine whether the tail is real acoustic content, echo, macOS dictation feedback, room noise, or decoder behavior.

## Memory Integrity Rules

For Jacob's Lattice, Postgres, pgvector, AGE graph memory, and agent collaboration:

```text
Raw STT is evidence, not truth.
Cleaned STT is a working transcript, not canonical memory.
Canonical memory requires distillation, provenance, and confidence.
```

Suggested data tiers:

1. Raw audio evidence
2. Raw STT output
3. Cleaned STT output
4. Agent-distilled interpretation
5. Confirmed canonical memory

Every stored memory should carry provenance:

- source device
- ant path
- utterance id
- VAD start/end metadata
- STT model
- confidence / review status
- whether the memory was confirmed or inferred

## Future Contract

The current Parakeet worker protocol is too thin:

```text
[i32 sample_count][f32 samples...]
```

Better contract:

```text
Utterance {
  id,
  sample_rate,
  sample_count,
  speech_start_sample,
  speech_end_sample,
  vad_threshold,
  vad_start_threshold,
  vad_confidence_summary,
  energy_summary,
  pcm_f32[]
}
```

Then `stt-ant` or `parakeet-worker` can hard-trim to `speech_end_sample` before transcription.

That is the real equivalent of:

```text
"Do not make up anything after this point."
```

There is no magic spoken phrase or text instruction that can force this through the current protocol. The audio boundary must be correct.

## Recommended Modes

### Hands-Free Mode

Default for conversational use.

- Silero detects start/end.
- Normal silence timeout.
- Good for brainstorming.
- Output can enter raw/clean transcript stores, but not canonical memory without distillation.

### Precision Command Mode

For command line, code changes, database memory, and architecture decisions.

Possible controls:

- push-to-talk
- release-to-send
- Enter-to-flush
- Hammerspoon hotkey
- explicit `speaker_control` bus flush

In this mode, a non-spoken control signal defines the utterance boundary. The user should not have to say "full stop" out loud.

## Design Direction

The next architectural step should not be more text hacks.

Priority order:

1. Capture exact bad utterance WAVs from `stt-ant`.
2. Listen/inspect whether extra words are acoustically present.
3. Add structured utterance metadata to the bus contract.
4. Make STT hard-trim to VAD speech end before Parakeet.
5. Preserve raw evidence separately from canonical memory.
6. Add precision dictation mode for high-stakes commands.
7. Reduce debug log noise once behavior is characterized.
8. Move temporary tmux-owned test sessions back into the normal daemon/launch path.

## Engineering Standard

Do not solve this class of bug by hiding it downstream.

Acceptable:

- drop uncertain tail audio
- mark low-confidence utterances
- preserve raw evidence for audit
- require confirmation for canonical memory
- provide precision mode for commands

Not acceptable:

- silently treating hallucinated STT as user intent
- storing raw transcript as canonical memory
- relying on type-ant as the cleanup layer
- pretending Parakeet is the sole cause without proving the audio boundary
- adding hidden probes or alternate paths that become architecture by accident

## Bottom Line

The voice path is usable, but it must become evidence-driven before it becomes a memory source.

For the future intelligence exoskeleton and Jacob's Lattice, the system must distinguish:

```text
what was heard
what was transcribed
what was inferred
what was confirmed
what became memory
```

That distinction is the difference between useful shared intelligence and a poisoned memory lattice.
