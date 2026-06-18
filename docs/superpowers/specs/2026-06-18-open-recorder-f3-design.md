# OpenRecorder — Design (F3: Webcam Overlay)

**Data:** 2026-06-18
**Status:** Aprovado para planejamento
**Escopo deste doc:** Fase 3 (F3). Depende da F1 (captura) e F2 (editor/export).

## Visão

F3 adiciona **overlay de webcam**: grava a câmera junto da tela, deixa posicionar
uma bolha (rosto) no editor com preview ao vivo, e compõe no export. Continua
**não-destrutivo**: a webcam é um stream separado, aplicada só no export — dá pra
mover/ajustar depois sem regravar.

## Decisões travadas (global)

- **Plataforma alvo:** macOS (testado); código mantém intenção cross-platform.
- **Stack:** Tauri 2 + Rust + React/TS (mesma da F1/F2).
- **Não-destrutivo:** webcam gravada em `REC-<ts>.webcam.mp4` (separada); overlay
  aplicado no export. Config do overlay vive no `REC-<ts>.zoom.json` (campo
  `webcam`, opcional, versionado).
- **Webcam = vídeo-only** (sem áudio; o áudio é o mic do `REC-<ts>.mp4`).
- **Overlay fixo por cima**, aplicado **depois** do zoom (rosto não amplia junto).
- **Crate de câmera:** `nokhwa` (capturado como o `video_capture`: frames → ffmpeg).
- Export landscape (9:16 é F4). Commits sem co-author/histórico do Claude; inglês `tipo: descrição`.
- ffmpeg é dependência de runtime (resolvido por caminho absoluto, F1).
- Branch `f3-webcam-overlay` (saiu do tip de `f2-auto-zoom`).

## Estado herdado (F1/F2 — NÃO refazer)

- Captura: `VideoCapture` (scap→ffmpeg, com `catch_unwind` anti-crash), `AudioCapture`
  (cpal), `InputListener` (CGEventTap mac), `RecordingCoordinator`.
- `metadata.json`: `RecordingMetadata { version, recording{width,height,fps,duration_ms},
  source{type,id,rect}, events[] }`.
- F2: `ZoomModel`/`zoom_at` (Rust+TS paridade), `generate`, `store` (`zoom.json`),
  `export` (ffmpeg zoompan + progresso), editor (`EditorView`, `PreviewCanvas`,
  `Timeline`, `SegmentInspector`), comandos `load_recording`/`save_zoom`/`export_with_zoom`.
- Source/Mic pickers usam o `Dropdown` custom; janelas via CGWindowList nativo.

## Captura de webcam (durante a gravação)

- Novo `capture/webcam_capture.rs`: `nokhwa` captura a câmera escolhida → frames →
  `ffmpeg` → `REC-<ts>.webcam.mp4` (H.264). Mesmo padrão do `video_capture` (com
  `catch_unwind` pra um panic do nokhwa virar erro tratado, não crash).
- `RecordingCoordinator` ganha uma 4ª via **opcional**: só inicia se uma câmera for
  selecionada. Tela + mic + input + webcam rodam em paralelo; no stop, todas param.
- UI de gravação: seletor de **Câmera** (`Dropdown` custom) com opção **"Nenhuma"**.
  Lista via comando `list_cameras`.
- `metadata.json`: `recording.has_webcam: bool` + `recording.camera_name: Option<String>`.
- Permissão macOS Câmera (TCC): gate como o do mic/screen. Sem permissão → grava sem
  webcam e avisa; `has_webcam=false`.

## Modelo de overlay (config de edição)

Adicionado ao `ZoomModel` como campo opcional (serde `default`, retrocompatível):
```
WebcamOverlay {
  enabled: bool,
  shape: String,        // "circle" | "rounded"
  x: f64, y: f64,       // posição normalizada 0..1 (canto superior-esquerdo) na saída
  size: f64,            // largura como fração da largura de saída (ex.: 0.22)
  border_width: u32,    // px
  border_color: String, // hex, ex.: "#ffffff"
  mirror: bool,         // espelho selfie
}
```
Defaults (quando há webcam e nenhuma config salva): `enabled true`, `shape "circle"`,
canto inferior-direito (`x ≈ 0.76, y ≈ 0.74`), `size 0.22`, `border_width 3`,
`border_color "#ffffff"`, `mirror true`. Aspecto da bolha = quadrado (size×size em
relação à largura), recortado pela forma.

## Editor (estende o da F2)

Se `metadata.recording.has_webcam`:
- Barra de controles do overlay: ligar/desligar, alternar **círculo ↔ arredondado**,
  largura/cor da borda, espelho.
- **Bolha** sobre o `PreviewCanvas`: segundo `<video>` (o `webcam.mp4`) posicionado/
  dimensionado por `WebcamOverlay`, recortado via `clip-path: circle()` (círculo) ou
  `border-radius` (arredondado), espelhado via `transform: scaleX(-1)`. **Arrastável**
  (move → atualiza x,y) e **redimensionável** por uma alça (→ atualiza size).
- A bolha é **fixa** (não recebe o transform de zoom do preview).
- Sincronizada ao mesmo playhead (mesmo `currentTime` do vídeo da tela).

## Export (estende o `zoom/export.rs`)

Quando `webcam.enabled` e o `webcam.mp4` existe:
- Entradas ffmpeg: tela (`REC.mp4`) + webcam (`REC.webcam.mp4`).
- Filtro: `[0]zoompan(...)[bg]` (F2); `[1]scale=S:S, hflip(se mirror), <máscara
  círculo/arredondado via geq/alpha>, <borda>[fg]`; `[bg][fg]overlay=x=X:y=Y`.
  `S = round(size * W)`, `X = round(x * W)`, `Y = round(y * H)`.
- Máscara círculo: alpha via `geq` (`alpha = 255 dentro do raio, 0 fora`). Arredondado:
  `geq` com cantos. Borda: círculo/retângulo colorido levemente maior atrás.
- Áudio: `-map` do áudio da tela (mic). Progresso e exit-status como na F2.
- **Risco conhecido (maior da F3):** montar o filtro de máscara/borda + overlay e
  bater com o preview. Plano detalha; implementer valida com render de teste.

## Comandos Tauri (novos/alterados)

- `list_cameras() -> [{ id, name }]` (via nokhwa).
- `start_recording(..., camera_id: Option<String>)` — adiciona a câmera opcional.
- `load_recording` já devolve o `zoom.json` (agora com `webcam`); `save_zoom` persiste.
- `export_with_zoom` já recebe o `ZoomModel` (agora com `webcam`) — usa o `webcam.mp4`
  ao lado do `video_path`.

## Fluxo de dados

```
gravar: tela→REC.mp4(+mic) · webcam(se câmera)→REC.webcam.mp4 · metadata.has_webcam=true
editor: load_recording → metadata + zoom.json(webcam); preview sobrepõe os 2 vídeos
exportar: ffmpeg [tela zoompan] + [webcam scale/mirror/máscara/borda] overlay → out.mp4
```

## Tratamento de erros

- Sem permissão de Câmera (mac) → grava sem webcam, `has_webcam=false`, aviso na UI.
- nokhwa falha ao iniciar → `catch_unwind` → grava sem webcam (degrada), aviso.
- `webcam.mp4` ausente no export mas `enabled` → pula o overlay, exporta só a tela, aviso.
- Câmera escolhida some/ocupada → erro claro no start, sem derrubar o app.

## Testes

- **Rust unit:** round-trip serde do `WebcamOverlay` (incl. compat: `zoom.json` sem
  campo `webcam` desserializa com `None`); builder do filtro de overlay (assert nos
  termos `overlay`/`hflip`/`geq` quando `enabled`, e ausência quando `!enabled`);
  cálculo de geometria (x,y,size → S,X,Y).
- **TS unit (vitest):** mapeamento normalizado→px do preview pro overlay (posição/
  tamanho), e clamp dentro da moldura.
- **Smoke manual (macOS):** gravar com câmera (conceder permissão) → editor mostra a
  bolha → mover/redimensionar/alternar forma/espelho → exportar → overlay assado bate
  com o preview; também: gravar sem câmera (overlay ausente); sem permissão (degrada).

## Faseamento interno (sub-entregas testáveis)

1. Captura de webcam (nokhwa) + `list_cameras` + seletor + `has_webcam` no metadata.
2. `WebcamOverlay` (modelo + persistência) + editor (controles + bolha + preview).
3. Export: composição ffmpeg (scale/mirror/máscara/borda/overlay).

Cada uma roda/testa por si; o plano detalha em tarefas.
