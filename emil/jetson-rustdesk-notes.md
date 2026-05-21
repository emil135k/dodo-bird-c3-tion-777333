# Jetson RustDesk & Desktop Notes
## Sparked Matter LLC — April 1, 2026

---

## RustDesk Connection
- **RustDesk ID**: 112434933
- **Password**: test123
- **Auto-login user**: rocketman (no password needed at desktop)

## Reset X Windows (if desktop freezes)
From SSH terminal:
```bash
sudo systemctl restart lightdm
```

## Start/Stop Desktop
```bash
# Start desktop
sudo systemctl start lightdm

# Stop desktop (saves resources when not needed)
sudo systemctl stop lightdm
```

## RustDesk Zoom
- **Scale adaptive**: Right-click toolbar → Scale adaptive (auto-fit)
- **Ctrl + Scroll wheel**: zoom in/out
- **View menu**: zoom options

## Installed Diagnostic Tools
| Tool | Launch Command | Purpose |
|------|---------------|---------|
| coppwr | `flatpak run io.github.dimtpap.coppwr &` | Deep PipeWire diagnostics, timing graphs |
| qpwgraph | `qpwgraph &` | Visual PipeWire patchbay |
| Wireshark | `sudo wireshark &` | Network packet capture (Twilio, Bluetooth, USB) |
| Firefox | `firefox &` | Web browser |
| pw-top | `pw-top` (terminal) | Live PipeWire buffer/xrun monitor |

## Wireshark Capture Interfaces
| Interface | What it captures |
|-----------|-----------------|
| Loopback (lo) | Twilio WebSocket (port 5000), local IPC |
| bluetooth-monitor | Bluetooth HCI, L2CAP, Sony HFP debugging |
| usbmon | USB devices (Blackwire, future DJI drone) |

## Wireshark Useful Filters
```
tcp.port == 5000          # Twilio WebSocket traffic
bluetooth                  # All Bluetooth traffic
btatt                      # BLE Attribute Protocol
bthci_acl                  # HCI ACL packets
```

## Enable USB/Bluetooth Monitoring
```bash
sudo modprobe usbmon
```

## Virtual Display (headless, no TV needed)
Config at `/etc/X11/xorg.conf.d/10-virtual-display.conf`
Uses dummy driver at 1920x1080.

## Troubleshooting
- **Desktop frozen**: `sudo systemctl restart lightdm` from SSH
- **RustDesk won't connect**: Check `sudo systemctl status rustdesk`
- **No display**: `sudo systemctl start lightdm`
- **Snap stuck**: `snap changes` to check, `sudo snap abort <id>` to cancel
- **Firefox not found**: Installed at `/opt/firefox/firefox`, symlinked to `/usr/local/bin/firefox`
