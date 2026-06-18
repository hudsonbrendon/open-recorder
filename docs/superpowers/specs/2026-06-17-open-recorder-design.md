# OpenRecorder — Design (F1: Fundação de Captura)

**Data:** 2026-06-17
**Status:** Aprovado para planejamento
**Escopo deste doc:** Fase 1 (F1). F2–F4 registradas como roadmap no fim.

## Visão

OpenRecorder é um gravador de tela desktop, open source, com foco em dois
diferenciais: **zoom automático no clique do mouse** e **export em 9:16 para
redes sociais** (incluindo split-screen com webcam e preview de como o vídeo
aparece no Instagram/TikTok). Inspirações: cap.so e openscreen.net.

Princípio de produto: app **leve** e com **cara nativa**.

## Decisões travadas (global)

- **Stack:** Tauri 2 + core em Rust + UI em React.
- **Plataforma alvo agora:** **macOS apenas** (universal binary arm64+x86_64,
  distribuição `.dmg`). Codebase mantida **portável** usando crates
  cross-platform; portar para Windows/Linux depois é re-testar, não reescrever.
  Sem CI para Windows/Linux por enquanto.
- **Gravação não-destrutiva (espinha-dorsal):** grava vídeo cru em alta
  resolução + um `metadata.json` com eventos de clique e trajetória do mouse.
  Zoom, recorte 9:16 e posição da webcam são aplicados **no export**, nunca na
  gravação. Gravar é barato; render é sob demanda. É o que torna o editor
  possível.
- **ffmpeg:** empacotado como **binário sidecar** do Tauri, não dependência do
  sistema.
- **Áudio v1:** apenas **microfone** (system audio fica para depois).
- **Commits:** sem co-author ou histórico do Claude.

## Stack / crates

| Função | Crate / ferramenta |
|--------|--------------------|
| Captura de tela | `scap` (cross-platform; usa ScreenCaptureKit no Mac) |
| Captura de áudio (mic) | `cpal` |
| Captura de webcam (F3) | `nokhwa` |
| Eventos de mouse/clique | `rdev` (escuta global) |
| Encode / mux / export | `ffmpeg` (sidecar) |
| Shell desktop / IPC | Tauri 2 |
| UI | React + Vite |

## Faseamento

Cada fase é um sub-projeto sequencial que entrega software usável sozinho.
Cada fase terá seu próprio spec + plano + implementação. **Este doc detalha a
F1**; F2–F4 são roadmap.

| Fase | Entrega | Usável? |
|------|---------|---------|
| **F1 — Fundação de captura** | Grava tela/janela/região + mic → `.mp4` cru + `metadata.json` de cliques. Export landscape simples (passthrough). | Sim: gravador de tela funcional |
| **F2 — Auto-zoom no clique** | Aplica zoom suave nos cliques no export. Editor mínimo para ajustar/remover zooms. | Sim: o diferencial principal |
| **F3 — Webcam overlay** | Captura webcam + compõe como bolha/canto no export. | Sim: tela + rosto |
| **F4 — Export 9:16 + preview social** | Layouts vertical (full / split-screen) + preview com mock IG/TikTok (comentários, UI). | Sim: produto completo |

---

# F1 — Fundação de Captura (detalhe)

## O que a F1 entrega

Gravar tela/janela/região + microfone, produzindo:
1. `REC-<timestamp>.mp4` — vídeo cru (H.264) com áudio do mic muxado.
2. `REC-<timestamp>.metadata.json` — eventos de clique e movimento do mouse.

A metadata é **gravada mas não consumida** na F1 (a F2 a consome). Gravamos
desde já para o formato nascer pronto e evitar retrabalho.

## Componentes (unidades isoladas)

Cada unidade tem uma responsabilidade clara e interface bem definida.

1. **Source enumerator** (Rust) — lista displays e janelas abertas via `scap`.
   Região = sub-retângulo de um display escolhido na UI.
2. **Capture engine** (Rust) — `scap` entrega frames crus → pipe para o
   `ffmpeg` (stdin) → encoda H.264 em arquivo de vídeo temporário.
3. **Audio recorder** (Rust) — `cpal` captura o mic escolhido → arquivo de
   áudio temporário.
4. **Input recorder** (Rust) — `rdev` escuta cliques + movimento do mouse
   globalmente, com timestamp relativo ao início da gravação → buffer em
   memória → `metadata.json` no stop.
5. **Muxer/finalizer** (Rust) — no stop: junta vídeo + áudio via `ffmpeg` →
   `.mp4` final; serializa o buffer → `metadata.json` ao lado.
6. **UI React** — telas:
   - (a) seletor de fonte (grid de telas/janelas + botão de seleção de região),
   - (b) seletor de microfone,
   - (c) controles start/stop com timer,
   - (d) lista de gravações feitas (abre a pasta).

## Fluxo de dados

```
UI: escolhe fonte + mic → invoke start_recording(source, mic_id)
  Rust dispara 3 threads paralelas:
    - capture: scap frames → ffmpeg stdin → video.tmp
    - audio:   cpal → audio.tmp
    - input:   rdev → buffer[] de {t_ms, x, y, tipo}
UI: invoke stop_recording()
  Rust: para threads → ffmpeg mux(video.tmp, audio.tmp) → REC-<ts>.mp4
        serializa buffer → REC-<ts>.metadata.json
        retorna {video_path, metadata_path} → UI mostra na lista
```

## Formato do `metadata.json`

Versionado para evoluir sem quebrar. Espinha para a F2 nascer pronta.

```json
{
  "version": 1,
  "recording": { "width": 2560, "height": 1440, "fps": 30, "duration_ms": 18450 },
  "source": { "type": "display|window|region", "id": "...", "rect": [0, 0, 2560, 1440] },
  "events": [
    { "t_ms": 1200, "type": "click", "x": 840, "y": 410, "button": "left" },
    { "t_ms": 1200, "type": "move",  "x": 840, "y": 410 }
  ]
}
```

Coordenadas dos eventos são relativas ao retângulo da fonte capturada (origem
no canto superior-esquerdo da fonte), não à tela física — para a F2 mapear
direto sobre o vídeo.

## Comandos Tauri (interface UI ↔ Rust)

- `list_sources() -> { displays: [...], windows: [...] }`
- `list_microphones() -> [{ id, name }]`
- `start_recording({ source, mic_id }) -> { recording_id }`
- `stop_recording() -> { video_path, metadata_path, duration_ms }`
- `list_recordings() -> [{ id, video_path, created_at, duration_ms }]`
- `reveal_in_finder(path)` — abre a pasta da gravação.

## Permissões (macOS)

| Permissão | Para quê | Comportamento sem ela |
|-----------|----------|------------------------|
| Screen Recording (TCC) | captura de tela | bloqueia gravação; tela explicativa + abrir Preferências; exige restart do app após conceder (avisar na UI) |
| Microphone (TCC) | mic | bloqueia áudio; oferece gravar sem áudio |
| Accessibility | `rdev` ler cliques/mouse | **degrada**: grava vídeo normal, avisa "cliques não serão registrados" |

Na primeira gravação, um **permission check** verifica cada permissão; se faltar,
mostra tela explicativa com botão que abre o painel certo das Preferências do
Sistema.

## Tratamento de erros

Cada erro vira estado visível na UI — nunca crash.

- `ffmpeg` sidecar ausente/corrompido → erro claro no start.
- Nenhum display/mic encontrado → desabilita start, mensagem.
- Disco cheio / falha de escrita → aborta gravação, preserva temp se possível.
- Thread de captura morre no meio → para tudo, salva o que tem, avisa.
- Janela escolhida fechada durante gravação → para e finaliza com o que gravou.

## Testes

- **Rust unit:** round-trip de serialização do `metadata.json`; cálculo de
  timestamps relativos; parsing do rect da fonte; montagem do comando ffmpeg
  (assert nos args, sem rodar).
- **Rust unit (input):** alimenta eventos sintéticos no recorder → assert no
  buffer/JSON.
- **Integração (muxer):** fixtures pequenos de vídeo+áudio → roda ffmpeg real →
  assert que o `.mp4` sai válido (via ffprobe).
- **Smoke manual (macOS):** checklist de gravação real (tela/janela/região +
  mic + permissões). Captura não é confiável headless; documentamos o checklist.
- TDD onde dá (serialização, comando ffmpeg, input buffer); smoke manual onde
  captura exige hardware/permissão real.

---

# Roadmap (F2–F4) — não detalhado aqui

- **F2 — Auto-zoom no clique:** consumir `metadata.json`; gerar keyframes de
  zoom (ease-in/out) centrados nos cliques; aplicar no export via filtros
  ffmpeg (zoompan/crop+scale). Editor mínimo em React para ajustar intensidade,
  duração e remover zooms.
- **F3 — Webcam overlay:** captura via `nokhwa`; compor como bolha/canto
  configurável no export.
- **F4 — Export 9:16 + preview social:** layouts vertical full e split-screen
  (tela + webcam); preview com mock de UI do Instagram/TikTok (área segura,
  comentários, botões) para posicionar elementos antes de postar.

Cada um vira seu próprio spec quando chegar a hora.
