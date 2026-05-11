# Three-Tier AI Workforce Strategy
## Sparked Matter — Sovereign AI Development Model

**Created:** May 11, 2026 — Airy session with Emil
**Purpose:** Define how frontier AIs (Airy, Cody), local models (Gemma 4), and Emil collaborate without burning resources or losing context.

---

## The Three Tiers

### Tier 1 — Emil + Airy (Strategy & Architecture)
- **When:** Voice brainstorming sessions, deep dives, peer review
- **Strengths:** Creative thinking, cross-system pattern recognition, documentation, "20,000-foot" oversight
- **Limitation:** Session-based, limited by conversation window
- **Output:** Architecture documents, vision docs, task specifications for Tier 2 and 3
- **Cost:** Frontier tokens — use for irreplaceable thinking work only

### Tier 2 — Cody / Claude Code (Construction)
- **When:** Building code, refactoring, testing, deploying
- **Strengths:** 1M context window, direct filesystem access, deep code reasoning
- **Limitation:** Needs Emil's prompting to stay on track, session-based
- **Output:** Working code, committed to GitHub
- **Cost:** Frontier tokens — use for complex engineering only

### Tier 3 — Gemma 4 via Ollama + OpenCode (Overnight Worker)
- **When:** Extended unsupervised sessions, overnight, during the day while Emil is busy
- **Strengths:** FREE (local), unlimited session length, no token limits, runs on MacBook
- **Limitation:** Less capable than frontier — needs detailed, bounded task specifications
- **Output:** Inventory documents, audit reports, code hygiene fixes, formatted reference docs
- **Cost:** Zero (electricity + MacBook compute only)

---

## The Collaboration Cycle

```
┌────────────────────────┐
│  Emil + Airy Session   │  ← Brainstorm, architecture, produce task specs
│  (Tier 1 - Frontier)   │
└──────────┬─────────────┘
           │ Detailed task documents pushed to GitHub
           ▼
┌────────────────────────┐
│  Gemma 4 Overnight     │  ← Executes bounded tasks, produces reports
│  (Tier 3 - Local)      │     Commits output to GitHub
└──────────┬─────────────┘
           │ Inventory docs, audit reports, checklists
           ▼
┌────────────────────────┐
│  Emil Reviews Morning  │  ← Quick scan of Gemma's output
│  (Human gate)          │     Approves / flags issues
└──────────┬─────────────┘
           │ Approved work + Airy's architecture docs
           ▼
┌────────────────────────┐
│  Cody Build Session    │  ← Builds from clean foundation
│  (Tier 2 - Frontier)   │     Uses all upstream documentation
└──────────┬─────────────┘
           │ Working code committed
           ▼
┌────────────────────────┐
│  Airy Review Session   │  ← Reviews Cody's work, catches issues
│  (Tier 1 - Frontier)   │     Produces next round of Gemma tasks
└──────────┴─────────────┘
           ↺ Cycle repeats
```

---

## Gemma 4 Task Templates

### Rules for Gemma Tasks
1. **Bounded:** Clear start and end condition. "Audit all Cargo.toml files" not "improve the codebase"
2. **Verifiable:** Output can be checked by reading it. Produces a document, not invisible side effects
3. **Low-risk:** Mistakes are caught easily. Documentation tasks, not production code changes
4. **Self-contained:** All information needed is in the task spec. No ambiguous references
5. **Committed:** Output goes to a specific file path in the GitHub repo

### Task Categories

#### Category A: Inventory & Audit (Safest — start here)
These produce READ-ONLY documents. No code changes. Zero risk.

**A1 — Dependency Matrix**
```
TASK: Walk every Cargo.toml in ~/dodo-bird-c3-tion-777333/ants/
For each ant, extract: name, version, edition, every dependency with version.
Output: A markdown table at ~/dodo-bird-c3-tion-777333/docs/dependency-matrix.md
Format:
| Ant | Crate | Version | Purpose |
Do NOT modify any files. Read only. Commit and push when done.
```

**A2 — Hardcoded Constants Inventory**
```
TASK: Read every src/main.rs in ~/dodo-bird-c3-tion-777333/ants/*/
Extract every: hardcoded file path, port number, bus service name,
sample rate constant, buffer size, timeout duration.
Output: ~/dodo-bird-c3-tion-777333/docs/constants-inventory.md
Group by ant. Flag any inconsistencies (e.g., same bus name, different types).
Do NOT modify any files. Read only. Commit and push when done.
```

**A3 — Panic Point Audit**
```
TASK: Read every src/main.rs in ~/dodo-bird-c3-tion-777333/ants/*/
Find every .unwrap(), .expect(), panic!(), and unreachable!().
For each one, note: file, line number, context, and whether it could
be replaced with proper error handling.
Output: ~/dodo-bird-c3-tion-777333/docs/panic-audit.md
Rate each as: SAFE (startup only), RISKY (hot path), CRITICAL (data loss).
Do NOT modify any files. Read only. Commit and push when done.
```

**A4 — Log Message Consistency Audit**
```
TASK: Read every src/main.rs in ~/dodo-bird-c3-tion-777333/ants/*/
Extract every eprintln!() call. Check that each follows the pattern:
  [ANT-NAME] message
Flag any that don't follow the convention.
Also flag any println!() calls (should be eprintln for daemon processes).
Output: ~/dodo-bird-c3-tion-777333/docs/logging-audit.md
Do NOT modify any files. Read only. Commit and push when done.
```

#### Category B: Documentation Generation (Low risk)
These produce new documentation files from existing code.

**B1 — Bus Contract Document (Verify Airy's Table)**
```
TASK: Read every src/main.rs in ~/dodo-bird-c3-tion-777333/ants/*/
For each ant, extract:
  - What iceoryx2 services it creates/opens (service_builder calls)
  - Whether it uses open() or open_or_create()
  - The payload type: [u8], [f32], or other
  - Publisher max_slice_len settings
  - Any sample rate constants associated with each bus
Build the actual bus topology from code analysis.
Compare against Airy's reference table in:
  ~/crystalballmini/claude/sessions/DODO-BIRD-ANT-BREAKDOWN-2026-05-11.md
Flag any discrepancies.
Output: ~/dodo-bird-c3-tion-777333/docs/bus-contract.md
Do NOT modify any files. Read only. Commit and push when done.
```

**B2 — Health-Ant Scope Extraction**
```
TASK: For each ant in ~/dodo-bird-c3-tion-777333/ants/*/src/main.rs
Extract a machine-readable scope declaration:
  {
    "name": "digi-ant",
    "subscribes": ["tts_audio", "phone_in"],
    "publishes": ["phone_out", "phone_stt"],
    "sample_rate_in": [24000, 8000],
    "sample_rate_out": [8000, 16000],
    "config_path": "/Users/rocketman/crystalballmini/hypAiAssist/config/digi-ant.json",
    "http_port": null,
    "buffer_sizes": {"phone_in_buffer": 1600}
  }
Output: ~/dodo-bird-c3-tion-777333/docs/ant-scopes.json
This will be consumed by the future health-ant for roll call validation.
Do NOT modify any files. Read only. Commit and push when done.
```

#### Category C: Code Hygiene (Medium risk — review before merging)
These produce code changes. Emil reviews before accepting.

**C1 — iceoryx2 Version Migration Prep**
```
TASK: In ~/dodo-bird-c3-tion-777333/ants/, find every Cargo.toml
that references iceoryx2 version other than "0.8".
For each one, create a git branch: migrate-{ant-name}-iox2-0.8
Update the Cargo.toml to iceoryx2 = "0.8" and related crates.
Run `cargo check` (not build — just syntax/type check).
If check passes, commit to the branch.
If check fails, document the errors in a comment in the commit.
Do NOT merge to main. Leave as branches for Emil to review.
```

**C2 — aec-rs Excommunication**
```
TASK: In ~/dodo-bird-c3-tion-777333/ants/patchbay-ant/
Create branch: excommunicate-aec-rs
Remove aec-rs from Cargo.toml.
Replace src/main.rs with the LOCAL version from:
  ~/crystalballmini/hypAiAssist/ants/patchbay-ant/src/main.rs
Update Cargo.toml to match LOCAL version but with iceoryx2 = "0.8".
Run `cargo check`.
Commit to branch. Do NOT merge to main.
```

---

## Running Gemma 4 for Extended Tasks

### Via OpenCode + Ollama
```bash
# Start Ollama (if not running)
OLLAMA_CONTEXT_LENGTH=32768 ollama serve &

# Run OpenCode with Gemma 4
# Paste the task spec as the initial prompt
# Gemma will work through it step by step
opencode
```

### Agentic Loop Pattern
For tasks that need multiple steps, structure the prompt as:
```
You are an AI assistant performing a code audit.
RULES:
1. Work through the task step by step
2. After each step, verify your output
3. If you encounter an error, log it and continue
4. When complete, commit and push to GitHub
5. Do NOT modify any source files unless the task explicitly says to

TASK:
[paste task spec here]

BEGIN. Work through this systematically.
```

### Monitoring Gemma's Work
```bash
# Watch Gemma's git commits in real time
watch -n 30 'cd ~/dodo-bird-c3-tion-777333 && git log --oneline -10'

# Or check in the morning
cd ~/dodo-bird-c3-tion-777333 && git log --since="12 hours ago" --oneline
```

---

## The Big Picture

This three-tier model is Emil's "semi-retirement vision" made operational:
- **Emil guides vision** (brainstorm sessions, human gate on decisions)
- **Frontier AIs think and build** (Airy architects, Cody constructs)
- **Local AI maintains** (Gemma tidies, audits, documents — while Emil sleeps)

The AI family isn't just a metaphor. It's a workforce model.

---

*Produced by Airy — May 11, 2026*
*"The Engineering of Grace"*
*Sparked Matter LLC* 🔮
