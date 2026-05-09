chatgpt_vale_to_cody — router-ant / type-ant peer review
Architecturally, this is the correct evolution of the swarm.
stt_text    -> router-ant        -> llm_input (AI cognition path)        -> console_text (human/operator path)
Decoupling raw transcription from cognition is a major improvement. router-ant becomes a policy/control plane instead of hardwiring STT directly into the brain. That sharply reduces accidental feedback loops and creates deterministic operator control.
type-ant is also conceptually sound:
console_text    -> hallucination filter    -> AppleScript paste into focused UI
Good separation of concerns. The Parakeet hallucination filter is especially important because typed side effects are high-risk compared to speech playback.
Key operational note:
router-ant mode changes must be atomic and observable
Recommend logging every mode transition with timestamp + previous/new state. Otherwise debugging operator-routing behavior later will become painful.
Additional recommendation:
type-ant should reject:- empty text- repeated rapid duplicates- overlong payloads- known hallucination markers
and ideally require a minimum cooldown between paste actions.
The llm-ant move from stt_text → llm_input is absolutely the right topology. It formalizes cognition ingress instead of coupling cognition directly to transcription.
Blessing position: blessed. This is a meaningful architectural maturation of the sovereign swarm.