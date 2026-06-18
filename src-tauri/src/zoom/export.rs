use std::process::{Command, Stdio};
use crate::model::zoom::ZoomModel;
use crate::capture::ffmpeg::ffmpeg_binary;

/// Build the zoompan filter expressions for z (scale), x and y (pan).
/// zoompan evaluates expressions per output frame; `on` = output frame index.
/// We map time in seconds as `on/<fps>`.
///
/// Returns `(z_expr, x_expr, y_expr)` — each is a bare expression string
/// (no surrounding quotes) suitable for use with zoompan=z=...:x=...:y=...
///
/// Simplification: multi-target panning in export uses only the first target
/// per segment. The preview (Task 9) interpolates all targets; export does not.
pub fn build_zoompan_expr(model: &ZoomModel, fps: u32) -> (String, String, String) {
    if model.segments.is_empty() {
        return (
            "1".into(),
            "iw/2-(iw/zoom/2)".into(),
            "ih/2-(ih/zoom/2)".into(),
        );
    }

    // Build z expression: piecewise zoom scale per segment, default 1.
    let mut z = String::new();
    let mut depth = 0usize;
    let fps_f = fps as f64;
    for seg in &model.segments {
        let s0 = seg.start_ms as f64 / 1000.0;
        let s1 = seg.end_ms as f64 / 1000.0;
        let ein = seg.ease_in_ms as f64 / 1000.0;
        let eout = seg.ease_out_ms as f64 / 1000.0;
        let dur = s1 - s0;
        let scale = seg.scale;
        // t = on/fps (time in seconds for this output frame)
        // smoothstep(p) = p*p*(3-2*p) with p clamped to [0,1] via min/max
        // We use min(1,max(0,p)) instead of clip() for portability.
        // Ease-in ramp: p = (t-s0)/ein
        // Ease-out ramp: p = (s1-t)/eout
        // Plateau (middle): ramp factor = 1
        z.push_str(&format!(
            "if(between(on/{fps_f},{s0},{s1}),1+({scale}-1)*if(lt(on/{fps_f}-{s0},{ein}),(min(1,max(0,(on/{fps_f}-{s0})/{ein})))*(min(1,max(0,(on/{fps_f}-{s0})/{ein})))*(3-2*(min(1,max(0,(on/{fps_f}-{s0})/{ein})))),if(gt(on/{fps_f}-{s0},{dur}-{eout}),(min(1,max(0,({s1}-on/{fps_f})/{eout})))*(min(1,max(0,({s1}-on/{fps_f})/{eout})))*(3-2*(min(1,max(0,({s1}-on/{fps_f})/{eout})))),1)),"
        ));
        depth += 1;
    }
    z.push('1');
    for _ in 0..depth {
        z.push(')');
    }

    // Build x and y center expressions (first target per segment, default 0.5)
    let cx_expr = center_expr(model, fps, true);
    let cy_expr = center_expr(model, fps, false);

    // x = iw*cx - (iw/zoom)/2  — top-left corner of the zoom window in source
    // y = ih*cy - (ih/zoom)/2
    let x = format!("iw*{cx_expr}-(iw/zoom/2)");
    let y = format!("ih*{cy_expr}-(ih/zoom/2)");

    (z, x, y)
}

fn center_expr(model: &ZoomModel, fps: u32, is_x: bool) -> String {
    let mut e = String::new();
    let mut depth = 0usize;
    let fps_f = fps as f64;
    for seg in &model.segments {
        let s0 = seg.start_ms as f64 / 1000.0;
        let s1 = seg.end_ms as f64 / 1000.0;
        let v = if is_x {
            seg.targets[0].x
        } else {
            seg.targets[0].y
        };
        e.push_str(&format!(
            "if(between(on/{fps_f},{s0},{s1}),{v},"
        ));
        depth += 1;
    }
    e.push_str("0.5");
    for _ in 0..depth {
        e.push(')');
    }
    e
}

/// Export a video applying zoompan filter with progress reporting.
///
/// `on_progress` is called with values in [0.0, 1.0] as encoding proceeds.
/// The zoompan filter preserves frame dimensions (probed via ffprobe).
///
/// Note: `d=1` ensures zoompan does not hold/duplicate frames; input and
/// output FPS remain the same as the source.
pub fn export<F: FnMut(f64)>(
    video_path: &str,
    model: &ZoomModel,
    out_path: &str,
    fps: u32,
    total_ms: u64,
    mut on_progress: F,
) -> Result<(), String> {
    // Probe the input dimensions so we can pass an explicit size to zoompan.
    let probe = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=width,height",
            "-of", "csv=s=x:p=0",
            video_path,
        ])
        .output()
        .map_err(|e| format!("ffprobe failed: {e}"))?;
    let dims = String::from_utf8_lossy(&probe.stdout);
    let dims = dims.trim();
    // Use probed dimensions; fall back to iw:ih if ffprobe unavailable
    let size_arg = if dims.contains('x') && !dims.is_empty() {
        dims.to_string()
    } else {
        "iw:ih".to_string()
    };

    let (z, x, y) = build_zoompan_expr(model, fps);
    // d=1: output exactly as many frames as input (no frame holding)
    // s=WxH: keep original resolution
    let vf = format!(
        "zoompan=z='{z}':x='{x}':y='{y}':d=1:fps={fps}:s={size_arg}"
    );

    let mut child = Command::new(ffmpeg_binary())
        .args([
            "-y",
            "-i", video_path,
            "-vf", &vf,
            "-c:v", "libx264",
            "-preset", "fast",
            "-pix_fmt", "yuv420p",
            "-c:a", "copy",
            "-progress", "pipe:1",
            "-nostats",
            out_path,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("falha ao iniciar ffmpeg: {e}"))?;

    if let Some(out) = child.stdout.take() {
        use std::io::{BufRead, BufReader};
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(v) = line.strip_prefix("out_time_ms=") {
                if let Ok(us) = v.trim().parse::<u64>() {
                    let done = (us / 1000) as f64 / (total_ms.max(1) as f64);
                    on_progress(done.clamp(0.0, 1.0));
                }
            }
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("export ffmpeg falhou (status {status})"));
    }
    on_progress(1.0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::zoom::{ZoomSegment, ZoomTarget};

    #[test]
    fn builds_zoompan_expr_for_one_segment() {
        let m = ZoomModel {
            version: 1,
            segments: vec![ZoomSegment {
                start_ms: 0,
                end_ms: 2000,
                ease_in_ms: 300,
                ease_out_ms: 400,
                scale: 2.0,
                targets: vec![ZoomTarget { t_ms: 0, x: 0.25, y: 0.75 }],
            }],
        };
        let (z, x, y) = build_zoompan_expr(&m, 30);
        // Expression uses on/fps for time
        assert!(z.contains("on/30"), "z should use on/30 for time: {z}");
        // Contains the plateau scale of the segment
        assert!(z.contains("2"), "z should contain scale 2: {z}");
        // x and y reference input dimensions
        assert!(x.contains("iw"), "x should reference iw: {x}");
        assert!(y.contains("ih"), "y should reference ih: {y}");
        // Default case (outside all segments) produces 1
        assert!(z.contains("1"), "z should fall back to 1: {z}");
    }

    #[test]
    fn empty_model_is_identity() {
        let m = ZoomModel { version: 1, segments: vec![] };
        let (z, _x, _y) = build_zoompan_expr(&m, 30);
        assert_eq!(z.trim(), "1");
    }

    #[test]
    fn two_segments_both_appear_in_expr() {
        let m = ZoomModel {
            version: 1,
            segments: vec![
                ZoomSegment {
                    start_ms: 0,
                    end_ms: 1000,
                    ease_in_ms: 100,
                    ease_out_ms: 100,
                    scale: 1.5,
                    targets: vec![ZoomTarget { t_ms: 0, x: 0.3, y: 0.4 }],
                },
                ZoomSegment {
                    start_ms: 2000,
                    end_ms: 3000,
                    ease_in_ms: 200,
                    ease_out_ms: 200,
                    scale: 3.0,
                    targets: vec![ZoomTarget { t_ms: 2000, x: 0.6, y: 0.7 }],
                },
            ],
        };
        let (z, x, y) = build_zoompan_expr(&m, 30);
        // Both scales appear
        assert!(z.contains("1.5"), "z should contain 1.5: {z}");
        assert!(z.contains("3"), "z should contain 3: {z}");
        // Both center coords appear in x and y
        assert!(x.contains("0.3"), "x should contain 0.3: {x}");
        assert!(x.contains("0.6"), "x should contain 0.6: {x}");
        assert!(y.contains("0.4"), "y should contain 0.4: {y}");
        assert!(y.contains("0.7"), "y should contain 0.7: {y}");
    }

    // Smoke render test — requires ffmpeg at /opt/homebrew/bin/ffmpeg.
    // Run explicitly with: cargo test export_smoke -- --ignored
    #[test]
    #[ignore]
    fn export_smoke_render() {
        // Generate test input
        let gen_status = Command::new("/opt/homebrew/bin/ffmpeg")
            .args([
                "-y", "-f", "lavfi",
                "-i", "testsrc=size=640x480:rate=30",
                "-t", "2",
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                "/tmp/f2in.mp4",
            ])
            .status()
            .expect("ffmpeg should be available");
        assert!(gen_status.success(), "test input generation failed");

        let model = ZoomModel {
            version: 1,
            segments: vec![ZoomSegment {
                start_ms: 200,
                end_ms: 1800,
                ease_in_ms: 300,
                ease_out_ms: 300,
                scale: 2.0,
                targets: vec![ZoomTarget { t_ms: 200, x: 0.4, y: 0.4 }],
            }],
        };

        let mut last_progress = 0.0_f64;
        let result = export(
            "/tmp/f2in.mp4",
            &model,
            "/tmp/f2out.mp4",
            30,
            2000,
            |p| { last_progress = p; },
        );
        assert!(result.is_ok(), "export should succeed: {:?}", result);
        assert!(last_progress >= 0.99, "progress should reach ~1.0, got {last_progress}");

        // Verify output exists and is non-empty
        let meta = std::fs::metadata("/tmp/f2out.mp4").expect("output should exist");
        assert!(meta.len() > 1000, "output should be non-trivially large");
    }
}
