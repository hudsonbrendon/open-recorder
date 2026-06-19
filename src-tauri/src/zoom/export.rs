use std::process::{Command, Stdio};
use std::fs;
use crate::model::zoom::ZoomModel;
use crate::model::webcam::WebcamOverlay;
use crate::capture::ffmpeg::{ffmpeg_binary, ffprobe_binary};

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
        z.push_str(&format!(
            "if(between(on/{fps_f},{s0},{s1}),1+({scale}-1)*if(lt(on/{fps_f}-{s0},{ein}),(min(1,max(0,(on/{fps_f}-{s0})/{ein})))*(min(1,max(0,(on/{fps_f}-{s0})/{ein})))*(3-2*(min(1,max(0,(on/{fps_f}-{s0})/{ein})))),if(gt(on/{fps_f}-{s0},{dur}-{eout}),(min(1,max(0,({s1}-on/{fps_f})/{eout})))*(min(1,max(0,({s1}-on/{fps_f})/{eout})))*(3-2*(min(1,max(0,({s1}-on/{fps_f})/{eout})))),1)),"
        ));
        depth += 1;
    }
    z.push('1');
    for _ in 0..depth {
        z.push(')');
    }

    let cx_expr = center_expr(model, fps, true);
    let cy_expr = center_expr(model, fps, false);

    let x = format!("iw*max(0.5/zoom,min({cx_expr},1-0.5/zoom))-(iw/zoom/2)");
    let y = format!("ih*max(0.5/zoom,min({cy_expr},1-0.5/zoom))-(ih/zoom/2)");

    (z, x, y)
}

fn center_expr(model: &ZoomModel, fps: u32, is_x: bool) -> String {
    let mut e = String::new();
    let mut depth = 0usize;
    let fps_f = fps as f64;
    for seg in &model.segments {
        let s0 = seg.start_ms as f64 / 1000.0;
        let s1 = seg.end_ms as f64 / 1000.0;
        let v = seg.targets.first().map(|t| if is_x { t.x } else { t.y }).unwrap_or(0.5);
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

/// Build the webcam overlay portion of a filter_complex string.
///
/// Produces:
///   `[1:v]scale=SxS,<hflip,>format=rgba,<mask>[fg];[bg][fg]overlay=X:Y:format=auto[outv]`
///
/// - Scale webcam input to SxS (from geometry)
/// - Apply hflip only when mirror is true
/// - Convert to rgba for alpha transparency support
/// - Apply circle mask via geq alpha expression
/// - Overlay [fg] onto [bg] at computed X,Y
pub fn build_overlay_filter(ov: &WebcamOverlay, out_w: u32, out_h: u32) -> String {
    let (s, x, y) = ov.geometry(out_w, out_h);
    let hflip = if ov.mirror { "hflip," } else { "" };
    let r = s / 2;

    let mask = match ov.shape.as_str() {
        "rounded" => {
            let rad = (s as f64 * 0.15) as u32;
            format!(
                "geq=lum='p(X,Y)':a='if(gt(min(min(X,{w}-1-X),min(Y,{h}-1-Y)),{rad}),255,if(lte(hypot(max({rad}-X,0)+max(X-({w}-1-{rad}),0),max({rad}-Y,0)+max(Y-({h}-1-{rad}),0)),{rad}),255,0))'",
                w = s, h = s, rad = rad
            )
        }
        _ => {
            // Circle mask: alpha=255 inside radius r, 0 outside
            format!(
                "geq=lum='p(X,Y)':a='if(lte(hypot(X-{r},Y-{r}),{r}),255,0)'"
            )
        }
    };

    format!(
        "[1:v]scale={s}:{s},{hflip}format=rgba,{mask}[fg];[bg][fg]overlay={x}:{y}:format=auto[outv]"
    )
}

/// Export a video applying zoompan filter with progress reporting.
///
/// `on_progress` is called with values in [0.0, 1.0] as encoding proceeds.
/// The zoompan filter preserves frame dimensions (probed via ffprobe).
///
/// When model.webcam is Some(enabled) and the companion webcam file exists,
/// the export composites the webcam as a circle overlay via filter_complex.
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
    let probe = Command::new(ffprobe_binary())
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

    // Parse out_w and out_h for webcam overlay geometry
    let (out_w, out_h) = if dims.contains('x') {
        let parts: Vec<&str> = dims.splitn(2, 'x').collect();
        if parts.len() == 2 {
            let w = parts[0].trim().parse::<u32>().unwrap_or(1280);
            let h = parts[1].trim().parse::<u32>().unwrap_or(720);
            (w, h)
        } else {
            (1280, 720)
        }
    } else {
        (1280, 720)
    };

    let (z, x, y) = build_zoompan_expr(model, fps);
    // d=1: output exactly as many frames as input (no frame holding)
    // s=WxH: keep original resolution
    let zoompan = format!(
        "zoompan=z='{z}':x='{x}':y='{y}':d=1:fps={fps}:s={size_arg}"
    );

    // Check if we have an enabled webcam overlay with existing webcam file
    let webcam_file = crate::zoom::store::webcam_path(video_path);
    let use_webcam = model.webcam
        .as_ref()
        .filter(|ov| ov.enabled)
        .is_some()
        && webcam_file.exists();

    // Capture stderr to a temp file so we can include it in error messages
    // without risking a deadlock between stdout progress reads and stderr draining.
    let stderr_path = format!("{out_path}.ffmpeg-stderr.tmp");
    let stderr_file = fs::File::create(&stderr_path)
        .map_err(|e| format!("não foi possível criar arquivo de stderr temporário: {e}"))?;

    let mut child = if use_webcam {
        let ov = model.webcam.as_ref().unwrap();
        let overlay_filter = build_overlay_filter(ov, out_w, out_h);
        // filter_complex: zoompan labels its output [bg]; webcam overlay produces [outv]
        let filter_complex = format!("{zoompan}[bg];{overlay_filter}");
        let webcam_str = webcam_file.to_string_lossy().into_owned();

        Command::new(ffmpeg_binary())
            .args([
                "-y",
                "-i", video_path,
                "-i", &webcam_str,
                "-filter_complex", &filter_complex,
                "-map", "[outv]",
                "-map", "0:a?",
                "-c:v", "libx264",
                "-preset", "fast",
                "-pix_fmt", "yuv420p",
                "-c:a", "copy",
                "-progress", "pipe:1",
                "-nostats",
                out_path,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr_file))
            .spawn()
            .map_err(|e| format!("falha ao iniciar ffmpeg: {e}"))?
    } else {
        Command::new(ffmpeg_binary())
            .args([
                "-y",
                "-i", video_path,
                "-vf", &zoompan,
                "-c:v", "libx264",
                "-preset", "fast",
                "-pix_fmt", "yuv420p",
                "-c:a", "copy",
                "-progress", "pipe:1",
                "-nostats",
                out_path,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr_file))
            .spawn()
            .map_err(|e| format!("falha ao iniciar ffmpeg: {e}"))?
    };

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
    // Read and clean up stderr temp file regardless of exit status.
    let stderr_snippet = fs::read_to_string(&stderr_path).unwrap_or_default();
    let _ = fs::remove_file(&stderr_path);
    if !status.success() {
        // Include a trailing snippet of stderr (last ~1 KB) for diagnosability.
        let tail: String = stderr_snippet
            .chars()
            .rev()
            .take(1024)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        return Err(format!("export ffmpeg falhou (status {status})\n{}", tail.trim()));
    }
    on_progress(1.0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::zoom::{ZoomModel, ZoomSegment, ZoomTarget};
    use crate::model::webcam::WebcamOverlay;

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
            webcam: None,
        };
        let (z, x, y) = build_zoompan_expr(&m, 30);
        assert!(z.contains("on/30"), "z should use on/30 for time: {z}");
        assert!(z.contains("2"), "z should contain scale 2: {z}");
        assert!(x.contains("iw"), "x should reference iw: {x}");
        assert!(y.contains("ih"), "y should reference ih: {y}");
        assert!(x.contains("0.5/zoom"), "x should clamp with 0.5/zoom: {x}");
        assert!(x.contains("1-0.5/zoom"), "x should clamp with 1-0.5/zoom: {x}");
        assert!(y.contains("0.5/zoom"), "y should clamp with 0.5/zoom: {y}");
        assert!(y.contains("1-0.5/zoom"), "y should clamp with 1-0.5/zoom: {y}");
        assert!(z.contains("1"), "z should fall back to 1: {z}");
    }

    #[test]
    fn empty_model_is_identity() {
        let m = ZoomModel { version: 1, segments: vec![], webcam: None };
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
            webcam: None,
        };
        let (z, x, y) = build_zoompan_expr(&m, 30);
        assert!(z.contains("1.5"), "z should contain 1.5: {z}");
        assert!(z.contains("3"), "z should contain 3: {z}");
        assert!(x.contains("0.3"), "x should contain 0.3: {x}");
        assert!(x.contains("0.6"), "x should contain 0.6: {x}");
        assert!(y.contains("0.4"), "y should contain 0.4: {y}");
        assert!(y.contains("0.7"), "y should contain 0.7: {y}");
    }

    #[test]
    fn segment_with_empty_targets_does_not_panic() {
        let m = ZoomModel {
            version: 1,
            segments: vec![ZoomSegment {
                start_ms: 0,
                end_ms: 1000,
                ease_in_ms: 100,
                ease_out_ms: 100,
                scale: 2.0,
                targets: vec![],
            }],
            webcam: None,
        };
        let (z, x, y) = build_zoompan_expr(&m, 30);
        assert!(!z.is_empty(), "z expression should be non-empty");
        assert!(x.contains("0.5"), "x should use 0.5 fallback center: {x}");
        assert!(y.contains("0.5"), "y should use 0.5 fallback center: {y}");
    }

    #[test]
    fn overlay_filter_circle_has_terms() {
        let ov = WebcamOverlay::default_overlay(); // circle, mirror=true
        let f = build_overlay_filter(&ov, 1920, 1080);
        assert!(f.contains("scale="), "{f}");
        assert!(f.contains("hflip"), "{f}");      // mirror on
        assert!(f.contains("geq"), "{f}");        // circle mask via geq alpha
        assert!(f.contains("overlay="), "{f}");
    }

    #[test]
    fn overlay_filter_no_mirror_omits_hflip() {
        let mut ov = WebcamOverlay::default_overlay();
        ov.mirror = false;
        let f = build_overlay_filter(&ov, 1920, 1080);
        assert!(!f.contains("hflip"), "{f}");
    }

    // Smoke render test — requires ffmpeg at /opt/homebrew/bin/ffmpeg.
    // Run explicitly with: cargo test export_smoke -- --ignored
    #[test]
    #[ignore]
    fn export_smoke_render() {
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
            webcam: None,
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

        let meta = std::fs::metadata("/tmp/f2out.mp4").expect("output should exist");
        assert!(meta.len() > 1000, "output should be non-trivially large");
    }

    /// Render validation test for webcam overlay compositing.
    /// Requires ffmpeg at /opt/homebrew/bin/ffmpeg and creates test videos at /tmp/scr.mp4 and /tmp/scr.webcam.mp4
    /// Run: cargo test test_export_webcam_overlay_render -- --ignored
    #[test]
    #[ignore]
    fn test_export_webcam_overlay_render() {
        // Create test screen recording
        let scr_status = Command::new("/opt/homebrew/bin/ffmpeg")
            .args([
                "-y", "-f", "lavfi",
                "-i", "testsrc=size=1280x720:rate=30",
                "-t", "2",
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                "/tmp/scr.mp4",
            ])
            .status()
            .expect("ffmpeg should be available");
        assert!(scr_status.success(), "screen test video creation failed");

        // webcam_path("/tmp/scr.mp4") -> "/tmp/scr.webcam.mp4"
        let cam_status = Command::new("/opt/homebrew/bin/ffmpeg")
            .args([
                "-y", "-f", "lavfi",
                "-i", "testsrc2=size=640x480:rate=30",
                "-t", "2",
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                "/tmp/scr.webcam.mp4",
            ])
            .status()
            .expect("ffmpeg should be available");
        assert!(cam_status.success(), "webcam test video creation failed");

        let model = ZoomModel {
            version: 1,
            segments: vec![],
            webcam: Some(WebcamOverlay::default_overlay()),
        };

        let result = export(
            "/tmp/scr.mp4",
            &model,
            "/tmp/scrout.mp4",
            30,
            2000,
            |_| {},
        );
        assert!(result.is_ok(), "export with webcam overlay should succeed: {:?}", result);

        let meta = std::fs::metadata("/tmp/scrout.mp4").expect("output /tmp/scrout.mp4 should exist");
        assert!(meta.len() > 1000, "output should be non-trivially large, got {} bytes", meta.len());
    }
}
