# OpenRecorder F1 Smoke Test Checklist

Manual verification checklist for the capture foundation (F1) on macOS. Follow each step in order. Mark completed items with an ✓.

## Pre-Test Setup

- [ ] Clone and build: `pnpm install && pnpm tauri build`
- [ ] Open built app in `/dist/` or run `pnpm tauri dev`
- [ ] System preferences: Grant **Screen Recording**, **Microphone**, and **Input Monitoring** (Accessibility) permissions to the Tauri app process
  - Settings > Privacy & Security > Screen Recording: Add the Tauri app
  - Settings > Privacy & Security > Microphone: Allow
  - Settings > Privacy & Security > Accessibility: Add the Tauri app (for rdev click capture)

## Test 1: Full Screen + Microphone

- [ ] Open the recording UI
- [ ] Select "Full Screen" as capture source
- [ ] Verify microphone is **On** (not muted)
- [ ] Click **Start Recording**
- [ ] Speak into mic (e.g., "Test audio") while moving mouse and clicking for ~10 seconds
- [ ] Click **Stop Recording**
- [ ] Confirm UI returns to idle (no error modal)
- [ ] Open **~/Movies/OpenRecorder** folder
- [ ] Verify file appears: `REC-*.mp4` and `REC-*.metadata.json`

## Test 2: Verify MP4 Playback

- [ ] Double-click `REC-*.mp4` to open in QuickTime Player (or VLC)
- [ ] Verify video plays: full screen content visible, no corruption
- [ ] Verify audio plays: your spoken test phrases are audible at correct volume
- [ ] Verify duration: ~10 seconds (or whatever you recorded)

## Test 3: Verify Metadata JSON

- [ ] Open `REC-*.metadata.json` in a text editor
- [ ] Check structure contains:
  - `"version": 1`
  - `"recording": { "start_ts": "...", "end_ts": "...", "duration_ms": ... }`
  - `"source": { "kind": "FullScreen", "display_id": ... }`
  - `"audio": { "channels": 2, "sample_rate": 48000 }`
  - `"events": [ { "type": "...", "ts": ..., ... }, ... ]` (event array, may be empty if Input Monitoring denied)

## Test 4: Window Capture

- [ ] Open a second window (e.g., Terminal, Finder, or browser)
- [ ] In OpenRecorder, select that window from the source dropdown
- [ ] Verify it shows in the list as "Window: <app name>"
- [ ] Click **Start Recording**
- [ ] Interact with the window for ~5 seconds
- [ ] Click **Stop Recording**
- [ ] Verify new `REC-*.mp4` and metadata appear in `~/Movies/OpenRecorder`
- [ ] Play the MP4: confirm only the selected window content is recorded

## Test 5: Microphone Off

- [ ] In the recording UI, toggle microphone to **Off**
- [ ] Select "Full Screen"
- [ ] Click **Start Recording**
- [ ] Speak into mic, move mouse for ~5 seconds
- [ ] Click **Stop Recording**
- [ ] Open `REC-*.mp4` in QuickTime
- [ ] Verify video plays: no audio track (silent)
- [ ] Open `REC-*.metadata.json`: confirm `"audio"` object is still present, sample_rate matches (no audio frames written)

## Test 6: Input Monitoring Not Granted

- [ ] Revoke Input Monitoring from System Preferences > Accessibility
- [ ] Restart the Tauri app
- [ ] Select "Full Screen", mic **On**
- [ ] Click **Start Recording**
- [ ] Move mouse and click for ~5 seconds (clicks may not be logged)
- [ ] Click **Stop Recording**
- [ ] Verify `REC-*.mp4` and metadata appear (no error)
- [ ] Open `REC-*.metadata.json`: confirm `"events": []` (empty array, graceful degradation)
- [ ] Play the MP4: video and audio still record correctly, despite missing click events

## Test 7: Multiple Record/Stop Cycles

- [ ] Perform 3–5 consecutive record/stop cycles without closing the app
  - Start, record for 3–5 seconds, Stop
  - Repeat with different sources (Full Screen, then a Window)
- [ ] After each cycle, confirm a new timestamped pair (`REC-*.mp4` + `.metadata.json`) appears in folder
- [ ] App does not crash, freeze, or leak files
- [ ] All recorded MP4s play without corruption

## Test 8: Folder Opening

- [ ] In the recording list, click **Mostrar** (Show) button next to any recording
- [ ] Verify Finder opens to `~/Movies/OpenRecorder` with that file highlighted
- [ ] Confirm folder is not empty and contains all test recordings

## Test 9: Known Limitations

- [ ] Non-primary display: If you have multiple displays, attempt to record from a secondary monitor
  - Note: May fall back to primary display (document in metadata if observed)
- [ ] Window geometry: Record a partially off-screen window
  - Expect best-effort capture of visible area; no error modal
- [ ] Click event precision: Open metadata and inspect `events[].x` / `events[].y` coordinates
  - Note: Currently coarse; coordinates may be placeholder or relative to display origin (limitation noted for F2)

## Post-Test Summary

- [ ] All recordings are present and named with `REC-` prefix + timestamp
- [ ] All MP4s play with correct video + audio (or audio-off if mic disabled)
- [ ] All metadata.json files are valid JSON with `version: 1` and expected fields
- [ ] App did not crash or hang during any test
- [ ] No permission warnings in Console (or expected warnings logged, not fatal)

---

**If any check fails:** Note the failure details (error message, expected vs. actual output, system state) and open a GitHub issue with reproduction steps.
