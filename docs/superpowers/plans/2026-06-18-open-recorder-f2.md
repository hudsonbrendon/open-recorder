# OpenRecorder F2 (Auto-zoom no Clique) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Gerar zoom automático nos cliques do mouse, com editor (timeline + preview ao vivo) e export landscape com o zoom "assado" no vídeo.

**Architecture:** Sobre as gravações da F1 (`REC-<ts>.mp4` + `REC-<ts>.metadata.json`). Um `ZoomModel` (segmentos de zoom) é gerado dos cliques no Rust; o front edita e mostra preview ao vivo aplicando `transform` CSS guiado por um sampler `zoom_at`; o export usa ffmpeg (`zoompan`) com o MESMO modelo. Sampler vive em Rust e TS com paridade testada. Captura de clique real via CGEventTap nativo (mac).

**Tech Stack:** Tauri 2, Rust (serde, core-graphics), React + Vite + TS, ffmpeg (zoompan). Testes: `cargo test` + `vitest`.

## Global Constraints

- Plataforma alvo testada: **macOS**; código mantém intenção cross-platform.
- Stack: **Tauri 2 + Rust + React/TS** (mesma da F1).
- **Não-destrutivo:** F2 consome `REC-<ts>.mp4` + `REC-<ts>.metadata.json`; nunca regrava. Edições em `REC-<ts>.zoom.json` (versionado, `version: 1`).
- Export F2: **landscape** (resolução original). 9:16 é F4.
- **Sampler único:** `zoom_at` (easing `smoothstep`, clamp) replicado em Rust e TS — **paridade testada** com fixture compartilhada.
- Defaults de geração: `scale 2.0`, `ease_in 300ms`, `hold 1500ms`, `ease_out 400ms`.
- Coordenadas de `target` normalizadas 0..1 no retângulo da fonte.
- Commits: **sem co-author/histórico do Claude**; inglês `tipo: descrição`.
- Crate Rust `open_recorder_lib`; ffmpeg resolvido por `ffmpeg::ffmpeg_binary()` (já existe na F1, resolve caminho absoluto).
- Branch `f2-auto-zoom` (saiu do tip de `f1-capture-foundation`).

## Estado herdado da F1 (NÃO refazer)

- App Tauri+React compila; `cargo test` 16 + `vitest` 2 verdes.
- `metadata.json`: `RecordingMetadata { version, recording{width,height,fps,duration_ms}, source{type,id,rect:[i64;4]}, events: Vec<InputEvent> }`; `InputEvent { t_ms, kind, x, y, button }` (kind `"click"`/`"move"`).
- Captura: `VideoCapture`, `AudioCapture`, `RecordingCoordinator`, comandos `list_sources/list_microphones/start_recording/stop_recording/reveal_in_folder`.
- `ffmpeg.rs`: `ffmpeg_binary()` (caminho absoluto), `ensure_ffmpeg()`, `encode_args`, `mux_args`.
- `coordinator.rs`: o listener `rdev` está **desligado** (crash no macOS); `spawn_rdev_listener` existe com `#[allow(dead_code)]`. F2 substitui isso por captura nativa.
- Front: `src/lib/api.ts`, `src/lib/format.ts`(+test), `src/state/useRecorder.ts`, componentes da lista/controles.

## File Structure (F2)

```
src-tauri/src/
├── capture/
│   ├── input_mac.rs        # NOVO: CGEventTap nativo de mouse (macOS)
│   └── input.rs            # NOVO: dispatch por cfg (mac->nativo, outros->rdev) + InputMsg
├── model/
│   └── zoom.rs             # NOVO: ZoomModel/ZoomSegment/ZoomTarget, smoothstep, zoom_at, clamp
├── zoom/
│   ├── mod.rs              # NOVO
│   ├── generate.rs         # NOVO: events -> ZoomModel (merge)
│   ├── store.rs            # NOVO: ler/gravar REC-<ts>.zoom.json + nomes
│   └── export.rs           # NOVO: builder do filtro zoompan + export com progresso
├── recording/coordinator.rs # MOD: usar capture::input em vez do rdev desligado
└── commands.rs             # MOD: load_recording, save_zoom, export_with_zoom

src/
├── lib/
│   ├── zoom.ts             # NOVO: espelho de zoom_at/smoothstep/clamp (paridade)
│   ├── zoom.test.ts        # NOVO: vitest + fixture de paridade
│   └── api.ts              # MOD: tipos ZoomModel + wrappers load/save/export
├── state/useEditor.ts      # NOVO
└── components/
    ├── EditorView.tsx       # NOVO
    ├── PreviewCanvas.tsx    # NOVO
    ├── Timeline.tsx         # NOVO
    └── SegmentInspector.tsx # NOVO
docs/SMOKE-TEST-F2.md        # NOVO
```

---

## Task 1: Captura de clique nativa (macOS CGEventTap) + dispatch por cfg

Substitui o `rdev` desligado. macOS usa um CGEventTap só de mouse (sem teclado → sem o crash). Outros SOs mantêm `rdev`. Integração — sem unit test; verificação = gravar e ver `events` preenchido.

**Files:**
- Create: `src-tauri/src/capture/input.rs`, `src-tauri/src/capture/input_mac.rs`
- Modify: `src-tauri/src/capture/mod.rs` (declarar `input`, `input_mac`), `src-tauri/src/recording/coordinator.rs`

**Interfaces:**
- Consumes: `InputRecorder` (F1, `ingest(x,y,kind,button,now_ms)`).
- Produces:
  - `capture::input::InputMsg = (i64, i64, String, Option<String>, u64)` (x, y, kind, button, now_ms).
  - `capture::input::InputListener` com:
    - `fn start() -> InputListener` — inicia o listener nativo (uma vez por processo), gateado por flag.
    - `fn set_recording(&self, on: bool)`
    - `fn drain(&self, rec: &mut InputRecorder)` — escoa eventos pendentes p/ o recorder.

- [ ] **Step 1: Implementar o CGEventTap nativo (`input_mac.rs`)**

Usa o crate `core-graphics` (já transitivo via scap; adicionar explícito). Listener mouse-only numa thread com CFRunLoop.

Adicionar ao `src-tauri/Cargo.toml` `[dependencies]` (verifique a versão atual no docs.rs; use a 0.x compatível com o que o scap já traz):
```toml
[target.'cfg(target_os = "macos")'.dependencies]
core-graphics = "0.24"
core-foundation = "0.10"
```

`src-tauri/src/capture/input_mac.rs`:
```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use core_graphics::event::{
    CGEvent, CGEventTap, CGEventTapLocation, CGEventTapPlacement, CGEventTapOptions,
    CGEventType, CGEventField,
};
use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
use crate::capture::input::InputMsg;

pub fn spawn(recording: Arc<AtomicBool>, tx: Sender<InputMsg>) {
    std::thread::spawn(move || {
        let events = vec![
            CGEventType::LeftMouseDown,
            CGEventType::RightMouseDown,
            CGEventType::MouseMoved,
            CGEventType::LeftMouseDragged,
        ];
        let tap = CGEventTap::new(
            CGEventTapLocation::Session,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            events,
            |_proxy, etype, event: &CGEvent| {
                if recording.load(Ordering::Relaxed) {
                    let loc = event.location();
                    let now = crate::recording::coordinator::now_ms_pub();
                    let msg: Option<InputMsg> = match etype {
                        CGEventType::LeftMouseDown =>
                            Some((loc.x as i64, loc.y as i64, "click".into(), Some("left".into()), now)),
                        CGEventType::RightMouseDown =>
                            Some((loc.x as i64, loc.y as i64, "click".into(), Some("right".into()), now)),
                        CGEventType::MouseMoved | CGEventType::LeftMouseDragged =>
                            Some((loc.x as i64, loc.y as i64, "move".into(), None, now)),
                        _ => None,
                    };
                    if let Some(m) = msg { let _ = tx.send(m); }
                }
                None // ListenOnly: don't modify the event
            },
        );
        if let Ok(tap) = tap {
            let loop_source = tap.mach_port.create_runloop_source(0)
                .expect("runloop source");
            let current = CFRunLoop::get_current();
            unsafe { current.add_source(&loop_source, kCFRunLoopCommonModes); }
            tap.enable();
            CFRunLoop::run_current(); // blocks this thread
        }
    });
}
```

> A API exata do `core-graphics` (assinatura de `CGEventTap::new`, retorno do
> callback, `event.location()`) pode variar por versão — **valide no crate real**
> (docs.rs / fonte em ~/.cargo) e ajuste. `event.location()` retorna `CGPoint`
> em coords globais (origem sup-esq do display principal), que é o que o
> `InputRecorder` espera (mesma origem do `metadata`). Reporte DONE_WITH_CONCERNS
> se a API divergir do esboço.

- [ ] **Step 2: Implementar o dispatch (`input.rs`)**

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use crate::capture::input_recorder::InputRecorder;

pub type InputMsg = (i64, i64, String, Option<String>, u64);

pub struct InputListener {
    recording: Arc<AtomicBool>,
    rx: Receiver<InputMsg>,
}

impl InputListener {
    pub fn start() -> InputListener {
        let recording = Arc::new(AtomicBool::new(false));
        let (tx, rx): (Sender<InputMsg>, Receiver<InputMsg>) = mpsc::channel();
        #[cfg(target_os = "macos")]
        crate::capture::input_mac::spawn(recording.clone(), tx);
        #[cfg(not(target_os = "macos"))]
        spawn_rdev(recording.clone(), tx);
        InputListener { recording, rx }
    }

    pub fn set_recording(&self, on: bool) {
        self.recording.store(on, Ordering::Relaxed);
        if on { while self.rx.try_recv().is_ok() {} } // drop stale
    }

    pub fn drain(&self, rec: &mut InputRecorder) {
        while let Ok((x, y, kind, button, now_ms)) = self.rx.try_recv() {
            rec.ingest(x, y, &kind, button, now_ms);
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn spawn_rdev(recording: Arc<AtomicBool>, tx: Sender<InputMsg>) {
    std::thread::spawn(move || {
        let _ = rdev::listen(move |event: rdev::Event| {
            if !recording.load(Ordering::Relaxed) { return; }
            let now = crate::recording::coordinator::now_ms_pub();
            match event.event_type {
                rdev::EventType::MouseMove { x, y } => {
                    let _ = tx.send((x as i64, y as i64, "move".into(), None, now));
                }
                rdev::EventType::ButtonPress(btn) => {
                    let label = match btn {
                        rdev::Button::Left => "left", rdev::Button::Right => "right",
                        rdev::Button::Middle => "middle", rdev::Button::Unknown(_) => "unknown",
                    };
                    let _ = tx.send((0, 0, "click".into(), Some(label.into()), now));
                }
                _ => {}
            }
        });
    });
}
```

- [ ] **Step 3: Expor `now_ms_pub` e declarar módulos**

Em `src-tauri/src/capture/mod.rs` adicionar:
```rust
pub mod input;
#[cfg(target_os = "macos")]
pub mod input_mac;
```
Em `src-tauri/src/recording/coordinator.rs`, tornar o helper de tempo público (ele já existe como `fn now_ms()`); adicionar ao lado:
```rust
pub fn now_ms_pub() -> u64 { now_ms() }
```

- [ ] **Step 4: Ligar no coordinator**

Em `coordinator.rs`: adicionar campo `input: Option<crate::capture::input::InputListener>` ao `Coordinator`. Remover o bloco morto do rdev (o `spawn_rdev_listener` com `#[allow(dead_code)]`, `RdevHandle`, `InputMsg`, `drain_rdev`, campo `rdev`). No `start()`, no ponto onde antes estava o rdev:
```rust
        // Native input capture (mouse-only on macOS; rdev elsewhere).
        if self.input.is_none() {
            self.input = Some(crate::capture::input::InputListener::start());
        }
        if let Some(ref l) = self.input {
            l.set_recording(true);
        }
```
E manter o `let mut input = InputRecorder::new(source.rect, start_ms);` guardado no `Active`. No `stop()`, antes de `take_events`:
```rust
        if let Some(ref l) = self.input {
            l.set_recording(false);
            l.drain(&mut a.input);
        }
```
(`a.input` é o `InputRecorder` do `Active`.)

- [ ] **Step 5: Compilar**

Run: `cd src-tauri && cargo build 2>&1 | tail -15`
Expected: `Finished` (baixa core-graphics/core-foundation explícitos). Corrigir à API real do crate se divergir.

- [ ] **Step 6: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: native macOS mouse event tap for click capture"
```

---

## Task 2: Modelo de zoom + smoothstep (Rust)

**Files:**
- Create: `src-tauri/src/model/zoom.rs`
- Modify: `src-tauri/src/model/mod.rs` (add `pub mod zoom;`)

**Interfaces:**
- Produces:
  - `ZoomTarget { t_ms: u64, x: f64, y: f64 }`
  - `ZoomSegment { start_ms: u64, end_ms: u64, ease_in_ms: u64, ease_out_ms: u64, scale: f64, targets: Vec<ZoomTarget> }`
  - `ZoomModel { version: u32, segments: Vec<ZoomSegment> }`
  - Todos `#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]`.
  - `fn smoothstep(p: f64) -> f64` (clamp 0..1 + cúbica).

- [ ] **Step 1: Escrever testes falhando**

Em `src-tauri/src/model/zoom.rs`:
```rust
use serde::{Serialize, Deserialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothstep_endpoints_and_mid() {
        assert!((smoothstep(0.0) - 0.0).abs() < 1e-9);
        assert!((smoothstep(1.0) - 1.0).abs() < 1e-9);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-9);
        assert!((smoothstep(-3.0) - 0.0).abs() < 1e-9); // clamp
        assert!((smoothstep(7.0) - 1.0).abs() < 1e-9);  // clamp
    }

    #[test]
    fn model_round_trips() {
        let m = ZoomModel {
            version: 1,
            segments: vec![ZoomSegment {
                start_ms: 0, end_ms: 2000, ease_in_ms: 300, ease_out_ms: 400,
                scale: 2.0, targets: vec![ZoomTarget { t_ms: 0, x: 0.25, y: 0.75 }],
            }],
        };
        let j = serde_json::to_string(&m).unwrap();
        let back: ZoomModel = serde_json::from_str(&j).unwrap();
        assert_eq!(m, back);
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test zoom 2>&1 | tail -10`
Expected: `cannot find function smoothstep` / tipos.

- [ ] **Step 3: Implementar**

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ZoomTarget { pub t_ms: u64, pub x: f64, pub y: f64 }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ZoomSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub ease_in_ms: u64,
    pub ease_out_ms: u64,
    pub scale: f64,
    pub targets: Vec<ZoomTarget>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ZoomModel { pub version: u32, pub segments: Vec<ZoomSegment> }

/// Cubic smoothstep with input clamped to [0,1].
pub fn smoothstep(p: f64) -> f64 {
    let p = p.clamp(0.0, 1.0);
    p * p * (3.0 - 2.0 * p)
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run: `cd src-tauri && cargo test zoom 2>&1 | tail -10`
Expected: `ok. 2 passed`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add ZoomModel types and smoothstep"
```

---

## Task 3: Sampler `zoom_at` (Rust)

**Files:**
- Modify: `src-tauri/src/model/zoom.rs`

**Interfaces:**
- Consumes: `ZoomModel`, `ZoomSegment`, `ZoomTarget`, `smoothstep` (Task 2).
- Produces:
  - `ZoomAt { scale: f64, cx: f64, cy: f64 }` (`#[derive(Debug, Clone, Copy, PartialEq)]`).
  - `fn zoom_at(model: &ZoomModel, t_ms: u64) -> ZoomAt`.
  - Semântica (verbatim — espelhada no TS):
    - Fora de qualquer segmento (`t_ms < start || t_ms >= end`): `{ 1.0, 0.5, 0.5 }`.
    - `rel = t - start`, `dur = end - start`.
    - envelope `e`: se `rel < ease_in` → `smoothstep(rel/ease_in)`; senão se `rel > dur - ease_out` → `smoothstep((dur - rel)/ease_out)`; senão `1.0`. Se `ease_in + ease_out > dur`, usar o MENOR dos dois ramos (proteção).
    - `scale_t = 1 + (segment.scale - 1) * e`.
    - alvo `(tx,ty)`: se 1 target → seus x,y; senão clamp `t` a `[first.t_ms, last.t_ms]` e interpolar linearmente entre os dois targets que o cercam.
    - clamp do centro: `m = 0.5 / scale_t`; `cx = clamp(tx, m, 1-m)`; `cy = clamp(ty, m, 1-m)`.

- [ ] **Step 1: Escrever testes falhando (valores concretos = fixture de paridade)**

Adicionar ao `mod tests` de `zoom.rs`:
```rust
    fn fixture() -> ZoomModel {
        ZoomModel { version: 1, segments: vec![ZoomSegment {
            start_ms: 0, end_ms: 2000, ease_in_ms: 300, ease_out_ms: 400,
            scale: 2.0, targets: vec![ZoomTarget { t_ms: 0, x: 0.25, y: 0.75 }],
        }]}
    }
    fn close(a: f64, b: f64) -> bool { (a - b).abs() < 1e-6 }

    #[test]
    fn zoom_at_outside_is_identity() {
        let z = zoom_at(&fixture(), 2500);
        assert!(close(z.scale, 1.0) && close(z.cx, 0.5) && close(z.cy, 0.5));
    }
    #[test]
    fn zoom_at_ease_in_mid() {
        let z = zoom_at(&fixture(), 150); // smoothstep(0.5)=0.5 -> scale 1.5
        assert!(close(z.scale, 1.5), "{}", z.scale);
        assert!(close(z.cx, 1.0/3.0), "{}", z.cx); // clamp(0.25, 0.333..,0.666..)
        assert!(close(z.cy, 2.0/3.0), "{}", z.cy); // clamp(0.75, ...)
    }
    #[test]
    fn zoom_at_plateau_full_scale() {
        let z = zoom_at(&fixture(), 1000);
        assert!(close(z.scale, 2.0));
        assert!(close(z.cx, 0.25) && close(z.cy, 0.75)); // m=0.25
    }
    #[test]
    fn zoom_at_ease_out_mid() {
        let z = zoom_at(&fixture(), 1800); // (2000-1800)/400=0.5 -> 0.5 -> scale 1.5
        assert!(close(z.scale, 1.5));
    }
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test zoom_at 2>&1 | tail -10`
Expected: `cannot find function zoom_at`.

- [ ] **Step 3: Implementar**

Acima do `mod tests`:
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomAt { pub scale: f64, pub cx: f64, pub cy: f64 }

fn target_at(seg: &ZoomSegment, t_ms: u64) -> (f64, f64) {
    let ts = &seg.targets;
    if ts.len() == 1 { return (ts[0].x, ts[0].y); }
    let first = ts[0].t_ms;
    let last = ts[ts.len() - 1].t_ms;
    let t = t_ms.clamp(first, last);
    for w in ts.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if t >= a.t_ms && t <= b.t_ms {
            let span = (b.t_ms - a.t_ms).max(1) as f64;
            let f = (t - a.t_ms) as f64 / span;
            return (a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f);
        }
    }
    (ts[ts.len() - 1].x, ts[ts.len() - 1].y)
}

pub fn zoom_at(model: &ZoomModel, t_ms: u64) -> ZoomAt {
    for seg in &model.segments {
        if t_ms < seg.start_ms || t_ms >= seg.end_ms { continue; }
        let rel = (t_ms - seg.start_ms) as f64;
        let dur = (seg.end_ms - seg.start_ms) as f64;
        let ein = seg.ease_in_ms as f64;
        let eout = seg.ease_out_ms as f64;
        let e_in = if ein > 0.0 { smoothstep(rel / ein) } else { 1.0 };
        let e_out = if eout > 0.0 { smoothstep((dur - rel) / eout) } else { 1.0 };
        let e = if rel < ein && rel > dur - eout {
            e_in.min(e_out) // overlapping ramps: take the smaller
        } else if rel < ein {
            e_in
        } else if rel > dur - eout {
            e_out
        } else {
            1.0
        };
        let scale_t = 1.0 + (seg.scale - 1.0) * e;
        let (tx, ty) = target_at(seg, t_ms);
        let m = 0.5 / scale_t;
        let cx = tx.clamp(m, 1.0 - m);
        let cy = ty.clamp(m, 1.0 - m);
        return ZoomAt { scale: scale_t, cx, cy };
    }
    ZoomAt { scale: 1.0, cx: 0.5, cy: 0.5 }
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run: `cd src-tauri && cargo test zoom 2>&1 | tail -10`
Expected: todos os testes de zoom `ok`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add zoom_at sampler with easing and center clamp"
```

---

## Task 4: Geração de zoom a partir dos cliques (Rust)

**Files:**
- Create: `src-tauri/src/zoom/mod.rs`, `src-tauri/src/zoom/generate.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod zoom;`)

**Interfaces:**
- Consumes: `InputEvent` (model::metadata), `ZoomModel`/`ZoomSegment`/`ZoomTarget` (Task 2).
- Produces:
  - `GenOpts { scale: f64, ease_in_ms: u64, hold_ms: u64, ease_out_ms: u64 }` + `impl Default` (2.0/300/1500/400).
  - `fn generate(events: &[InputEvent], source_rect: [i64;4], opts: &GenOpts) -> ZoomModel`.
  - Regras: só eventos `kind == "click"`; normaliza `x/width`, `y/height` (`width=rect[2]`,`height=rect[3]`). `start_ms = primeiro clique`; `end_ms = último clique + hold + ease_out`. **Merge:** novo clique funde se `t <= last_click_t + hold_ms + ease_out_ms`; senão abre novo segmento. `ease_in/out/scale` dos opts.

- [ ] **Step 1: Escrever testes falhando**

`src-tauri/src/zoom/mod.rs`:
```rust
pub mod generate;
pub mod store;
pub mod export;
```
(crie `store.rs`/`export.rs` como placeholders `// later` para compilar; preenchidos nas Tasks 6/7.)

`src-tauri/src/zoom/generate.rs`:
```rust
use crate::model::metadata::InputEvent;
use crate::model::zoom::{ZoomModel, ZoomSegment, ZoomTarget};

#[cfg(test)]
mod tests {
    use super::*;
    fn click(t: u64, x: i64, y: i64) -> InputEvent {
        InputEvent { t_ms: t, kind: "click".into(), x, y, button: Some("left".into()) }
    }

    #[test]
    fn single_click_one_segment() {
        let evs = vec![click(500, 250, 750)];
        let m = generate(&evs, [0, 0, 1000, 1000], &GenOpts::default());
        assert_eq!(m.segments.len(), 1);
        let s = &m.segments[0];
        assert_eq!(s.start_ms, 500);
        assert_eq!(s.end_ms, 500 + 1500 + 400);
        assert_eq!(s.targets.len(), 1);
        assert!((s.targets[0].x - 0.25).abs() < 1e-9);
        assert!((s.targets[0].y - 0.75).abs() < 1e-9);
    }

    #[test]
    fn nearby_clicks_merge() {
        let evs = vec![click(500, 250, 750), click(1000, 500, 500)];
        let m = generate(&evs, [0, 0, 1000, 1000], &GenOpts::default());
        assert_eq!(m.segments.len(), 1);
        let s = &m.segments[0];
        assert_eq!(s.start_ms, 500);
        assert_eq!(s.end_ms, 1000 + 1500 + 400);
        assert_eq!(s.targets.len(), 2);
    }

    #[test]
    fn distant_clicks_separate() {
        let evs = vec![click(500, 250, 750), click(5000, 500, 500)];
        let m = generate(&evs, [0, 0, 1000, 1000], &GenOpts::default());
        assert_eq!(m.segments.len(), 2);
    }

    #[test]
    fn moves_ignored() {
        let mut evs = vec![InputEvent { t_ms: 100, kind: "move".into(), x: 1, y: 2, button: None }];
        evs.push(click(500, 250, 750));
        let m = generate(&evs, [0, 0, 1000, 1000], &GenOpts::default());
        assert_eq!(m.segments.len(), 1);
        assert_eq!(m.segments[0].start_ms, 500);
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test generate 2>&1 | tail -10`
Expected: `cannot find function generate` / `GenOpts`.

- [ ] **Step 3: Implementar**

Acima do `mod tests`:
```rust
pub struct GenOpts { pub scale: f64, pub ease_in_ms: u64, pub hold_ms: u64, pub ease_out_ms: u64 }

impl Default for GenOpts {
    fn default() -> Self {
        GenOpts { scale: 2.0, ease_in_ms: 300, hold_ms: 1500, ease_out_ms: 400 }
    }
}

pub fn generate(events: &[InputEvent], source_rect: [i64; 4], opts: &GenOpts) -> ZoomModel {
    let w = source_rect[2].max(1) as f64;
    let h = source_rect[3].max(1) as f64;
    let merge_window = opts.hold_ms + opts.ease_out_ms;

    let mut clicks: Vec<&InputEvent> = events.iter().filter(|e| e.kind == "click").collect();
    clicks.sort_by_key(|e| e.t_ms);

    let mut segments: Vec<ZoomSegment> = Vec::new();
    let mut last_click_t: Option<u64> = None;

    for ev in clicks {
        let nx = (ev.x as f64 / w).clamp(0.0, 1.0);
        let ny = (ev.y as f64 / h).clamp(0.0, 1.0);
        let target = ZoomTarget { t_ms: ev.t_ms, x: nx, y: ny };

        let merge = matches!(last_click_t, Some(lt) if ev.t_ms <= lt + merge_window);
        if merge {
            let seg = segments.last_mut().unwrap();
            seg.targets.push(target);
            seg.end_ms = ev.t_ms + opts.hold_ms + opts.ease_out_ms;
        } else {
            segments.push(ZoomSegment {
                start_ms: ev.t_ms,
                end_ms: ev.t_ms + opts.hold_ms + opts.ease_out_ms,
                ease_in_ms: opts.ease_in_ms,
                ease_out_ms: opts.ease_out_ms,
                scale: opts.scale,
                targets: vec![target],
            });
        }
        last_click_t = Some(ev.t_ms);
    }

    ZoomModel { version: 1, segments }
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run: `cd src-tauri && cargo test generate 2>&1 | tail -10`
Expected: `ok. 4 passed`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: generate zoom segments from click events with merge"
```

---

## Task 5: Sampler TS + paridade (vitest)

**Files:**
- Create: `src/lib/zoom.ts`, `src/lib/zoom.test.ts`

**Interfaces:**
- Produces (espelho exato da semântica do Rust):
  - tipos `ZoomTarget`, `ZoomSegment`, `ZoomModel`, `ZoomAt`.
  - `smoothstep(p): number`, `zoomAt(model, tMs): ZoomAt`.
- A fixture e os valores esperados são os MESMOS do teste Rust (Task 3) — garante paridade.

- [ ] **Step 1: Escrever testes falhando (mesma fixture do Rust)**

`src/lib/zoom.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { smoothstep, zoomAt, type ZoomModel } from "./zoom";

const fixture: ZoomModel = {
  version: 1,
  segments: [{
    start_ms: 0, end_ms: 2000, ease_in_ms: 300, ease_out_ms: 400,
    scale: 2.0, targets: [{ t_ms: 0, x: 0.25, y: 0.75 }],
  }],
};
const close = (a: number, b: number) => Math.abs(a - b) < 1e-6;

describe("smoothstep", () => {
  it("endpoints, mid, clamp", () => {
    expect(close(smoothstep(0), 0)).toBe(true);
    expect(close(smoothstep(1), 1)).toBe(true);
    expect(close(smoothstep(0.5), 0.5)).toBe(true);
    expect(close(smoothstep(-3), 0)).toBe(true);
    expect(close(smoothstep(7), 1)).toBe(true);
  });
});

describe("zoomAt parity", () => {
  it("outside -> identity", () => {
    const z = zoomAt(fixture, 2500);
    expect(close(z.scale, 1) && close(z.cx, 0.5) && close(z.cy, 0.5)).toBe(true);
  });
  it("ease-in mid", () => {
    const z = zoomAt(fixture, 150);
    expect(close(z.scale, 1.5)).toBe(true);
    expect(close(z.cx, 1 / 3)).toBe(true);
    expect(close(z.cy, 2 / 3)).toBe(true);
  });
  it("plateau", () => {
    const z = zoomAt(fixture, 1000);
    expect(close(z.scale, 2) && close(z.cx, 0.25) && close(z.cy, 0.75)).toBe(true);
  });
  it("ease-out mid", () => {
    expect(close(zoomAt(fixture, 1800).scale, 1.5)).toBe(true);
  });
});
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `pnpm test 2>&1 | tail -15`
Expected: falha — `./zoom` não existe.

- [ ] **Step 3: Implementar `src/lib/zoom.ts`**

```ts
export interface ZoomTarget { t_ms: number; x: number; y: number }
export interface ZoomSegment {
  start_ms: number; end_ms: number; ease_in_ms: number; ease_out_ms: number;
  scale: number; targets: ZoomTarget[];
}
export interface ZoomModel { version: number; segments: ZoomSegment[] }
export interface ZoomAt { scale: number; cx: number; cy: number }

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

export function smoothstep(p: number): number {
  const c = clamp(p, 0, 1);
  return c * c * (3 - 2 * c);
}

function targetAt(seg: ZoomSegment, t: number): [number, number] {
  const ts = seg.targets;
  if (ts.length === 1) return [ts[0].x, ts[0].y];
  const first = ts[0].t_ms;
  const last = ts[ts.length - 1].t_ms;
  const tc = clamp(t, first, last);
  for (let i = 0; i < ts.length - 1; i++) {
    const a = ts[i], b = ts[i + 1];
    if (tc >= a.t_ms && tc <= b.t_ms) {
      const span = Math.max(1, b.t_ms - a.t_ms);
      const f = (tc - a.t_ms) / span;
      return [a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f];
    }
  }
  return [ts[ts.length - 1].x, ts[ts.length - 1].y];
}

export function zoomAt(model: ZoomModel, tMs: number): ZoomAt {
  for (const seg of model.segments) {
    if (tMs < seg.start_ms || tMs >= seg.end_ms) continue;
    const rel = tMs - seg.start_ms;
    const dur = seg.end_ms - seg.start_ms;
    const ein = seg.ease_in_ms, eout = seg.ease_out_ms;
    const eIn = ein > 0 ? smoothstep(rel / ein) : 1;
    const eOut = eout > 0 ? smoothstep((dur - rel) / eout) : 1;
    let e: number;
    if (rel < ein && rel > dur - eout) e = Math.min(eIn, eOut);
    else if (rel < ein) e = eIn;
    else if (rel > dur - eout) e = eOut;
    else e = 1;
    const scale = 1 + (seg.scale - 1) * e;
    const [tx, ty] = targetAt(seg, tMs);
    const m = 0.5 / scale;
    return { scale, cx: clamp(tx, m, 1 - m), cy: clamp(ty, m, 1 - m) };
  }
  return { scale: 1, cx: 0.5, cy: 0.5 };
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run: `pnpm test 2>&1 | tail -10`
Expected: `Test Files 2 passed`, todos os casos de zoom ok (paridade com Rust).

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add TS zoom sampler with parity tests against Rust"
```

---

## Task 6: Persistência (`zoom.json`) + comandos load/save

**Files:**
- Modify: `src-tauri/src/zoom/store.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `ZoomModel` (Task 2), `generate`/`GenOpts` (Task 4), `RecordingMetadata` (F1).
- Produces:
  - `store::zoom_path(video_path: &str) -> PathBuf` — troca a extensão `.mp4` por `.zoom.json` (mesmo prefixo `REC-<ts>`).
  - `store::load(video_path) -> Option<ZoomModel>` / `store::save(video_path, &ZoomModel) -> Result<(), String>`.
  - Comando `load_recording(video_path) -> LoadedRecording { metadata: RecordingMetadata, zoom: ZoomModel }` — carrega `metadata.json` (mesmo prefixo) e o `zoom.json`; se não houver zoom salvo, gera de `metadata.events` com `GenOpts::default()`.
  - Comando `save_zoom(video_path, zoom: ZoomModel) -> Result<(), String>`.

- [ ] **Step 1: Escrever teste de path/round-trip**

`src-tauri/src/zoom/store.rs`:
```rust
use std::path::{Path, PathBuf};
use crate::model::zoom::ZoomModel;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::zoom::{ZoomSegment, ZoomTarget};

    #[test]
    fn zoom_path_swaps_extension() {
        let p = zoom_path("/x/REC-123.mp4");
        assert_eq!(p, PathBuf::from("/x/REC-123.zoom.json"));
    }

    #[test]
    fn save_then_load_round_trip() {
        let dir = std::env::temp_dir();
        let video = dir.join(format!("REC-test-{}.mp4", std::process::id()));
        let model = ZoomModel { version: 1, segments: vec![ZoomSegment {
            start_ms: 0, end_ms: 1000, ease_in_ms: 100, ease_out_ms: 100,
            scale: 2.0, targets: vec![ZoomTarget { t_ms: 0, x: 0.5, y: 0.5 }],
        }]};
        save(video.to_str().unwrap(), &model).unwrap();
        let loaded = load(video.to_str().unwrap()).unwrap();
        assert_eq!(loaded, model);
        let _ = std::fs::remove_file(zoom_path(video.to_str().unwrap()));
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test store 2>&1 | tail -10`
Expected: `cannot find function zoom_path`.

- [ ] **Step 3: Implementar `store.rs`**

```rust
pub fn zoom_path(video_path: &str) -> PathBuf {
    let p = Path::new(video_path);
    p.with_extension("zoom.json")
}

pub fn load(video_path: &str) -> Option<ZoomModel> {
    let path = zoom_path(video_path);
    let txt = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&txt).ok()
}

pub fn save(video_path: &str, model: &ZoomModel) -> Result<(), String> {
    let path = zoom_path(video_path);
    let json = serde_json::to_string_pretty(model).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}
```

> Nota: `Path::with_extension("zoom.json")` em `REC-123.mp4` produz
> `REC-123.zoom.json` (troca só o último componente após o ponto). Confirme no
> teste; se o comportamento divergir (ex.: nome com vários pontos), ajuste para
> remover `.mp4` e anexar `.zoom.json` manualmente.

- [ ] **Step 4: Implementar os comandos em `commands.rs`**

Adicionar:
```rust
use crate::model::metadata::RecordingMetadata;
use crate::model::zoom::ZoomModel;
use crate::zoom::{store, generate::{generate, GenOpts}};

#[derive(serde::Serialize)]
pub struct LoadedRecording {
    pub metadata: RecordingMetadata,
    pub zoom: ZoomModel,
}

fn metadata_path(video_path: &str) -> std::path::PathBuf {
    std::path::Path::new(video_path).with_extension("metadata.json")
}

#[tauri::command]
pub fn load_recording(video_path: String) -> Result<LoadedRecording, String> {
    let mtxt = std::fs::read_to_string(metadata_path(&video_path))
        .map_err(|e| format!("metadata não encontrada: {e}"))?;
    let metadata: RecordingMetadata = serde_json::from_str(&mtxt).map_err(|e| e.to_string())?;
    let zoom = store::load(&video_path)
        .unwrap_or_else(|| generate(&metadata.events, metadata.source.rect, &GenOpts::default()));
    Ok(LoadedRecording { metadata, zoom })
}

#[tauri::command]
pub fn save_zoom(video_path: String, zoom: ZoomModel) -> Result<(), String> {
    store::save(&video_path, &zoom)
}
```
> `metadata.source.rect` é `[i64;4]` (F1). `with_extension("metadata.json")` em
> `REC-123.mp4` → `REC-123.metadata.json` (mesma convenção do store).

Registrar `load_recording` e `save_zoom` no `invoke_handler` de `lib.rs`.

- [ ] **Step 5: Rodar testes + build**

Run: `cd src-tauri && cargo test store 2>&1 | tail -10 && cargo build 2>&1 | tail -5`
Expected: `store` ok; build `Finished`.

- [ ] **Step 6: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: zoom.json persistence and load_recording/save_zoom commands"
```

---

## Task 7: Export com ffmpeg (zoompan) + progresso

**Files:**
- Modify: `src-tauri/src/zoom/export.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `ZoomModel`/`ZoomSegment` (Task 2), `ffmpeg::ffmpeg_binary` (F1).
- Produces:
  - `export::build_zoompan_expr(model: &ZoomModel, fps: u32) -> (String, String, String)` — devolve `(z_expr, x_expr, y_expr)` para o filtro `zoompan` (puro, testado).
  - `export::export(video_path, model, out_path, fps, on_progress) -> Result<(), String>` — roda ffmpeg (smoke).
  - Comando `export_with_zoom(app, video_path, zoom, out_path) -> Result<(), String>` — emite eventos Tauri `export-progress` (0..1).

- [ ] **Step 1: Escrever teste do builder de expressão**

`zoom/export.rs`:
```rust
use crate::model::zoom::ZoomModel;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::zoom::{ZoomSegment, ZoomTarget};

    #[test]
    fn builds_zoompan_expr_for_one_segment() {
        let m = ZoomModel { version: 1, segments: vec![ZoomSegment {
            start_ms: 0, end_ms: 2000, ease_in_ms: 300, ease_out_ms: 400,
            scale: 2.0, targets: vec![ZoomTarget { t_ms: 0, x: 0.25, y: 0.75 }],
        }]};
        let (z, x, y) = build_zoompan_expr(&m, 30);
        // tempo = on/fps
        assert!(z.contains("on/30"), "{z}");
        // contém o platô de escala do segmento
        assert!(z.contains("2"), "{z}");
        // x/y derivam do centro e do zoom
        assert!(x.contains("iw"), "{x}");
        assert!(y.contains("ih"), "{y}");
        // sem segmento ativo o zoom default é 1
        assert!(z.contains("1"), "{z}");
    }

    #[test]
    fn empty_model_is_identity() {
        let m = ZoomModel { version: 1, segments: vec![] };
        let (z, _x, _y) = build_zoompan_expr(&m, 30);
        assert_eq!(z.trim(), "1");
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `cd src-tauri && cargo test export 2>&1 | tail -10`
Expected: `cannot find function build_zoompan_expr`.

- [ ] **Step 3: Implementar o builder**

`zoompan` avalia por frame de saída; usamos `on` (índice do frame) e `t = on/fps`.
`z` = piecewise; `x`,`y` = canto superior-esquerdo da janela na fonte:
`x = iw*cx - (iw/z)/2`, `y = ih*cy - (ih/z)/2`. O centro `cx/cy` aqui é
constante por segmento (alvo do primeiro target); panning multi-target no export
é simplificação aceitável da F2 (preview mostra o pan; export usa o 1º alvo —
documentar). Easing por `smoothstep` inline.

```rust
fn seg_scale_expr(seg: &ZoomSegment, fps: u32) -> String {
    // t in seconds = on/fps
    let s0 = seg.start_ms as f64 / 1000.0;
    let s1 = seg.end_ms as f64 / 1000.0;
    let ein = seg.ease_in_ms as f64 / 1000.0;
    let eout = seg.ease_out_ms as f64 / 1000.0;
    let dur = s1 - s0;
    let scale = seg.scale;
    // envelope e(t): ramps via smoothstep; p clamped by zoompan's clip()
    // rel = (on/fps - s0)
    let _ = fps;
    format!(
        "if(between(t,{s0},{s1}),\
1+({scale}-1)*\
(if(lt(t-{s0},{ein}),\
(clip((t-{s0})/{ein},0,1))*(clip((t-{s0})/{ein},0,1))*(3-2*clip((t-{s0})/{ein},0,1)),\
if(gt(t-{s0},{dur}-{eout}),\
(clip(({dur}-(t-{s0}))/{eout},0,1))*(clip(({dur}-(t-{s0}))/{eout},0,1))*(3-2*clip(({dur}-(t-{s0}))/{eout},0,1)),\
1))),"
    )
}

pub fn build_zoompan_expr(model: &ZoomModel, fps: u32) -> (String, String, String) {
    if model.segments.is_empty() {
        return ("1".into(), "iw/2-(iw/zoom/2)".into(), "ih/2-(ih/zoom/2)".into());
    }
    // zoompan uses `on` for output frame; expose t via on/fps in expressions.
    // Build nested z; default 1 at the end.
    let mut z = String::new();
    let mut depth = 0;
    for seg in &model.segments {
        z.push_str(&seg_scale_expr(seg, fps));
        depth += 1;
    }
    z.push('1');
    for _ in 0..depth { z.push(')'); }
    // Replace the time variable: zoompan expressions use `on` (frame). Map t->on/fps.
    let z = format!("'{}'", z.replace('t', &format!("(on/{fps})")));

    // center cx,cy piecewise (first target per segment), default 0.5
    let cx = center_expr(model, fps, true);
    let cy = center_expr(model, fps, false);
    // x = iw*cx - (iw/zoom)/2 ; y = ih*cy - (ih/zoom)/2
    let x = format!("'iw*{cx}-(iw/zoom/2)'");
    let y = format!("'ih*{cy}-(ih/zoom/2)'");
    (z, x, y)
}

fn center_expr(model: &ZoomModel, fps: u32, is_x: bool) -> String {
    let mut e = String::new();
    let mut depth = 0;
    for seg in &model.segments {
        let s0 = seg.start_ms as f64 / 1000.0;
        let s1 = seg.end_ms as f64 / 1000.0;
        let v = if is_x { seg.targets[0].x } else { seg.targets[0].y };
        e.push_str(&format!("if(between(T,{s0},{s1}),{v},"));
        depth += 1;
    }
    e.push_str("0.5");
    for _ in 0..depth { e.push(')'); }
    e.replace('T', &format!("(on/{fps})"))
}
```

> O `t` no `seg_scale_expr` é substituído por `(on/fps)` no fim de
> `build_zoompan_expr`. **Risco:** sintaxe/var do `zoompan` pode exigir ajuste
> (alguns builds expõem `time`; `clip` vs `clip`/`min`/`max`). O implementador
> valida no smoke (Step 5) e ajusta as expressões até o render sair sem erro.
> Panning multi-target no export é simplificado para o 1º alvo (documentado).

- [ ] **Step 4: Implementar `export()` + comando**

`export.rs`:
```rust
use std::process::{Command, Stdio};
use crate::capture::ffmpeg::ffmpeg_binary;

pub fn export<F: Fn(f64)>(
    video_path: &str, model: &ZoomModel, out_path: &str, fps: u32, total_ms: u64, on_progress: F,
) -> Result<(), String> {
    let (z, x, y) = build_zoompan_expr(model, fps);
    let vf = format!(
        "zoompan=z={z}:x={x}:y={y}:d=1:fps={fps}:s=iw0xih0",
    );
    // Note: s=iw0xih0 keeps original size; if the build rejects iw0/ih0, pass
    // explicit WxH read from the input (validate in smoke).
    let mut child = Command::new(ffmpeg_binary())
        .args(["-y", "-i", video_path, "-vf", &vf, "-c:a", "copy", out_path, "-progress", "pipe:1", "-nostats"])
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
```

`commands.rs`:
```rust
use tauri::Emitter;

#[tauri::command]
pub fn export_with_zoom(
    app: tauri::AppHandle,
    video_path: String,
    zoom: crate::model::zoom::ZoomModel,
    out_path: String,
    fps: u32,
    total_ms: u64,
) -> Result<(), String> {
    crate::zoom::export::export(&video_path, &zoom, &out_path, fps, total_ms, |p| {
        let _ = app.emit("export-progress", p);
    })
}
```
Registrar `export_with_zoom` no `invoke_handler`.

- [ ] **Step 5: Rodar testes + smoke de render**

Run: `cd src-tauri && cargo test export 2>&1 | tail -10 && cargo build 2>&1 | tail -5`
Expected: testes do builder ok; build `Finished`.

Smoke (manual, com uma gravação real da F1 em `~/Movies/OpenRecorder`):
gere um `out.mp4` chamando o caminho de export por um teste `#[ignore]` ou pela
UI (Task 11). Confirme via `ffprobe` que o `out.mp4` é válido e que o zoom
aparece. Se o `zoompan` rejeitar a expressão, ajuste a sintaxe (var de tempo,
`s=`) até renderizar.

- [ ] **Step 6: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: ffmpeg zoompan export with progress events"
```

---

## Task 8: API TS + abrir editor a partir da lista

**Files:**
- Modify: `src/lib/api.ts`, `src/state/useRecorder.ts`, `src/components/RecordingsList.tsx`, `src/App.tsx`
- Create: `src/state/useEditor.ts`

**Interfaces:**
- Consumes: comandos `load_recording`/`save_zoom`/`export_with_zoom` (Tasks 6-7), `ZoomModel`/`zoomAt` (Task 5).
- Produces:
  - `api.ts`: tipos `ZoomTarget/ZoomSegment/ZoomModel` (import de `./zoom`), `RecordingMetadata`, `LoadedRecording`; wrappers `loadRecording(videoPath)`, `saveZoom(videoPath, zoom)`, `exportWithZoom(videoPath, zoom, outPath, fps, totalMs)`; helper `onExportProgress(cb)` via `@tauri-apps/api/event`.
  - `useEditor(videoPath)`: estado `{ metadata, model, setModel, selectedId, ... }`, carrega via `loadRecording`, autosave via `saveZoom` (debounce simples).
  - `App.tsx`: estado `editing: string | null` (video path); a lista abre o editor; o editor tem botão "voltar".

- [ ] **Step 1: Estender `api.ts`**

```ts
import type { ZoomModel } from "./zoom";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface InputEventDTO { t_ms: number; type: string; x: number; y: number; button?: string }
export interface RecordingMetadata {
  version: number;
  recording: { width: number; height: number; fps: number; duration_ms: number };
  source: { type: string; id: string; rect: [number, number, number, number] };
  events: InputEventDTO[];
}
export interface LoadedRecording { metadata: RecordingMetadata; zoom: ZoomModel }

export const loadRecording = (videoPath: string) =>
  invoke<LoadedRecording>("load_recording", { videoPath });
export const saveZoom = (videoPath: string, zoom: ZoomModel) =>
  invoke<void>("save_zoom", { videoPath, zoom });
export const exportWithZoom = (
  videoPath: string, zoom: ZoomModel, outPath: string, fps: number, totalMs: number,
) => invoke<void>("export_with_zoom", { videoPath, zoom, outPath, fps, totalMs });
export const onExportProgress = (cb: (p: number) => void) =>
  listen<number>("export-progress", (e) => cb(e.payload));
```

- [ ] **Step 2: Criar `useEditor.ts`**

```ts
import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "../lib/api";
import type { ZoomModel } from "../lib/zoom";

export function useEditor(videoPath: string) {
  const [metadata, setMetadata] = useState<api.RecordingMetadata | null>(null);
  const [model, setModelState] = useState<ZoomModel>({ version: 1, segments: [] });
  const [selected, setSelected] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const saveTimer = useRef<number | null>(null);

  useEffect(() => {
    api.loadRecording(videoPath)
      .then((r) => { setMetadata(r.metadata); setModelState(r.zoom); })
      .catch((e) => setError(String(e)));
  }, [videoPath]);

  const setModel = useCallback((m: ZoomModel) => {
    setModelState(m);
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      api.saveZoom(videoPath, m).catch((e) => setError(String(e)));
    }, 500);
  }, [videoPath]);

  return { metadata, model, setModel, selected, setSelected, error };
}
```

- [ ] **Step 3: Lista abre o editor**

Em `RecordingsList.tsx`, adicionar prop `onEdit(videoPath)` e um botão "Editar" por item:
```tsx
        <button className="btn small" onClick={() => props.onEdit(r.video_path)}>Editar</button>
```
(estender a assinatura de props com `onEdit: (p: string) => void`).

Em `App.tsx`: adicionar `const [editing, setEditing] = useState<string | null>(null);`, passar `onEdit={setEditing}` pra `RecordingsList`, e renderizar `<EditorView videoPath={editing} onBack={() => setEditing(null)} />` quando `editing` não for nulo (em vez da tela principal).

- [ ] **Step 4: Build + testes**

Run: `pnpm build 2>&1 | tail -5 && pnpm test 2>&1 | tail -5`
Expected: build ok (pode dar erro de `EditorView` ainda não existir — crie um stub mínimo `export function EditorView(_: {videoPath: string; onBack: () => void}) { return null; }` em `components/EditorView.tsx` pra compilar; preenchido na Task 9). Testes vitest seguem verdes.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: editor API wrappers, useEditor state, open editor from list"
```

---

## Task 9: PreviewCanvas (vídeo + transform ao vivo)

**Files:**
- Create: `src/components/PreviewCanvas.tsx`
- Modify: `src/components/EditorView.tsx`

**Interfaces:**
- Consumes: `zoomAt` (Task 5), `ZoomModel`.
- Produces: `PreviewCanvas({ videoSrc, model, onTimeUpdate, registerSeek })` — toca o vídeo e aplica `transform` por frame; expõe o tempo atual e uma função de seek via callbacks.

- [ ] **Step 1: Implementar `PreviewCanvas.tsx`**

```tsx
import { useEffect, useRef } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { zoomAt, type ZoomModel } from "../lib/zoom";

export function PreviewCanvas(props: {
  videoPath: string; model: ZoomModel;
  onTime: (ms: number) => void; playRef: (v: HTMLVideoElement) => void;
}) {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const rafRef = useRef<number | null>(null);

  useEffect(() => {
    const v = videoRef.current;
    if (!v) return;
    props.playRef(v);
    const tick = () => {
      const ms = v.currentTime * 1000;
      const z = zoomAt(props.model, ms);
      v.style.transformOrigin = `${z.cx * 100}% ${z.cy * 100}%`;
      v.style.transform = `scale(${z.scale})`;
      props.onTime(ms);
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);
    return () => { if (rafRef.current) cancelAnimationFrame(rafRef.current); };
  }, [props.model]);

  return (
    <div style={{ overflow: "hidden", background: "#000", aspectRatio: "16/9", width: "100%" }}>
      <video
        ref={videoRef}
        src={convertFileSrc(props.videoPath)}
        controls
        style={{ width: "100%", height: "100%", display: "block", willChange: "transform" }}
      />
    </div>
  );
}
```

> `convertFileSrc` serve o arquivo local pro webview. Pode exigir habilitar o
> protocolo `asset`/escopo em `tauri.conf.json` (`app.security.assetProtocol`
> com `enable: true` e `scope` cobrindo `$HOME/Movies/OpenRecorder/**`). Ajuste a
> config se o vídeo não carregar (validar no smoke).

- [ ] **Step 2: Montar `EditorView.tsx` (preview + voltar)**

Substituir o stub:
```tsx
import { useState } from "react";
import { useEditor } from "../state/useEditor";
import { PreviewCanvas } from "./PreviewCanvas";

export function EditorView(props: { videoPath: string; onBack: () => void }) {
  const ed = useEditor(props.videoPath);
  const [, setT] = useState(0);
  return (
    <div className="editor">
      <button className="btn small" onClick={props.onBack}>← Voltar</button>
      {ed.error && <p className="error">{ed.error}</p>}
      <PreviewCanvas
        videoPath={props.videoPath}
        model={ed.model}
        onTime={setT}
        playRef={() => {}}
      />
    </div>
  );
}
```

- [ ] **Step 3: Habilitar assetProtocol (se necessário) + build**

Em `tauri.conf.json` `app.security`, garantir:
```json
"assetProtocol": { "enable": true, "scope": ["$HOME/Movies/OpenRecorder/**"] }
```
Run: `pnpm build 2>&1 | tail -5`
Expected: build ok.

- [ ] **Step 4: Smoke manual (preview)**

Pela app (após Task 11 ter o fluxo completo, ou via `pnpm tauri dev`): abrir uma gravação no editor; o vídeo toca e dá zoom suave nos momentos de clique. (Sem cliques na gravação, o preview fica sem zoom — esperado.)

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: live preview canvas applying zoom transform per frame"
```

---

## Task 10: Timeline (barras de segmento, playhead, seleção)

**Files:**
- Create: `src/components/Timeline.tsx`, `src/lib/timeline.ts`, `src/lib/timeline.test.ts`
- Modify: `src/components/EditorView.tsx`

**Interfaces:**
- Consumes: `ZoomModel`, `RecordingMetadata` (duração).
- Produces:
  - `timeline.ts`: `msToX(ms, durationMs, widthPx)` e `xToMs(x, durationMs, widthPx)` (puros, testados).
  - `Timeline({ model, durationMs, currentMs, selected, onSelect, onSeek })` — barras dos segmentos + marcadores de clique + playhead; clicar numa barra seleciona; clicar na régua faz seek.

- [ ] **Step 1: Testes dos helpers (`timeline.test.ts`)**

```ts
import { describe, it, expect } from "vitest";
import { msToX, xToMs } from "./timeline";

describe("timeline mapping", () => {
  it("maps ms to x and back", () => {
    expect(msToX(0, 10000, 500)).toBe(0);
    expect(msToX(10000, 10000, 500)).toBe(500);
    expect(msToX(5000, 10000, 500)).toBe(250);
    expect(Math.round(xToMs(250, 10000, 500))).toBe(5000);
  });
  it("clamps out of range", () => {
    expect(msToX(20000, 10000, 500)).toBe(500);
    expect(xToMs(-10, 10000, 500)).toBe(0);
  });
});
```

- [ ] **Step 2: Rodar pra ver falhar**

Run: `pnpm test 2>&1 | tail -10`
Expected: falha — `./timeline` não existe.

- [ ] **Step 3: Implementar `timeline.ts`**

```ts
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

export function msToX(ms: number, durationMs: number, widthPx: number): number {
  if (durationMs <= 0) return 0;
  return clamp((ms / durationMs) * widthPx, 0, widthPx);
}
export function xToMs(x: number, durationMs: number, widthPx: number): number {
  if (widthPx <= 0) return 0;
  return clamp((x / widthPx) * durationMs, 0, durationMs);
}
```

- [ ] **Step 4: Implementar `Timeline.tsx`**

```tsx
import { useRef } from "react";
import type { ZoomModel } from "../lib/zoom";
import type { RecordingMetadata } from "../lib/api";
import { msToX, xToMs } from "../lib/timeline";

export function Timeline(props: {
  model: ZoomModel; meta: RecordingMetadata; currentMs: number;
  selected: number | null; onSelect: (i: number) => void; onSeek: (ms: number) => void;
}) {
  const ref = useRef<HTMLDivElement | null>(null);
  const W = 800;
  const dur = props.meta.recording.duration_ms;
  return (
    <div
      ref={ref}
      style={{ position: "relative", height: 56, width: W, background: "#1c1c1c", marginTop: 12 }}
      onClick={(e) => {
        const rect = (e.currentTarget as HTMLDivElement).getBoundingClientRect();
        props.onSeek(xToMs(e.clientX - rect.left, dur, W));
      }}
    >
      {props.meta.events.filter((ev) => ev.type === "click").map((ev, i) => (
        <div key={`c${i}`} style={{ position: "absolute", left: msToX(ev.t_ms, dur, W),
          top: 0, width: 2, height: 10, background: "#888" }} />
      ))}
      {props.model.segments.map((s, i) => (
        <div key={`s${i}`}
          onClick={(e) => { e.stopPropagation(); props.onSelect(i); }}
          style={{ position: "absolute", left: msToX(s.start_ms, dur, W),
            width: Math.max(4, msToX(s.end_ms, dur, W) - msToX(s.start_ms, dur, W)),
            top: 16, height: 28, borderRadius: 4,
            background: props.selected === i ? "#3b82f6" : "#2563eb88",
            cursor: "pointer" }} />
      ))}
      <div style={{ position: "absolute", left: msToX(props.currentMs, dur, W),
        top: 0, width: 1, height: 56, background: "red" }} />
    </div>
  );
}
```

- [ ] **Step 5: Ligar no `EditorView`**

Adicionar ao `EditorView` o estado de tempo e o seek (via `playRef` guardando o `<video>`), e renderizar `<Timeline ... />` abaixo do preview, passando `ed.model`, `ed.metadata`, `currentMs`, `ed.selected`, `ed.setSelected`, e `onSeek` (seta `video.currentTime = ms/1000`).

- [ ] **Step 6: Rodar testes + build**

Run: `pnpm test 2>&1 | tail -5 && pnpm build 2>&1 | tail -5`
Expected: vitest ok (timeline + zoom), build ok.

- [ ] **Step 7: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: timeline with segment bars, click markers, playhead, seek"
```

---

## Task 11: SegmentInspector + export (com progresso)

**Files:**
- Create: `src/components/SegmentInspector.tsx`
- Modify: `src/components/EditorView.tsx`

**Interfaces:**
- Consumes: `ZoomModel`, `exportWithZoom`/`onExportProgress` (Task 8), `RecordingMetadata`.
- Produces: `SegmentInspector({ model, selected, onChange, onDelete })` — edita `scale`/`start_ms`/`end_ms` do segmento selecionado; deletar; export com barra de progresso.

- [ ] **Step 1: Implementar `SegmentInspector.tsx`**

```tsx
import type { ZoomModel } from "../lib/zoom";

export function SegmentInspector(props: {
  model: ZoomModel; selected: number | null;
  onChange: (m: ZoomModel) => void; onDelete: (i: number) => void;
}) {
  if (props.selected === null) return <p className="muted">Selecione um zoom na timeline.</p>;
  const seg = props.model.segments[props.selected];
  if (!seg) return null;
  const update = (patch: Partial<typeof seg>) => {
    const segments = props.model.segments.map((s, i) =>
      i === props.selected ? { ...s, ...patch } : s);
    props.onChange({ ...props.model, segments });
  };
  return (
    <div className="inspector">
      <label>Zoom (escala)
        <input type="range" min={1} max={4} step={0.1} value={seg.scale}
          onChange={(e) => update({ scale: Number(e.target.value) })} />
        <span>{seg.scale.toFixed(1)}×</span>
      </label>
      <label>Início (ms)
        <input type="number" value={seg.start_ms}
          onChange={(e) => update({ start_ms: Number(e.target.value) })} />
      </label>
      <label>Fim (ms)
        <input type="number" value={seg.end_ms}
          onChange={(e) => update({ end_ms: Number(e.target.value) })} />
      </label>
      <button className="btn small" onClick={() => props.onDelete(props.selected!)}>Deletar zoom</button>
    </div>
  );
}
```

- [ ] **Step 2: Export + progresso no `EditorView`**

Adicionar ao `EditorView`:
```tsx
import { useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import * as api from "../lib/api";
// ...dentro do componente:
const [progress, setProgress] = useState<number | null>(null);
useEffect(() => {
  const un = api.onExportProgress((p) => setProgress(p));
  return () => { un.then((f) => f()); };
}, []);
async function doExport() {
  if (!ed.metadata) return;
  const out = await save({ defaultPath: "OpenRecorder-export.mp4",
    filters: [{ name: "MP4", extensions: ["mp4"] }] });
  if (!out) return;
  setProgress(0);
  try {
    await api.exportWithZoom(props.videoPath, ed.model, out,
      ed.metadata.recording.fps, ed.metadata.recording.duration_ms);
  } finally { setProgress(null); }
}
```
Renderizar `<SegmentInspector model={ed.model} selected={ed.selected} onChange={ed.setModel} onDelete={...} />`, um botão "Exportar" (chama `doExport`, desabilita enquanto `progress !== null`), e a barra `progress`.
Adicionar a dep `@tauri-apps/plugin-dialog`: `pnpm add @tauri-apps/plugin-dialog` e registrar `tauri_plugin_dialog::init()` no builder do `lib.rs` (+ `tauri-plugin-dialog` no `Cargo.toml`).
`onDelete(i)`: `ed.setModel({ ...ed.model, segments: ed.model.segments.filter((_, j) => j !== i) }); ed.setSelected(null);`

- [ ] **Step 3: Build + testes**

Run: `pnpm build 2>&1 | tail -5 && (cd src-tauri && cargo build 2>&1 | tail -5)`
Expected: ambos compilam (instala o plugin-dialog).

- [ ] **Step 4: Smoke manual fim-a-fim (precisa de você)**

`pnpm tauri dev` → gravar clicando → Parar → Editar → ver auto-zooms na timeline + preview com zoom → ajustar/deletar um zoom → Exportar → escolher destino → barra de progresso → abrir o mp4 exportado e conferir que o zoom "assado" bate com o preview.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: segment inspector and zoom export with progress UI"
```

---

## Task 12: Smoke checklist + docs

**Files:**
- Create: `docs/SMOKE-TEST-F2.md`
- Modify: `README.md` (status F2)

**Interfaces:**
- Consumes: tudo.
- Produces: checklist manual + README atualizado.

- [ ] **Step 1: Escrever `docs/SMOKE-TEST-F2.md`**

Checklist macOS: conceder Monitoramento de Entrada (pros cliques); gravar clicando em vários pontos; abrir editor; auto-zooms aparecem na timeline e no preview; ajustar escala/início/fim de um zoom; deletar um zoom; reabrir editor (edições persistem via `zoom.json`); exportar e conferir que o mp4 assado bate com o preview; sem permissão de input → sem auto-zoom (adicionar manual ainda funciona). Itens em checkbox.

- [ ] **Step 2: Atualizar `README.md`**

Mover F2 de "roadmap" pra status atual: auto-zoom no clique + editor (timeline + preview ao vivo) + export landscape. Manter requisito ffmpeg. Notar que cliques exigem permissão de Monitoramento de Entrada (mac).

- [ ] **Step 3: Executar o smoke** (seguir `docs/SMOKE-TEST-F2.md`; corrigir o que falhar voltando à task correspondente).

- [ ] **Step 4: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "docs: add F2 smoke checklist and update README"
```

---

## Self-Review (autor do plano)

**1. Cobertura do spec:**
- Captura de clique nativa (mac) + cfg rdev → Task 1.
- Modelo + sampler `zoom_at` + clamp → Tasks 2, 3.
- Geração/merge dos cliques → Task 4.
- Paridade Rust↔TS → Task 5 (fixture/valores idênticos aos da Task 3).
- Persistência `zoom.json` + comandos load/save → Task 6.
- Export ffmpeg (zoompan) + progresso → Task 7.
- Preview ao vivo (transform CSS) → Task 9.
- Timeline + seleção → Task 10.
- Inspector + defaults globais + add/delete + export UI → Task 11 (defaults globais: regeneração via `GenOpts` está em load_recording/generate; ajuste global de scale por-segmento no inspector — regeneração global completa é refinamento, o inspector cobre por-segmento + delete).
- Erros (sem input, zoom.json corrompido, falha ffmpeg, metadata ausente) → tratados nos comandos (Task 6: regenera; Task 7: status; Task 1: degrada).
- Testes (unit Rust, paridade, vitest, smoke) → Tasks 2-7, 10 + smoke 9/11/12.

**2. Placeholders:** sem "TBD". Tasks de integração (1, 7, 9, 11) trazem código concreto + nota explícita de validar a API real (core-graphics/zoompan/assetProtocol/plugin-dialog) e reportar DONE_WITH_CONCERNS se divergir — risco conhecido, não placeholder.

**3. Consistência de tipos:** `ZoomModel/ZoomSegment/ZoomTarget/ZoomAt` idênticos em Rust e TS; `zoom_at`/`zoomAt` mesma semântica (fixture compartilhada); `generate`/`GenOpts`; `store::zoom_path/load/save`; comandos `load_recording`(retorna `LoadedRecording{metadata,zoom}`)/`save_zoom`/`export_with_zoom` ↔ wrappers `loadRecording/saveZoom/exportWithZoom`; campos snake_case no JSON batem (TS usa as mesmas chaves).

**Riscos conhecidos (documentados):**
- API do `core-graphics` (CGEventTap) pode divergir do esboço (Task 1).
- Sintaxe do `zoompan` (var de tempo, `clip`, `s=`) — validar no smoke (Task 7).
- `convertFileSrc`/assetProtocol scope pra servir o vídeo no webview (Task 9).
- Panning multi-target no export usa o 1º alvo do segmento (preview mostra pan completo) — simplificação F2, documentada.
- Paridade preview×export depende dos dois usarem o mesmo modelo/easing; pan é a única divergência conhecida (aceitável F2).
