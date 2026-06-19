// Webcam capture via nokhwa → ffmpeg stdin.
//
// Frames are captured as RGB24 by nokhwa (AVFoundation on macOS) and piped
// to an ffmpeg child process as raw video. The stop signal is sent via an
// mpsc channel; the capture thread drops stdin after stopping, which signals
// EOF to ffmpeg so it can finalize the output file.
//
// IMPORTANT: nokhwa can panic internally (e.g. when the OS denies camera
// access or the device disappears). All nokhwa open/init calls are wrapped in
// `std::panic::catch_unwind` so a panic becomes a clean Err instead of
// aborting the whole app — this mirrors the defensive pattern used in
// video_capture.rs for scap.
//
// THREADING NOTE: nokhwa::Camera is NOT Send unless the `camera-sync-impl`
// feature is enabled (which we don't enable). To work around this, the camera
// is created and used entirely inside the capture thread. We coordinate the
// resolution back to the main thread via a one-shot mpsc channel so that
// ffmpeg can be spawned with the correct frame dimensions.

use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread::JoinHandle;

use crate::capture::ffmpeg::ffmpeg_binary;

/// List all available cameras.
///
/// Returns `(id, name)` pairs where `id` is a stringified [`CameraIndex`] and
/// `name` is the human-readable label from the OS.
///
/// Returns an empty Vec if no backend is available or if the query panics.
pub fn list_cameras() -> Vec<(String, String)> {
    use nokhwa::utils::ApiBackend;

    std::panic::catch_unwind(|| {
        nokhwa::query(ApiBackend::Auto)
            .map(|list| {
                list.into_iter()
                    .map(|c| (c.index().to_string(), c.human_name()))
                    .collect()
            })
            .unwrap_or_default()
    })
    .unwrap_or_default()
}

pub struct WebcamCapture {
    stop_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
    ffmpeg: Child,
}

impl WebcamCapture {
    /// Start capturing the webcam identified by `camera_id` (numeric index as
    /// a string), encoding at `fps` frames per second into `out_path` via ffmpeg.
    ///
    /// # Errors
    /// Returns an `Err` if the camera cannot be opened, if the stream fails to
    /// start, or if ffmpeg cannot be spawned. nokhwa panics are caught and
    /// converted to errors.
    pub fn start(camera_id: &str, fps: u32, out_path: &Path) -> Result<Self, String> {
        let idx: u32 = camera_id
            .parse()
            .map_err(|_| format!("invalid camera id: {camera_id}"))?;

        let out_path_str = out_path
            .to_str()
            .ok_or("caminho de saída inválido")?
            .to_string();

        // Channel to receive (width, height) or an error from the camera thread
        // before we can spawn ffmpeg.
        let (dim_tx, dim_rx) = mpsc::channel::<Result<(u32, u32), String>>();
        // Channel to send the ffmpeg stdin pipe into the capture thread.
        let (stdin_tx, stdin_rx) = mpsc::channel::<std::process::ChildStdin>();
        // Channel to request the capture thread to stop.
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        let handle: JoinHandle<()> = std::thread::spawn(move || {
            use nokhwa::pixel_format::RgbFormat;
            use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};

            // Open camera — wrapped in catch_unwind because nokhwa can panic.
            let camera_result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let requested = RequestedFormat::new::<RgbFormat>(
                        RequestedFormatType::AbsoluteHighestFrameRate,
                    );
                    nokhwa::Camera::new(CameraIndex::Index(idx), requested)
                }));

            let mut camera = match camera_result {
                Err(_) => {
                    let _ = dim_tx.send(Err(
                        "falha ao abrir a câmera (panic interno do nokhwa)".to_string(),
                    ));
                    return;
                }
                Ok(Err(e)) => {
                    let _ = dim_tx.send(Err(format!("falha ao abrir a câmera: {e}")));
                    return;
                }
                Ok(Ok(cam)) => cam,
            };

            // Open stream — also wrapped.
            let stream_result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| camera.open_stream()));
            if let Err(_) | Ok(Err(_)) = stream_result {
                let err_msg = match stream_result {
                    Err(_) => "falha ao iniciar stream da câmera (panic)".to_string(),
                    Ok(Err(e)) => format!("falha ao iniciar stream da câmera: {e}"),
                    Ok(Ok(_)) => unreachable!(),
                };
                let _ = dim_tx.send(Err(err_msg));
                return;
            }

            // Send the negotiated resolution to the main thread so it can build
            // the correct ffmpeg command.
            let res = camera.resolution();
            let (w, h) = (res.width_x, res.height_y);
            if dim_tx.send(Ok((w, h))).is_err() {
                // Main thread gone; just clean up.
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    camera.stop_stream()
                }));
                return;
            }

            // Wait for ffmpeg stdin from the main thread.
            let mut stdin = match stdin_rx.recv() {
                Ok(s) => s,
                Err(_) => {
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        camera.stop_stream()
                    }));
                    return;
                }
            };

            // Capture loop.
            loop {
                if stop_rx.try_recv().is_ok() {
                    break;
                }

                let frame_result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| camera.frame()));

                match frame_result {
                    Ok(Ok(buf)) => {
                        // decode_image::<RgbFormat>() converts the raw camera buffer
                        // (MJPEG / YUV / etc.) into a contiguous RGB24 ImageBuffer.
                        // .as_raw() gives a &Vec<u8> without extra allocation.
                        match buf.decode_image::<RgbFormat>() {
                            Ok(img) => {
                                if stdin.write_all(img.as_raw()).is_err() {
                                    break;
                                }
                            }
                            Err(_) => {
                                // Decoding failed for this frame; skip it.
                            }
                        }
                    }
                    Ok(Err(_)) | Err(_) => {
                        // Frame read failed or panicked; stop the loop.
                        break;
                    }
                }
            }

            // Best-effort stream stop.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                camera.stop_stream()
            }));

            // Dropping stdin signals EOF to ffmpeg → it finalises the output file.
            drop(stdin);
        });

        // Wait for the camera thread to report dimensions (or an error).
        let dim_result = dim_rx
            .recv()
            .map_err(|_| "thread da câmera encerrou inesperadamente".to_string())?;
        let (w, h) = match dim_result {
            Ok(dims) => dims,
            Err(e) => {
                // Thread failed; join it to clean up.
                let _ = handle.join();
                return Err(e);
            }
        };

        // Spawn ffmpeg now that we know the real frame dimensions.
        let args: Vec<String> = vec![
            "-y".into(),
            "-f".into(),
            "rawvideo".into(),
            "-pix_fmt".into(),
            "rgb24".into(),
            "-s".into(),
            format!("{w}x{h}"),
            "-r".into(),
            fps.to_string(),
            "-i".into(),
            "-".into(),
            "-c:v".into(),
            "libx264".into(),
            "-preset".into(),
            "ultrafast".into(),
            "-pix_fmt".into(),
            "yuv420p".into(),
            out_path_str,
        ];

        let mut ffmpeg = Command::new(ffmpeg_binary())
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("falha ao iniciar ffmpeg: {e}"))?;

        let stdin = ffmpeg.stdin.take().ok_or("ffmpeg sem stdin")?;

        // Hand stdin to the capture thread so it can write frames.
        stdin_tx
            .send(stdin)
            .map_err(|_| "thread da câmera encerrou antes de receber stdin do ffmpeg".to_string())?;

        Ok(Self {
            stop_tx,
            handle: Some(handle),
            ffmpeg,
        })
    }

    /// Signal the capture thread to stop, wait for it, then wait for ffmpeg to
    /// finish encoding.
    pub fn stop(mut self) -> Result<(), String> {
        let _ = self.stop_tx.send(());
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        self.ffmpeg
            .wait()
            .map_err(|e| format!("ffmpeg wait error: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: list_cameras() must not panic and returns a Vec.
    /// Run with: cargo test list_cameras -- --ignored --nocapture
    #[test]
    #[ignore]
    fn list_cameras_smoke() {
        let cameras = list_cameras();
        println!("list_cameras count: {}", cameras.len());
        for (id, name) in &cameras {
            println!("  camera id={id} name={name}");
        }
        // No assertion — empty is valid when no camera permission is granted.
    }
}
