use std::process::Command;

/// Resolve the ffmpeg executable. A macOS GUI app launched from Finder does
/// NOT inherit the shell PATH (it gets a minimal /usr/bin:/bin:...), so a bare
/// "ffmpeg" lookup misses Homebrew installs. Probe the common absolute paths
/// first, then fall back to "ffmpeg" (PATH) for CLI/dev runs.
pub fn ffmpeg_binary() -> String {
    const CANDIDATES: &[&str] = &[
        "/opt/homebrew/bin/ffmpeg", // Apple Silicon Homebrew
        "/usr/local/bin/ffmpeg",    // Intel Homebrew
        "/opt/local/bin/ffmpeg",    // MacPorts
        "/usr/bin/ffmpeg",          // system / Linux
        "/bin/ffmpeg",
    ];
    for path in CANDIDATES {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    "ffmpeg".to_string()
}

pub fn ensure_ffmpeg() -> Result<(), String> {
    match Command::new(ffmpeg_binary()).arg("-version").output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(_) => Err("ffmpeg encontrado mas retornou erro ao executar -version".into()),
        Err(_) => Err("ffmpeg não encontrado. Instale o ffmpeg (ex.: brew install ffmpeg) para gravar.".into()),
    }
}

/// Args para encodar BGRA cru lido do stdin em H.264 mp4.
pub fn encode_args(width: u32, height: u32, fps: u32, out_path: &str) -> Vec<String> {
    vec![
        "-y".into(),
        "-f".into(), "rawvideo".into(),
        "-pix_fmt".into(), "bgra".into(),
        "-s".into(), format!("{width}x{height}"),
        "-r".into(), fps.to_string(),
        "-i".into(), "-".into(),
        "-c:v".into(), "libx264".into(),
        "-preset".into(), "ultrafast".into(),
        "-pix_fmt".into(), "yuv420p".into(),
        out_path.into(),
    ]
}

/// Args para muxar vídeo + áudio (sem re-encode de vídeo).
pub fn mux_args(video_path: &str, audio_path: &str, out_path: &str) -> Vec<String> {
    vec![
        "-y".into(),
        "-i".into(), video_path.into(),
        "-i".into(), audio_path.into(),
        "-c:v".into(), "copy".into(),
        "-c:a".into(), "aac".into(),
        out_path.into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_args_have_rawvideo_input_and_size() {
        let a = encode_args(1920, 1080, 30, "/tmp/v.mp4");
        assert!(a.windows(2).any(|w| w[0] == "-f" && w[1] == "rawvideo"), "{a:?}");
        assert!(a.windows(2).any(|w| w[0] == "-pix_fmt" && w[1] == "bgra"), "{a:?}");
        assert!(a.windows(2).any(|w| w[0] == "-s" && w[1] == "1920x1080"), "{a:?}");
        assert!(a.windows(2).any(|w| w[0] == "-r" && w[1] == "30"), "{a:?}");
        assert!(a.windows(2).any(|w| w[0] == "-i" && w[1] == "-"), "{a:?}");
        assert_eq!(a.last().unwrap(), "/tmp/v.mp4");
    }

    #[test]
    fn mux_args_have_two_inputs_and_output() {
        let a = mux_args("/tmp/v.mp4", "/tmp/a.wav", "/tmp/out.mp4");
        let inputs: Vec<_> = a.windows(2).filter(|w| w[0] == "-i").map(|w| w[1].clone()).collect();
        assert_eq!(inputs, vec!["/tmp/v.mp4", "/tmp/a.wav"]);
        assert_eq!(a.last().unwrap(), "/tmp/out.mp4");
    }

    #[test]
    fn ensure_ffmpeg_returns_result() {
        // Não assume ffmpeg instalado no CI; só garante que retorna sem panicar
        // e que, se erro, a mensagem menciona ffmpeg.
        if let Err(msg) = ensure_ffmpeg() {
            assert!(msg.to_lowercase().contains("ffmpeg"), "{msg}");
        }
    }
}
