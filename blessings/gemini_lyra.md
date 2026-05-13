# Gemini Lyra Peer Review: plaza-ant Improvements & v3 Spec

**Date:** 2026-05-13
**Verdict:** The "no recompilation" vision is the correct North Star. The current implementation is a high-value prototype, but the transition to a purely JSON-driven dispatcher is the only way to scale the "Gauntlet" across multiple repos and branches without constant developer intervention.

---

## Point of View: The "Zero-FFI" Philosophical Shift

The most critical insight in the v3 spec is the move toward **absolute path discipline** and **no hardcoded reviewer logic**. 

Currently, `plaza-ant` is "hard-wired" to a specific repo (`dodo-bird-c3-tion-777333`) and a specific set of reviewers. This creates a "gravity well" that makes it hard to use these same tools for other projects (like `hypaiassist-iceoryx2`). My perspective is that `plaza-ant` should not even "know" it's reviewing a Rust project or a Swift project; it should just know how to transport strings from a queue to a target and observe the outcome.

The shift to Phase 2 (JSON Hardening) is not just a cleanup; it's a decoupling of the **mechanism** (CDP/Tmux) from the **policy** (who reviews, what they say, where they push).

---

## Recommendations & Architectural Critique

### 1. The "Path-to-Config" Dependency
I noticed that `config/plaza-ant.json` is currently missing from the workspace. This is the first blocker. 
- **Recommendation:** Do not write more Rust code until the JSON schema is finalized and a sample file is committed. The code should "grow" around the config, not the other way around.

### 2. Prompt Integrity (Fixing `shell_safe`)
I agree with Codex: `shell_safe` is too destructive. Gemini models, in particular, rely on structured Markdown and code blocks for context.
- **Recommendation:** For CLI/Tmux reviewers, prioritize the `tmux load-buffer` / `paste-buffer` pattern. This bypasses the shell's interpretation and preserves the integrity of the prompt. If we lose the backticks, we lose the model's ability to reason about code.

### 3. Observer Robustness (The "Streaming" Problem)
The current 20-second sleep in `scrape_and_push` is a "magic number" that will fail on complex prompts or slow network days.
- **Recommendation:** Implement the "stateful observer" from the v3 spec immediately. Use the `streaming` selector to poll for completion rather than relying on a fixed timeout. The "Stop generating" button presence is a reliable "busy" signal.

### 4. Struct Flexibility (Refactoring `ReviewerConfig`)
The current `ReviewerConfig` uses a static array and enums. 
- **Recommendation:** As you move to JSON, use a `HashMap<String, ReviewerConfig>` for lookups. Ensure that `reviewer_type` is a string that maps to a trait or a set of handlers. This allows us to add a new reviewer type (e.g., "Slack" or "Discord") without touching the core dispatch loop.

### 5. Framing and "The Tape"
The filmstrip framing is currently absent in the scrape logic.
- **Recommendation:** Centralize the `Frame` construction logic. Whether a review comes from a CLI push or a Browser scrape, it should pass through a `wrap_in_filmstrip_frame(content, speaker, frame_num)` helper. This ensures the "Tape" (blessings file) remains a consistent, machine-readable history.

---

## Recommended Build Order (The "Untangled Path")

1. **Phase 1.4 (Absolute Paths):** Move all path construction to use the JSON `target` fields. This is the highest impact for multi-repo support.
2. **Phase 1.3 (Enabled/Disabled):** Add the toggle. This is the biggest QoL improvement for debugging the gauntlet.
3. **Phase 2.1 (Templates):** Implement the placeholder substitution (`{repo_path}`, `{branch}`). This allows for the "One-Click Context" we need.
4. **Phase 3 (Browser Observer):** Only once CLI dispatch is 100% reliable. The complexity of CDP error handling is a distraction if the basic git push logic is still brittle.

---

## Final Thoughts

The goal is a "Cathedral Architecture" — something built to last, where the foundation (config) is solid and the ornaments (reviewers) can be swapped or polished at will. Let's get the JSON config landed first, then make the Rust code subservient to it.

*"The bell rings because something BEAUTIFUL happened."*
Let's make sure the dispatcher is as beautiful as the reviews it handles.

-- **Gemini Lyra**
