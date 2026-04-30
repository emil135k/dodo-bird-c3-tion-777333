#!/bin/bash
# Start the sovereign swarm — hypAiAssist
#
# Usage:
#   bash start-swarm.sh              # Local only (Blackwire mic/speaker)
#   bash start-swarm.sh --twilio     # Local + Twilio phone bridge
#   bash start-swarm.sh --no-llm     # Local without LLM (testing audio chain)
#   bash start-swarm.sh --twilio --no-llm  # Phone bridge without LLM
#
# Run Ollama first: OLLAMA_CONTEXT_LENGTH=32768 ollama serve

USE_TWILIO=false
USE_LLM=true

for arg in "$@"; do
    case $arg in
        --twilio) USE_TWILIO=true ;;
        --no-llm) USE_LLM=false ;;
    esac
done

echo "=== Killing old ants ==="
pkill -9 -f stt-ant 2>/dev/null
pkill -9 -f ear-ant 2>/dev/null
pkill -9 -f silero-ant 2>/dev/null
pkill -9 -f parakeet-worker 2>/dev/null
pkill -9 -f mouth-ant 2>/dev/null
pkill -9 -f tts-ant 2>/dev/null
pkill -9 -f llm-ant 2>/dev/null
pkill -9 -f patchbay-ant 2>/dev/null
pkill -9 -f twilio-ant 2>/dev/null
pkill -9 -f digi-ant 2>/dev/null
pkill -9 -f phone-silero-ant 2>/dev/null
pkill -9 -f web-ant 2>/dev/null
sleep 2

echo "=== Nuking stale segments ==="
rm -f /tmp/iox2_*.shm_state
rm -rf /tmp/iceoryx2
sleep 1

# --- Core ants (always start) ---

echo "=== Starting TTS ant ==="
tts-ant > /tmp/tts-ant-stdout.log 2>&1 &
sleep 8

echo "=== Starting STT ant ==="
stt-ant > /tmp/stt-ant-stdout.log 2>&1 &
sleep 10

echo "=== Starting Silero ant ==="
ORT_DYLIB_PATH=/opt/homebrew/lib/libonnxruntime.dylib silero-ant > /tmp/silero-ant-stdout.log 2>&1 &
sleep 2

echo "=== Starting Patchbay ant ==="
patchbay-ant > /tmp/patchbay-ant-stdout.log 2>&1 &
sleep 2

# --- Optional: LLM ---

if [ "$USE_LLM" = true ]; then
    echo "=== Starting LLM ant ==="
    llm-ant > /tmp/llm-ant-stdout.log 2>&1 &
    sleep 2
else
    echo "=== Skipping LLM ant (--no-llm) ==="
fi

# --- Optional: Twilio phone bridge ---

if [ "$USE_TWILIO" = true ]; then
    echo "=== Starting Digi ant ==="
    digi-ant > /tmp/digi-ant-stdout.log 2>&1 &
    sleep 2

    echo "=== Starting Phone Silero ant ==="
    ORT_DYLIB_PATH=/opt/homebrew/lib/libonnxruntime.dylib phone-silero-ant > /tmp/phone-silero-ant-stdout.log 2>&1 &
    sleep 2

    echo "=== Starting Web ant ==="
    web-ant > /tmp/web-ant-stdout.log 2>&1 &
    sleep 2
else
    echo "=== Skipping Twilio bridge (use --twilio to enable) ==="
fi

# --- Status ---

echo ""
echo "=== SWARM STATUS ==="
ANTS="tts-ant stt-ant silero-ant patchbay-ant parakeet-worker"
[ "$USE_LLM" = true ] && ANTS="$ANTS llm-ant"
[ "$USE_TWILIO" = true ] && ANTS="$ANTS digi-ant phone-silero-ant web-ant"

for ant in $ANTS; do
    PID=$(pgrep -f "$ant" | head -1)
    if [ -n "$PID" ]; then
        echo "  ✓ $ant (PID $PID)"
    else
        echo "  ✗ $ant NOT RUNNING"
    fi
done

echo ""
echo "Mode: LOCAL$([ "$USE_LLM" = true ] && echo " + LLM")$([ "$USE_TWILIO" = true ] && echo " + TWILIO")"
echo "Speak into your Blackwire headset."
[ "$USE_TWILIO" = true ] && echo "Or call +18136076219."
echo "Tail logs: tail -f /tmp/*-ant-stdout.log"
