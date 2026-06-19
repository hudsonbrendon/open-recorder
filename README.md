# OpenRecorder

An open-source, cross-platform screen recorder built with **Tauri 2** (Rust core) + **React + TypeScript** (frontend). Record your screen, window, or region with audio; save as H.264 MP4 + machine-readable metadata (timestamps, events, source info).

## Status

**Foundation (F1):** Screen/window/region capture + microphone + click/mouse event logging.

**Roadmap:**
- **F2:** Auto-zoom on click + integrated mini editor
- **F3:** Webcam overlay
- **F4:** 9:16 export + Instagram/TikTok preview

## Features

- **Multiple Capture Sources:** Full screen, active window, or custom region
- **Audio Recording:** Simultaneous microphone capture
- **Event Logging:** Click and mouse movement timestamps (platform permissions permitting)
- **Metadata Export:** Machine-readable JSON with recording info, source, and events
- **Cross-Platform:** macOS native (Windows/Linux support planned)

## Requirements

### System Dependencies
- **Node.js** 18+ with **pnpm** (package manager)
- **Rust** + **Cargo** (for Tauri build)
- **ffmpeg** on PATH (H.264 encoding + container muxing)

### macOS Permissions
The app gracefully degrades if permissions are not granted:
- **Screen Recording:** Required to capture video. Without it, recording fails with an error.
- **Microphone:** Required to record audio. Without it, recordings proceed with video only (no audio track).
- **Input Monitoring** (Accessibility): Required to log mouse clicks and movements. Without it, videos and audio still record; events array remains empty (no error).

Grant permissions in **System Preferences > Privacy & Security**:
1. Screen Recording: Add the Tauri app
2. Microphone: Enable
3. Accessibility: Add the Tauri app

## Installation & Setup

```bash
# Clone the repository
git clone https://github.com/your-org/open-recorder.git
cd open-recorder

# Install dependencies
pnpm install

# Verify ffmpeg is on PATH
which ffmpeg  # Should return a path; if not, install via Homebrew: brew install ffmpeg
```

## Commands

### Development
```bash
# Run the app in dev mode (Tauri + hot reload)
pnpm tauri dev
```

### Testing
```bash
# Frontend unit tests (Vitest)
pnpm test

# Rust backend tests
cd src-tauri && cargo test
```

### Build
```bash
# Create production app bundle
pnpm tauri build
# Output: src-tauri/target/release/bundle/ (macOS .app, Windows .msi, Linux .deb, etc.)
```

## How It Works

1. **Capture:** Uses `scap` (Rust) to grab screen/window pixels at ~30 FPS
2. **Audio:** Uses `cpal` (Rust) to capture microphone as WAV stream
3. **Events:** Uses `rdev` (Rust) to log mouse and keyboard events
4. **Encode:** Feeds video/audio to `ffmpeg` (via PATH) for H.264 + AAC mux → MP4
5. **Metadata:** Writes JSON with recording info, source, and timestamped events
6. **UI:** React/TypeScript frontend for source selection, start/stop, and file browser

## Output

Recordings are saved to **`~/Movies/OpenRecorder/`** (macOS):
- **`REC-<timestamp>.mp4`** — H.264 video + AAC audio (or video-only if mic disabled)
- **`REC-<timestamp>.metadata.json`** — Version 1 metadata structure:
  ```json
  {
    "version": 1,
    "recording": {
      "width": 2560,
      "height": 1440,
      "fps": 30,
      "duration_ms": 18450
    },
    "source": {
      "type": "display",
      "id": "1",
      "rect": [0, 0, 2560, 1440]
    },
    "events": [
      {
        "t_ms": 1200,
        "type": "click",
        "x": 840,
        "y": 410,
        "button": "left"
      },
      {
        "t_ms": 1300,
        "type": "move",
        "x": 845,
        "y": 412
      }
    ]
  }
  ```
  **Key fields:**
  - `recording`: width/height (pixels), fps, duration_ms
  - `source.type`: "display", "window", or "region"; `id` is a string; `rect` is [x, y, width, height]
  - `events`: optional (empty if Input Monitoring not granted); `t_ms` is milliseconds, `type` is "click" or "move", and `button` is present only for clicks ("left" or "right")

## Known Limitations (F1)

- **Non-Primary Display:** Recordings on secondary displays may fall back to the primary display. Workaround: use Full Screen capture of primary, or record a window instead.
- **Window Geometry:** Window-only capture is best-effort. Partially off-screen windows will be clipped to available area.
- **Click Event Precision:** Event coordinates are currently coarse and may serve as placeholders; F2 will refine with precise pixel-level click zones.
- **ffmpeg Bundling:** The app requires `ffmpeg` on PATH. Bundled sidecar is planned for distribution phase.

## Troubleshooting

### "Screen Recording permission denied"
- Open System Preferences > Privacy & Security > Screen Recording
- Add the Tauri app (you may need to drag it from /Applications)
- Restart the app

### "ffmpeg not found"
```bash
# Install via Homebrew
brew install ffmpeg

# Verify
ffmpeg -version
```

### "Events array is empty"
- Check System Preferences > Privacy & Security > Accessibility
- Add the Tauri app
- Restart the app
- This is expected behavior if Input Monitoring is denied; video and audio still record

### App crashes during record/stop
- Check Console.app for error logs (filter by app name)
- Report the error message to GitHub Issues with your macOS version and app build

## Testing

Run the **smoke test checklist** in `/docs/SMOKE-TEST.md` to verify capture, audio, metadata, and event logging on your macOS system.

## License

MIT (details to be added)

## Contributing

Contributions are welcome. Please open an issue or PR on GitHub.

---

**Links:**
- [Smoke Test Checklist](./docs/SMOKE-TEST.md)
- [GitHub Issues](https://github.com/your-org/open-recorder/issues)
