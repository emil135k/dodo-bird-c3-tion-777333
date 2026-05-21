# System Baseline — 2026-03-18 22:52:57

## State: CLEAN (only Claude running, all services stopped)

## Memory Overview

| Metric | Value |
|--------|-------|
| Total RAM | 16384 MB |
| Free | 107 MB |
| Inactive (reclaimable) | 5430 MB |
| Active | 7189 MB |
| Wired (kernel) | 1296 MB |
| Speculative | 1745 MB |
| **Available (free+inactive)** | **5537 MB** |
| Swap Used | 0 MB |

## Top 15 Processes by Memory

| Rank | MB | Process |
|------|-----|---------|
| 1 | 696 | claude |
| 2 | 181 | /Applications/superwhisper.app/Contents/MacOS/superwhisper |
| 3 | 168 | /System/Library/Frameworks/CoreServices.framework/Frameworks/Metadata.framework/Versions/A/Support/mds_stores |
| 4 | 129 | /System/Library/PrivateFrameworks/SkyLight.framework/Resources/WindowServer |
| 5 | 108 | /System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal |
| 6 | 97 | /System/Applications/System |
| 7 | 95 | /System/Library/CoreServices/Finder.app/Contents/MacOS/Finder |
| 8 | 86 | /System/Applications/Stocks.app/Contents/PlugIns/StocksWidget.appex/Contents/MacOS/StocksWidget |
| 9 | 81 | /System/Library/CoreServices/NotificationCenter.app/Contents/MacOS/NotificationCenter |
| 10 | 81 | /System/Library/PrivateFrameworks/CoreSuggestions.framework/Versions/A/Support/suggestd |
| 11 | 80 | /System/Library/CoreServices/ControlCenter.app/Contents/MacOS/ControlCenter |
| 12 | 76 | /System/Applications/News.app/Contents/PlugIns/NewsToday2.appex/Contents/MacOS/NewsToday2 |
| 13 | 76 | /Applications/ExpressVPN.app/Contents/MacOS/ExpressVPN |
| 14 | 75 | /System/Library/CoreServices/Dock.app/Contents/MacOS/Dock |
| 15 | 73 | /Applications/ExpressVPN.app/Contents/MacOS/expressvpnd |

## Service Ports

| Port | Service | Status |
|------|---------|--------|
| 3000 | Old Jarvina | Free |
| 3001 | Pipecat/Jarvis | Free |
| 8880 | Kokoro-FastAPI | Free |

## Notes
- This is the pristine baseline after reboot/clean state
- Only Claude Code (620 MB) and system processes running
- No swap pressure, no zombies
- Use this to compare against loaded state

---

# Loaded State — 2026-03-18 22:54:13

## State: ALL SERVICES RUNNING (Kokoro-FastAPI + Pipecat/Jarvis)

## Memory Overview

| Metric | Clean | Loaded | Delta |
|--------|-------|--------|-------|
| Free | 107 MB | 168 MB | 61 MB |
| Inactive | 5430 MB | 7104 MB | 1674 MB |
| Active | 7189 MB | 7119 MB | -70 MB |
| Wired | 1296 MB | 1315 MB | 19 MB |
| **Available** | **5537 MB** | **7272 MB** | **1735 MB** |
| Swap | 0 MB | 0.00 MB | 0.00 MB |

## Top 15 Processes by Memory (Loaded)

| Rank | MB | Process |
|------|-----|---------|
| 1 | 1308 | /opt/homebrew/Cellar/python@3.12/3.12.13/Frameworks/Python.framework/Versions/3.12/Resources/Python.app/Contents/MacOS/Python |
| 2 | 710 | claude |
| 3 | 414 | /opt/homebrew/Cellar/python@3.12/3.12.13/Frameworks/Python.framework/Versions/3.12/Resources/Python.app/Contents/MacOS/Python |
| 4 | 181 | /Applications/superwhisper.app/Contents/MacOS/superwhisper |
| 5 | 155 | /System/Library/Frameworks/CoreServices.framework/Frameworks/Metadata.framework/Versions/A/Support/mds_stores |
| 6 | 130 | /System/Library/PrivateFrameworks/SkyLight.framework/Resources/WindowServer |
| 7 | 109 | /System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal |
| 8 | 108 | /System/Applications/System |
| 9 | 91 | /System/Library/CoreServices/Finder.app/Contents/MacOS/Finder |
| 10 | 85 | /System/Applications/Stocks.app/Contents/PlugIns/StocksWidget.appex/Contents/MacOS/StocksWidget |
| 11 | 81 | /System/Library/CoreServices/NotificationCenter.app/Contents/MacOS/NotificationCenter |
| 12 | 80 | /System/Library/PrivateFrameworks/CoreSuggestions.framework/Versions/A/Support/suggestd |
| 13 | 80 | /System/Library/CoreServices/ControlCenter.app/Contents/MacOS/ControlCenter |
| 14 | 79 | /System/Applications/News.app/Contents/PlugIns/NewsToday2.appex/Contents/MacOS/NewsToday2 |
| 15 | 76 | /Applications/ExpressVPN.app/Contents/MacOS/ExpressVPN |

## Service Ports (Loaded)

| Port | Service | Status |
|------|---------|--------|
| 3001 | Pipecat/Jarvis | UP |
| 8880 | Kokoro-FastAPI | UP |
| Funnel | Tailscale | UP |

## Service Memory Footprint

| Service | PID | MB |
|---------|-----|----|
| /opt/homebrew/Cellar/python@3.12/3.12.13/Frameworks/Python.framework/Versions/3.12/Resources/Python.app/Contents/MacOS/Python | 1529 | 1308 |
| /opt/homebrew/Cellar/python@3.12/3.12.13/Frameworks/Python.framework/Versions/3.12/Resources/Python.app/Contents/MacOS/Python | 1569 | 414 |
| Python: /opt/homebrew/Cellar/python@3.12/3.12.13/Frameworks/Python.framework/Versions/3.12/Resources/Python.app/Contents/MacOS/Python | 1529 | 1308 |
| Python: /opt/homebrew/Cellar/python@3.12/3.12.13/Frameworks/Python.framework/Versions/3.12/Resources/Python.app/Contents/MacOS/Python | 1569 | 414 |
