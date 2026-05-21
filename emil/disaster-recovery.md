# Disaster Recovery — Binary Backup Strategy
## Sparked Matter LLC — March 30, 2026

---

## Problem
GitHub stores source code but NOT large binaries (ML models, CUDA libraries, ONNX runtimes). Need a way to backup and restore these for the Jetson Orin Nano.

## What Needs Backing Up

| Type | Size | Location on Jetson |
|------|------|--------------------|
| Parakeet STT model (INT8) | ~600MB | `/home/rocketman/downloads/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/` |
| Kokoro TTS model | ~300MB | `/home/rocketman/downloads/kokoro-sherpa-model.onnx` |
| Kokoro voices | ~50MB | `/home/rocketman/downloads/kokoro-sherpa-voices.bin` |
| Silero VAD | ~2MB | `/home/rocketman/downloads/silero_vad.onnx` |
| ONNX Runtime (CUDA) | ~350MB | `/home/rocketman/downloads/sherpa-onnx-v1.12.34-linux-aarch64-shared-gpu-onnxruntime-1.18.1/` |
| espeak-ng data | ~30MB | `/home/rocketman/downloads/espeak-ng-data/` |

**Total: ~1.3GB**

---

## Top 3 Tools

### 1. DVC (Data Version Control) — RECOMMENDED
- Built for ML models and large binaries alongside GitHub
- Pointer files (`.dvc`) go to GitHub, actual binaries go to your storage backend
- Supports: local drive, Google Drive, S3, Backblaze B2, SSH

```bash
# Setup
pip install dvc
cd ~/crystalballmini
dvc init
dvc remote add samsung /media/rocketman/Samsung/jetson-backup
# or
dvc remote add gdrive gdrive://folder-id

# Track files
dvc add /home/rocketman/downloads/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/
dvc push    # backs up to Samsung/Google Drive

# Restore on new Jetson
git clone repo
dvc pull    # downloads all models from backup
```

**Why DVC**: Knows which repo files belong to. Organized by content hash (deduped). Git-integrated. Purpose-built for ML.

---

### 2. git-annex — Power Tool
- Replaces large files with symlinks in Git, manages blobs separately
- `git annex whereis` — tells you where every copy lives (Samsung? Cloud? Jetson?)
- Location tracking across devices

```bash
# Setup
apt install git-annex   # or brew install git-annex
git annex init
git annex add *.so *.onnx

# Backup
git annex copy --to samsung-drive

# Restore
git annex get .
```

**Why git-annex**: Full redundancy tracking. Knows if file is on external drive, cloud, or both. Most flexible.

---

### 3. rclone — Simplest
- rsync for cloud storage (Google Drive, Dropbox, S3, local drives)
- No Git integration — just mirrors directories
- 70+ backends supported

```bash
# Setup
curl https://rclone.org/install.sh | bash
rclone config   # walks through OAuth for Google Drive

# Backup
rclone sync /home/rocketman/downloads/ gdrive:/jetson-models/

# Restore
rclone sync gdrive:/jetson-models/ /home/rocketman/downloads/
```

**Why rclone**: Zero learning curve. Just works. No repo changes needed.

---

## Decision Matrix

| Need | Use |
|------|-----|
| ML models tied to a GitHub project | DVC |
| Full location tracking across devices | git-annex |
| Just mirror a directory, no Git changes | rclone |

---

## Also: Download Script (Belt + Suspenders)
Keep a script in the repo that can download everything fresh from source:

```bash
# jetson/scripts/setup-models.sh
# Downloads all models from their original release URLs
# Run on a fresh Jetson to bootstrap without any backup
```

This way even if all backups are lost, you can rebuild from scratch.

---

*Earmarked for future implementation. Current priority: get Twilio bridge working.*
