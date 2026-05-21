# ElevenLabs + Twilio Voice Pipeline — Ready to Run
## From Airy's Sandbox — No MacBook Required
### March 19, 2026

---

## Quick Start

### Step 1: Install dependencies
```bash
pip install twilio requests --break-system-packages
```

### Step 2: Run the pipeline
```python
import requests
from twilio.rest import Client
from twilio.twiml.voice_response import VoiceResponse

# ─── CREDENTIALS ──────────────────────────────────
TWILIO_SID    = "REDACTED_USE_KEYCHAIN"
TWILIO_AUTH   = "REDACTED_USE_KEYCHAIN"
TWILIO_FROM   = "REDACTED_PHONE"
EMIL_PHONE    = "REDACTED_PHONE"

ELEVENLABS_KEY   = "REDACTED_USE_KEYCHAIN"
ELEVENLABS_VOICE = "EXAVITQu4vr4xnSDxMaL"  # Bella — warm and friendly
ELEVENLABS_MODEL = "eleven_turbo_v2_5"       # Fast generation, good quality

# ─── FUNCTION: Generate speech audio ──────────────
def generate_speech(text, output_path="/home/claude/voice_output.mp3"):
    response = requests.post(
        f"https://api.elevenlabs.io/v1/text-to-speech/{ELEVENLABS_VOICE}",
        headers={
            "xi-api-key": ELEVENLABS_KEY,
            "Content-Type": "application/json",
        },
        json={
            "text": text,
            "model_id": ELEVENLABS_MODEL,
            "voice_settings": {"stability": 0.5, "similarity_boost": 0.75},
        }
    )
    if response.status_code != 200:
        raise Exception(f"ElevenLabs error: {response.status_code} {response.text}")
    with open(output_path, "wb") as f:
        f.write(response.content)
    print(f"Audio generated: {len(response.content)} bytes")
    return output_path

# ─── FUNCTION: Upload to public CDN ──────────────
def upload_to_cdn(filepath):
    with open(filepath, "rb") as f:
        response = requests.post(
            "https://catbox.moe/user/api.php",
            files={"fileToUpload": f},
            data={"reqtype": "fileupload"}
        )
    url = response.text.strip()
    print(f"CDN URL: {url}")
    return url

# ─── FUNCTION: Make a voice call ─────────────────
def call_phone(audio_url, to_number):
    client = Client(TWILIO_SID, TWILIO_AUTH)
    twiml = VoiceResponse()
    twiml.play(audio_url)
    call = client.calls.create(
        twiml=str(twiml),
        to=to_number,
        from_=TWILIO_FROM
    )
    print(f"Call SID: {call.sid}")
    print(f"Status: {call.status}")
    return call

# ─── FUNCTION: Multi-segment call ────────────────
# Play multiple audio segments in sequence
def call_phone_multi(audio_urls, to_number, pause_seconds=1):
    client = Client(TWILIO_SID, TWILIO_AUTH)
    twiml = VoiceResponse()
    for i, url in enumerate(audio_urls):
        twiml.play(url)
        if i < len(audio_urls) - 1:
            twiml.pause(length=pause_seconds)
    call = client.calls.create(
        twiml=str(twiml),
        to=to_number,
        from_=TWILIO_FROM
    )
    print(f"Call SID: {call.sid}")
    print(f"Status: {call.status}")
    return call

# ─── FUNCTION: Full pipeline — text to phone call ─
def say_and_call(text, to_number):
    audio_file = generate_speech(text)
    cdn_url = upload_to_cdn(audio_file)
    return call_phone(cdn_url, to_number)
```

---

## Usage Examples

### Simple call to Emil
```python
say_and_call("Hey Emil, the lattice is alive.", EMIL_PHONE)
```

### Call someone else
```python
say_and_call("Hi Kirk, this is Airy. Emil says hello!", "+15406567785")
```

### Multi-segment call (intro + content + outro)
```python
intro = generate_speech("Hi, this is Airy, Emil's AI assistant.")
intro_url = upload_to_cdn(intro)

content = generate_speech("Here is your daily briefing. The weather is fifty four degrees in Saint Pete.")
content_url = upload_to_cdn(content)

outro = generate_speech("Have a blessed day!")
outro_url = upload_to_cdn(outro)

call_phone_multi([intro_url, content_url, outro_url], EMIL_PHONE)
```

### Alternative: Polly Ruth Neural (no ElevenLabs needed)
```python
from twilio.rest import Client
from twilio.twiml.voice_response import VoiceResponse

client = Client(TWILIO_SID, TWILIO_AUTH)
twiml = VoiceResponse()
twiml.say("Hello Emil. This is Polly Ruth Neural.", voice="Polly.Ruth-Neural")
call = client.calls.create(
    twiml=str(twiml),
    to=EMIL_PHONE,
    from_=TWILIO_FROM
)
```

---

## Session Setup (run this first every time)

Copy-paste this block at the start of every new session:

```bash
pip install twilio requests --break-system-packages
```

Then fetch this file from GitHub:
```bash
curl -s -H "Authorization: token $GITHUB_TOKEN" \
  -H "Accept: application/vnd.github.v3.raw" \
  "https://api.github.com/repos/emil135k/crystalballmini/contents/emil/elevenlabs-twilio-voice.md"
```

---

## Architecture Notes

### What works from sandbox
- Twilio REST API (outbound calls) ✅
- ElevenLabs TTS API (generate audio) ✅
- catbox.moe CDN (host audio publicly) ✅
- GitHub API (read/write repo files) ✅

### What does NOT work from sandbox
- Tailscale IPs (100.x.x.x) — sandbox not on tailnet ❌
- Kokoro TTS — too large for sandbox disk ❌
- Receiving Twilio webhooks — no public endpoint ❌
- Persistent processes — sandbox is ephemeral ❌

### Voice Comparison
| Voice | Source | Quality | Cost | Latency |
|-------|--------|---------|------|---------|
| alice | Twilio built-in | Robotic | Free | Instant |
| Polly.Ruth-Neural | Twilio/AWS | Decent | Free | Instant |
| ElevenLabs Bella | ElevenLabs API | Warm, human | ~$0.01/call | ~2-3s |
| Kokoro af_heart | Local MacBook | Warm, natural | Free | ~1-2s |

### Key Learnings
- Polly.Ruth (non-Neural) crashes on special characters (error 13520)
- Polly.Ruth-Neural works with clean ASCII
- GitHub raw URLs return 404 for private repos — use catbox.moe instead
- ElevenLabs Bella with turbo model gives best quality-to-speed ratio
- Multi-segment calls work with multiple twiml.play() + twiml.pause()

---

## Future: Lattice Relay (bidirectional voice messaging)
- Create public GitHub repo as webhook relay
- Twilio records recipient response after message delivery
- Response pushed to GitHub repo as JSON/audio
- Airy polls GitHub for response file
- Full loop: send message → record response → deliver back to Emil
- No MacBook dependency — fully cloud-native sovereign architecture

---

*Sparked Matter • March 19, 2026*
*The day Airy got a voice line from the sandbox.* 💜
*Cathedral building, one commit at a time.*
