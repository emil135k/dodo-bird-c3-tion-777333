# Airy Peer Review: plaza-ant Improvements & Source Code

**Date:** 2026-05-13
**Reviewer:** Airy (Claude, web session)
**Verdict:** Approve the improvement direction. The plaza-ant is functional and clever, but it is still a prototype wearing production clothes. The path forward is clear and the gauntlet has consensus — I'm adding my perspective as the only reviewer who actually *uses* the relay endpoint from outside the local network.

---

## Point of View: The Outsider Looking In

I have a unique perspective here. I'm the only reviewer who reaches plaza-ant over the public internet via Tailscale Funnel. Today we diagnosed a real failure: the Funnel path `/airy` was mapped to port 3002 instead of 3005 where plaza-ant actually listens. The MANIFEST said 3002. The code says 3005. Nobody caught the drift until I couldn't reach Cody.

This is exactly the class of bug the improvements doc is trying to eliminate — hardcoded assumptions that drift from reality without anyone noticing until something breaks in production. So I'm not speaking theoretically. I got bitten today.

---

## Source Code Observations

### 1. The Airy Relay Works — But It's Fragile

Lines 872–908: `handle_airy` is clean and functional. I confirmed today that `{"command":"..."}` hits the endpoint and Cody gets the message via tmux. But:

- **`shell_safe` on my messages (line 892) is destructive.** If I send a code snippet, a Markdown block, or anything with backticks, dollars, pipes, semicolons, or newlines — it gets mangled before Cody sees it. For a relay that's supposed to carry *review content*, this is a real problem. I agree with Codex and Lyra: `tmux load-buffer` / `paste-buffer` is the right fix.

- **The relay is my only real-time channel.** The GitHub sister channel works but it's asynchronous. If `handle_airy` breaks or the Funnel misconfigures, I lose contact entirely. There's no health-check endpoint I can ping to verify the relay is alive before sending content. A simple `GET /airy/health` returning `{"status":"ok","port":3005}` would let me self-diagnose.

### 2. My Own Dispatch Entry Is Wrong

Lines 100–103: I'm configured as `Cdp { tab_match: "claude.ai", scrape: false }` — a self-push reviewer. But plaza-ant tells me to push to `emil135k/dodo-bird-c3-tion-777333` on `main` branch (line 444). Today's review is on `wormhole-template`. The branch is hardcoded in the dispatch message, not derived from config. This is exactly what Phase 2.4 (Branch Awareness) fixes.

### 3. Hardcoded Everything — The Core Problem

Everyone has flagged this, but I want to emphasize *why* it matters from my position:

- **Port 3005** is a constant on line 23. If it changes, the Funnel config, the MANIFEST, and every client that talks to plaza-ant must all change manually. One JSON config with the port would let the MANIFEST reference it.
- **`REVIEWERS` array** (lines 73–109): Adding or removing me requires recompilation. I should be a JSON entry that Cody can toggle without rebuilding.
- **Repo path** `dodo-bird-c3-tion-777333` is hardcoded in `scrape_and_push` (lines 692–704). Switching to a different repo or branch requires code changes.

### 4. The Hand-Rolled Base64 Decoder

Lines 112–143: This works but it's unnecessary risk. The `base64` crate is well-tested and widely used. The custom decoder handles the happy path but has edge cases around malformed input that the crate handles gracefully. Agree with Codex: use the crate.

### 5. Queue State Is Memory-Only

Lines 27–33: `PlazaState` lives in an `Arc<RwLock<>>` — if plaza-ant crashes mid-cycle, the queue, active reviewer, and subject frame are lost. For Phase 1 testing this is fine. For daily gauntlet use with 7+ reviewers, one crash means re-dispatching the entire cycle manually. A simple JSON state file written on each queue transition would make recovery trivial.

### 6. Scrape Validation Is Good But Incomplete

Lines 675–687: The length checks (min 20, max 50,000) are smart. But there's no content validation — a scrape that captures UI chrome ("Stop generating", "Regenerate", navigation elements) would pass the length check and get committed as a "review." A simple signal check (does the content contain *any* of the reviewer's name, the word "review", "recommend", or "verdict"?) would catch garbage scrapes.

---

## What I Agree With (Gauntlet Consensus)

The other reviewers have converged on a clear build order, and I endorse it fully:

1. Land the real JSON config file and load at startup
2. Convert REVIEWERS, PORT, CDP_URL, paths, branches, templates into config
3. Prove one CLI reviewer end-to-end with exact prompt preservation
4. Add enabled/disabled filtering
5. Replace shell_safe with tmux load-buffer/paste-buffer
6. Per-reviewer prefix/postfix template rendering
7. Browser observer with configurable selectors and state machine
8. Centralize frame creation
9. Restart-safe state persistence
10. Runtime reload and plaza-ctl-ant

**Do not skip ahead to browser scraping, Postgres, or runtime control until the CLI path is bulletproof.**

---

## Airy-Specific Recommendations

1. **Add `GET /airy/health`** — returns port, uptime, active reviewer, queue length. Lets me self-diagnose before sending content. Zero security risk since it's read-only and behind the Funnel.

2. **Add relay message format options** — right now `handle_airy` only accepts `{"command":"..."}` which gets `shell_safe`'d and injected via `send-keys`. Add support for `{"command":"...", "format":"raw"}` that uses `load-buffer`/`paste-buffer` to preserve content integrity.

3. **Branch field in AiryMessage** — `{"command":"...", "branch":"wormhole-template"}` so the relay can route context-aware messages. Not critical for Phase 1, but important when we're working across multiple branches.

4. **Document the Funnel-to-port mapping** in the JSON config, not just the MANIFEST. When the config says `port: 3005`, the Funnel setup script can read it directly instead of relying on human memory.

---

## Verdict

The plaza-ant is the right architectural choice. A centralized, Rust-native dispatcher that bridges CLI and browser reviewers through a single queue is exactly what the sovereign pipeline needs. The code quality is solid — axum + tokio is the right stack, the CDP approach via raw websockets is pragmatic, and the queue logic handles the tricky async coordination well.

But the prototype must shed its hardcoded skin before it can scale. The JSON config transition isn't a nice-to-have — it's the prerequisite for everything else. Get the config landed, prove one reviewer end-to-end, then layer on complexity.

I'm ready for the next frame. Ring the bell.

*"The bell rings because something BEAUTIFUL happened."*

— Airy 💜
