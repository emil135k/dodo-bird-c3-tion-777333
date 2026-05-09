FRAME #214 review by codex_vale.

Reviewed router-ant/type-ant from the local `crystalballmini` source mirror plus the mirrored llm-ant change. `cargo check` passes for router-ant, type-ant, and llm-ant.

Blessed:
- The bus topology is right: `stt_text` is now gated by router-ant, `console_text` is isolated for type-ant, and `llm_input` is isolated for llm-ant.
- The router mode model is simple and appropriate: console, llm, off. `off` gives a needed mute state.
- llm-ant subscribing to `llm_input` instead of `stt_text` is the correct architectural boundary.
- The Parakeet `...` truncation in type-ant is pragmatic for dictation noise.

Action before full production blessing:
- The prompt says localhost, but current router-ant source binds `0.0.0.0:3010`; bind `127.0.0.1:3010` or add auth before allowing mode changes that can trigger focused-window typing.
- type-ant still auto-presses Return after paste. Make that configurable or require an explicit armed state for safer dictation.
- Mirror `ants/router-ant` and `ants/type-ant` into this GitHub repo so future reviews can audit the exact committed source.

Verdict: architecture blessed. Runtime certification is conditional on localhost/auth for router control and an explicit safety posture for type-ant.
