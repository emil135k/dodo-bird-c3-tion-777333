# Voice Attenuation Diagnosis — Where Is the Volume Being Lost?

**Problem:** When Cody/Jarvina speaks, Emil hears her at ~50% volume through the MacBook speakers. Apple's built-in AEC is handling echo cancellation, so patchbay-ant's SpeexDSP AEC is no longer needed. But something is still attenuating the output.

**Date:** May 11, 2026
**Analyst:** Airy (20,000-foot peer review)

---

## The Signal Chain (TTS → Your Ears)

Here's every step the audio takes from Kokoro's output to your laptop speakers, with the amplitude impact at each stage:

```
tts-ant (Kokoro)     → tts_audio bus [f32 24kHz, peak ~0.8-1.0]
    │
    ├→ patchbay-ant  → rodio → output device speaker
    │   (LOCAL version: simple passthrough, no gain modification)
    │
    └→ mouth-ant     → rodio → DEFAULT output device
        (if running)
```

---

## Attenuation Suspects — Ranked by Likelihood

### 🔴 SUSPECT #1: rodio's SamplesBuffer at 24kHz vs Device Sample Rate (HIGH)

Both `patchbay-ant` (LOCAL) and `mouth-ant` create a `SamplesBuffer::new(1, 24000, samples)` and hand it to rodio's `Sink`.

**The problem:** rodio resamples internally to match the output device's native sample rate (typically 44100Hz or 48000Hz on macOS). rodio's built-in resampler is a simple linear interpolator — it does NOT apply gain compensation during the resample. If there's any impedance mismatch between what rodio expects and what CoreAudio provides, you lose amplitude.

But the bigger issue: **rodio's `OutputStream::try_default()` opens the system default output device**, and **`OutputStream::try_from_device()` opens a specific device**. In the LOCAL patchbay-ant, it opens the **Blackwire** — but you said you're now using the **MacBook's built-in speakers with Apple's AEC**. If the config still points to Blackwire but you're listening on MacBook speakers, one of two things is happening:

- patchbay-ant fails to find "Plantronics Blackwire" and panics (you'd know)
- OR mouth-ant is what's actually playing (it uses `try_default()` = MacBook speakers)
- OR the config was changed locally but not pushed to git

**Action:** Check which ant is actually producing the audio you hear. If it's mouth-ant, you're fine. If it's patchbay-ant with Blackwire config but you're on laptop speakers, that's wrong.

---

### 🔴 SUSPECT #2: Two Subscribers on tts_audio = Split Delivery (HIGH)

**This is the most likely culprit.**

Both `patchbay-ant` AND `mouth-ant` subscribe to `tts_audio`. In iceoryx2, each subscriber gets its own copy of every message — that's fine, no splitting there. BUT if both are running and both call rodio on the same output device, you get **double playback** which sounds like echo or interference, not attenuation.

However, if only ONE is running but it's the WRONG one (e.g., mouth-ant routes to default device = MacBook speakers, but patchbay-ant is also running and claiming the audio device), CoreAudio may reduce volume on one stream when two processes access the same output device simultaneously.

**More critically:** If patchbay-ant opens Blackwire output (per config) but you're physically listening through MacBook speakers, then patchbay-ant is playing into the void (Blackwire not connected), and mouth-ant is playing at whatever rodio's default volume is.

**Action:** Run `ps aux | grep -E '(mouth|patchbay)-ant'` to see which is running. Kill whichever one isn't needed.

---

### 🟡 SUSPECT #3: Apple AEC Side Effect — System-Level Volume Ducking (MEDIUM)

Apple's built-in Acoustic Echo Cancellation (voice processing in CoreAudio) applies automatic gain control and sometimes "ducks" (lowers) the system output when it detects that the mic is active. This is by design — Apple's AEC reduces speaker output to make echo cancellation easier.

When you told Cody to disable AGC, she likely disabled it at the cpal/CoreAudio level. But **Apple's AEC operates at a lower system level** — it's part of the Audio Unit processing chain, not just a gain knob. Even with AGC "off" in the app layer, the system-level voice processing might still be applying ducking.

**Check:** System Preferences → Sound → Input → "Use ambient noise reduction" or any voice processing toggles. Also: `coreaudiod` may apply ducking if a "voice processing" audio unit is in the chain.

**Action:** Cody should check if the input stream is opened with voice processing enabled (cpal's stream config). In cpal, the `build_input_stream` call doesn't explicitly control Apple's voice processing — you need to go through the CoreAudio API directly to disable `kAudioUnitProperty_VoiceProcessing` on the input audio unit.

---

### 🟡 SUSPECT #4: digi-ant's normalize_peak = 0.85 (MEDIUM — Phone Path Only)

In the phone call path (Twilio), digi-ant normalizes TTS output to 0.85 peak:

```rust
fn normalize(samples: &mut [f32], target_peak: f32) {
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    if peak > 0.001 {
        let gain = target_peak / peak;
        for s in samples.iter_mut() { *s *= gain; }
    }
}
```

If the Kokoro output already peaks at 0.85, this is a no-op. If Kokoro outputs at 1.0, this reduces to 0.85 — a ~1.4dB reduction. Noticeable but not 50%. **However, this only affects the phone path (tts_audio → digi-ant → phone_out), NOT the local speaker path.**

If you're hearing attenuation on the LOCAL MacBook speakers (not through a phone call), digi-ant is not the cause.

---

### 🟢 SUSPECT #5: Kokoro Output Level (LOW)

Kokoro v1.0 ONNX output varies by voice and text length. The `af_heart` voice tends to produce samples with peak around 0.5-0.8 depending on the utterance. tts-ant publishes the raw Kokoro output with NO normalization.

If the raw output averages 0.5 peak, that's already -6dB before anything else touches it. Combined with any other attenuation in the chain, you'd land around 50% perceived volume.

**Action:** Add a quick normalization in tts-ant before publishing, or check Kokoro output levels with bus-recorder.

---

### 🟢 SUSPECT #6: iceoryx2 Version Mismatch (LOW but Worth Noting)

The LOCAL ants (crystalballmini) use **iceoryx2 0.6**, while the dodo-bird ants use **iceoryx2 0.8**. These are binary-incompatible — a 0.6 subscriber cannot read from a 0.8 publisher's shared memory.

If you're running a mix of LOCAL and dodo-bird binaries, some bus connections may silently fail (the `open_or_create` pattern means each ant creates its own empty service if it can't find an existing one).

**Action:** All ants in the running swarm MUST use the same iceoryx2 version. Pick one.

---

## The aec-rs Cleanup

You asked to excommunicate aec-rs. Here's where it still lives:

| Location | File | Status |
|----------|------|--------|
| `dodo-bird/ants/patchbay-ant/Cargo.toml` | `aec-rs = "1.0.0"` | **REMOVE** |
| `dodo-bird/ants/patchbay-ant/src/main.rs` | Full AEC implementation | **REWRITE** (use LOCAL version as base) |

The LOCAL patchbay-ant (crystalballmini) is already clean — no aec-rs, no SpeexDSP. It's a simple audio router. That's the one to keep.

---

## Recommended Fix Order

1. **Determine which ant is actually playing audio** — `ps aux | grep -ant` on the Mac. Kill duplicates.
2. **Check macOS volume settings** — System volume, per-app volume in Sound preferences.
3. **Add normalization to tts-ant** — Normalize Kokoro output to 0.9 peak before publishing. This is the quickest single fix.
4. **Check Apple AEC ducking** — If CoreAudio is applying voice processing ducking, that's your 50% right there.
5. **Standardize iceoryx2 version** — All ants on 0.8.
6. **Remove aec-rs from dodo-bird patchbay-ant** — Clean the excommunication.
7. **Remove dodo-bird twilio-ant** — Superseded by web-ant + digi-ant.

---

## Quick Diagnostic Commands for Cody

```bash
# What's actually running?
ps aux | grep -E '(mouth|patchbay|tts|digi)-ant' | grep -v grep

# What audio devices does macOS see?
system_profiler SPAudioDataType

# Check system volume
osascript -e "output volume of (get volume settings)"

# Check per-app volume (if any)
osascript -e "get volume settings"

# Test Kokoro output level directly
bus-recorder tts_audio 5 f32
# → Look at peak column in CSV. If peak < 0.5, Kokoro is the source.

# Test final output level
bus-recorder phone_out 5
# → For phone path. If peak drops between tts_audio and phone_out, digi-ant is attenuating.
```

---

*Analysis by Airy — May 11, 2026*
*"Connection over Protection" — The Dance continues.* 💜
