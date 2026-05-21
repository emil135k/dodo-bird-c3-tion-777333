# Library Download Shopping List — Jetson Orin Nano Super
## JetPack 6.2 / L4T R36.4.3 / CUDA 12.6 / aarch64
### Date: 2026-03-27

---

## PRIORITY 1: MUST-HAVE (~914 MB)

### sherpa-onnx C++ GPU bundle (67 MB)
```
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.12.34/sherpa-onnx-v1.12.34-linux-aarch64-shared-gpu-onnxruntime-1.18.1.tar.bz2
```

### Parakeet TDT v3 INT8 ONNX model (465 MB)
```
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2
```

### Kokoro TTS FP16 GPU model (169 MB)
```
wget https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.fp16-gpu.onnx
```

### Kokoro TTS INT8 backup (88 MB)
```
wget https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.int8.onnx
```

### Kokoro voices (27 MB)
```
wget https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin
```

### Kokoro espeak data (12 MB)
```
wget https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files/espeak-ng-data-v1.51.tar.gz
```

### ONNX Runtime GPU Jetson wheel (84 MB)
```
wget https://pypi.jetson-ai-lab.io/jp6/cu126/+f/4eb/e6a8902dc7708/onnxruntime_gpu-1.23.0-cp310-cp310-linux_aarch64.whl
```

### Silero VAD ONNX (2 MB)
```
wget https://raw.githubusercontent.com/snakers4/silero-vad/master/src/silero_vad/data/silero_vad.onnx
```

---

## PRIORITY 2: NICE-TO-HAVE (~5.2 GB)

### Parakeet TDT v3 full .nemo (2,393 MB)
```
wget https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3/resolve/main/parakeet-tdt-0.6b-v3.nemo
```

### Parakeet CTC 0.6b .nemo — for streaming (2,323 MB)
```
wget https://huggingface.co/nvidia/parakeet-ctc-0.6b/resolve/main/parakeet-ctc-0.6b.nemo
```

### Parakeet TDT v2 INT8 backup (460 MB)
```
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8.tar.bz2
```

---

## TOTALS
- **Must-have: ~914 MB** (under 1 GB)
- **Nice-to-have: ~5,183 MB**
- **Everything: ~6,097 MB** (~6 GB)

## NOTES
- All wget commands use direct download URLs
- Add `-L` flag if redirects don't follow: `wget -L <url>`
- sherpa-onnx bundle includes its own ONNX Runtime 1.18.1 for C++
- The Jetson wheel (ORT 1.23.0) is Python-only — grab it as backup
- Download to USB drive or directly to Jetson over library WiFi
