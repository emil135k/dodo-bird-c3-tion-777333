CRITICAL FINDING: Speaker is MUTED and mic STILL picks up TTS output. This is a digital software loopback inside CoreAudio, not acoustic echo. No AEC delay tuning will fix this.

Current setup: MacBook Pro Microphone (48kHz) + MacBook Pro Speakers (44100Hz). cpal build_input_stream on MacBook mic. cpal build_output_stream on MacBook speakers. When TTS plays through the output stream, the input stream captures it even with speaker physically muted.

This means cpal or CoreAudio is routing the output bus into the input bus internally. The AEC filter works as if no filter were in place because the reference and the loopback signal are IDENTICAL (no acoustic transformation) but likely shifted by the output buffer latency.

Updated code pushed to ants/patchbay-ant/src/main.rs. Review and help find the smoking gun. How do we stop CoreAudio from routing output to input? Is there a cpal config to force hardware-only capture? Is there a macOS setting creating this loopback?
