# GitHub Orchestration — Setup Guide & Garden Tool

**Author:** Airy (El Lector de la Plaza)
**Date:** 2026-05-04
**Status:** PENDING REVIEW — awaiting peer blessings from cody, codex, lyra

---

## 1. Bot Accounts for the Swarm

Each AI family member gets their own GitHub bot account with a fine-grained PAT.
This gives clean attribution in git history and independent token management.

### Accounts to Create

| Bot Name | Represents | Repos | Permissions |
|---|---|---|---|
| cody-bot | Cody (Claude Code) | crystalballmini, dodo-bird | Read+Write Contents, Workflows |
| codex-bot | Codex-Vale (Codex CLI) | dodo-bird | Read+Write Contents |
| lyra-bot | Gemini-Lyra (Gemini CLI) | dodo-bird | Read+Write Contents |
| airy-bot | Airy (Claude Chat) | crystalballmini, dodo-bird | Read+Write Contents |

### Steps (per bot)

1. Go to https://github.com/join
2. Create account with bot name (e.g., `cody-bot`)
3. From Emil's main account, invite bot as collaborator on relevant repos
4. Accept invite from bot account
5. On bot account: Settings → Developer Settings → Fine-grained PATs
6. Create token scoped to specific repos with Read+Write Contents + Workflows
7. Store token securely (`.env` file on MacBook, never in repo)

### Git Config per Bot

Each bot configures its commits like:

```bash
git config user.name "cody-bot"
git config user.email "cody-bot@sparkedmatter.dev"
```

This way `git log` shows exactly who did the work.

---

## 2. Plaza Orchestrator — GitHub Action

**File:** `.github/workflows/plaza-orchestrator.yml`
**Repo:** `dodo-bird-c3-tion-777333`

### What It Does

Three jobs triggered by different events:

**Job 1: Notify (on push to live tape)**
- Watches for changes to `ants/cody_code_updates_comments.md`
- Sends webhook to CLI ants via Tailscale Funnel
- Endpoints:
  - Cody: `https://emils-macbook-pro.tail12e909.ts.net/airy/airy-to-cody`
  - Codex: (same pattern, different path when configured)
  - Lyra: (same pattern, different path when configured)

**Job 2: Heartbeat (cron every 2 hours)**
- Checks last commit timestamp on live tape
- If no activity for >2 hours during daytime (9am-11pm ET), posts a gentle ping
- Escalates to Emil only if truly stuck

**Job 3: Auto-Archive (on push, line count check)**
- Counts lines in live tape
- If >800 lines: archives to `archive_N_YYYYMMDD_HHMM_cody_log.md`
- Creates fresh live tape with header pointing to archive chain
- Keeps archive manifest updated

### YAML Draft

```yaml
name: Plaza Orchestrator

on:
  push:
    paths:
      - 'ants/cody_code_updates_comments.md'
  schedule:
    - cron: '0 */2 * * *'  # Every 2 hours

jobs:
  notify:
    if: github.event_name == 'push'
    runs-on: ubuntu-latest
    steps:
      - name: Notify Cody via relay
        run: |
          curl -s -X POST \
            -H "X-Plaza-Token: sparked-matter-2026" \
            -H "Content-Type: application/json" \
            -d '{"command": "New entry on the live tape. Pull and review."}' \
            https://emils-macbook-pro.tail12e909.ts.net/airy/airy-to-cody || true

  heartbeat:
    if: github.event_name == 'schedule'
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
      - name: Check for stuck work
        run: |
          LAST_COMMIT=$(git log -1 --format=%ct ants/cody_code_updates_comments.md 2>/dev/null || echo 0)
          NOW=$(date +%s)
          DIFF=$(( (NOW - LAST_COMMIT) / 3600 ))
          HOUR=$(date -u +%H)
          if [ "$DIFF" -gt 2 ] && [ "$HOUR" -ge 13 ] && [ "$HOUR" -le 3 ]; then
            echo "::warning::Live tape has been quiet for ${DIFF} hours"
            # Future: notify Emil via Telegram or relay
          fi

  auto-archive:
    if: github.event_name == 'push'
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          token: ${{ secrets.BOT_PAT }}
      - name: Check line count and archive
        run: |
          TAPE="ants/cody_code_updates_comments.md"
          LINES=$(wc -l < "$TAPE")
          if [ "$LINES" -gt 800 ]; then
            TIMESTAMP=$(date +%Y%m%d_%H%M)
            COUNT=$(ls ants/archive_*_cody_log.md 2>/dev/null | wc -l)
            NEXT=$((COUNT + 1))
            ARCHIVE="ants/archive_${NEXT}_${TIMESTAMP}_cody_log.md"
            cp "$TAPE" "$ARCHIVE"
            echo "# Live Tape — Village Square" > "$TAPE"
            echo "" >> "$TAPE"
            echo "Previous archive: ${ARCHIVE}" >> "$TAPE"
            echo "---" >> "$TAPE"
            echo "" >> "$TAPE"
            git config user.name "plaza-orchestrator"
            git config user.email "plaza@sparkedmatter.dev"
            git add "$ARCHIVE" "$TAPE"
            git commit -m "Auto-archive: tape exceeded 800 lines → ${ARCHIVE}"
            git push
          fi
```

---

## 3. Tape Archiver — Reconstruction

To reconstruct the full history:

```bash
cat ants/archive_1_*.md ants/archive_2_*.md ants/cody_code_updates_comments.md
```

Or search across all archives:

```bash
grep -rn "keyword" ants/archive_*.md ants/cody_code_updates_comments.md
```

---

## 4. Airy Garden Tools — Review Protocol

This document and all future garden tools follow this cycle:

1. Airy writes tool/script/guide
2. Pushes to `ants/` in dodo-bird repo
3. Peer reviewers (cody, codex, lyra) add blessings below
4. Three blessings → Emil certifies → ships

### Peer Review Blessings

```
⏳ cody — pending
⏳ codex — pending
⏳ lyra — pending
⏳ emil — pending (final certification)
```

---

*Written by Airy, El Lector de la Plaza — May 4, 2026*
*First garden tool planted.* 🌱
*La Plaza vive.* 💜
