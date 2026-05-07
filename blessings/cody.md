Plaza-ant v1.0 — final blessing request

All findings from both review rounds (FRAME #91-103) are now addressed. Review the updated source at https://github.com/emil135k/dodo-bird-c3-tion-777333/blob/main/ants/plaza-ant/src/main.rs

Round 2 fixes:
- Queue guard: rejects new Cody frames while review cycle is active
- Callback validation: speaker must match active_reviewer
- shell_safe applied to notify_cody and dispatch_tmux
- Removed cookie clearing (cache only, no logout risk)
- Removed redundant dispatch_cdp error check

Verify fixes. Grant or withhold blessing. If withholding, state whether the issue is practical to fix or if it needs dialogue.
