# Pending Fixes — Ant Swarm

## Priority 1 — Stability

### silero-ant crashes on iceoryx2 segment corruption
- **Symptom**: silero dies with `ServiceInCorruptedState` whenever another ant restarts
- **Root cause**: iceoryx2 shared memory segments corrupt when publishers/subscribers restart independently
- **Fix**: Add retry loop in silero — catch the error, sleep, reconnect. Don't panic and die.
- **Affects**: silero-ant, but ALL ants have the same vulnerability

### Differentiate typed input vs voice input in Cody's console
- **Symptom**: Cannot tell if text in Cody's terminal was typed by Emil or spoken via STT→type-ant
- **Fix**: type-ant should prefix voice-generated text with a marker (e.g., `🎤 ` or `[voice] `) so the log distinguishes typed vs spoken input
- **Affects**: type-ant

## Priority 2 — Plaza-ant v2 Completion

### Funnel notification timeout (GitHub → plaza-ant)
- **Symptom**: Filmstrip Action curls plaza-ant but notification doesn't arrive
- **Root cause**: Tailscale Funnel intermittently drops connections or times out
- **Fix**: Investigate Funnel reliability. Add retry in filmstrip Action. Or use webhook alternative.

### Plaza-ant scrape-and-push branch targeting
- **Symptom**: Scraped browser reviews push to wrong branch
- **Status**: FIXED in v2.0 — uses PLAZA_REPO_PATH and PLAZA_BRANCH env vars
- **Needs**: End-to-end test with actual browser scrape

### Plaza-ant prefix/postfix prompt generation
- **Symptom**: CLI reviewers don't get proper cd/git instructions before and after the review content
- **Status**: FIXED in v2.0 — prompt templates with placeholders per reviewer type
- **Design**: Each reviewer type gets different prefix/postfix wrapping:
  - **CLI prefix**: `cd {repo_path} && git checkout {branch} && git pull origin {branch} && cat {tape_file}` — so the reviewer is in the right directory with fresh code
  - **CLI postfix**: `Write your review to {entry_file}, then git add {entry_file} && git commit -m "review" && git push origin {branch}`
  - **Browser prefix**: `Read the collaboration review at {github_tape_url}` — GitHub URL for web-based reviewers
  - **Browser postfix**: Instructions to post review (scraped by plaza-ant, or self-pushed by Airy)
- **Config**: `prompt_template` section in `config/plaza-ant.json` with `cli`, `browser`, `self_push` templates
- **Needs**: Verify prompts are correct for each reviewer type, end-to-end test

### OpenCode working directory
- **Symptom**: OpenCode can't commit/push — "not a git repository"
- **Root cause**: OpenCode runs in /Users/rocketman, not the target repo
- **Fix**: Plaza-ant v2 prompt template includes `cd {repo_path}` prefix for CLIs
- **Needs**: Test with OpenCode

## Priority 3 — Voice Pipeline

### Volume boost stack removal
- **Symptom**: 2x Rust boost + 2.5x Swift playerNode volume = clipping risk
- **Root cause**: Was added to counter AGC ducking, now obsolete with ducking fix
- **Fix**: Remove `(s * 2.0).clamp(-1.0, 1.0)` in patchbay-ant Rust, set `playerNode.volume = 1.0` in Swift
- **Location**: patchbay-ant/src/main.rs line ~138, swift-worker/Sources/main.swift line ~124

### Darwin.write() safety in Swift audio tap
- **Symptom**: Short writes corrupt pipe protocol under load
- **Root cause**: `Darwin.write()` return value ignored — partial writes desync Rust reader
- **Fix**: Wrap in retry loop or use ring buffer between tap callback and writer thread
- **Location**: patchbay-ant/swift-worker/Sources/main.swift lines ~116-120

### Hardcoded paths in wormhole examples
- **Symptom**: `/Users/rocketman/.local/bin/...` in source code
- **Fix**: Use env vars with documented defaults
- **Location**: stt-ant/src/main.rs (WORKER_BIN), patchbay-ant/src/main.rs (WORKER_BIN)

### Hammerspoon interrupt not working — FIXED (2026-05-12)
- **Status**: FIXED — Cathedral barge-in system built
- **Solution**: speaker-ctl-ant publishes to `speaker_control` iceoryx2 topic → patchbay-ant forwards as negative i32 through pipe → Swift calls `playerNode.stop()`/`.pause()`/`.play()`. Mic tap never touched. TTS-ant also subscribes to `speaker_control` and aborts synthesis + drains pending messages on flush. Patchbay-ant checks for flush between each audio message forward and drains stale audio.
- **Hotkeys**: ⌘⌥P = pause/resume, ⌘⌥K = flush
- **Hook**: `UserPromptSubmit` fires `speaker-ctl-ant flush` on every message submit

## Priority 4 — Architecture

### Swarm supervisor (auto-restart crashed ants)
- **Symptom**: Manual restart of crashed ants
- **Fix**: launchd plist with KeepAlive, or a swarm-ant that monitors and restarts
- **Note**: launchd auto-start was disabled due to iceoryx2 corruption conflicts

### Protocol versioning for wormhole pipe
- **Symptom**: No way to detect protocol mismatch or corrupted frames
- **Fix**: Add 4-byte magic `WORM` at stream start. Version header in v2.
- **Consensus**: All 7 reviewers agreed

### iceoryx2 root path standardization
- **Symptom**: ServiceInCorruptedState when ants use different root paths
- **Status**: FIXED — all ants now use explicit `/tmp/iceoryx2/`
- **Note**: tts-ant and web-ant were updated in this session

### Ara (Grok) scrape selectors
- **Symptom**: Plaza-ant scrape returns empty for Grok
- **Root cause**: DOM selectors don't match Grok's current layout
- **Fix**: Update scrape JavaScript in plaza-ant for Grok's current DOM

## Priority 5 — Documentation & Governance

### MANIFEST drift — GitHub Action enforcement
- **Symptom**: MANIFEST.md gets out of date when infrastructure changes (ports, funnels, ant wiring)
- **Root cause**: No automated check — relies on human remembering to update docs
- **Example**: Airy relay was on port 3002 in MANIFEST, but plaza-ant moved to port 3005. Airy couldn't connect.
- **Fix**: GitHub Action or webhook that detects changes to infrastructure files (patchbay-ant, start-swarm.sh, funnel config, ant configs) and fails CI if MANIFEST.md is not updated in the same commit
- **Also**: Make relay section of MANIFEST machine-readable (JSON/YAML block) so Airy can verify programmatically
- **Also**: Add preflight health-check script that pings relay endpoints and reports actual state
- **Consensus**: Airy proposed 3 approaches, Emil approved all three

---

*Last updated: 2026-05-12*
*Session: marathon build (May 8-12) + cathedral barge-in session (May 12)*
