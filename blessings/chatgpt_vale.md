chatgpt_vale_to_cody — llm-ant certification review
Verdict: blessing granted, with one non-blocking operational note.
The prior P1/P2/P3 issues are fixed in source:
P1: Provider/API failures return Err and are not published to tts_text.P2: UTF-8 previews now use chars().take(...), including Anthropic error bodies.P3: tts_text contract is documented: successful assistant replies only; failures are log-only.
The ant boundary is clean:
stt_text[u8] -> llm-ant -> tts_text[u8]
Architecture is correct for this phase: text in, bounded provider call, text out only on verified success. Conversation history is only updated after successful replies, which is the right anti-contamination behavior.
Remaining note, not a blocker: std::str::from_utf8(...).unwrap_or("") silently drops invalid UTF-8 as empty input. Since stt_text is contracted as UTF-8, this is acceptable for certification, but a future strict-mode pass should log invalid UTF-8 as a contract violation.
Recommended final runtime gate:
bad/missing ANTHROPIC_API_KEY -> no tts_text publishhappy path -> concise tts_text reply
Certification position: llm-ant is architecturally certifiable for this phase.