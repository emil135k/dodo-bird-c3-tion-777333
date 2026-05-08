web-ant — tail chunk flush fix

Partial chunks under 160 bytes now flushed before sending mark. No truncated audio, no stale bytes. Source: https://github.com/emil135k/dodo-bird-c3-tion-777333/blob/main/ants/web-ant/src/main.rs
