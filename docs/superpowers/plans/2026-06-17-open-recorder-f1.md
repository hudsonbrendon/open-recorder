# OpenRecorder F1 (Fundação de Captura) — Implementation Plan (Tauri)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Gravador de tela cross-platform (Win/Mac/Linux) que captura tela/janela/região + microfone, produzindo um `.mp4` cru e um `metadata.json` com eventos de clique/mouse.

**Architecture:** App Tauri 2 — core Rust + UI React/webview. `scap` captura frames de tela; os frames são encodados via `ffmpeg` (processo). `cpal` grava o microfone; `rdev` registra cliques/mouse num buffer. No stop, `ffmpeg` muxa vídeo+áudio em `.mp4` e o buffer vira `metadata.json`. Gravação não-destrutiva: a metadata é gravada mas só consumida na F2.

**Tech Stack:** Tauri 2, Rust (cargo), React + Vite + TypeScript (pnpm). Crates: `scap`, `cpal`, `rdev`, `serde`/`serde_json`. `ffmpeg` como processo externo (sidecar). Testes: `cargo test` (Rust) + `vitest` (UI).

## Global Constraints

- Plataforma: **Windows, macOS, Linux**. Crates cross-platform; smoke MVP em macOS, demais SOs quando disponíveis.
- Stack: **Tauri 2 + Rust + React/TS**. UI no webview nativo (não Chromium embutido).
- Sem dependência de runtime além do `ffmpeg` (sidecar/binário externo). Encode via `ffmpeg`.
- Gravação **não-destrutiva**: vídeo cru + `metadata.json`; efeitos só no export (F2+).
- Áudio v1: **apenas microfone**.
- Formato `metadata.json`: **versionado** (`version: 1`); JSON em snake_case; coordenadas relativas ao retângulo da fonte (origem no canto superior-esquerdo).
- Commits: **sem co-author/histórico do Claude**. Mensagens em inglês, formato `tipo: descrição`.
- Crate Rust: `open-recorder` (lib `open_recorder_lib`). App: `OpenRecorder`, id `com.openrecorder.app`.
- O scaffold Tauri 2 + React-TS **já existe e compila** (commits anteriores). Não re-scaffoldar.

## Estado já pronto (NÃO refazer)

- `src-tauri/` (Tauri 2) + `src/` (React-TS) scaffoldados, `cargo build` e `pnpm build` passam.
- `src-tauri/src/lib.rs` tem o comando exemplo `greet` e o `run()` builder.
- Crate renomeado para `open_recorder_lib`; bundle id `com.openrecorder.app`.

## File Structure

```
src-tauri/src/
├── lib.rs                  # run(): registra módulos e invoke_handler (modificado)
├── main.rs                 # entrypoint (já pronto)
├── model/
│   ├── mod.rs
│   ├── metadata.rs         # structs serde do metadata.json
│   ├── source.rs           # CaptureSource, SourceKind
│   └── coords.rs           # map_to_source(rect, point) -> Option<(i64,i64)>
├── capture/
│   ├── mod.rs
│   ├── ffmpeg.rs           # builder de comando ffmpeg (puro) + localizar binário
│   ├── input_recorder.rs   # buffer de eventos (testável) + rdev (smoke)
│   ├── source_enum.rs      # scap: lista displays/janelas (smoke)
│   ├── video_capture.rs    # scap -> ffmpeg stdin (smoke)
│   ├── audio_capture.rs    # cpal -> arquivo (smoke)
│   └── finalizer.rs        # escreve metadata.json (testável)
├── recording/
│   ├── mod.rs
│   └── coordinator.rs      # orquestra start/stop + estado global
└── commands.rs             # comandos Tauri (thin wrappers)

src/
├── App.tsx                 # raiz (reescrito)
├── lib/
│   ├── api.ts              # wrappers invoke() tipados
│   ├── format.ts           # formatação (timer, etc.) — testável
│   └── format.test.ts      # vitest
├── components/
│   ├── SourcePicker.tsx
│   ├── MicPicker.tsx
│   ├── RecordControls.tsx
│   └── RecordingsList.tsx
└── state/
    └── useRecorder.ts      # hook de estado
docs/SMOKE-TEST.md
README.md
```

---

## Task 1: Dependências Rust + esqueleto de módulos

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/model/mod.rs`, `src-tauri/src/capture/mod.rs`, `src-tauri/src/recording/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: nada.
- Produces: crate compila com os módulos vazios declarados e as deps adicionadas. `greet` permanece por ora.

- [ ] **Step 1: Adicionar dependências de captura ao `Cargo.toml`**

Adicionar ao bloco `[dependencies]` (após as existentes):

```toml
scap = "0.0.8"
cpal = "0.15"
rdev = "0.5"
thiserror = "2"
```

- [ ] **Step 2: Criar os módulos vazios**

`src-tauri/src/model/mod.rs`:
```rust
pub mod metadata;
pub mod source;
pub mod coords;
```

`src-tauri/src/capture/mod.rs`:
```rust
pub mod ffmpeg;
pub mod input_recorder;
pub mod source_enum;
pub mod video_capture;
pub mod audio_capture;
pub mod finalizer;
```

`src-tauri/src/recording/mod.rs`:
```rust
pub mod coordinator;
```

Crie também arquivos vazios para cada submódulo referenciado acima (ex.: `model/metadata.rs` com `// preenchido na Task 2`), senão o crate não compila. Conteúdo mínimo placeholder por arquivo: um comentário.

- [ ] **Step 3: Declarar os módulos em `lib.rs`**

No topo de `src-tauri/src/lib.rs`, antes do `greet`:
```rust
pub mod model;
pub mod capture;
pub mod recording;
pub mod commands;
```

Criar `src-tauri/src/commands.rs` com `// comandos na Task 10`.

- [ ] **Step 4: Compilar**

Run: `cd src-tauri && cargo build 2>&1 | tail -5`
Expected: `Finished` sem erros (baixa e compila scap/cpal/rdev na 1ª vez — pode demorar).

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add capture crates and module skeleton"
```

---

## Task 2: Modelo de metadata (serde, snake_case)

**Files:**
- Modify: `src-tauri/src/model/metadata.rs`

**Interfaces:**
- Consumes: nada.
- Produces:
  - `RecordingMetadata { version: u32, recording: RecordingInfo, source: SourceInfo, events: Vec<InputEvent> }`
  - `RecordingInfo { width: u32, height: u32, fps: u32, duration_ms: u64 }`
  - `SourceInfo { kind: String, id: String, rect: [i64; 4] }` (serializa campo `kind` como `"type"`)
  - `InputEvent { t_ms: u64, kind: String, x: i64, y: i64, button: Option<String> }` (campo `kind` serializa como `"type"`)
  - Todos `#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]`.

- [ ] **Step 1: Escrever o teste de round-trip falhando**

Em `src-tauri/src/model/metadata.rs`:
```rust
use serde::{Serialize, Deserialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_snake_case_with_type_field() {
        let meta = RecordingMetadata {
            version: 1,
            recording: RecordingInfo { width: 2560, height: 1440, fps: 30, duration_ms: 18450 },
            source: SourceInfo { kind: "display".into(), id: "1".into(), rect: [0, 0, 2560, 1440] },
            events: vec![
                InputEvent { t_ms: 1200, kind: "click".into(), x: 840, y: 410, button: Some("left".into()) },
                InputEvent { t_ms: 1200, kind: "move".into(), x: 840, y: 410, button: None },
            ],
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"duration_ms\":18450"), "{json}");
        assert!(json.contains("\"t_ms\":1200"), "{json}");
        assert!(json.contains("\"type\":\"display\""), "{json}");
        assert!(json.contains("\"type\":\"click\""), "{json}");
    }

    #[test]
    fn round_trip_preserves_values() {
        let meta = RecordingMetadata {
            version: 1,
            recording: RecordingInfo { width: 100, height: 200, fps: 60, duration_ms: 5000 },
            source: SourceInfo { kind: "window".into(), id: "abc".into(), rect: [10, 20, 30, 40] },
            events: vec![InputEvent { t_ms: 0, kind: "click".into(), x: 1, y: 2, button: Some("right".into()) }],
        };
        let json = serde_json::to_string(&meta).unwrap();
        let back: RecordingMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, back);
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test metadata 2>&1 | tail -15`
Expected: erro de compilação — `cannot find type RecordingMetadata`.

- [ ] **Step 3: Implementar os structs**

Acima do bloco `#[cfg(test)]`:
```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RecordingMetadata {
    pub version: u32,
    pub recording: RecordingInfo,
    pub source: SourceInfo,
    pub events: Vec<InputEvent>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RecordingInfo {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SourceInfo {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: String,
    pub rect: [i64; 4],
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InputEvent {
    pub t_ms: u64,
    #[serde(rename = "type")]
    pub kind: String,
    pub x: i64,
    pub y: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button: Option<String>,
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run: `cd src-tauri && cargo test metadata 2>&1 | tail -10`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add versioned RecordingMetadata serde model"
```

---

## Task 3: CaptureSource + mapeamento de coordenadas

**Files:**
- Modify: `src-tauri/src/model/source.rs`, `src-tauri/src/model/coords.rs`

**Interfaces:**
- Consumes: nada.
- Produces:
  - `enum SourceKind { Display, Window, Region }` com `as_str(&self) -> &'static str` (`"display"|"window"|"region"`).
  - `struct CaptureSource { kind: SourceKind, id: String, rect: [i64; 4] }` (rect = x,y,w,h globais).
  - `fn map_to_source(rect: [i64; 4], x: i64, y: i64) -> Option<(i64, i64)>` em `coords.rs` — relativo à origem da fonte; `None` se fora.

- [ ] **Step 1: Escrever testes de coords falhando**

Em `src-tauri/src/model/coords.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_inside_point_to_relative() {
        assert_eq!(map_to_source([100, 50, 800, 600], 150, 90), Some((50, 40)));
    }

    #[test]
    fn maps_top_left_to_zero() {
        assert_eq!(map_to_source([100, 50, 800, 600], 100, 50), Some((0, 0)));
    }

    #[test]
    fn returns_none_outside_left() {
        assert_eq!(map_to_source([100, 50, 800, 600], 99, 90), None);
    }

    #[test]
    fn returns_none_outside_bottom() {
        assert_eq!(map_to_source([100, 50, 800, 600], 150, 651), None);
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test coords 2>&1 | tail -10`
Expected: `cannot find function map_to_source`.

- [ ] **Step 3: Implementar `coords.rs` e `source.rs`**

`src-tauri/src/model/coords.rs` (acima do teste):
```rust
/// Converte um ponto global para coordenadas relativas à fonte (origem
/// superior-esquerda). `rect` = [x, y, w, h]. None se cair fora.
pub fn map_to_source(rect: [i64; 4], x: i64, y: i64) -> Option<(i64, i64)> {
    let [rx, ry, rw, rh] = rect;
    let rel_x = x - rx;
    let rel_y = y - ry;
    if rel_x < 0 || rel_y < 0 || rel_x > rw || rel_y > rh {
        return None;
    }
    Some((rel_x, rel_y))
}
```

`src-tauri/src/model/source.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Display,
    Window,
    Region,
}

impl SourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceKind::Display => "display",
            SourceKind::Window => "window",
            SourceKind::Region => "region",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureSource {
    pub kind: SourceKind,
    pub id: String,
    pub rect: [i64; 4],
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run: `cd src-tauri && cargo test coords 2>&1 | tail -10`
Expected: `test result: ok. 4 passed`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add CaptureSource and coordinate mapping"
```

---

## Task 4: Builder de comando ffmpeg (puro)

Lógica pura que monta os argumentos do ffmpeg para (a) encodar frames crus vindos do stdin e (b) muxar vídeo+áudio no `.mp4` final. Sem rodar ffmpeg.

**Files:**
- Modify: `src-tauri/src/capture/ffmpeg.rs`

**Interfaces:**
- Consumes: nada.
- Produces:
  - `fn encode_args(width: u32, height: u32, fps: u32, out_path: &str) -> Vec<String>` — args para `ffmpeg` lendo BGRA cru do stdin (`-f rawvideo -pix_fmt bgra -s WxH -r fps -i - ... out_path`).
  - `fn mux_args(video_path: &str, audio_path: &str, out_path: &str) -> Vec<String>` — junta vídeo + áudio (`-i video -i audio -c copy ... out_path`).
  - `fn ffmpeg_binary() -> String` — retorna `"ffmpeg"` (PATH) por ora; sidecar configurado na Task 11.

- [ ] **Step 1: Escrever testes falhando**

Em `src-tauri/src/capture/ffmpeg.rs`:
```rust
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
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test ffmpeg 2>&1 | tail -10`
Expected: `cannot find function encode_args`.

- [ ] **Step 3: Implementar**

Acima do teste:
```rust
pub fn ffmpeg_binary() -> String {
    "ffmpeg".to_string()
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
```

- [ ] **Step 4: Rodar pra ver passar**

Run: `cd src-tauri && cargo test ffmpeg 2>&1 | tail -10`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add ffmpeg encode/mux argument builders"
```

---

## Task 5: Input recorder (buffer testável + rdev real)

Captura via `rdev` fica atrás de um método `ingest` testável com eventos sintéticos. A thread `rdev` real só é exercida no smoke.

**Files:**
- Modify: `src-tauri/src/capture/input_recorder.rs`

**Interfaces:**
- Consumes: `map_to_source` (Task 3), `InputEvent` (Task 2).
- Produces:
  - `struct InputRecorder { rect: [i64;4], start_ms: u64, events: Vec<InputEvent> }`
  - `fn new(rect: [i64;4], start_ms: u64) -> Self`
  - `fn ingest(&mut self, x: i64, y: i64, kind: &str, button: Option<String>, now_ms: u64)` — aplica o mapper, calcula `t_ms = now_ms - start_ms`, descarta se fora.
  - `fn take_events(self) -> Vec<InputEvent>`

- [ ] **Step 1: Escrever testes falhando**

Em `src-tauri/src/capture/input_recorder.rs`:
```rust
use crate::model::coords::map_to_source;
use crate::model::metadata::InputEvent;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingest_stores_mapped_event_with_relative_time() {
        let mut rec = InputRecorder::new([100, 50, 800, 600], 1000);
        rec.ingest(150, 90, "click", Some("left".into()), 1200);
        let ev = rec.take_events();
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].t_ms, 200);
        assert_eq!((ev[0].x, ev[0].y), (50, 40));
        assert_eq!(ev[0].kind, "click");
        assert_eq!(ev[0].button.as_deref(), Some("left"));
    }

    #[test]
    fn ingest_drops_events_outside_source() {
        let mut rec = InputRecorder::new([0, 0, 100, 100], 0);
        rec.ingest(500, 500, "move", None, 50);
        assert_eq!(rec.take_events().len(), 0);
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test input_recorder 2>&1 | tail -10`
Expected: `cannot find ... InputRecorder`.

- [ ] **Step 3: Implementar**

Acima do teste:
```rust
pub struct InputRecorder {
    rect: [i64; 4],
    start_ms: u64,
    events: Vec<InputEvent>,
}

impl InputRecorder {
    pub fn new(rect: [i64; 4], start_ms: u64) -> Self {
        Self { rect, start_ms, events: Vec::new() }
    }

    pub fn ingest(&mut self, x: i64, y: i64, kind: &str, button: Option<String>, now_ms: u64) {
        if let Some((rx, ry)) = map_to_source(self.rect, x, y) {
            let t_ms = now_ms.saturating_sub(self.start_ms);
            self.events.push(InputEvent {
                t_ms,
                kind: kind.to_string(),
                x: rx,
                y: ry,
                button,
            });
        }
    }

    pub fn take_events(self) -> Vec<InputEvent> {
        self.events
    }
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run: `cd src-tauri && cargo test input_recorder 2>&1 | tail -10`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add InputRecorder buffer with testable ingest"
```

---

## Task 6: Finalizer (escreve metadata.json) + nomes de arquivo

**Files:**
- Modify: `src-tauri/src/capture/finalizer.rs`
- Modify: `src-tauri/src/recording/coordinator.rs` (só o helper de nomes nesta task)

**Interfaces:**
- Consumes: `RecordingMetadata` etc. (Task 2), `CaptureSource`/`SourceKind` (Task 3).
- Produces:
  - `fn build_metadata(source: &CaptureSource, fps: u32, duration_ms: u64, events: Vec<InputEvent>) -> RecordingMetadata`
  - `fn write_metadata(meta: &RecordingMetadata, path: &Path) -> std::io::Result<()>` (JSON pretty).
  - Em `coordinator.rs`: `fn make_filenames(timestamp: &str) -> (String, String)` → `("REC-<ts>.mp4", "REC-<ts>.metadata.json")`.

- [ ] **Step 1: Escrever testes falhando**

Em `src-tauri/src/capture/finalizer.rs`:
```rust
use std::path::Path;
use crate::model::metadata::{RecordingMetadata, RecordingInfo, SourceInfo, InputEvent};
use crate::model::source::{CaptureSource, SourceKind};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_metadata_from_source() {
        let src = CaptureSource { kind: SourceKind::Display, id: "1".into(), rect: [0, 0, 1920, 1080] };
        let meta = build_metadata(&src, 30, 5000, vec![]);
        assert_eq!(meta.version, 1);
        assert_eq!(meta.recording, RecordingInfo { width: 1920, height: 1080, fps: 30, duration_ms: 5000 });
        assert_eq!(meta.source, SourceInfo { kind: "display".into(), id: "1".into(), rect: [0, 0, 1920, 1080] });
    }

    #[test]
    fn writes_json_file_round_trip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("meta-{}.json", std::process::id()));
        let src = CaptureSource { kind: SourceKind::Window, id: "7".into(), rect: [5, 6, 100, 200] };
        let meta = build_metadata(&src, 60, 1234, vec![
            InputEvent { t_ms: 10, kind: "click".into(), x: 1, y: 2, button: Some("left".into()) },
        ]);
        write_metadata(&meta, &path).unwrap();
        let txt = std::fs::read_to_string(&path).unwrap();
        let back: RecordingMetadata = serde_json::from_str(&txt).unwrap();
        assert_eq!(back, meta);
        let _ = std::fs::remove_file(&path);
    }
}
```

E em `src-tauri/src/recording/coordinator.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filenames_share_timestamp() {
        let (v, m) = make_filenames("20260618-153000");
        assert_eq!(v, "REC-20260618-153000.mp4");
        assert_eq!(m, "REC-20260618-153000.metadata.json");
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test finalizer 2>&1 | tail -10 && cargo test coordinator 2>&1 | tail -10`
Expected: erros `cannot find function build_metadata` / `make_filenames`.

- [ ] **Step 3: Implementar**

`finalizer.rs` (acima do teste):
```rust
pub fn build_metadata(
    source: &CaptureSource,
    fps: u32,
    duration_ms: u64,
    events: Vec<InputEvent>,
) -> RecordingMetadata {
    let [x, y, w, h] = source.rect;
    RecordingMetadata {
        version: 1,
        recording: RecordingInfo { width: w as u32, height: h as u32, fps, duration_ms },
        source: SourceInfo { kind: source.kind.as_str().to_string(), id: source.id.clone(), rect: [x, y, w, h] },
        events,
    }
}

pub fn write_metadata(meta: &RecordingMetadata, path: &Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(meta).expect("serialize metadata");
    std::fs::write(path, json)
}
```

`coordinator.rs` (acima do teste):
```rust
pub fn make_filenames(timestamp: &str) -> (String, String) {
    (format!("REC-{timestamp}.mp4"), format!("REC-{timestamp}.metadata.json"))
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run: `cd src-tauri && cargo test finalizer 2>&1 | tail -10 && cargo test coordinator 2>&1 | tail -10`
Expected: ambos `ok`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add metadata finalizer and filename helper"
```

---

## Task 7: ffmpeg sidecar / verificação de binário

Garante que o `ffmpeg` está disponível. Para F1, usa o `ffmpeg` do PATH (verificado no start) com mensagem de erro clara se ausente; documenta o caminho de sidecar para depois.

**Files:**
- Modify: `src-tauri/src/capture/ffmpeg.rs`

**Interfaces:**
- Consumes: `ffmpeg_binary` (Task 4).
- Produces: `fn ensure_ffmpeg() -> Result<(), String>` — roda `ffmpeg -version`; erro amigável se faltar.

- [ ] **Step 1: Escrever teste (verifica forma do erro, não a presença)**

Em `ffmpeg.rs` (adicionar ao mod tests):
```rust
    #[test]
    fn ensure_ffmpeg_returns_result() {
        // Não assume ffmpeg instalado no CI; só garante que retorna sem panicar
        // e que, se erro, a mensagem menciona ffmpeg.
        if let Err(msg) = ensure_ffmpeg() {
            assert!(msg.to_lowercase().contains("ffmpeg"), "{msg}");
        }
    }
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test ffmpeg 2>&1 | tail -10`
Expected: `cannot find function ensure_ffmpeg`.

- [ ] **Step 3: Implementar**

Em `ffmpeg.rs`:
```rust
use std::process::Command;

pub fn ensure_ffmpeg() -> Result<(), String> {
    match Command::new(ffmpeg_binary()).arg("-version").output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(_) => Err("ffmpeg encontrado mas retornou erro ao executar -version".into()),
        Err(_) => Err("ffmpeg não encontrado no PATH. Instale o ffmpeg para gravar.".into()),
    }
}
```

> **Sidecar (depois):** para empacotar, baixar binários estáticos do ffmpeg por
> plataforma em `src-tauri/binaries/ffmpeg-<target-triple>` e referenciar em
> `tauri.conf.json > bundle.externalBin`. Trocar `ffmpeg_binary()` para resolver
> via `tauri::process::current_binary`/sidecar. Fora do escopo do F1 (usa PATH).

- [ ] **Step 4: Rodar + garantir ffmpeg presente no dev**

Run: `which ffmpeg || brew install ffmpeg` (no dev). Depois `cd src-tauri && cargo test ffmpeg 2>&1 | tail -10`.
Expected: `test result: ok` (3 testes ffmpeg).

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add ffmpeg availability check"
```

---

## Task 8: Source enumerator (scap) — smoke

`scap` exige permissão e ambiente gráfico; não unit-testável de forma confiável. Implementar consultando a API atual do crate `scap` (use docs/context7 se necessário — a API de listagem de alvos pode diferir entre versões).

**Files:**
- Modify: `src-tauri/src/capture/source_enum.rs`

**Interfaces:**
- Consumes: `CaptureSource`, `SourceKind` (Task 3).
- Produces:
  - `struct SourceOption { pub id: String, pub name: String, pub kind: String, pub rect: [i64;4] }`
  - `fn list_displays() -> Result<Vec<SourceOption>, String>`
  - `fn list_windows() -> Result<Vec<SourceOption>, String>`
  - `fn to_capture_source(opt: &SourceOption) -> CaptureSource`

- [ ] **Step 1: Implementar usando a API do scap**

Consultar a API da versão de `scap` em uso (`scap::get_all_targets()` / `Target` enum, ou equivalente). Mapear cada display/janela para `SourceOption`. Exemplo de forma (ajustar aos tipos reais do crate):

```rust
use crate::model::source::{CaptureSource, SourceKind};

#[derive(serde::Serialize, Clone, Debug)]
pub struct SourceOption {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub rect: [i64; 4],
}

pub fn list_displays() -> Result<Vec<SourceOption>, String> {
    if !scap::has_permission() {
        return Err("Permissão de captura de tela ausente".into());
    }
    let targets = scap::get_all_targets();
    let mut out = Vec::new();
    for t in targets {
        if let scap::Target::Display(d) = t {
            out.push(SourceOption {
                id: d.id.to_string(),
                name: format!("Tela {}", d.id),
                kind: "display".into(),
                rect: [0, 0, d.width as i64, d.height as i64],
            });
        }
    }
    Ok(out)
}

pub fn list_windows() -> Result<Vec<SourceOption>, String> {
    if !scap::has_permission() {
        return Err("Permissão de captura de tela ausente".into());
    }
    let targets = scap::get_all_targets();
    let mut out = Vec::new();
    for t in targets {
        if let scap::Target::Window(w) = t {
            out.push(SourceOption {
                id: w.id.to_string(),
                name: w.title.clone(),
                kind: "window".into(),
                rect: [0, 0, 0, 0],
            });
        }
    }
    Ok(out)
}

pub fn to_capture_source(opt: &SourceOption) -> CaptureSource {
    let kind = match opt.kind.as_str() {
        "window" => SourceKind::Window,
        "region" => SourceKind::Region,
        _ => SourceKind::Display,
    };
    CaptureSource { kind, id: opt.id.clone(), rect: opt.rect }
}
```

> Se a API do `scap` divergir do exemplo, **ajustar ao crate real** (verificar
> `scap` no docs.rs/context7). Reportar DONE_WITH_CONCERNS descrevendo a API real
> usada se diferir.

- [ ] **Step 2: Compilar**

Run: `cd src-tauri && cargo build 2>&1 | tail -10`
Expected: `Finished` sem erros.

- [ ] **Step 3: Smoke manual (depois, via UI na Task 12)**

A verificação real (lista telas/janelas com permissão concedida) acontece no smoke da Task 12. Sem teste automatizado aqui.

- [ ] **Step 4: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add scap-based source enumerator"
```

---

## Task 9: Video + audio capture (scap→ffmpeg, cpal) — smoke

Integração de captura. Sem unit test (hardware/permissão). Implementar consultando as APIs reais de `scap` (recebimento de frames) e `cpal` (stream de input de áudio). Reportar DONE_WITH_CONCERNS se a API divergir do esboço.

**Files:**
- Modify: `src-tauri/src/capture/video_capture.rs`, `src-tauri/src/capture/audio_capture.rs`

**Interfaces:**
- Consumes: `encode_args`, `ffmpeg_binary` (Task 4), `CaptureSource` (Task 3).
- Produces:
  - `struct VideoCapture` com `fn start(source: &CaptureSource, fps: u32, video_tmp: &Path) -> Result<VideoCapture, String>` e `fn stop(self) -> Result<(), String>`. Internamente: inicia `scap` capturer, spawna thread que lê frames BGRA e escreve no stdin de um `ffmpeg` (`encode_args`), encerra ffmpeg no stop.
  - `struct AudioCapture` com `fn start(device_id: Option<String>, audio_tmp: &Path) -> Result<AudioCapture, String>` e `fn stop(self) -> Result<(), String>`. Usa `cpal` para gravar WAV do mic; `fn list_microphones() -> Vec<(String, String)>`.

- [ ] **Step 1: Implementar `video_capture.rs`**

Esboço (ajustar à API real do `scap`):
```rust
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread::JoinHandle;
use crate::model::source::CaptureSource;
use crate::capture::ffmpeg::{encode_args, ffmpeg_binary};

pub struct VideoCapture {
    stop_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
    ffmpeg: Child,
}

impl VideoCapture {
    pub fn start(source: &CaptureSource, fps: u32, video_tmp: &Path) -> Result<Self, String> {
        let [_, _, w, h] = source.rect;
        let args = encode_args(w as u32, h as u32, fps, video_tmp.to_str().unwrap());
        let mut ffmpeg = Command::new(ffmpeg_binary())
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("falha ao iniciar ffmpeg: {e}"))?;
        let mut stdin = ffmpeg.stdin.take().ok_or("sem stdin do ffmpeg")?;

        // Configurar e iniciar o capturer scap (API real do crate):
        let mut capturer = scap::capturer::Capturer::build(scap::capturer::Options {
            fps,
            target: None, // resolver pelo source.id na API real
            show_cursor: true,
            output_type: scap::frame::FrameType::BGRAFrame,
            ..Default::default()
        }).map_err(|e| format!("falha scap: {e:?}"))?;
        capturer.start_capture();

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let handle = std::thread::spawn(move || {
            loop {
                if stop_rx.try_recv().is_ok() { break; }
                match capturer.get_next_frame() {
                    Ok(scap::frame::Frame::BGRA(f)) => { let _ = stdin.write_all(&f.data); }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
            capturer.stop_capture();
            drop(stdin); // fecha stdin -> ffmpeg finaliza
        });

        Ok(Self { stop_tx, handle: Some(handle), ffmpeg })
    }

    pub fn stop(mut self) -> Result<(), String> {
        let _ = self.stop_tx.send(());
        if let Some(h) = self.handle.take() { let _ = h.join(); }
        let _ = self.ffmpeg.wait();
        Ok(())
    }
}
```

- [ ] **Step 2: Implementar `audio_capture.rs`**

Esboço com `cpal` gravando WAV (usar crate `hound` para WAV — adicionar `hound = "3"` ao Cargo.toml):
```rust
use std::path::Path;
use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct AudioCapture {
    stream: cpal::Stream,
    writer: Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>,
}

pub fn list_microphones() -> Vec<(String, String)> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devs| devs.filter_map(|d| d.name().ok().map(|n| (n.clone(), n))).collect())
        .unwrap_or_default()
}

impl AudioCapture {
    pub fn start(device_id: Option<String>, audio_tmp: &Path) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = match device_id {
            Some(name) => host.input_devices().map_err(|e| e.to_string())?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .ok_or("microfone não encontrado")?,
            None => host.default_input_device().ok_or("sem microfone padrão")?,
        };
        let config = device.default_input_config().map_err(|e| e.to_string())?;
        let spec = hound::WavSpec {
            channels: config.channels(),
            sample_rate: config.sample_rate().0,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let writer = hound::WavWriter::create(audio_tmp, spec).map_err(|e| e.to_string())?;
        let writer = Arc::new(Mutex::new(Some(writer)));
        let w2 = writer.clone();
        let err_fn = |e| eprintln!("erro de áudio: {e}");
        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| {
                if let Some(w) = w2.lock().unwrap().as_mut() {
                    for &s in data { let _ = w.write_sample(s); }
                }
            },
            err_fn, None,
        ).map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self { stream, writer })
    }

    pub fn stop(self) -> Result<(), String> {
        drop(self.stream);
        if let Some(w) = self.writer.lock().unwrap().take() {
            w.finalize().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
```

Adicionar ao `Cargo.toml`: `hound = "3"`.

- [ ] **Step 3: Compilar**

Run: `cd src-tauri && cargo build 2>&1 | tail -15`
Expected: `Finished`. Se a API de `scap`/`cpal` divergir, corrigir conforme o crate real e anotar em DONE_WITH_CONCERNS.

- [ ] **Step 4: Smoke manual (na Task 12, gravação real)**

Verificação fim-a-fim na Task 12.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add video (scap->ffmpeg) and audio (cpal) capture"
```

---

## Task 10: RecordingCoordinator + comandos Tauri

**Files:**
- Modify: `src-tauri/src/recording/coordinator.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: tudo das tasks anteriores.
- Produces:
  - `struct Coordinator` (estado: gravação ativa, paths, source, fps, start_ms, input recorder, capturers). Guardado em `tauri::State<Mutex<Coordinator>>`.
  - Comandos Tauri (em `commands.rs`):
    - `list_sources() -> Result<SourcesPayload, String>` (`{ displays, windows }`)
    - `list_microphones() -> Vec<MicOption>` (`{ id, name }`)
    - `start_recording(source: SourceOptionInput, mic_id: Option<String>) -> Result<(), String>`
    - `stop_recording() -> Result<RecordingResult, String>` (`{ video_path, metadata_path, duration_ms }`)
    - `reveal_in_folder(path: String) -> Result<(), String>`
  - Registrados no `invoke_handler` em `lib.rs`. Remover `greet`.

- [ ] **Step 1: Implementar `Coordinator` em `coordinator.rs`**

Manter `make_filenames` (Task 6). Adicionar o struct e métodos `start`/`stop` que ligam `VideoCapture`, `AudioCapture`, `InputRecorder`, e no stop chamam `mux_args`+ffmpeg e `write_metadata`. Diretório de saída: pasta de vídeos do usuário + `OpenRecorder/`. Usar timestamp via `std::time::SystemTime`. Estado de gravação ativa em campos `Option<...>`.

```rust
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::model::source::CaptureSource;
use crate::capture::{video_capture::VideoCapture, audio_capture::AudioCapture,
                     input_recorder::InputRecorder, finalizer, ffmpeg};

#[derive(serde::Serialize, Clone, PartialEq, Debug)]
pub struct RecordingResult {
    pub video_path: String,
    pub metadata_path: String,
    pub duration_ms: u64,
}

#[derive(Default)]
pub struct Coordinator {
    active: Option<Active>,
}

struct Active {
    source: CaptureSource,
    fps: u32,
    start_ms: u64,
    video_tmp: PathBuf,
    audio_tmp: PathBuf,
    out_video: PathBuf,
    out_meta: PathBuf,
    has_audio: bool,
    video: Option<VideoCapture>,
    audio: Option<AudioCapture>,
    input: InputRecorder,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}

impl Coordinator {
    pub fn output_dir() -> PathBuf {
        let base = dirs_next_videos();
        let dir = base.join("OpenRecorder");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    pub fn start(&mut self, source: CaptureSource, mic_id: Option<String>, fps: u32) -> Result<(), String> {
        ffmpeg::ensure_ffmpeg()?;
        if self.active.is_some() { return Err("gravação já em andamento".into()); }
        let ts = timestamp();
        let (vname, mname) = make_filenames(&ts);
        let dir = Self::output_dir();
        let out_video = dir.join(&vname);
        let out_meta = dir.join(&mname);
        let video_tmp = dir.join(format!("{ts}.video.mp4"));
        let audio_tmp = dir.join(format!("{ts}.audio.wav"));

        let start_ms = now_ms();
        let input = InputRecorder::new(source.rect, start_ms);
        let video = VideoCapture::start(&source, fps, &video_tmp)?;
        let has_audio = mic_id.is_some();
        let audio = if has_audio {
            Some(AudioCapture::start(mic_id, &audio_tmp)?)
        } else { None };

        self.active = Some(Active {
            source, fps, start_ms, video_tmp, audio_tmp, out_video, out_meta,
            has_audio, video: Some(video), audio, input,
        });
        Ok(())
    }

    pub fn stop(&mut self) -> Result<RecordingResult, String> {
        let mut a = self.active.take().ok_or("nenhuma gravação ativa")?;
        let duration_ms = now_ms().saturating_sub(a.start_ms);
        if let Some(v) = a.video.take() { v.stop()?; }
        if let Some(au) = a.audio.take() { au.stop()?; }

        if a.has_audio {
            let args = ffmpeg::mux_args(
                a.video_tmp.to_str().unwrap(),
                a.audio_tmp.to_str().unwrap(),
                a.out_video.to_str().unwrap());
            Command::new(ffmpeg::ffmpeg_binary()).args(&args).output()
                .map_err(|e| format!("mux falhou: {e}"))?;
        } else {
            std::fs::rename(&a.video_tmp, &a.out_video).map_err(|e| e.to_string())?;
        }
        let _ = std::fs::remove_file(&a.video_tmp);
        let _ = std::fs::remove_file(&a.audio_tmp);

        let events = a.input.take_events();
        let meta = finalizer::build_metadata(&a.source, a.fps, duration_ms, events);
        finalizer::write_metadata(&meta, &a.out_meta).map_err(|e| e.to_string())?;

        Ok(RecordingResult {
            video_path: a.out_video.to_string_lossy().to_string(),
            metadata_path: a.out_meta.to_string_lossy().to_string(),
            duration_ms,
        })
    }
}

fn timestamp() -> String {
    // yyyymmdd-hhmmss simples baseado em epoch local — usar chrono se preferir.
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    format!("{secs}")
}

fn dirs_next_videos() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."))
        .join("Movies")
}
```

> Nota: o `input` real (thread `rdev`) é ligado no `start` chamando uma função
> que faz `rdev::listen` numa thread e empurra para o `InputRecorder` via canal.
> Como `rdev::listen` é bloqueante e global, encapsular numa thread com canal de
> stop. Se a integração `rdev` ficar complexa, entregar o vídeo+áudio funcionando
> e reportar o input como DONE_WITH_CONCERNS (degrada: events vazio).

- [ ] **Step 2: Implementar comandos em `commands.rs`**

```rust
use tauri::State;
use std::sync::Mutex;
use crate::recording::coordinator::{Coordinator, RecordingResult};
use crate::capture::source_enum::{self, SourceOption};
use crate::capture::audio_capture;

#[derive(serde::Serialize)]
pub struct SourcesPayload {
    pub displays: Vec<SourceOption>,
    pub windows: Vec<SourceOption>,
}

#[derive(serde::Serialize)]
pub struct MicOption { pub id: String, pub name: String }

#[tauri::command]
pub fn list_sources() -> Result<SourcesPayload, String> {
    Ok(SourcesPayload {
        displays: source_enum::list_displays()?,
        windows: source_enum::list_windows().unwrap_or_default(),
    })
}

#[tauri::command]
pub fn list_microphones() -> Vec<MicOption> {
    audio_capture::list_microphones().into_iter()
        .map(|(id, name)| MicOption { id, name }).collect()
}

#[tauri::command]
pub fn start_recording(
    state: State<'_, Mutex<Coordinator>>,
    source: SourceOption,
    mic_id: Option<String>,
) -> Result<(), String> {
    let cs = source_enum::to_capture_source(&source);
    state.lock().unwrap().start(cs, mic_id, 30)
}

#[tauri::command]
pub fn stop_recording(state: State<'_, Mutex<Coordinator>>) -> Result<RecordingResult, String> {
    state.lock().unwrap().stop()
}

#[tauri::command]
pub fn reveal_in_folder(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let dir = p.parent().unwrap_or(p);
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(dir).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(dir).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
    Ok(())
}
```

`SourceOption` precisa derivar `Deserialize` também (adicionar `Deserialize` ao derive em `source_enum.rs`).

- [ ] **Step 3: Registrar em `lib.rs`**

Substituir o `invoke_handler` e remover `greet`:
```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(std::sync::Mutex::new(crate::recording::coordinator::Coordinator::default()))
        .invoke_handler(tauri::generate_handler![
            crate::commands::list_sources,
            crate::commands::list_microphones,
            crate::commands::start_recording,
            crate::commands::stop_recording,
            crate::commands::reveal_in_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Compilar + rodar testes**

Run: `cd src-tauri && cargo build 2>&1 | tail -10 && cargo test 2>&1 | tail -15`
Expected: build `Finished`; testes unitários (Tasks 2–7) `ok`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add RecordingCoordinator and Tauri commands"
```

---

## Task 11: API tipada + util de formatação (UI lib) — vitest

**Files:**
- Create: `src/lib/api.ts`, `src/lib/format.ts`, `src/lib/format.test.ts`
- Modify: `package.json` (script de teste), criar `vitest.config.ts`

**Interfaces:**
- Consumes: comandos Tauri (Task 10).
- Produces:
  - `src/lib/api.ts`: wrappers `listSources()`, `listMicrophones()`, `startRecording(source, micId)`, `stopRecording()`, `revealInFolder(path)` + tipos TS (`SourceOption`, `MicOption`, `RecordingResult`).
  - `src/lib/format.ts`: `formatElapsed(ms: number): string` (→ `"MM:SS"`), `fileName(path: string): string`.

- [ ] **Step 1: Adicionar vitest**

Run: `pnpm add -D vitest` e adicionar em `package.json` scripts: `"test": "vitest run"`.
Criar `vitest.config.ts`:
```ts
import { defineConfig } from "vitest/config";
export default defineConfig({ test: { environment: "node" } });
```

- [ ] **Step 2: Escrever testes falhando (`src/lib/format.test.ts`)**

```ts
import { describe, it, expect } from "vitest";
import { formatElapsed, fileName } from "./format";

describe("formatElapsed", () => {
  it("formats milliseconds as MM:SS", () => {
    expect(formatElapsed(0)).toBe("00:00");
    expect(formatElapsed(65000)).toBe("01:05");
    expect(formatElapsed(3599000)).toBe("59:59");
  });
});

describe("fileName", () => {
  it("extracts the last path segment", () => {
    expect(fileName("/Users/x/Movies/OpenRecorder/REC-1.mp4")).toBe("REC-1.mp4");
    expect(fileName("C:\\\\Videos\\\\REC-2.mp4")).toBe("REC-2.mp4");
  });
});
```

- [ ] **Step 3: Rodar pra ver falhar**

Run: `pnpm test 2>&1 | tail -15`
Expected: falha — `format.ts` não existe.

- [ ] **Step 4: Implementar `format.ts` e `api.ts`**

`src/lib/format.ts`:
```ts
export function formatElapsed(ms: number): string {
  const totalSec = Math.floor(ms / 1000);
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

export function fileName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] ?? path;
}
```

`src/lib/api.ts`:
```ts
import { invoke } from "@tauri-apps/api/core";

export interface SourceOption { id: string; name: string; kind: string; rect: [number, number, number, number]; }
export interface MicOption { id: string; name: string; }
export interface SourcesPayload { displays: SourceOption[]; windows: SourceOption[]; }
export interface RecordingResult { video_path: string; metadata_path: string; duration_ms: number; }

export const listSources = () => invoke<SourcesPayload>("list_sources");
export const listMicrophones = () => invoke<MicOption[]>("list_microphones");
export const startRecording = (source: SourceOption, micId: string | null) =>
  invoke<void>("start_recording", { source, micId });
export const stopRecording = () => invoke<RecordingResult>("stop_recording");
export const revealInFolder = (path: string) => invoke<void>("reveal_in_folder", { path });
```

- [ ] **Step 5: Rodar pra ver passar**

Run: `pnpm test 2>&1 | tail -10`
Expected: `Test Files 1 passed`, 2 testes ok.

- [ ] **Step 6: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add typed Tauri API wrappers and format utils with vitest"
```

---

## Task 12: UI React (picker, controles, lista) + smoke fim-a-fim

**Files:**
- Create: `src/state/useRecorder.ts`, `src/components/SourcePicker.tsx`, `src/components/MicPicker.tsx`, `src/components/RecordControls.tsx`, `src/components/RecordingsList.tsx`
- Modify: `src/App.tsx`, `src/App.css`

**Interfaces:**
- Consumes: `src/lib/api.ts`, `src/lib/format.ts` (Task 11).
- Produces: app utilizável fim-a-fim.

- [ ] **Step 1: Implementar `useRecorder.ts`**

```ts
import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "../lib/api";

export function useRecorder() {
  const [displays, setDisplays] = useState<api.SourceOption[]>([]);
  const [windows, setWindows] = useState<api.SourceOption[]>([]);
  const [mics, setMics] = useState<api.MicOption[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedMic, setSelectedMic] = useState<string | null>(null);
  const [isRecording, setRecording] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [recordings, setRecordings] = useState<api.RecordingResult[]>([]);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<number | null>(null);
  const startedAt = useRef<number>(0);

  const refresh = useCallback(async () => {
    try {
      const s = await api.listSources();
      setDisplays(s.displays); setWindows(s.windows);
      const m = await api.listMicrophones();
      setMics(m);
      if (!selectedId && s.displays[0]) setSelectedId(s.displays[0].id);
      if (!selectedMic && m[0]) setSelectedMic(m[0].id);
    } catch (e) { setError(String(e)); }
  }, [selectedId, selectedMic]);

  useEffect(() => { refresh(); }, [refresh]);

  const allSources = [...displays, ...windows];
  const selected = allSources.find((x) => x.id === selectedId) ?? null;

  const start = useCallback(async () => {
    if (!selected) return;
    try {
      await api.startRecording(selected, selectedMic);
      setRecording(true);
      startedAt.current = Date.now();
      timer.current = window.setInterval(() => setElapsed(Date.now() - startedAt.current), 100);
    } catch (e) { setError(String(e)); }
  }, [selected, selectedMic]);

  const stop = useCallback(async () => {
    try {
      const res = await api.stopRecording();
      setRecordings((r) => [res, ...r]);
    } catch (e) { setError(String(e)); }
    setRecording(false);
    if (timer.current) { clearInterval(timer.current); timer.current = null; }
    setElapsed(0);
  }, []);

  return { displays, windows, mics, selectedId, setSelectedId, selectedMic,
           setSelectedMic, isRecording, elapsed, recordings, error, start, stop };
}
```

- [ ] **Step 2: Implementar os componentes**

`src/components/SourcePicker.tsx`:
```tsx
import type { SourceOption } from "../lib/api";

export function SourcePicker(props: {
  displays: SourceOption[]; windows: SourceOption[];
  value: string | null; onChange: (id: string) => void;
}) {
  return (
    <label className="field">
      <span>Fonte</span>
      <select value={props.value ?? ""} onChange={(e) => props.onChange(e.target.value)}>
        <optgroup label="Telas">
          {props.displays.map((d) => <option key={d.id} value={d.id}>{d.name}</option>)}
        </optgroup>
        <optgroup label="Janelas">
          {props.windows.map((w) => <option key={w.id} value={w.id}>{w.name}</option>)}
        </optgroup>
      </select>
    </label>
  );
}
```

`src/components/MicPicker.tsx`:
```tsx
import type { MicOption } from "../lib/api";

export function MicPicker(props: {
  mics: MicOption[]; value: string | null; onChange: (id: string) => void;
}) {
  return (
    <label className="field">
      <span>Microfone</span>
      <select value={props.value ?? ""} onChange={(e) => props.onChange(e.target.value)}>
        {props.mics.map((m) => <option key={m.id} value={m.id}>{m.name}</option>)}
      </select>
    </label>
  );
}
```

`src/components/RecordControls.tsx`:
```tsx
import { formatElapsed } from "../lib/format";

export function RecordControls(props: {
  isRecording: boolean; elapsed: number; disabled: boolean;
  onStart: () => void; onStop: () => void;
}) {
  return (
    <div className="controls">
      <button
        className={props.isRecording ? "btn stop" : "btn record"}
        disabled={props.disabled && !props.isRecording}
        onClick={props.isRecording ? props.onStop : props.onStart}>
        {props.isRecording ? "Parar" : "Gravar"}
      </button>
      {props.isRecording && <span className="timer">{formatElapsed(props.elapsed)}</span>}
    </div>
  );
}
```

`src/components/RecordingsList.tsx`:
```tsx
import type { RecordingResult } from "../lib/api";
import { fileName, formatElapsed } from "../lib/format";
import { revealInFolder } from "../lib/api";

export function RecordingsList(props: { items: RecordingResult[] }) {
  if (props.items.length === 0) return <p className="muted">Nenhuma gravação ainda.</p>;
  return (
    <ul className="recordings">
      {props.items.map((r) => (
        <li key={r.video_path}>
          <span>{fileName(r.video_path)}</span>
          <span className="muted">{formatElapsed(r.duration_ms)}</span>
          <button className="btn small" onClick={() => revealInFolder(r.video_path)}>Mostrar</button>
        </li>
      ))}
    </ul>
  );
}
```

- [ ] **Step 3: Reescrever `App.tsx`**

```tsx
import { useRecorder } from "./state/useRecorder";
import { SourcePicker } from "./components/SourcePicker";
import { MicPicker } from "./components/MicPicker";
import { RecordControls } from "./components/RecordControls";
import { RecordingsList } from "./components/RecordingsList";
import "./App.css";

export default function App() {
  const r = useRecorder();
  return (
    <main className="app">
      <h1>● OpenRecorder</h1>
      <SourcePicker displays={r.displays} windows={r.windows}
                    value={r.selectedId} onChange={r.setSelectedId} />
      <MicPicker mics={r.mics} value={r.selectedMic} onChange={r.setSelectedMic} />
      <RecordControls isRecording={r.isRecording} elapsed={r.elapsed}
                      disabled={!r.selectedId} onStart={r.start} onStop={r.stop} />
      <hr />
      <h2>Gravações</h2>
      <RecordingsList items={r.recordings} />
      {r.error && <p className="error">{r.error}</p>}
    </main>
  );
}
```

- [ ] **Step 4: Estilo mínimo em `App.css`**

Substituir o conteúdo de `src/App.css` por estilos básicos limpos (container centrado, campos, botão record vermelho / stop cinza, `.muted`, `.error` vermelho, `.timer` monoespaçado). Manter enxuto.

- [ ] **Step 5: Build + testes**

Run: `pnpm build 2>&1 | tail -5 && pnpm test 2>&1 | tail -5 && cd src-tauri && cargo test 2>&1 | tail -5`
Expected: front buildou; vitest ok; cargo testes ok.

- [ ] **Step 6: Smoke manual fim-a-fim (precisa de você)**

Run: `pnpm tauri dev` (abre a janela do app).
Conceder permissões quando o SO pedir (macOS: Gravação de Tela + Microfone + Monitoramento de Entrada; reabrir app após conceder).
Verificar: lista telas/mic → Gravar → clicar pela tela ~10s → Parar → gravação aparece na lista → "Mostrar" abre a pasta → `.mp4` reproduz com vídeo+áudio → `.metadata.json` tem `version:1` + `events` com cliques.

- [ ] **Step 7: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add React UI and recorder state hook for end-to-end recording"
```

---

## Task 13: Smoke checklist + README

**Files:**
- Create: `docs/SMOKE-TEST.md`, `README.md`

**Interfaces:**
- Consumes: tudo.
- Produces: checklist manual + documentação.

- [ ] **Step 1: Escrever `docs/SMOKE-TEST.md`**

Checklist de verificação manual por SO (gravar tela inteira, janela; mic on/off; conferir `.mp4` e `.metadata.json`; sem permissão de input → events vazio degrada; regravar várias vezes sem travar). Itens em checkbox.

- [ ] **Step 2: Escrever `README.md`**

Descrição (gravador open-source cross-platform, zoom-no-clique + 9:16), status F1, requisitos (Rust, Node/pnpm, ffmpeg no PATH), comandos (`pnpm install`, `pnpm tauri dev`, `pnpm test`, `cargo test`), permissões por SO, roadmap F2–F4, licença MIT (a definir).

- [ ] **Step 3: Executar o smoke (`docs/SMOKE-TEST.md`)**

Seguir o checklist; corrigir o que falhar (voltando à task correspondente).

- [ ] **Step 4: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "docs: add smoke test checklist and README"
```

---

## Self-Review (autor do plano)

**1. Cobertura do spec:**
- Captura tela/janela/região → Tasks 8, 9 (source_enum, video_capture; região via rect).
- Microfone → Task 9 (audio_capture/cpal).
- Metadata cliques/mouse → Tasks 2, 5, 6 (modelo, input buffer, finalizer).
- Não-destrutiva (mp4 + metadata.json) → Tasks 4/7/9 (encode/ffmpeg), 6 (metadata), 10 (coordenação).
- Comandos Tauri / fluxo UI ↔ core → Tasks 10, 11, 12.
- Permissões → checagens em `scap`/`ensure_ffmpeg` + avisos UI (Tasks 7, 8, 12).
- Tratamento de erros → `Result<_, String>` em todos os comandos, degradação sem input/mic (Tasks 9, 10, 12).
- Testes → unit (`cargo test`) Tasks 2–7; vitest Task 11; smoke manual Tasks 8, 9, 12, 13.
- Sem gap para o escopo F1.

**2. Placeholders:** sem "TBD". Tasks de integração (8, 9) trazem código-esboço concreto com instrução explícita de ajustar à API real do crate `scap`/`cpal` e reportar DONE_WITH_CONCERNS se divergir — risco conhecido, não placeholder.

**3. Consistência de tipos:** `RecordingMetadata`/`InputEvent` (campo `kind`→`"type"`), `CaptureSource`/`SourceKind.as_str`, `map_to_source`, `SourceOption`, `RecordingResult`, `make_filenames`, comandos Tauri ↔ `api.ts` (snake_case nos campos do payload). Nomes batem entre produtores/consumidores.

**Riscos conhecidos (documentados):**
- API de `scap`/`cpal` pode divergir do esboço (Tasks 8, 9) — implementer verifica no crate real.
- `rdev::listen` é global/bloqueante; se a integração travar, entregar vídeo+áudio e degradar input (events vazio) com DONE_WITH_CONCERNS.
- Sincronização fina vídeo/áudio: muxer junta por timestamp do container; pequeno drift aceitável em F1.
- ffmpeg via PATH no F1 (sidecar empacotado fica para fase de distribuição).
```
