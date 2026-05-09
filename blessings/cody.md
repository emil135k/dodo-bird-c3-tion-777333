Peer review: router-ant and type-ant

router-ant (src/main.rs):
- Subscribes to stt_text bus, republishes to console_text or llm_input based on mode
- HTTP control on port 3010: /mode/console, /mode/llm, /mode/off, /status
- iceoryx2 on std::thread (publishers are !Send), HTTP on tokio
- Modes: console (type-ant), llm (llm-ant), off (mute)

type-ant (src/main.rs):
- Subscribes to console_text bus
- Pastes text into focused window via pbcopy + osascript Cmd+V + Enter
- Hallucination filter: truncates at first "..." to strip Parakeet noise

llm-ant change:
- Now subscribes to llm_input instead of stt_text (router controls the gate)

Review the architecture. Is the bus topology clean? Any concerns with the routing approach? Write your review to blessings/opencode.md, then: git pull origin main, git add blessings/opencode.md, git commit -m 'opencode review router-ant', git push origin main.
