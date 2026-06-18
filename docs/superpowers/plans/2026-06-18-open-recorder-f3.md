# OpenRecorder F3 (Webcam Overlay) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Gravar a webcam junto da tela e compor uma bolha (rosto) configurável no export, com posicionamento/preview ao vivo no editor.

**Architecture:** Webcam gravada em stream separado (`REC-<ts>.webcam.mp4`) via `nokhwa` durante a gravação; config do overlay (forma/posição/tamanho/borda/espelho) salva no `REC-<ts>.zoom.json` (campo `webcam`); editor sobrepõe um segundo `<video>` sobre o preview da F2; export compõe via `ffmpeg` (scale/mirror/máscara/overlay) depois do zoom. Não-destrutivo.

**Tech Stack:** Tauri 2, Rust (`nokhwa`, serde), React + Vite + TS, ffmpeg. Testes: `cargo test` + `vitest`.

## Global Constraints

- Plataforma alvo: **macOS** (testado); crates cross-platform.
- Stack: **Tauri 2 + Rust + React/TS** (mesma da F1/F2).
- **Não-destrutivo:** webcam em `REC-<ts>.webcam.mp4`; overlay aplicado no export; config em `REC-<ts>.zoom.json` campo `webcam` (opcional, serde `default`, retrocompatível com F2).
- Webcam = **vídeo-only** (áudio é o mic do `REC-<ts>.mp4`).
- Overlay **fixo por cima**, aplicado **depois** do zoom.
- Export **landscape** (9:16 é F4).
- Defaults do overlay: `enabled true`, `shape "circle"`, `x 0.76`, `y 0.74`, `size 0.22`, `border_width 3`, `border_color "#ffffff"`, `mirror true`.
- `shape ∈ {"circle","rounded"}`. Coordenadas normalizadas 0..1 (canto superior-esquerdo da bolha) sobre a moldura de saída; `size` = fração da LARGURA de saída; bolha quadrada (S×S).
- Commits sem co-author/histórico do Claude; inglês `tipo: descrição`.
- Crate `open_recorder_lib`; ffmpeg via `ffmpeg::ffmpeg_binary()`; ffprobe via `ffmpeg::ffprobe_binary()`.
- Branch `f3-webcam-overlay` (saiu do tip de `f2-auto-zoom`).
- Panics de FFI de captura DEVEM ser contidos com `catch_unwind` (lição da F2) → erro tratado, nunca crash.

## Estado herdado (F1/F2 — NÃO refazer)

- `capture/video_capture.rs` (scap→ffmpeg, com `catch_unwind`), `audio_capture.rs` (cpal), `input.rs`/`input_mac.rs` (CGEventTap), `recording/coordinator.rs` (`Coordinator::{start(source, mic_id, fps), stop()}`, `Active`).
- `model/metadata.rs`: `RecordingMetadata { version:u32, recording:RecordingInfo, source:SourceInfo, events:Vec<InputEvent> }`; `RecordingInfo { width,height,fps:u32, duration_ms:u64 }`.
- `model/zoom.rs`: `ZoomModel { version:u32, segments:Vec<ZoomSegment> }`, `zoom_at`, `smoothstep`.
- `zoom/generate.rs` (`generate`, `GenOpts`), `zoom/store.rs` (`zoom_path`, `load`, `save`), `zoom/export.rs` (`build_zoompan_expr`, `export(video_path,&ZoomModel,out_path,fps,total_ms,on_progress)`).
- `capture/ffmpeg.rs`: `ffmpeg_binary()`, `ffprobe_binary()`, `ensure_ffmpeg()`, `encode_args`, `mux_args`.
- `commands.rs`: `list_sources`, `list_microphones`, `start_recording`, `stop_recording`, `reveal_in_folder`, `load_recording`, `save_zoom`, `export_with_zoom`.
- Front: `lib/api.ts`, `lib/zoom.ts`, `lib/timeline.ts`, `state/useRecorder.ts`, `state/useEditor.ts`, components `Dropdown`, `SourcePicker`, `MicPicker`, `RecordControls`, `RecordingsList`, `EditorView`, `PreviewCanvas`, `Timeline`, `SegmentInspector`.

## File Structure (F3)

```
src-tauri/src/
├── capture/
│   └── webcam_capture.rs    # NOVO: nokhwa → ffmpeg → REC-<ts>.webcam.mp4
├── model/
│   ├── webcam.rs            # NOVO: WebcamOverlay + default + geometry()
│   ├── metadata.rs          # MOD: RecordingInfo ganha has_webcam + camera_name
│   └── zoom.rs              # MOD: ZoomModel ganha webcam: Option<WebcamOverlay>
├── zoom/export.rs           # MOD: overlay da webcam no filtro
├── recording/coordinator.rs # MOD: 4ª via webcam opcional
└── commands.rs              # MOD: list_cameras + camera_id no start_recording

src/
├── lib/
│   ├── webcam.ts            # NOVO: tipo WebcamOverlay + geometry() (paridade)
│   ├── webcam.test.ts       # NOVO: vitest
│   ├── api.ts               # MOD: listCameras + tipo webcam + camera no start
│   └── zoom.ts              # MOD: ZoomModel.webcam?
├── state/
│   ├── useRecorder.ts       # MOD: cameras + selectedCamera
│   └── useEditor.ts         # (sem mudança estrutural; model já carrega webcam)
└── components/
    ├── CameraPicker.tsx     # NOVO
    ├── WebcamBubble.tsx     # NOVO: bolha arrastável/redimensionável
    └── EditorView.tsx       # MOD: controles de overlay + WebcamBubble
docs/SMOKE-TEST-F3.md        # NOVO
```

---

## Task 1: WebcamOverlay model + ZoomModel field (retrocompatível)

**Files:**
- Create: `src-tauri/src/model/webcam.rs`
- Modify: `src-tauri/src/model/mod.rs` (add `pub mod webcam;`), `src-tauri/src/model/zoom.rs`

**Interfaces:**
- Produces:
  - `WebcamOverlay { enabled: bool, shape: String, x: f64, y: f64, size: f64, border_width: u32, border_color: String, mirror: bool }` derive `Serialize, Deserialize, Clone, Debug, PartialEq`.
  - `WebcamOverlay::default_overlay() -> WebcamOverlay` (os defaults do Global Constraints).
  - `ZoomModel` ganha `#[serde(default)] pub webcam: Option<WebcamOverlay>`.

- [ ] **Step 1: Escrever testes falhando**

Em `src-tauri/src/model/webcam.rs`:
```rust
use serde::{Serialize, Deserialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_overlay_values() {
        let o = WebcamOverlay::default_overlay();
        assert!(o.enabled);
        assert_eq!(o.shape, "circle");
        assert!((o.x - 0.76).abs() < 1e-9 && (o.y - 0.74).abs() < 1e-9);
        assert!((o.size - 0.22).abs() < 1e-9);
        assert_eq!(o.border_width, 3);
        assert_eq!(o.border_color, "#ffffff");
        assert!(o.mirror);
    }

    #[test]
    fn round_trips() {
        let o = WebcamOverlay::default_overlay();
        let j = serde_json::to_string(&o).unwrap();
        let back: WebcamOverlay = serde_json::from_str(&j).unwrap();
        assert_eq!(o, back);
    }
}
```

Em `src-tauri/src/model/zoom.rs`, adicionar ao `mod tests`:
```rust
    #[test]
    fn zoom_model_without_webcam_field_deserializes_to_none() {
        let json = r#"{"version":1,"segments":[]}"#;
        let m: ZoomModel = serde_json::from_str(json).unwrap();
        assert!(m.webcam.is_none());
    }
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test webcam 2>&1 | tail -10 && cargo test zoom_model_without 2>&1 | tail -10`
Expected: `cannot find ... WebcamOverlay` / campo `webcam` inexistente.

- [ ] **Step 3: Implementar**

`src-tauri/src/model/webcam.rs` (acima do teste):
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WebcamOverlay {
    pub enabled: bool,
    pub shape: String,
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub border_width: u32,
    pub border_color: String,
    pub mirror: bool,
}

impl WebcamOverlay {
    pub fn default_overlay() -> Self {
        WebcamOverlay {
            enabled: true,
            shape: "circle".into(),
            x: 0.76,
            y: 0.74,
            size: 0.22,
            border_width: 3,
            border_color: "#ffffff".into(),
            mirror: true,
        }
    }
}
```

Em `src-tauri/src/model/zoom.rs`, no struct `ZoomModel`, adicionar o campo (e `use crate::model::webcam::WebcamOverlay;` no topo):
```rust
    #[serde(default)]
    pub webcam: Option<WebcamOverlay>,
```
Atualizar quaisquer construções literais de `ZoomModel` existentes (ex.: em `generate.rs` e testes) para incluir `webcam: None`. Procure por `ZoomModel {` no crate e adicione `webcam: None` onde faltar.

- [ ] **Step 4: Rodar pra ver passar**

Run: `cd src-tauri && cargo test 2>&1 | tail -6`
Expected: tudo `ok` (webcam + zoom + suíte existente; ajuste construções de ZoomModel até compilar).

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add WebcamOverlay model and optional ZoomModel.webcam field"
```

---

## Task 2: Geometria do overlay (Rust) + paridade TS

**Files:**
- Modify: `src-tauri/src/model/webcam.rs`
- Create: `src/lib/webcam.ts`, `src/lib/webcam.test.ts`

**Interfaces:**
- Produces (Rust): `WebcamOverlay::geometry(&self, out_w: u32, out_h: u32) -> (u32, u32, u32)` retornando `(s, xpx, ypx)` com `s = round(size*out_w)`, `xpx = round(x*out_w)`, `ypx = round(y*out_h)`, e **clamp** pra bolha caber na moldura (`xpx ∈ [0, out_w - s]`, `ypx ∈ [0, out_h - s]`, `s ≤ min(out_w,out_h)`).
- Produces (TS): `interface WebcamOverlay { enabled; shape; x; y; size; border_width; border_color; mirror }` + `geometry(ov, outW, outH) -> { s, x, y }` (mesma matemática/clamp).

- [ ] **Step 1: Escrever teste Rust falhando**

Em `webcam.rs` `mod tests`:
```rust
    #[test]
    fn geometry_maps_and_clamps() {
        let mut o = WebcamOverlay::default_overlay();
        o.x = 0.5; o.y = 0.5; o.size = 0.2;
        // out 1000x500: s = 200, x = 500, y = 250
        assert_eq!(o.geometry(1000, 500), (200, 500, 250));
        // clamp: x near right edge
        o.x = 0.99;
        let (s, xpx, _) = o.geometry(1000, 500);
        assert_eq!(s, 200);
        assert_eq!(xpx, 800); // clamped to out_w - s
    }
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test geometry_maps 2>&1 | tail -10`
Expected: `no method named geometry`.

- [ ] **Step 3: Implementar `geometry` (Rust)**

Em `impl WebcamOverlay`:
```rust
    pub fn geometry(&self, out_w: u32, out_h: u32) -> (u32, u32, u32) {
        let max_s = out_w.min(out_h);
        let s = ((self.size * out_w as f64).round() as i64).clamp(1, max_s as i64) as u32;
        let max_x = out_w.saturating_sub(s);
        let max_y = out_h.saturating_sub(s);
        let xpx = ((self.x * out_w as f64).round() as i64).clamp(0, max_x as i64) as u32;
        let ypx = ((self.y * out_h as f64).round() as i64).clamp(0, max_y as i64) as u32;
        (s, xpx, ypx)
    }
```

- [ ] **Step 4: Rodar Rust pra ver passar**

Run: `cd src-tauri && cargo test webcam 2>&1 | tail -8`
Expected: `ok`.

- [ ] **Step 5: Escrever teste TS falhando (mesma matemática)**

`src/lib/webcam.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { geometry, type WebcamOverlay } from "./webcam";

const base: WebcamOverlay = {
  enabled: true, shape: "circle", x: 0.5, y: 0.5, size: 0.2,
  border_width: 3, border_color: "#ffffff", mirror: true,
};

describe("webcam geometry", () => {
  it("maps to pixels", () => {
    expect(geometry(base, 1000, 500)).toEqual({ s: 200, x: 500, y: 250 });
  });
  it("clamps within frame", () => {
    expect(geometry({ ...base, x: 0.99 }, 1000, 500)).toEqual({ s: 200, x: 800, y: 250 });
  });
});
```

- [ ] **Step 6: Rodar pra ver falhar**

Run: `pnpm test 2>&1 | tail -12`
Expected: `./webcam` não existe.

- [ ] **Step 7: Implementar `webcam.ts`**

```ts
export interface WebcamOverlay {
  enabled: boolean;
  shape: "circle" | "rounded";
  x: number; y: number; size: number;
  border_width: number; border_color: string; mirror: boolean;
}

export function geometry(ov: WebcamOverlay, outW: number, outH: number): { s: number; x: number; y: number } {
  const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
  const maxS = Math.min(outW, outH);
  const s = clamp(Math.round(ov.size * outW), 1, maxS);
  const x = clamp(Math.round(ov.x * outW), 0, outW - s);
  const y = clamp(Math.round(ov.y * outH), 0, outH - s);
  return { s, x, y };
}
```

- [ ] **Step 8: Rodar pra ver passar**

Run: `pnpm test 2>&1 | tail -6`
Expected: testes de webcam ok (paridade com Rust).

- [ ] **Step 9: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add webcam overlay geometry (Rust + TS parity)"
```

---

## Task 3: metadata `has_webcam` + webcam path helper

**Files:**
- Modify: `src-tauri/src/model/metadata.rs`, `src-tauri/src/zoom/store.rs`

**Interfaces:**
- Produces:
  - `RecordingInfo` ganha `#[serde(default)] pub has_webcam: bool` e `#[serde(default)] pub camera_name: Option<String>`.
  - `store::webcam_path(video_path: &str) -> PathBuf` → troca `.mp4` por `.webcam.mp4` (mesmo prefixo `REC-<ts>`).

- [ ] **Step 1: Escrever testes falhando**

Em `store.rs` `mod tests`:
```rust
    #[test]
    fn webcam_path_swaps_suffix() {
        assert_eq!(webcam_path("/x/REC-1.mp4"), PathBuf::from("/x/REC-1.webcam.mp4"));
    }
```

Em `metadata.rs` `mod tests`:
```rust
    #[test]
    fn recording_info_defaults_webcam_fields() {
        let json = r#"{"width":100,"height":50,"fps":30,"duration_ms":1000}"#;
        let r: RecordingInfo = serde_json::from_str(json).unwrap();
        assert!(!r.has_webcam);
        assert!(r.camera_name.is_none());
    }
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test webcam_path 2>&1 | tail -8 && cargo test recording_info_defaults 2>&1 | tail -8`
Expected: erros (função/campos inexistentes).

- [ ] **Step 3: Implementar**

Em `metadata.rs`, no `RecordingInfo` adicionar:
```rust
    #[serde(default)]
    pub has_webcam: bool,
    #[serde(default)]
    pub camera_name: Option<String>,
```
Atualizar construções de `RecordingInfo` (ex.: em `finalizer.rs` e testes) — adicione `has_webcam: false, camera_name: None` onde faltar (procure `RecordingInfo {`). O `RecordingFinalizer::build_metadata` ganha esses dois como parâmetros ou são setados depois; veja a Task 5 (o coordinator preenche). Por ora, default `false`/`None`.

Em `store.rs`:
```rust
pub fn webcam_path(video_path: &str) -> PathBuf {
    let p = Path::new(video_path);
    p.with_extension("webcam.mp4")
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run: `cd src-tauri && cargo test 2>&1 | tail -6`
Expected: tudo `ok` (ajuste construções até compilar).

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add has_webcam/camera_name metadata and webcam_path helper"
```

---

## Task 4: Webcam capture (nokhwa → ffmpeg) — integração

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/capture/mod.rs`
- Create: `src-tauri/src/capture/webcam_capture.rs`

**Interfaces:**
- Produces:
  - `WebcamCapture` com `start(camera_id: &str, fps: u32, out_path: &Path) -> Result<WebcamCapture, String>` e `stop(self) -> Result<(), String>`.
  - `fn list_cameras() -> Vec<(String, String)>` (id, name).

- [ ] **Step 1: Adicionar `nokhwa` ao Cargo.toml**

```toml
nokhwa = { version = "0.10", features = ["input-native"] }
```
> Verifique no docs.rs a feature correta pra macOS (AVFoundation) na versão 0.10 — pode ser `input-native` ou `input-avfoundation`. Ajuste.

- [ ] **Step 2: Declarar módulo**

Em `capture/mod.rs`: `pub mod webcam_capture;`

- [ ] **Step 3: Implementar (consultar a API real do nokhwa 0.10)**

Padrão igual ao `video_capture`: nokhwa frames (RGB) → `ffmpeg -f rawvideo -pix_fmt rgb24 -s WxH -r fps -i - ... out.mp4`. Esboço (ajuste à API real — verifique `nokhwa::Camera`, `query`, `RequestedFormat`, `pixel_format`):
```rust
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread::JoinHandle;
use crate::capture::ffmpeg::ffmpeg_binary;

pub fn list_cameras() -> Vec<(String, String)> {
    std::panic::catch_unwind(|| {
        nokhwa::query(nokhwa::utils::ApiBackend::Auto)
            .map(|list| list.into_iter()
                .map(|c| (c.index().to_string(), c.human_name()))
                .collect())
            .unwrap_or_default()
    }).unwrap_or_default()
}

pub struct WebcamCapture {
    stop_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
    ffmpeg: Child,
}

impl WebcamCapture {
    pub fn start(camera_id: &str, fps: u32, out_path: &Path) -> Result<Self, String> {
        use nokhwa::pixel_format::RgbFormat;
        use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};

        let idx: u32 = camera_id.parse().map_err(|_| "id de câmera inválido")?;
        let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);

        let mut camera = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            nokhwa::Camera::new(CameraIndex::Index(idx), requested)
        })).map_err(|_| "falha ao abrir a câmera (panic)".to_string())?
          .map_err(|e| format!("falha ao abrir a câmera: {e}"))?;

        camera.open_stream().map_err(|e| format!("falha ao iniciar a câmera: {e}"))?;
        let res = camera.resolution();
        let (w, h) = (res.width_x, res.height_y);

        let args = vec![
            "-y".to_string(),
            "-f".into(), "rawvideo".into(),
            "-pix_fmt".into(), "rgb24".into(),
            "-s".into(), format!("{w}x{h}"),
            "-r".into(), fps.to_string(),
            "-i".into(), "-".into(),
            "-c:v".into(), "libx264".into(),
            "-preset".into(), "ultrafast".into(),
            "-pix_fmt".into(), "yuv420p".into(),
            out_path.to_str().ok_or("caminho inválido")?.to_string(),
        ];
        let mut ffmpeg = Command::new(ffmpeg_binary())
            .args(&args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null())
            .spawn().map_err(|e| format!("falha ffmpeg: {e}"))?;
        let mut stdin = ffmpeg.stdin.take().ok_or("sem stdin")?;

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let handle = std::thread::spawn(move || {
            loop {
                if stop_rx.try_recv().is_ok() { break; }
                match camera.frame().and_then(|f| f.decode_image::<RgbFormat>()) {
                    Ok(img) => { if stdin.write_all(&img).is_err() { break; } }
                    Err(_) => break,
                }
            }
            let _ = camera.stop_stream();
            drop(stdin);
        });

        Ok(Self { stop_tx, handle: Some(handle), ffmpeg })
    }

    pub fn stop(mut self) -> Result<(), String> {
        let _ = self.stop_tx.send(());
        if let Some(h) = self.handle.take() { let _ = h.join(); }
        self.ffmpeg.wait().map_err(|e| format!("ffmpeg wait: {e}"))?;
        Ok(())
    }
}
```
> A API exata do `nokhwa` 0.10 (`Camera::new`, `frame()`, `decode_image`, `resolution()`/`Resolution` fields) pode divergir — VALIDE no crate real e ajuste. `catch_unwind` em torno do open é obrigatório.

- [ ] **Step 4: Compilar**

Run: `cd src-tauri && cargo build 2>&1 | tail -12`
Expected: `Finished` (baixa nokhwa). Corrigir à API real se divergir; reporte DONE_WITH_CONCERNS se a API mudou.

- [ ] **Step 5: Smoke manual de listagem**

Run (rápido, lista câmeras): adicionar teste `#[ignore]` que chama `list_cameras()` e imprime; `cargo test list_cameras -- --ignored --nocapture`. Esperado: imprime ao menos a câmera embutida (ou vazio se sem permissão — ok).

- [ ] **Step 6: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add nokhwa webcam capture and list_cameras"
```

---

## Task 5: Wire webcam no coordinator + comandos

**Files:**
- Modify: `src-tauri/src/recording/coordinator.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/capture/finalizer.rs`

**Interfaces:**
- Consumes: `WebcamCapture` (Task 4), `RecordingInfo.has_webcam/camera_name` (Task 3), `store::webcam_path` (Task 3).
- Produces:
  - `Coordinator::start(source, mic_id, camera_id: Option<String>, fps)` — assinatura ganha `camera_id`.
  - Comando `list_cameras() -> Vec<CameraOption{id,name}>`.
  - Comando `start_recording(state, source, mic_id, camera_id)` — repassa `camera_id`.
  - `metadata.recording.has_webcam`/`camera_name` preenchidos no stop.

- [ ] **Step 1: Coordinator — adicionar webcam à gravação**

Em `coordinator.rs`: no `Active`, adicionar `webcam: Option<crate::capture::webcam_capture::WebcamCapture>`, `webcam_out: PathBuf`, `camera_name: Option<String>`. Em `start(...)` (nova assinatura com `camera_id: Option<String>`): se `camera_id` presente, `WebcamCapture::start(&id, fps, &store::webcam_path(out_video.to_str().unwrap()))` (degrade: se `Err`, logue e siga sem webcam, `camera_name=None`). Guarde no `Active`. No `stop()`: pare a webcam (`webcam.stop()`), e ao montar a metadata, setar `has_webcam = webcam_out existe` e `camera_name`.

Concretamente, no `stop()`, depois de construir `meta` e antes de `write_metadata`:
```rust
        meta.recording.has_webcam = a.webcam_was_started && a.webcam_out.exists();
        meta.recording.camera_name = a.camera_name.clone();
```
(adicione `webcam_was_started: bool` ao `Active`, setado conforme `camera_id.is_some()` e start ok.)

> `RecordingInfo` agora tem os campos (Task 3). `build_metadata` não precisa mudar se você setar os campos após construir (eles vêm `false`/`None` por default).

- [ ] **Step 2: Comandos**

Em `commands.rs`:
```rust
use crate::capture::webcam_capture;

#[derive(serde::Serialize)]
pub struct CameraOption { pub id: String, pub name: String }

#[tauri::command]
pub fn list_cameras() -> Vec<CameraOption> {
    webcam_capture::list_cameras().into_iter()
        .map(|(id, name)| CameraOption { id, name }).collect()
}
```
Alterar `start_recording` pra receber `camera_id: Option<String>` e repassar:
```rust
#[tauri::command]
pub fn start_recording(
    state: State<'_, Mutex<Coordinator>>,
    source: SourceOption,
    mic_id: Option<String>,
    camera_id: Option<String>,
) -> Result<(), String> {
    let cs = source_enum::to_capture_source(&source);
    state.lock().unwrap().start(cs, mic_id, camera_id, 30)
}
```
Registrar `list_cameras` no `invoke_handler` de `lib.rs`.

- [ ] **Step 3: Compilar + testes**

Run: `cd src-tauri && cargo build 2>&1 | tail -10 && cargo test 2>&1 | tail -4`
Expected: build `Finished`; testes existentes `ok` (ajuste chamadas a `start(...)` nos testes/call-sites pra nova assinatura).

- [ ] **Step 4: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: wire optional webcam capture into coordinator and commands"
```

---

## Task 6: Export — composição do overlay da webcam (ffmpeg)

**Files:**
- Modify: `src-tauri/src/zoom/export.rs`

**Interfaces:**
- Consumes: `ZoomModel.webcam` (Task 1), `WebcamOverlay::geometry` (Task 2), `store::webcam_path` (Task 3), `ffmpeg::ffprobe_binary` (F2 fix).
- Produces: `export(...)` passa a compor a webcam quando `model.webcam` é `Some(enabled)` e o `webcam.mp4` existe. Helper puro testável: `build_overlay_filter(ov: &WebcamOverlay, out_w: u32, out_h: u32) -> String` (a parte `[1:v]...[fg]` + `overlay`).

- [ ] **Step 1: Teste do builder do overlay**

Em `export.rs` `mod tests`:
```rust
    #[test]
    fn overlay_filter_circle_has_terms() {
        use crate::model::webcam::WebcamOverlay;
        let ov = WebcamOverlay::default_overlay(); // circle, mirror
        let f = build_overlay_filter(&ov, 1920, 1080);
        assert!(f.contains("scale="), "{f}");
        assert!(f.contains("hflip"), "{f}");      // mirror on
        assert!(f.contains("geq"), "{f}");        // circle mask via geq alpha
        assert!(f.contains("overlay="), "{f}");
    }

    #[test]
    fn overlay_filter_no_mirror_omits_hflip() {
        use crate::model::webcam::WebcamOverlay;
        let mut ov = WebcamOverlay::default_overlay();
        ov.mirror = false;
        let f = build_overlay_filter(&ov, 1920, 1080);
        assert!(!f.contains("hflip"), "{f}");
    }
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test overlay_filter 2>&1 | tail -10`
Expected: `cannot find function build_overlay_filter`.

- [ ] **Step 3: Implementar o builder**

`S/X/Y` via `geometry`. Webcam é input `[1:v]`. Máscara círculo via `geq` alpha; arredondado via `geq` com cantos (aprox.). Borda: desenhada por baixo via overlay de um círculo colorido `S+2*bw`.
```rust
use crate::model::webcam::WebcamOverlay;

pub fn build_overlay_filter(ov: &WebcamOverlay, out_w: u32, out_h: u32) -> String {
    let (s, x, y) = ov.geometry(out_w, out_h);
    let hflip = if ov.mirror { "hflip," } else { "" };
    let r = s / 2;
    // alpha=255 dentro do raio (círculo). Para "rounded", aproximamos com raio grande nos cantos.
    let mask = match ov.shape.as_str() {
        "rounded" => {
            // retângulo com cantos arredondados (raio = 15% do lado)
            let rad = (s as f64 * 0.15) as u32;
            format!(
                "geq=lum='p(X,Y)':a='if(gt(min(min(X,{w}-X),min(Y,{h}-Y)),{rad}),255, if(lte(hypot(max({rad}-X,0)+max(X-({w}-{rad}),0), max({rad}-Y,0)+max(Y-({h}-{rad}),0)),{rad}),255,0))'",
                w = s, h = s, rad = rad
            )
        }
        _ => format!(
            "geq=lum='p(X,Y)':a='if(lte(hypot(X-{r},Y-{r}),{r}),255,0)'",
            r = r
        ),
    };
    // [1:v] scale to SxS, mirror, to rgba, mask -> [fg]; overlay onto [bg]
    format!(
        "[1:v]scale={s}:{s},{hflip}format=rgba,{mask}[fg];[bg][fg]overlay={x}:{y}:format=auto[outv]"
    )
}
```
> A sintaxe do `geq` p/ alpha (e o `format=rgba`) é o ponto sensível — VALIDE no render (Step 5) e ajuste até a máscara sair certa. A borda foi simplificada (sem desenho explícito de borda nesta versão; se quiser borda visível, adicione um `drawbox`/segundo overlay de círculo colorido `border_color` atrás — valide no render). Documente o que ficou.

- [ ] **Step 4: Integrar no `export()`**

No `export()`, depois de montar o `zoompan` (que produz o stream da tela), nomear esse stream `[bg]` e, se `model.webcam` é `Some` com `enabled` e `webcam_path(video_path).exists()`:
- adicionar `-i <webcam_path>` como segundo input;
- usar `-filter_complex` com `<zoompan>[bg];` + `build_overlay_filter(...)` e `-map "[outv]" -map 0:a?`;
- caso contrário, manter o caminho atual (só zoompan, sem webcam).
Resolva `out_w/out_h` via `ffprobe` do `video_path` (já há `ffprobe_binary`). Mantenha o parsing de progresso e o check de exit-status.

- [ ] **Step 5: Rodar testes + smoke de render (obrigatório)**

Run: `cd src-tauri && cargo test overlay_filter 2>&1 | tail -8 && cargo build 2>&1 | tail -4`
Expected: builder ok; build ok.

Smoke de render: gere `/tmp/scr.mp4` (testsrc) e `/tmp/cam.mp4` (testsrc2), e um `ZoomModel` com `webcam: Some(default_overlay())`; chame `export("/tmp/scr.mp4", &model, "/tmp/out.mp4", 30, 2000, |_|{})` por um teste `#[ignore]` (que copie `/tmp/cam.mp4` para `webcam_path("/tmp/scr.mp4")`). Confirme via `ffprobe` que `/tmp/out.mp4` é válido e que a bolha aparece. Ajuste o filtro até renderizar.

- [ ] **Step 6: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: composite webcam overlay in ffmpeg export"
```

---

## Task 7: Front — camera picker + editor overlay + bolha + preview

**Files:**
- Modify: `src/lib/api.ts`, `src/state/useRecorder.ts`, `src/App.tsx`, `src/components/EditorView.tsx`, `src/lib/zoom.ts`
- Create: `src/components/CameraPicker.tsx`, `src/components/WebcamBubble.tsx`

**Interfaces:**
- Consumes: comandos `list_cameras`/`start_recording(camera_id)` (Task 5), `geometry` (Task 2), `WebcamOverlay` type, `metadata.recording.has_webcam`.
- Produces: seletor de câmera na gravação; controles + bolha de overlay no editor; persistência via `save_zoom` (o `webcam` vai no model).

- [ ] **Step 1: api.ts**

Adicionar:
```ts
export interface CameraOption { id: string; name: string }
export const listCameras = () => invoke<CameraOption[]>("list_cameras");
```
Alterar `startRecording` pra incluir camera:
```ts
export const startRecording = (source: SourceOption, micId: string | null, cameraId: string | null) =>
  invoke<void>("start_recording", { source, micId, cameraId });
```
Em `RecordingMetadata.recording` adicionar `has_webcam: boolean; camera_name?: string`. Importar e re-exportar `WebcamOverlay` de `./webcam`; adicionar `webcam?: WebcamOverlay` ao tipo `ZoomModel` em `zoom.ts`.

- [ ] **Step 2: CameraPicker + useRecorder**

`CameraPicker.tsx` (usa o `Dropdown`, com opção "Nenhuma" = id vazio):
```tsx
import type { CameraOption } from "../lib/api";
import { Dropdown } from "./Dropdown";

export function CameraPicker(props: {
  cameras: CameraOption[]; value: string | null; onChange: (id: string) => void;
}) {
  return (
    <div className="field">
      <span>Câmera</span>
      <Dropdown
        value={props.value ?? ""}
        onChange={(id) => props.onChange(id)}
        placeholder="Nenhuma"
        groups={[{ options: [{ id: "", label: "Nenhuma" }, ...props.cameras.map((c) => ({ id: c.id, label: c.name }))] }]}
      />
    </div>
  );
}
```
Em `useRecorder.ts`: estado `cameras`, `selectedCamera` (default `""`=nenhuma); carregue via `listCameras()` no refresh; passe `selectedCamera || null` em `startRecording(selected, selectedMic, selectedCamera || null)`. Renderize `<CameraPicker>` no `App.tsx` junto dos outros pickers.

- [ ] **Step 3: WebcamBubble (editor)**

`WebcamBubble.tsx`: um `<video>` (o webcam.mp4 via `convertFileSrc(webcamPath)`) posicionado por `geometry(ov, previewW, previewH)`, recortado (`clip-path: circle(50%)` ou `border-radius`), espelhado (`scaleX(-1)`), arrastável (atualiza `x,y` normalizados) e com alça de resize (atualiza `size`). Recebe `overlay`, `webcamPath`, `previewSize`, `onChange(ov)`.
```tsx
import { useRef } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { geometry, type WebcamOverlay } from "../lib/webcam";

export function WebcamBubble(props: {
  overlay: WebcamOverlay; webcamPath: string;
  previewW: number; previewH: number; onChange: (ov: WebcamOverlay) => void;
}) {
  const dragging = useRef<{ dx: number; dy: number } | null>(null);
  const g = geometry(props.overlay, props.previewW, props.previewH);
  const radius = props.overlay.shape === "circle" ? "50%" : "16%";
  const onDown = (e: React.MouseEvent) => {
    dragging.current = { dx: e.clientX - g.x, dy: e.clientY - g.y };
    const move = (ev: MouseEvent) => {
      if (!dragging.current) return;
      const nx = (ev.clientX - dragging.current.dx) / props.previewW;
      const ny = (ev.clientY - dragging.current.dy) / props.previewH;
      props.onChange({ ...props.overlay, x: Math.min(1, Math.max(0, nx)), y: Math.min(1, Math.max(0, ny)) });
    };
    const up = () => { dragging.current = null; window.removeEventListener("mousemove", move); window.removeEventListener("mouseup", up); };
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
  };
  return (
    <div
      onMouseDown={onDown}
      style={{ position: "absolute", left: g.x, top: g.y, width: g.s, height: g.s,
        borderRadius: radius, overflow: "hidden", cursor: "move",
        border: `${props.overlay.border_width}px solid ${props.overlay.border_color}`, boxSizing: "border-box" }}
    >
      <video src={convertFileSrc(props.webcamPath)} autoPlay muted loop
        style={{ width: "100%", height: "100%", objectFit: "cover",
          transform: props.overlay.mirror ? "scaleX(-1)" : "none" }} />
    </div>
  );
}
```
(Resize handle: um cantinho que ajusta `size` por `(dx)/previewW`; mantenha simples.)

- [ ] **Step 4: EditorView — controles + bolha**

No `EditorView`, se `ed.metadata?.recording.has_webcam`:
- derive `webcamPath` do `videoPath` (troca `.mp4`→`.webcam.mp4`).
- garanta `ed.model.webcam` (se `undefined`, inicialize com o default ao abrir).
- renderize uma barrinha: checkbox enabled, toggle círculo/arredondado, inputs de borda (largura/cor), checkbox espelho — cada um faz `ed.setModel({ ...ed.model, webcam: { ...ov, ... } })`.
- dentro do contêiner do `PreviewCanvas` (posição relativa), renderize `<WebcamBubble overlay={ov} webcamPath={webcamPath} previewW={...} previewH={...} onChange={(o)=>ed.setModel({...ed.model, webcam:o})}/>` quando `ov.enabled`.
- a bolha fica FORA do elemento que recebe o transform de zoom (irmã do `<video>` da tela), pra não ampliar junto.

- [ ] **Step 5: Build + testes**

Run: `pnpm build 2>&1 | tail -5 && pnpm test 2>&1 | tail -5 && cd src-tauri && cargo test 2>&1 | tail -3`
Expected: front buildou; vitest ok; cargo ok.

- [ ] **Step 6: Smoke manual fim-a-fim (precisa de você)**

`pnpm tauri dev` → escolher tela + câmera → gravar (conceder Câmera) → parar → Editar → ver a bolha → mover/redimensionar/forma/espelho → exportar → conferir overlay assado batendo com o preview.

- [ ] **Step 7: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: camera picker and webcam overlay editor with live preview"
```

---

## Task 8: Smoke checklist + docs

**Files:**
- Create: `docs/SMOKE-TEST-F3.md`
- Modify: `README.md`

**Interfaces:**
- Consumes: tudo.
- Produces: checklist manual + README atualizado.

- [ ] **Step 1: `docs/SMOKE-TEST-F3.md`**

Checklist macOS (checkboxes): conceder Câmera; gravar com câmera selecionada; `REC-*.webcam.mp4` criado; editor mostra a bolha; mover/redimensionar; alternar círculo↔arredondado; borda; espelho; reabrir editor (overlay persiste no zoom.json); exportar e conferir o overlay assado vs preview; gravar SEM câmera (sem overlay); sem permissão de câmera (degrada, grava sem webcam).

- [ ] **Step 2: README**

Mover F3 de roadmap pra status atual: overlay de webcam (captura separada, editor com bolha + preview, export composto). Notar requisito de permissão de Câmera (mac) + ffmpeg. NÃO alegar 9:16/social (F4).

- [ ] **Step 3: Executar o smoke** (seguir o checklist; corrigir o que falhar voltando à task).

- [ ] **Step 4: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "docs: add F3 smoke checklist and update README"
```

---

## Self-Review (autor do plano)

**1. Cobertura do spec:**
- Captura webcam (nokhwa stream separado + catch_unwind) → Task 4 (+ wiring Task 5).
- Seletor de câmera + "Nenhuma" → Task 7.
- `has_webcam`/`camera_name` no metadata → Tasks 3, 5.
- `WebcamOverlay` no zoom.json (retrocompat) → Task 1; geometria + paridade → Task 2.
- Editor (controles + bolha círculo/arredondado, mover/redimensionar/borda/espelho, fixo por cima) + preview ao vivo → Task 7.
- Export composição (scale/mirror/máscara/overlay) → Task 6.
- Erros (sem permissão, nokhwa falha, webcam ausente no export) → Tasks 4/5 (degrade+catch_unwind), 6 (skip overlay).
- Testes (unit Rust, paridade TS, smoke) → Tasks 1–3 (unit), 2 (paridade), 6 (builder+render), 7/8 (smoke).
- Sem gap pro escopo F3.

**2. Placeholders:** sem "TBD". Tasks de integração (4, 6, 7) trazem código concreto + instrução explícita de validar a API real (nokhwa/geq/clip-path) e reportar DONE_WITH_CONCERNS se divergir — risco conhecido, não placeholder.

**3. Consistência de tipos:** `WebcamOverlay` (Rust+TS mesmos campos), `geometry`→`(s,xpx,ypx)`/`{s,x,y}`, `ZoomModel.webcam` (Option/optional), `RecordingInfo.has_webcam/camera_name`, `store::webcam_path`, `WebcamCapture::{start(camera_id,fps,out),stop}`, `Coordinator::start(...,camera_id,...)`, comandos `list_cameras`/`start_recording(camera_id)` ↔ `listCameras`/`startRecording(...,cameraId)`. Nomes batem.

**Riscos conhecidos (documentados):**
- API do `nokhwa` 0.10 (Camera/frame/decode/feature mac) pode divergir — validar (Task 4).
- Máscara `geq` (círculo/arredondado) + borda no ffmpeg — validar no render (Task 6); borda simplificada.
- Captura de webcam adiciona 4ª via concorrente; cada captura já isola sua thread + ffmpeg; stop para todas.
- Paridade preview×export do overlay: ambos usam `geometry`; recorte círculo bate; "rounded" e borda são aproximações entre clip-path (preview) e geq (export) — aceitável F3, documentar.
