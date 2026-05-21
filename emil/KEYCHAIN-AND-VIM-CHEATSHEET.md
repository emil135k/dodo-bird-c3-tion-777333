# macOS Keychain for API Keys + Vim Tips

## Storing a Key in Keychain

```bash
security add-generic-password -a rocketman -s KEY_NAME -w "your-secret-key-here"
```

Example:
```bash
security add-generic-password -a rocketman -s ANTHROPIC_API_KEY -w "sk-ant-xxxxx"
security add-generic-password -a rocketman -s TWILIO_ACCOUNT_SID -w "AC08945xxxxx"
security add-generic-password -a rocketman -s TWILIO_AUTH_TOKEN -w "your-token"
```

## Retrieving a Key

```bash
security find-generic-password -a rocketman -s ANTHROPIC_API_KEY -w
```

## Deleting a Key (to rotate)

```bash
security delete-generic-password -a rocketman -s ANTHROPIC_API_KEY
```

Then add the new one with `add-generic-password` again.

## Loading Keys into Shell (in ~/.bashrc)

Add one line per key:
```bash
export ANTHROPIC_API_KEY=$(security find-generic-password -a rocketman -s ANTHROPIC_API_KEY -w 2>/dev/null)
export TWILIO_ACCOUNT_SID=$(security find-generic-password -a rocketman -s TWILIO_ACCOUNT_SID -w 2>/dev/null)
export TWILIO_AUTH_TOKEN=$(security find-generic-password -a rocketman -s TWILIO_AUTH_TOKEN -w 2>/dev/null)
```

The `2>/dev/null` suppresses errors if a key doesn't exist yet.

After editing, reload with: `source ~/.bashrc`

## How It Works

1. Key lives in Keychain (encrypted on disk)
2. Shell starts, .bashrc calls `security`, key goes into environment variable (in memory only)
3. Your program reads the env var — never sees a file, never touches git
4. Key never leaves the Mac

## Rules

- NO .env files. Ever.
- NO keys in code or config JSON.
- NO keys in git repos.
- Rotate a key? Delete old, add new, source ~/.bashrc.

---

# Vim/NVim: Joining Lines (the J trick)

When you paste a long command and it breaks into two lines:

- `J` — join next line onto current line (adds a space between)
- `gJ` — join next line, no space added
- `3J` — join next 3 lines onto current line

## Why Lines Break When Pasting

If NVim has `textwidth` set (e.g., 80), it auto-wraps long lines as you type or paste. Fix:

```
:set textwidth=0
```

Make it permanent in `~/.config/nvim/init.lua`:
```lua
vim.opt.textwidth = 0
```

## Seeing Invisible Characters

```
:set list
```

Shows `$` at end of each line — so you can tell if a line is actually broken or just visually wrapped.

## Visual Wrap vs Real Break

- `:set nowrap` — stops visual wrapping (display only, doesn't fix real breaks)
- `J` — fixes real breaks by joining lines

---

# Linux Equivalent (for Jetson Orin Nano)

Use `pass` instead of macOS Keychain:

```bash
# Store
pass insert ANTHROPIC_API_KEY

# Retrieve
export ANTHROPIC_API_KEY=$(pass show ANTHROPIC_API_KEY 2>/dev/null)
```

Same pattern, same security. Just different command.
