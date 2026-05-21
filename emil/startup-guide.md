# Startup Guide — Plaza-Ant & Tailscale Funnel

After a reboot, run these in order.

---

## 1. Tailscale Funnel Routes

Tailscale needs to be connected first. Wait a few seconds after login, then:

```bash
tailscale funnel --bg --set-path /plaza http://localhost:3005
tailscale funnel --bg --set-path /airy http://localhost:3005
tailscale funnel --bg --set-path /dashboard http://localhost:3001
```

Verify:
```bash
tailscale funnel status
```

Should show:
```
/          proxy http://localhost:5050    (web-ant)
/airy      proxy http://localhost:3005   (plaza-ant — Airy relay)
/plaza     proxy http://localhost:3005   (plaza-ant — reviewer dispatch)
/dashboard proxy http://localhost:3001
```

---

## 2. Plaza-Ant

```bash
bash ~/crystalballmini/hypAiAssist/scripts/start-plaza.sh
```

This will:
- Check Sovereign Mound is mounted
- Check Cody's tmux session exists
- Check port 3005 is free
- Load PLAZA_SECRET from Keychain
- Build if needed
- Start plaza-ant on port 3005

Verify:
```bash
curl -s http://localhost:3005/plaza && echo "plaza-ant is up"
```

---

## 3. Chrome Debug Mode (for ChatGPT Vale CDP)

Only needed if using web-based reviewer dispatch:

```bash
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222 --user-data-dir=/Users/rocketman/.chrome-debug-profile &
```

Also saved at: `~/Desktop/chrome-debug.txt`

---

## 4. CLI Reviewer Sessions (Codex, Gemini)

Only needed if using CLI reviewers:

```bash
tmux new-session -d -s codex-vale -c /Volumes/Sovereign_Mound/village-square/dodo-bird-codex-vale
tmux new-session -d -s gemini-lyra -c /Volumes/Sovereign_Mound/village-square/dodo-bird-gemini-lyra
```

Then attach and launch the CLI in each:
```bash
tmux attach -t codex-vale   # then type: codex
tmux attach -t gemini-lyra  # then type: gemini
```

---

## Future: launchd Automation

TODO: Create a launchd plist at `~/Library/LaunchAgents/com.sparkedmatter.funnel-routes.plist` that runs the Funnel commands automatically at login. This eliminates the manual step after every reboot.
