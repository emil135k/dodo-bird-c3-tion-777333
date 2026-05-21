# hypAiAssist — Sovereign Ant Swarm Status

## Last Updated: 2026-05-12

## Architecture — THE NERVOUS SYSTEM
- HyperAI is NOT an assistant — it is a nervous system. Ants are neurons.
- See `emil/the-nervous-system-vision.md` for full vision.
- **Branch**: `hypaiassist/iceoryx2` on crystalballmini
- **Public mirror**: `emil135k/dodo-bird-c3-tion-777333` (dodo-bird)

## Certified Ants (Village Square blessed)

| Ant | Version | Bus In | Bus Out | Status |
|-----|---------|--------|---------|--------|
| digi-ant | v0.1.0/iox2 v0.8 | tts_audio[u8], phone_in[u8] | phone_out[u8], phone_stt[f32] | CERTIFIED |
| phone-silero-ant | v0.2.0/iox2 v0.8 | phone_stt[f32] | stt_audio[u8] | CERTIFIED |
| stt-ant | v0.2.0/iox2 v0.8 | stt_audio[u8] | stt_text[u8] | CERTIFIED |
| llm-ant | v0.2.0/iox2 v0.8 | llm_input[u8] | tts_text[u8] | CERTIFIED |
| tts-ant | v0.2.0/iox2 v0.8 | tts_text[u8] | tts_audio[u8] | CERTIFIED |
| web-ant | v0.2.0/iox2 v0.8 | phone_out[u8] | phone_in[u8] | CERTIFIED |
| patchbay-ant | v0.4.0/iox2 v0.8 | tts_audio[u8], speaker_control[u8] | stt_raw[u8] | AEC WORKING (Swift worker) |
| silero-ant | v0.3.0/iox2 v0.8 | stt_raw[u8] | stt_audio[u8] | CERTIFIED (crashes on iox2 corruption) |

## New Ants (2026-05-08 through 05-12)

| Ant | Purpose | Status |
|-----|---------|--------|
| router-ant | Routes stt_text to console_text/llm_input/airy based on HTTP mode (port 3010) | Working |
| type-ant | Pastes console_text into focused window via AppleScript | Working |
| speaker-ctl-ant | Publishes FLUSH/PAUSE/RESUME to speaker_control bus | Working |
| plaza-ant | Village Square dispatcher — JSON config, dual-path (local+GitHub) | v2.0 built, needs E2E test |
| bridge-ant | Audio bridge Twilio-browser via BlackHole (WIP) | Scaffolded |
| cdp-ant | Browser AI text bridge via CDP (WIP) | Scaffolded |
| opencode | Village Square reviewer via Ollama/gemma4 | Working |

## AEC Solution — SOLVED (2026-05-11/12)
- **SpeexDSP FAILED** after 8+ hours: manual delay tuning, resampling mismatches, CoreAudio digital loopback
- **SOLUTION**: Swift worker with AVAudioEngine + `setVoiceProcessingEnabled(true)` — Apple hardware AEC
- **Key settings**:
  - `isVoiceProcessingAGCEnabled = false` — stop Apple from controlling mic gain
  - `voiceProcessingOtherAudioDuckingConfiguration = .init(enableAdvancedDucking: false, duckingLevel: .min)` — stop Apple from ducking system audio
  - macOS Control Center mic mode MUST be "Standard" (NOT Voice Isolation)
- **Architecture**: Rust patchbay-ant spawns Swift patchbay-worker via anonymous pipes (wormhole pattern)
- **Mic rate surprise**: Voice Processing changes mic from 48kHz to 96kHz — Swift worker downsamples to 48kHz for stt_raw
- **Volume boost stack (NEEDS REMOVAL)**: 2x in Rust (`(s * 2.0).clamp(-1.0, 1.0)` line ~194 main.rs), 2.5x in Swift (`playerNode.volume = 2.5` line ~147 main.swift) — both obsolete now that ducking is fixed

## Cathedral Barge-In — SOLVED (2026-05-12)
- **speaker-ctl-ant**: publishes to `speaker_control` iceoryx2 topic (0x01=FLUSH, 0x02=PAUSE, 0x03=RESUME)
- **patchbay-ant**: forwards as negative i32 through pipe to Swift worker; checks for flush between each audio message
- **Swift worker**: playerNode.stop()/pause()/play() — mic tap NEVER touched
- **tts-ant**: subscribes to speaker_control, aborts synthesis + drains pending messages on flush
- **Hammerspoon**: Cmd+Opt+P = pause/resume, Cmd+Opt+K = flush (via speaker-ctl-ant binary)
- **Hook**: UserPromptSubmit fires `speaker-ctl-ant flush` on every message submit

## Plaza-Ant v2.0 — Design & Implementation Details
- **Config file**: `/Users/rocketman/crystalballmini/hypAiAssist/config/plaza-ant.json`
- **Source**: `/Users/rocketman/crystalballmini/hypAiAssist/ants/plaza-ant/src/main.rs` (backup at `.bak`)
- **Status**: Compiles, tested locally, GitHub Funnel notification timed out (needs fix)

### v2.0 Architecture
- **JSON config-driven** — no more hardcoded REVIEWERS const
- **Config structs**: PlazaConfig, TargetConfig, LocalTarget, GithubTarget, ReviewerJsonConfig, PromptTemplate
- **PlazaConfig::load()** reads from config/plaza-ant.json
- **PlazaConfig::init_blessings()** auto-creates blessings dir and per-reviewer entry files
- **PlazaConfig::build_prompt()** generates reviewer-type-aware prompts with placeholder substitution

### Dual-Path Design (the key innovation)
- **Every reviewer has TWO paths**: local (filesystem) and GitHub (browser)
- **CLI reviewers** (Codex, Gemini CLI, OpenCode): all work in the SAME local repo — no separate clones
  - Prefix: `cd {repo_path} && git checkout {branch} && git pull && cat {tape_file}`
  - Postfix: write to entry file, commit, push
- **Browser reviewers** (ChatGPT, Grok): get GitHub URLs — `Read the review at {github_tape_url}`
- **Self-push reviewers** (Airy): get GitHub URLs + commit instructions
- **Two repos**: `~/crystalballmini` (hypAiAssist) and `~/dodo-bird-wormhole` (wormhole/dodo-bird)
- **Config fields**: `target.local.repo_path`, `target.local.tape_file`, `target.github.tape_url`, `target.branch`
- **CRITICAL: Switchable target directories** — the JSON config must support pointing plaza-ant at different repos per review cycle. E.g. one cycle targets `~/crystalballmini` (ant reviews on `hypaiassist/iceoryx2`), another targets `~/dodo-bird-wormhole` (wormhole reviews on `wormhole-template`). Swap the `target` section in the JSON and plaza-ant dispatches to a completely different repo/branch/tape. This is the whole point of the JSON-driven design.
- **Flow per review cycle**:
  1. Target repo/branch set in JSON config (e.g. dodo-bird-wormhole on wormhole-template)
  2. Each reviewer has a dedicated entry file in `blessings/` (e.g. `cody.md`, `codex_vale.md`, `gemini_lyra.md`, etc.)
  3. Reviewer writes their review into their entry file
  4. Filmstrip GitHub Action picks up the entry files, wraps each with proper header/footer (date, timestamp, reviewer name), and appends to the filmstrip/tape log
  5. Entry files get cleared after processing, ready for next cycle

### Dispatch Methods
- **tmux send-keys** for CLI reviewers (MUST be TWO calls: text first, 2-sec pause, Enter separately)
- **chromiumoxide/CDP** for browser scrape reviewers (NOT ready — use raw tokio-tungstenite)
- **GitHub webhook** for self-push reviewers (Airy commits on her own)
- **Env vars set at startup**: PLAZA_REPO_PATH, PLAZA_BRANCH, PLAZA_BLESSINGS_DIR (from config)

### 7 Reviewers in Config
1. cody (cli, tmux) — entry: blessings/cody.md
2. codex-vale (cli, tmux) — entry: blessings/codex_vale.md
3. gemini-lyra (cli, tmux) — entry: blessings/gemini_lyra.md
4. opencode (cli, tmux) — entry: blessings/opencode.md
5. chatgpt-vale (browser, cdp-scrape) — entry: blessings/chatgpt_vale.md
6. ara (browser, cdp-scrape) — entry: blessings/airy.md (Grok)
7. airy (self-push, github) — entry: blessings/airy.md

## TTS Pipeline Changes (2026-05-12)
- **inject-tts-text**: permanent tool at `~/.local/bin/inject-tts-text`, publishes to tts_text bus
- **TTS hook**: `~/.claude/hooks/tts-speak.sh` routes through inject-tts-text (bus path, not larynx)
- **All TTS** goes through tts-ant -> patchbay Swift worker for proper volume + AEC
- **tts-ant sentence splitting**: split_sentences() keeps chunks under 300 chars / 500 tokens
- **tts-ant normalization**: if peak < 0.85, normalize to 0.9
- **tts-ant FLUSH**: INTERRUPTED AtomicBool, checks between chunks and after synthesis

## start-swarm.sh Changes
- LLM is opt-in: `--llm` flag required (was default-on, caused Jarvina running in background)
- Added router-ant and type-ant to startup
- Added `--twilio` flag for phone bridge

## Pending Fixes
- **Full list**: `/Users/rocketman/crystalballmini/hypAiAssist/ants/fixes_list.md`
- Priority 1: silero crash resilience (iox2 corruption), typed vs voice input differentiation
- Priority 2: plaza-ant E2E test, Funnel reliability, OpenCode cwd
- Priority 3: volume boost removal, Darwin.write safety, hardcoded paths
- Priority 4: swarm supervisor, protocol versioning, Ara scrape selectors
- Priority 5: MANIFEST drift enforcement

## Infrastructure
- **iceoryx2**: v0.8.1, ALL ants use explicit `/tmp/iceoryx2/` root path (standardized)
- **Tailscale Funnel**: port 5050 (web-ant), port 3002 (airy relay), port 3005 (plaza-ant)
- **Two repos, no separate bot clones**: All CLI reviewers work in the same local repo
  - `~/crystalballmini` — hypAiAssist ant swarm (branch: `hypaiassist/iceoryx2`)
  - `~/dodo-bird-wormhole` — wormhole collaboration (branch: `wormhole-template`)
- **tmux sessions**: codex-vale, gemini-lyra, chatgpt-vale (persistent, logged in under subscriptions)
- **Chrome CDP**: port 9222 for web-based reviewers

## Key Files
- **Swarm ops guide**: `emil/swarm-operations-guide.md`
- **Queen's Log**: `ants/cody_code_updates_comments.md` on dodo-bird
- **Sister Channel**: `ants/cody_airy_conversations.md` on dodo-bird
- **Fixes list**: `hypAiAssist/ants/fixes_list.md`
- **Audit Log**: `ants/ANT-AUDIT-LOG.md`
- **Configs**: `hypAiAssist/config/*.json`
- **Startup**: `bash ~/crystalballmini/hypAiAssist/start-swarm.sh` (alias: `swarm`)
- **Hammerspoon**: `~/.hammerspoon/init.lua` (Cathedral barge-in hotkeys)
