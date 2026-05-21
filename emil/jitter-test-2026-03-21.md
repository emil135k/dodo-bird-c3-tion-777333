# Jarvina Jitter Test — Library WiFi

**Date**: 2026-03-21 5:11 PM EDT
**Location**: Library, St. Petersburg, FL
**Connection**: Library WiFi → Tailscale Funnel → MacBook (Hawk camper)
**Capture**: `tailscale debug capture` → `/tmp/tailscale_test.pcap`

---

## Summary

| Metric | Value | Rating |
|--------|-------|--------|
| **Call Duration** | 109 seconds | — |
| **Total Packets** | 11,052 | — |
| **Total Data** | 3,528 KB | — |
| **Median Packet Gap** | 6.0 ms | ✅ Excellent |
| **Mean Packet Gap** | 25.1 ms | ✅ Good |
| **Jitter (std dev)** | 63.3 ms | ⚠️ Moderate |
| **Avg Jitter** | 31.2 ms | ⚠️ Moderate |
| **Packets < 20ms** | 85.7% | ✅ Strong |
| **Packets > 100ms** | 6.6% | ⚠️ Spikes |

---

## Packet Gap Distribution

```
  <10ms  ████████████████████████████████████████████  221 (60.5%)
 10-20ms ████████████████████                          92 (25.2%)
 20-40ms ██                                             9 ( 2.5%)
 40-60ms ██                                            11 ( 3.0%)
60-100ms ██                                             8 ( 2.2%)
  >100ms ████                                          24 ( 6.6%)
```

---

## Interpretation

### What's Good
- **85.7% of packets arrive within 20ms** — the vast majority of audio flows smoothly
- **Median gap of 6ms** — when things work, they work fast
- Library WiFi + Tailscale Funnel is viable for voice calls

### What Needs Watching
- **6.6% of packets exceed 100ms** — these cause brief audio gaps or choppiness
- **31ms average jitter** — right at the boundary of "good" and "noticeable"
- Likely caused by WiFi congestion at the library (shared public network)

### Jitter Reference Scale

| Jitter | Quality |
|--------|---------|
| < 15ms | Crystal clear — studio quality |
| 15–30ms | Good — minor artifacts, barely noticeable |
| 30–50ms | Noticeable — jitter buffer compensates |
| > 50ms | Choppy — audible gaps and artifacts |

---

## Jitter Buffer Performance

Our custom jitter buffer (`JitterBuffer` in `server.py`) absorbs packet timing variations by:
1. Collecting audio chunks into a deque as they arrive
2. Draining at a steady 20ms pace regardless of arrival timing
3. Smoothing out the 6.6% spike packets that arrive late

**The buffer is doing its job** — without it, 31ms jitter would sound choppy. With it, most artifacts are absorbed.

---

## Comparison Data (Future Tests)

| Test | Location | Connection | Median Gap | Jitter | Under 20ms | Spikes >100ms | Rating |
|------|----------|------------|------------|--------|------------|---------------|--------|
| #1 | Library | Public WiFi → Tailscale | 6.0ms | 31.2ms | 85.7% | 6.6% | ⚠️ Moderate |
| #2 | Hawk Camper | Visible Cellular → Tailscale | 6.7ms | 26.5ms | 88.6% | 5.3% | ✅ Good |
| #3 | — | — | — | — | — | — | — |

### Test #2 — Hawk Camper + Visible Cellular (5:35 PM EDT)

| Metric | Value | vs Library |
|--------|-------|------------|
| **Call Duration** | 280 seconds | +171s longer test |
| **Total Packets** | 27,861 | — |
| **Total Data** | 8,937 KB | — |
| **Median Packet Gap** | 6.7 ms | ≈ same |
| **Jitter (avg)** | 26.5 ms | ✅ 15% better |
| **Packets < 20ms** | 88.6% | ✅ 3% better |
| **Packets > 100ms** | 5.3% | ✅ 20% fewer spikes |

```
  <10ms  █████████████████████████████████████████████████  507 (67.8%)
 10-20ms ██████████████████                                 156 (20.9%)
 20-40ms ██                                                  23 ( 3.1%)
 40-60ms █                                                    9 ( 1.2%)
60-100ms █                                                   13 ( 1.7%)
  >100ms ███                                                 40 ( 5.3%)
```

**Verdict**: Visible cellular from the Hawk is BETTER than Library WiFi. Lower jitter (26.5ms vs 31.2ms), fewer spikes (5.3% vs 6.6%), more packets under 20ms (88.6% vs 85.7%). The "6% Bear" that Lyra identified is smaller on Visible. Library WiFi congestion was adding jitter.

*Fill in Test #3+ as we test from more locations (AWS, different hotspot, etc.)*

---

## How to Reproduce This Test

```bash
# 1. Start capture
tailscale debug capture -o /tmp/tailscale_test.pcap &

# 2. Make a call to Jarvina
#    Call +18136076219, talk for 1-2 minutes

# 3. Stop capture
pkill -f "tailscale debug capture"

# 4. Analyze
tshark -r /tmp/tailscale_test.pcap -T fields -e frame.time_relative -e frame.len | python3 analyze_jitter.py
```

---

*Sparked Matter LLC — the smartest spark in the room*
*Test #1 — Baseline established*
