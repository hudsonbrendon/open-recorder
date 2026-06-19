# OpenRecorder F2 Smoke Test Checklist

Manual verification checklist for the auto-zoom editor (F2) on macOS. Follow each step in order. Mark completed items with an ✓.

## Pre-Test Setup

- [ ] Clone and build: `pnpm install && pnpm tauri build`
- [ ] Open built app in `src-tauri/target/release/bundle/` or run `pnpm tauri dev`
- [ ] System preferences: Grant **Screen Recording**, **Microphone**, and **Input Monitoring** (Accessibility) permissions to the Tauri app process
  - Settings > Privacy & Security > Screen Recording: Add the Tauri app
  - Settings > Privacy & Security > Microphone: Allow
  - Settings > Privacy & Security > Accessibility: Add the Tauri app (required for click capture in auto-zoom)
- [ ] Verify **ffmpeg** is on PATH: `which ffmpeg`

## Test 1: Record with Multiple Clicks (Input Monitoring Granted)

- [ ] Ensure Input Monitoring **is granted** (Accessibility permission enabled)
- [ ] Open the recording UI
- [ ] Select "Full Screen" as capture source
- [ ] Click **Start Recording**
- [ ] Move mouse and **click multiple times** (in different locations on screen) for ~15 seconds
  - Click at least 3–5 distinct positions (top-left, center, bottom-right, etc.)
  - This populates the `metadata.json` events array with click coordinates
- [ ] Click **Stop Recording**
- [ ] Confirm UI returns to idle (no error modal)
- [ ] Verify files appear in `~/Movies/OpenRecorder`: `REC-*.mp4` and `REC-*.metadata.json`
- [ ] Open `REC-*.metadata.json` in a text editor
- [ ] Verify `events` array contains multiple click entries with `x`, `y`, `t_ms`, and `button` fields
  - Example: `{ "t_ms": 1500, "type": "click", "x": 840, "y": 410, "button": "left" }`

## Test 2: Open Editor and Verify Auto-Zoom Segments

- [ ] In the recording list, click **Edit** button next to the recording from Test 1
- [ ] Verify the editor opens, showing:
  - **Live Preview:** Video player with current frame displayed (top section)
  - **Timeline:** Horizontal bar with segment rectangles (one per detected click)
  - **Playhead:** Vertical marker on timeline, synced with preview
  - **Segment Inspector:** Side panel showing segment details (scale, start time, end time, delete button)
- [ ] Confirm **auto-zoom segments appear on timeline:**
  - One segment (zoomed rectangle/bar) for each click detected in `metadata.json`
  - Segments should be positioned at click timestamps
- [ ] Click on any segment in the timeline
  - Verify it highlights in the timeline
  - Verify **Live Preview zooms**: video in preview shows the auto-zoomed region (CSS transform)
  - Verify **Segment Inspector updates** with that segment's scale, start_time, end_time

## Test 3: Adjust Segment Parameters (Scale, Start, End)

- [ ] With a segment selected in the timeline:
- [ ] In the **Segment Inspector**, adjust **Scale** (e.g., 1.5 → 2.0):
  - Verify the preview updates in real-time (zoomed video magnification changes)
  - Verify the timeline segment bar updates (visual indicator of scale)
- [ ] Adjust **Start Time** (e.g., shift earlier by 100ms):
  - Verify preview updates (pan/zoom begins at new frame)
- [ ] Adjust **End Time** (e.g., extend by 200ms):
  - Verify preview updates (zoom ends at new frame)
- [ ] Confirm all changes are **reflected live** in the preview without exporting

## Test 4: Delete a Segment

- [ ] With a segment still selected in the timeline:
- [ ] Click **Delete** button in the Segment Inspector
- [ ] Verify segment **disappears from timeline**
- [ ] Verify preview returns to full (non-zoomed) view
- [ ] Confirm timeline reflow (remaining segments adjust positions if needed)

## Test 5: Reopen Editor (Persistence via zoom.json)

- [ ] Close the editor (back to recording list)
- [ ] Verify that an `REC-<timestamp>.zoom.json` file was created in `~/Movies/OpenRecorder`
  - Contains segments with scale, start_time, end_time, click data
- [ ] Click **Edit** again on the same recording
- [ ] Verify **all previous edits persist:**
  - Deleted segment stays deleted
  - Adjusted segments (scale, start, end) show modified values in Inspector
  - Timeline matches previous session
- [ ] Confirm editor state matches saved `zoom.json`

## Test 6: Export MP4 with Baked Auto-Zoom

- [ ] With the editor open (same recording as Test 5):
- [ ] Click **Export** button
- [ ] Verify an **export progress dialog** appears
  - Shows ffmpeg processing (zoompan filter + H.264 encode)
  - Displays progress percentage
- [ ] Wait for export to complete (may take 30–60 seconds depending on recording duration)
- [ ] Verify export dialog closes and a new file appears: `REC-<timestamp>-exported.mp4`
  - Original `REC-*.mp4` remains unchanged
- [ ] Play exported MP4 in QuickTime Player:
  - Verify **video zooms in at each click** (matches preview segments)
  - Verify **pan/zoom timing** matches timeline segments
  - Verify **zoom scales** match Inspector values (e.g., 2.0× magnification)
  - Verify **video quality** is H.264 (no corruption, smooth playback)
  - [ ] **Verify the exported video resolution/aspect ratio matches the source (landscape).** Confirm it is NOT portrait/9:16 (9:16 is a future phase, F4).

## Test 7: Compare Preview vs. Exported MP4

- [ ] Still in editor with the exported file ready:
- [ ] Play exported MP4 side-by-side with the live preview:
  - Pause preview at a segment start
  - Verify exported video shows the **same zoom region** at the same timestamp
  - Confirm **scale/magnification matches** in both
- [ ] Repeat for 2–3 segments
- [ ] Confirm preview and export are **visually consistent**
- [ ] **NOTE:** full preview↔export parity holds for single-target zoom segments. For merged/multi-target segments the export pans to the segment's first target only (see Test 10) — a multi-target pan mismatch here is expected, not a failure.

## Test 8: Record with Input Monitoring Denied

- [ ] Revoke Input Monitoring from System Preferences > Accessibility
- [ ] Restart the app
- [ ] Select "Full Screen", mic **On**
- [ ] Click **Start Recording** and move/click for ~10 seconds
- [ ] Click **Stop Recording**
- [ ] Verify recording completes without error
- [ ] Open editor for this recording
- [ ] Verify timeline shows **no auto-zoom segments** (events array was empty)
  - Segment Inspector may be blank or disabled
- [ ] Confirm zooms can still be managed manually via the inspector/timeline:
  - User can manually add segments even without auto-detected clicks
- [ ] Attempt export: should work without auto-zoom segments (flat video or manual zooms only)

## Test 9: Export with No Segments (Flat Video)

- [ ] Create a recording with no clicks (or with Input Monitoring denied)
- [ ] Open editor
- [ ] Confirm timeline has no segments (or only manually added segments)
- [ ] Click **Export**
- [ ] Verify export completes successfully
  - ffmpeg should output the video unchanged (no zoompan filter, or zoompan with identity transforms)
- [ ] Play exported file: confirm it matches the original recording (flat, no zoom)

## Test 10: Known Limitations & Edge Cases

- [ ] **Multi-target zoom:** If segments span multiple click locations, exported zoom shows only the **first target's pan path** (preview may show full pan; export simplified to primary target for F2)
  - This is documented as acceptable F2 simplification
- [ ] **Preview vs. Export Pan Divergence:** Preview shows pan from start→end across all targets; export may only use primary target
  - Confirm this behavior is noted (acceptable F2 divergence)
- [ ] **ffmpeg syntax:** Verify zoompan filter `s=<scale>:x=<pan_x>:y=<pan_y>` is correctly formatted in export command
  - Check Console.app logs if export fails

### Known Limitation: Auto-Zoom Click Targeting (Capture Source)

Auto-zoom click targeting is accurate for **full-screen primary-display capture only**. For window or region capture sources the `source.rect` can be `[0, 0, 0, 0]` and the native CGEventTap reports global screen coordinates, so the auto-zoom focus point may not align with the correct location in the captured frame. Full-screen primary display capture is the supported and tested path for F2 auto-zoom.

## Post-Test Summary

- [ ] All recordings with clicks show auto-zoom segments in editor
- [ ] Timeline displays segments at correct timestamps
- [ ] Live preview zooms in real-time when segment selected
- [ ] Segment Inspector allows scale/start/end edits with live preview updates
- [ ] Delete removes segments from timeline and zoom.json
- [ ] zoom.json persists edits across editor reopens
- [ ] Exported MP4 matches preview zoom behavior and scale
- [ ] Recordings without Input Monitoring gracefully skip auto-zoom (no error, manual still works)
- [ ] Export completes without error and produces valid H.264 MP4
- [ ] App does not crash during edit, delete, or export operations

---

**If any check fails:** Note the failure details (error message, expected vs. actual output, system state) and open a GitHub issue with reproduction steps, including the `REC-*.metadata.json` and `REC-*.zoom.json` files if relevant.
