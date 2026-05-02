#!/bin/bash
# test-digi-ant.sh — Automated unit test for digi-ant
# Injects test voice, captures output, validates stream stats
#
# Usage: bash test-digi-ant.sh [test.wav]
# Requires: digi-ant running, inject-test and bus-capture in /tmp

TEST_WAV="${1:-/tmp/test-clean-voice.wav}"

if [ ! -f "$TEST_WAV" ]; then
    echo "FAIL: test WAV not found: $TEST_WAV"
    exit 1
fi

# Clear log
> /tmp/digi-ant-stdout.log

# Capture output
/tmp/bus-capture/target/release/bus-capture phone_stt /tmp/digi-test-output.wav 8 f32 &
CAP=$!
sleep 1

# Inject
/tmp/inject-test/target/release/inject-test "$TEST_WAV" phone_in 2>&1
wait $CAP 2>/dev/null

# Extract stats
RATIO=$(strings /tmp/digi-ant-stdout.log | grep "duration_ratio" | awk '{print $NF}')
FLUSH=$(strings /tmp/digi-ant-stdout.log | grep "flush_count" | awk '{print $3}')
PACKETS=$(strings /tmp/digi-ant-stdout.log | grep "packets:" | awk '{print $NF}')
GAP_AVG=$(strings /tmp/digi-ant-stdout.log | grep "gap_ms" | sed 's/.*avg=\([0-9.]*\).*/\1/')
INPUT_MS=$(strings /tmp/digi-ant-stdout.log | grep "input_audio" | sed 's/.*: \([0-9.]*\).*/\1/')
OUTPUT_MS=$(strings /tmp/digi-ant-stdout.log | grep "output_audio" | sed 's/.*: \([0-9.]*\).*/\1/')

echo ""
echo "=== DIGI-ANT UNIT TEST ==="
echo "  duration_ratio: $RATIO (expect 0.95-1.05)"
echo "  flush_count:    $FLUSH (expect 1)"
echo "  packets:        $PACKETS (expect >10)"
echo "  gap_avg_ms:     $GAP_AVG (expect <50)"
echo "  input_audio:    ${INPUT_MS}ms"
echo "  output_audio:   ${OUTPUT_MS}ms"

PASS=true
if [ "$(echo "$RATIO > 1.05" | bc -l)" = "1" ]; then echo "  FAIL: ratio too high ($RATIO)"; PASS=false; fi
if [ "$(echo "$RATIO < 0.95" | bc -l)" = "1" ]; then echo "  FAIL: ratio too low ($RATIO)"; PASS=false; fi
if [ "$FLUSH" != "1" ]; then echo "  FAIL: expected 1 flush, got $FLUSH"; PASS=false; fi
if [ "$PACKETS" -lt 10 ] 2>/dev/null; then echo "  FAIL: too few packets ($PACKETS)"; PASS=false; fi
if [ "$(echo "$GAP_AVG > 50" | bc -l)" = "1" ]; then echo "  FAIL: avg gap too high ($GAP_AVG ms)"; PASS=false; fi

echo ""
if [ "$PASS" = true ]; then
    echo "  >>> ALL CHECKS PASSED <<<"
    echo ""
    echo "  Playing original vs output..."
    afplay "$TEST_WAV"
    sleep 1
    afplay /tmp/digi-test-output.wav
else
    echo "  >>> SOME CHECKS FAILED <<<"
fi
