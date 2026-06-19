# OpenRecorder — Design (F2: Auto-zoom no Clique)

**Data:** 2026-06-18
**Status:** Aprovado para planejamento
**Escopo deste doc:** Fase 2 (F2). Depende da F1 (captura).

## Visão

F2 adiciona o diferencial central do OpenRecorder: **zoom automático nos cliques
do mouse**, com um **editor (timeline + preview ao vivo)** e **export landscape
com o zoom "assado"** no vídeo. Continua não-destrutivo: a F1 grava o vídeo cru
+ os eventos; a F2 gera/edita os zooms e só os aplica no export. (9:16 é F4.)

## Decisões travadas (global)

- **Plataforma:** macOS é o alvo testado; código mantém intenção cross-platform.
- **Stack:** Tauri 2 + Rust + React/TS (mesma da F1).
- **Não-destrutivo:** F2 consome o `REC-<ts>.mp4` + `REC-<ts>.metadata.json` que
  a F1 já produz; nunca regrava. O zoom editado vive em `REC-<ts>.zoom.json`.
- **Export F2:** landscape (resolução original). 9:16 fica para F4.
- **Commits:** sem co-author/histórico do Claude; mensagens em inglês `tipo: descrição`.
- **Branch:** `f2-auto-zoom`, saída do tip de `f1-capture-foundation`; retarget
  para `main` quando o PR da F1 (#1) fundir.

## Pré-requisito embutido: captura de clique real

A F1 desligou o `rdev` porque ele crasha no macOS 14+/26 (chama APIs de Text
Services só-main-thread a partir de uma thread de fundo ao decodificar eventos
de teclado). Sem cliques, não há dados de zoom. A F2 conserta isso:

- **macOS:** **CGEventTap nativo só de mouse** (`mouseDown` left/right,
  `mouseMoved`/`dragged`) numa thread dedicada com `CFRunLoop`. Por capturar
  **apenas** mouse, não toca no caminho de teclado que quebrava o `rdev`.
  Bônus: eventos de mouse trazem coordenada real → conserta o `(0,0)` do clique
  da F1.
- **Windows/Linux:** mantém `rdev` atrás de `#[cfg(not(target_os = "macos"))]`
  (lá ele funciona).
- O formato do `metadata.json` **não muda** (já versionado). A diferença é que
  `events` passa a sair preenchido com cliques (x,y reais) e movimentos.

## Modelo de zoom (único, compartilhado por preview e export)

```
ZoomSegment {
  start_ms: u64,
  end_ms: u64,
  target: { x: f64, y: f64 },   // normalizado 0..1 no retângulo da fonte
  scale: f64,                   // ex. 2.0
  ease_in_ms: u64,
  ease_out_ms: u64,
}
ZoomModel { version: 1, segments: Vec<ZoomSegment> }
```

**Sampler** (a função-chave, replicada em Rust e TS, com paridade testada):
- `zoom_at(model, t_ms) -> { scale: f64, center: { x, y } }`
- Dentro de um segmento: `progress` sobe de 0→1 em `ease_in_ms` (curva
  `smoothstep`), mantém `scale` no platô, e desce 1→0 em `ease_out_ms` antes de
  `end_ms`. Fora de qualquer segmento: `scale = 1.0`, `center = {0.5, 0.5}`.
- `center` interpola o `target` (em merges, desliza entre pontos).
- **Clamp:** o centro é limitado para a janela ampliada não passar das bordas
  (`center ∈ [0.5/scale, 1 - 0.5/scale]` em cada eixo).
- `smoothstep(p) = p*p*(3 - 2*p)`.

**Geração a partir dos cliques (só no Rust):**
- Defaults: `scale 2.0`, `ease_in 300ms`, `hold 1500ms`, `ease_out 400ms`.
- Cada clique vira um segmento centrado no ponto (coords do `metadata.json`
  normalizadas pelo `rect` da fonte).
- **Merge:** se um clique ocorre antes do `ease_out` do segmento anterior
  começar/terminar (janela = `hold + ease_out` desde o último clique), funde:
  estende `end_ms` e adiciona o novo ponto como alvo de pan (o sampler desliza
  o `center` entre os pontos). Sequências de cliques viram um zoom contínuo.

## Preview ao vivo (front, sem render)

- Elemento `<video>` tocando o `REC-<ts>.mp4`.
- Loop `requestAnimationFrame`: lê `currentTime`, chama `zoom_at` (TS), aplica
  `transform: scale(z)` com `transform-origin: cx% cy%` no contêiner do vídeo
  (com o mesmo clamp do modelo). Sem ffmpeg, sem re-render — só transform CSS.
- Timeline embaixo: scrubber + playhead + barras dos segmentos; cliques como
  marcadores. Arrastar bordas = início/duração; clicar = selecionar; arrastar na
  imagem = mover `target` do segmento selecionado.

## Export (Rust + ffmpeg, "assa" o mesmo modelo)

- Comando `export_with_zoom(video_path, zoom_model, out_path)`.
- Filtro: `scale=w='iw*Z':h='ih*Z':eval=frame` seguido de `crop` centrado em
  `center(t)`, com `Z` e o centro como expressões em `t` derivadas dos segmentos
  (piecewise `if(between(t,...))` + `smoothstep`), saída na resolução original.
- Progresso reportado à UI via eventos Tauri (parse do stderr do ffmpeg).
- **Risco conhecido (o maior da F2):** montar o filtrograma exato e bater a
  paridade com o preview. O plano inclui (a) builder do filtro testado por
  unidade e (b) um render de teste no smoke conferindo que o assado bate com o
  preview num frame de referência.

## Componentes (unidades isoladas)

### Rust (`src-tauri/src/`)
- `capture/input_mac.rs` — CGEventTap nativo de mouse (macOS).
- `capture/input.rs` — dispatch por `#[cfg]` (mac → tap nativo; outros → rdev).
- `model/zoom.rs` — `ZoomSegment`/`ZoomModel`, easing, `zoom_at` (sampler), clamp. (puro, testado)
- `zoom/generate.rs` — `events → Vec<ZoomSegment>` com merge. (puro, testado)
- `zoom/export.rs` — builder do filtro ffmpeg + execução com progresso. (builder testado; render smoke)
- `commands.rs` — `load_recording`, `save_zoom`, `export_with_zoom`.

### Front (`src/`)
- `lib/zoom.ts` — espelho de `zoom_at`/easing/clamp (paridade com Rust). (vitest)
- `state/useEditor.ts` — estado do editor.
- `components/EditorView.tsx`, `PreviewCanvas.tsx`, `Timeline.tsx`, `SegmentInspector.tsx`.

## Comandos Tauri (interface UI ↔ Rust)

- `load_recording(video_path) -> { metadata, zoom_model }` — devolve o
  `metadata.json` e o `REC-<ts>.zoom.json` salvo, ou um ZoomModel gerado dos
  events se ainda não existir.
- `save_zoom(video_path, zoom_model) -> ()` — grava `REC-<ts>.zoom.json`.
- `export_with_zoom(video_path, zoom_model, out_path) -> ()` — render landscape;
  emite eventos de progresso.

## Fluxo de dados

```
lista de gravações → abrir editor(rec)
  invoke load_recording → { metadata, zoom_model (gerado ou salvo) }
  front: preview (sampler TS) + timeline; usuário edita (estado puro)
  invoke save_zoom (autosave/ao sair)
exportar:
  invoke export_with_zoom(video, zoom_model, out) → ffmpeg + progresso → mp4 assado
```

## Persistência

- `REC-<ts>.zoom.json` ao lado do `.mp4`/`.metadata.json`. Versionado
  (`version: 1`). Reabrir o editor carrega as edições.

## Tratamento de erros

- Sem permissão de Monitoramento de Entrada (mac) → `events` vazio → editor abre
  sem auto-zooms (usuário pode adicionar manuais); aviso na UI.
- `zoom.json` corrompido → regenera dos events, avisa.
- Falha no ffmpeg de export (status ≠ 0) → erro claro na UI, preserva o `.mp4`
  original.
- `metadata.json` ausente → editor abre só com preview, sem auto-zoom.

## Testes

- **Rust unit:** `zoom_at` (ease-in/hold/ease-out/fora; clamp nas bordas);
  `generate` (1 clique → 1 segmento; cliques próximos → merge; cliques distantes
  → segmentos separados); builder do filtro ffmpeg (assert nas expressões).
- **Paridade:** fixture de `(model, t's)` com valores esperados de
  `scale`/`center`; mesma fixture rodada no Rust e no TS (vitest) — devem bater.
- **TS unit (vitest):** `zoom_at` (sampler) + lógica de timeline (mapear
  tempo↔pixel).
- **Smoke manual (macOS):** gravar clicando → abrir editor → ver auto-zooms →
  preview confere → exportar → o `.mp4` assado bate com o preview; também:
  editar/mover/deletar zoom; sem permissão de input → sem auto-zoom degrada.

## Faseamento interno (sub-entregas testáveis)

1. Captura de clique nativa (mac) → `events` preenchidos (verificável via F1).
2. Modelo + sampler + geração/merge (Rust) + paridade TS.
3. Comandos `load_recording`/`save_zoom` + persistência.
4. Preview ao vivo + timeline + inspector (front).
5. Export com ffmpeg + progresso.

Cada uma roda/testa por si; o plano detalha em tarefas.
