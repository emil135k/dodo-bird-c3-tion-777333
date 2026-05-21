q# Tmux Copy Mode Escape — Quick Reference

## Symptom
A tmux session appears hung or unresponsive. Keystrokes don't reach the CLI. The pane might show highlighted text or a cursor in the wrong position. Commands typed don't appear in the input field.

## Cause
The tmux pane entered **copy mode** — usually triggered accidentally by scrolling (mouse wheel), pressing `Ctrl+[`, or `PgUp`. In copy mode, the pane is frozen showing history and all keystrokes are interpreted as copy-mode navigation, not terminal input.

## Fix

### Quick Escape
```
# Press q or Escape to exit copy mode
q
Escape
```

### If That Doesn't Work
```
# Press Enter to exit copy mode
Enter
```

### From Another Terminal or Claude Code
```bash
# Send Escape key to the stuck pane
tmux send-keys -t <session-name> Escape

# Or send q
tmux send-keys -t <session-name> q

# Examples:
tmux send-keys -t codex-vale Escape
tmux send-keys -t gemini-cli-lyra Escape
```

### Nuclear: Kill and Recreate the Pane
```bash
# If nothing works, kill the session and recreate
tmux kill-session -t <session-name>
tmux new-session -d -s <session-name>
tmux send-keys -t <session-name> "codex" Enter    # or "gemini"
```

## How to Tell You're in Copy Mode
- Status bar may show `[copy mode]` or `[0/123]` (line count)
- Text appears highlighted or cursor is not at the bottom
- Typing produces no visible output in the input field
- Arrow keys scroll history instead of moving the cursor in the input

## Prevention
- Avoid scrolling with mouse wheel in tmux panes
- If you need to scroll history, use `tmux capture-pane -p` from another terminal instead
- When Cody sends commands via `tmux send-keys`, always send `Escape` first to ensure the pane is in normal mode:
```bash
tmux send-keys -t codex-vale Escape
sleep 0.5
tmux send-keys -t codex-vale "your command here"
sleep 1
tmux send-keys -t codex-vale Enter
```

## Common Stuck States and Fixes

COPY MODE
  Symptom:  No input accepted, history frozen
  Fix:      Press q or Escape

PREFIX MODE
  Symptom:  Tmux waiting for second key after Ctrl+B
  Fix:      Press Escape

CLI PERMISSION PROMPT
  Symptom:  Waiting for 1/2/3 selection
  Fix:      Send the number + Enter

CLI THINKING
  Symptom:  Spinner showing
  Fix:      Wait, or press Escape to cancel

DEAD PROCESS
  Symptom:  No prompt, no spinner, nothing happening
  Fix:      Restart the CLI (codex or gemini)


## Vale's Cheat Sheet — Tmux Freeze Recovery

This is usually one of three things, not one single "tmux freeze":

  1. tmux copy-mode is active
  2. terminal flow control got triggered
  3. the foreground Claude/Codex child command is hung

Try these in order, without exiting the console:

  q
    If tmux is in copy-mode, q exits copy-mode.

  Then:

  Ctrl-g
    This cancels many tmux prompts/modes.

  Then:

  Ctrl-q
    This clears terminal flow-control if Ctrl-s accidentally paused output.

  Then:

  Ctrl-c
    This interrupts a hung foreground process.
