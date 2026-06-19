# OpenRecorder F3 Smoke Test Checklist

Manual verification checklist for the webcam overlay feature (F3) on macOS. Follow each step in order. Mark completed items with an ✓.

## Pre-Test Setup

- [ ] Clone and build: `pnpm install && pnpm tauri build`
- [ ] Open built app in `src-tauri/target/release/bundle/` or run `pnpm tauri dev`
- [ ] System preferences: Grant **Screen Recording**, **Microphone**, and **Camera** permissions to the Tauri app process
  - Settings > Privacy & Security > Screen Recording: Add the Tauri app
  - Settings > Privacy & Security > Microphone: Allow
  - Settings > Privacy & Security > Camera: Add the Tauri app (required for webcam capture)
- [ ] Verify **ffmpeg** is on PATH: `which ffmpeg`
- [ ] Ensure a **camera device** is connected and available (e.g., built-in FaceTime camera, external USB webcam)

## Test 1: Grant Camera Permission and Record with Webcam

- [ ] Ensure Camera permission **is granted** (System Preferences > Privacy & Security > Camera)
- [ ] Open the recording UI
- [ ] Verify a **camera selector** (Câmera) dropdown appears with available cameras:
  - Expected: At least one camera option (e.g., "FaceTime HD Camera") plus a "Nenhuma" (None) option
- [ ] Select a camera (e.g., "FaceTime HD Camera")
- [ ] Select "Full Screen" as capture source
- [ ] Click **Start Recording**
- [ ] Record for ~15 seconds (move/click as normal, this also exercises the webcam capture)
- [ ] Click **Stop Recording**
- [ ] Confirm UI returns to idle (no error modal)
- [ ] Verify files appear in `~/Movies/OpenRecorder`:
  - `REC-<timestamp>.mp4` (main screen recording)
  - `REC-<timestamp>.webcam.mp4` (separate webcam video)
  - `REC-<timestamp>.metadata.json` (metadata with `has_webcam: true` and `camera_name`)

## Test 2: Verify Metadata Records Webcam Info

- [ ] Open `REC-<timestamp>.metadata.json` in a text editor
- [ ] Verify it contains:
  - `"has_webcam": true`
  - `"camera_name": "<camera_name>"` (the name of the selected camera)
  - Example: `{ "has_webcam": true, "camera_name": "FaceTime HD Camera", ... }`

## Test 3: Open Editor and Verify Webcam Bubble

- [ ] In the recording list, click **Edit** button next to the recording from Test 1
- [ ] Verify the editor opens, showing:
  - **Live Preview:** Video player with current frame displayed (top section)
  - **Timeline:** Horizontal bar with segments (auto-zoom segments from clicks, if Input Monitoring granted)
  - **Webcam Bubble:** A circular or rounded-rectangle overlay on top of the preview
    - Default shape: circle
    - Default position: top-right corner
    - Live webcam feed visible inside the bubble
- [ ] Confirm the bubble is **on top of the screen recording** and shows live webcam video

## Test 4: Move the Webcam Bubble

- [ ] Click and drag the webcam bubble to different positions on the preview:
  - Move to top-left corner
  - Move to center
  - Move to bottom-right corner
- [ ] Verify the bubble **responds to drag** with smooth movement
- [ ] Release and confirm position is retained

## Test 5: Resize the Webcam Bubble

- [ ] Click and drag the **edge/corner** of the bubble to resize:
  - Make it smaller (e.g., 80×80 pixels)
  - Make it larger (e.g., 200×200 pixels)
- [ ] Verify the bubble **resizes** while maintaining aspect ratio or proportions
- [ ] Confirm live webcam feed remains visible and scales within the bubble

## Test 6: Toggle Shape (Circle ↔ Rounded Rectangle)

- [ ] Verify a **shape toggle** control appears in the editor UI (or inspector panel):
  - Option to switch between "Circle" and "Rounded Rectangle"
- [ ] Select "Circle":
  - Verify bubble becomes circular with no corners
- [ ] Select "Rounded Rectangle":
  - Verify bubble becomes a rounded square/rectangle with visible corner radius
- [ ] Toggle back to "Circle":
  - Confirm shape changes back smoothly

## Test 7: Adjust Border

- [ ] Verify a **border control** appears in the editor (color, width, or visibility toggle):
- [ ] Enable border (if toggle):
  - Verify a colored outline appears around the bubble edge
- [ ] Adjust border width (if slider available):
  - Small width (e.g., 1–2px)
  - Large width (e.g., 4–5px)
  - Confirm preview updates in real-time
- [ ] If color picker available, change border color and confirm it updates in preview

## Test 8: Toggle Mirror

- [ ] Verify a **mirror toggle** control appears in the editor:
- [ ] Enable mirror:
  - Verify webcam feed flips horizontally (left-right mirror)
  - Compare: your left hand appears on the right side of the bubble
- [ ] Disable mirror:
  - Verify webcam feed returns to normal (left-right not flipped)
- [ ] Toggle back and forth several times to confirm smooth state changes

## Test 9: Reopen Editor (Overlay Persistence via zoom.json)

- [ ] Close the editor (back to recording list)
- [ ] Verify that the `REC-<timestamp>.zoom.json` file includes a `webcam` field:
  - Should contain overlay config: scale (`s`), position (`x`, `y`) — all as fractions 0..1 — shape (circle/rounded), border (width, color), mirror (true/false), and enabled flag
  - Example:
    ```json
    {
      "segments": [...],
      "webcam": {
        "enabled": true,
        "shape": "circle",
        "x": 0.85,
        "y": 0.1,
        "s": 0.1,
        "border_width": 2,
        "border_color": "#ffffff",
        "mirror": true
      }
    }
    ```
- [ ] Click **Edit** again on the same recording
- [ ] Verify **all webcam overlay settings persist:**
  - Bubble position matches previous session
  - Size/scale matches previous session
  - Shape (circle/rounded) is as saved
  - Border settings match previous session
  - Mirror state matches previous session
- [ ] Confirm editor state matches saved `zoom.json`

## Test 10: Export MP4 with Baked Webcam Overlay

- [ ] With the editor open (same recording as Test 9):
- [ ] Click **Export** button
- [ ] Verify an **export progress dialog** appears
  - Shows ffmpeg processing (composite + H.264 encode)
  - Displays progress percentage
- [ ] Wait for export to complete (may take 1–2 minutes depending on recording duration and overlay composite)
- [ ] Verify export dialog closes and a new file appears: `REC-<timestamp>-exported.mp4`
  - Original `REC-*.mp4` and `REC-*.webcam.mp4` remain unchanged
- [ ] Play exported MP4 in QuickTime Player:
  - Verify **webcam overlay appears** at the configured position and size
  - Verify **overlay shape matches** preview (circle or rounded rectangle)
  - Verify **overlay contains the webcam video** (baked into the composite)
  - Verify **mirror setting is applied** if enabled
  - Verify **video quality** is H.264 (no corruption, smooth playback)
  - Verify **overlay is composited on top** of the screen recording, not replacing it

## Test 11: Compare Preview vs. Exported Overlay

- [ ] Still in editor with the exported file ready:
- [ ] Play the exported `.mp4` file and scrub the editor preview to the **same timestamp**, then compare the overlay position/size/shape:
  - In the exported video, pause at a known frame (e.g., 5 seconds)
  - In the editor preview, drag the playhead to the same timestamp (5 seconds)
  - Verify the overlay appears at the **same position** and **size** in both
  - Verify **mirror state matches** in both
- [ ] Repeat for 2–3 different timestamps
- [ ] Confirm preview and export are **visually consistent**
- [ ] **Known Limitation Note:** The border may appear slightly different in the export vs. preview (CSS border vs. ffmpeg border approximation) — this divergence is expected and documented

## Test 12: Record with Camera "Nenhuma" (None)

- [ ] In the recording UI, select the camera dropdown
- [ ] Select "Nenhuma" (None)
- [ ] Select "Full Screen" as capture source
- [ ] Click **Start Recording** and record for ~10 seconds
- [ ] Click **Stop Recording**
- [ ] Verify recording completes without error
- [ ] Check `~/Movies/OpenRecorder` for output files:
  - `REC-<timestamp>.mp4` should exist (main screen recording)
  - `REC-<timestamp>.webcam.mp4` should **not** exist (no webcam selected)
  - `REC-<timestamp>.metadata.json` should have `"has_webcam": false`
- [ ] Open editor for this recording
- [ ] Verify the **webcam bubble does not appear** (no overlay controls visible)
- [ ] Confirm export works normally (flat screen recording, no webcam composite)

## Test 13: Record with Camera Permission Denied

- [ ] Revoke Camera permission from System Preferences > Privacy & Security > Camera
- [ ] Restart the app
- [ ] Open the recording UI
- [ ] Verify the camera selector still appears but may show:
  - Empty list (no cameras available due to permission denial)
  - Or disabled state
- [ ] Proceed with recording anyway (camera selector defaults to "Nenhuma" or unavailable):
  - Select "Full Screen"
  - Click **Start Recording** and record for ~10 seconds
  - Click **Stop Recording**
- [ ] Verify recording completes **without error** (degrades gracefully):
  - `REC-<timestamp>.mp4` created (main screen recording only)
  - `REC-<timestamp>.webcam.mp4` **not** created
  - `REC-<timestamp>.metadata.json` has `"has_webcam": false`
- [ ] Open editor:
  - Verify **no webcam bubble** appears
  - Confirm export works normally (flat screen recording)
- [ ] Re-grant Camera permission and restart to restore functionality for remaining tests

## Test 14: Verify Landscape-Only Export (No Portrait Mode)

- [ ] With any recording open in the editor:
- [ ] Click **Export**
- [ ] Wait for export to complete
- [ ] Play the exported MP4 in QuickTime Player:
  - Verify it is **landscape orientation** (width > height)
  - Verify it matches the **source recording aspect ratio** (e.g., 16:9, 4:3, or custom)
  - Confirm it is **not portrait/9:16** (9:16 is a future phase, F4)

## Test 15: Known Limitations & Edge Cases

- [ ] **Border in Export:** The border around the overlay may appear slightly different in the exported video vs. the live preview
  - Preview uses CSS borders (sharp, pixel-perfect)
  - Export uses ffmpeg/geq masks (anti-aliased approximation)
  - This divergence is expected and documented
- [ ] **Rounded Shape Approximation:** The "rounded rectangle" mask in the export is an approximation using ffmpeg geometry
  - It may not exactly match the CSS clip-path in the preview
  - Circle mask should be accurate; rounded rectangles have slight edge differences
- [ ] **Overlay Does Not Zoom:** The webcam bubble position and size remain **fixed** (do not scale with auto-zoom segments)
  - Auto-zoom zooms the **background** (screen recording), not the overlay
  - This is by design; overlay coordinates are in preview space, not zoomed space
  - Confirm this behavior is as expected

## Post-Test Summary

- [ ] Camera permission prompt respected (grants allow capture, deny degrades gracefully)
- [ ] Webcam selection dropdown shows available cameras + "Nenhuma" option
- [ ] Recording with camera creates `REC-*.webcam.mp4` (separate video file)
- [ ] Metadata includes `has_webcam` and `camera_name` fields
- [ ] Editor displays webcam bubble with live feed from selected camera
- [ ] Bubble is draggable, resizable, and customizable (shape, border, mirror)
- [ ] Overlay settings persist across editor reopens via `zoom.json`
- [ ] Export composites the webcam overlay on top of screen recording via ffmpeg
- [ ] Exported overlay matches preview position, size, and mirror state
- [ ] "Nenhuma" selection skips webcam capture and overlay (no error, degrades)
- [ ] Camera permission denial is handled gracefully (no crash, records without webcam)
- [ ] Exported video is landscape (not portrait), matching source aspect ratio
- [ ] App does not crash during overlay edit, export, or permission handling
- [ ] Known limitations (border divergence, rounded approximation, fixed overlay during zoom) are documented and acceptable

---

**If any check fails:** Note the failure details (error message, expected vs. actual output, system state) and open a GitHub issue with reproduction steps, including the `REC-*.metadata.json`, `REC-*.zoom.json`, and relevant video files if applicable.
