# OpenRecorder — Design (F1: Fundação de Captura)

**Data:** 2026-06-17
**Status:** Aprovado para planejamento
**Escopo deste doc:** Fase 1 (F1). F2–F4 registradas como roadmap no fim.

## Visão

OpenRecorder é um gravador de tela desktop, open source, com foco em dois
diferenciais: **zoom automático no clique do mouse** e **export em 9:16 para
redes sociais** (incluindo split-screen com webcam e preview de como o vídeo
aparece no Instagram/TikTok). Inspirações: cap.so e openscreen.net.

Princípio de produto: app **leve** e com **cara nativa** (referência de leveza:
OpenWhisper).

## Decisões travadas (global)

- **Plataforma:** **macOS apenas — nativo, foco total.** Sem ambição
  cross-platform. Captura de tela/áudio/input é profundamente específica de SO;
  um único alvo mantém o código simples, leve e genuinamente nativo.
- **Stack:** **Swift + SwiftUI** (AppKit onde necessário), projeto Xcode.
  Distribuição `.app` / `.dmg` (notarização depois).
- **APIs nativas (sem dependências externas, sem sidecar):**
  - Captura de tela/janela: **ScreenCaptureKit**.
  - Encode e export: **AVFoundation**.
  - Microfone: **AVFoundation** (`AVCaptureSession`).
  - Eventos de mouse/clique: **CGEventTap** (Quartz Event Services).
- **Gravação não-destrutiva (espinha-dorsal):** grava vídeo cru em alta
  resolução + um `metadata.json` com eventos de clique e trajetória do mouse.
  Zoom, recorte 9:16 e posição da webcam são aplicados **no export**, nunca na
  gravação. Gravar é barato; render é sob demanda. É o que torna o editor
  possível.
- **Áudio v1:** apenas **microfone** (system audio fica para depois; viria via
  ScreenCaptureKit).
- **Commits:** sem co-author ou histórico do Claude.

## Stack / frameworks

| Função | API nativa Apple |
|--------|------------------|
| Captura tela/janela/região | ScreenCaptureKit (`SCStream`, `SCShareableContent`, `SCStreamConfiguration`) |
| Encode → arquivo | AVFoundation (`AVAssetWriter`, H.264/HEVC) |
| Microfone | AVFoundation (`AVCaptureSession` / `AVCaptureDevice`) |
| Eventos de mouse/clique | CGEventTap (Quartz Event Services) |
| Captura de webcam (F3) | AVFoundation (`AVCaptureDevice`) |
| Export com zoom/composição (F2+) | AVFoundation (`AVVideoComposition`) + Core Image / Metal |
| UI | SwiftUI (+ AppKit onde necessário) |

Nenhum binário externo empacotado: ScreenCaptureKit entrega `CMSampleBuffer`s,
AVAssetWriter escreve `.mp4` direto. Isso mantém o app leve.

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
1. `REC-<timestamp>.mp4` — vídeo cru (H.264/HEVC) com áudio do mic.
2. `REC-<timestamp>.metadata.json` — eventos de clique e movimento do mouse.

A metadata é **gravada mas não consumida** na F1 (a F2 a consome). Gravamos
desde já para o formato nascer pronto e evitar retrabalho.

## Componentes (unidades isoladas)

Cada unidade tem uma responsabilidade clara e interface bem definida.

1. **SourceEnumerator** (Swift) — lista displays e janelas via
   `SCShareableContent`. Região = retângulo de crop sobre um display escolhido
   na UI (via `SCStreamConfiguration.sourceRect`).
2. **CaptureEngine** (Swift) — configura `SCStream` → recebe `CMSampleBuffer`s
   de vídeo no callback → grava via `AVAssetWriter` (trilha de vídeo H.264/HEVC)
   em arquivo temporário.
3. **AudioRecorder** (Swift) — captura o mic escolhido via `AVCaptureSession` →
   `CMSampleBuffer`s de áudio → trilha de áudio no mesmo `AVAssetWriter`.
4. **InputRecorder** (Swift) — `CGEventTap` escuta cliques + movimento do mouse
   globalmente, com timestamp relativo ao início da gravação → buffer em
   memória → `metadata.json` no stop.
5. **RecordingFinalizer** (Swift) — no stop: finaliza o `AVAssetWriter` →
   `.mp4`; serializa o buffer de eventos (`Codable`) → `metadata.json` ao lado.
6. **UI SwiftUI** — telas:
   - (a) seletor de fonte (grid de telas/janelas + seleção de região),
   - (b) seletor de microfone,
   - (c) controles start/stop com timer,
   - (d) lista de gravações feitas (revela no Finder).

## Fluxo de dados

```
UI: escolhe fonte + mic → CaptureCoordinator.start(source, micID)
  Inicia em paralelo:
    - CaptureEngine:  SCStream → CMSampleBuffer(vídeo) → AVAssetWriter
    - AudioRecorder:  AVCaptureSession → CMSampleBuffer(áudio) → AVAssetWriter
    - InputRecorder:  CGEventTap → buffer[] de {t_ms, x, y, tipo}
UI: CaptureCoordinator.stop()
  Para streams → AVAssetWriter.finishWriting() → REC-<ts>.mp4
  Serializa buffer → REC-<ts>.metadata.json
  Retorna {videoURL, metadataURL, durationMs} → UI mostra na lista
```

`AVAssetWriter` recebe vídeo e áudio das duas fontes na mesma sessão (dois
`AVAssetWriterInput`), produzindo um único `.mp4` muxado — sem passo de mux
separado.

## Formato do `metadata.json`

Versionado para evoluir sem quebrar. Espinha para a F2 nascer pronta.
Serializado/desserializado via `Codable`.

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

## Permissões (macOS)

| Permissão | Para quê | Comportamento sem ela |
|-----------|----------|------------------------|
| Screen Recording (TCC) | ScreenCaptureKit | bloqueia gravação; tela explicativa + abrir Preferências; pode exigir restart do app após conceder (avisar na UI) |
| Microphone (TCC) | mic via AVCaptureSession | bloqueia áudio; oferece gravar sem áudio |
| Input Monitoring / Accessibility | `CGEventTap` ler cliques/mouse | **degrada**: grava vídeo normal, avisa "cliques não serão registrados" |

Na primeira gravação, um **permission check** verifica cada permissão (APIs
nativas: `SCShareableContent`, `AVCaptureDevice.authorizationStatus`,
`CGPreflightListenEventAccess` / `AXIsProcessTrusted`); se faltar, mostra tela
explicativa com botão que abre o painel certo de Ajustes do Sistema.

## Tratamento de erros

Cada erro vira estado visível na UI — nunca crash.

- `SCStream` falha ao iniciar (permissão/recurso) → erro claro no start.
- Nenhum display/mic encontrado → desabilita start, mensagem.
- Disco cheio / falha de escrita do `AVAssetWriter` → aborta, preserva o que der.
- Stream interrompido no meio (`SCStreamDelegate` erro) → para tudo, finaliza
  com o que tem, avisa.
- Janela escolhida fechada durante gravação → para e finaliza com o que gravou.

## Testes

- **XCTest (unit):** round-trip `Codable` do `metadata.json`; cálculo de
  timestamps relativos; mapeamento de coordenadas para o rect da fonte; lógica
  de configuração do `SCStreamConfiguration` (asserts em valores, sem stream
  real).
- **XCTest (input):** alimenta eventos sintéticos no `InputRecorder` → assert no
  buffer/JSON (com a captura `CGEventTap` isolada atrás de um protocolo
  mockável).
- **Integração (writer):** alimenta `CMSampleBuffer`s de fixture no
  `AVAssetWriter` → assert que o `.mp4` resultante é válido e legível
  (`AVAsset`).
- **Smoke manual (macOS):** checklist de gravação real (tela/janela/região +
  mic + permissões). Captura não é confiável headless; documentamos o checklist.
- TDD onde dá (serialização, config, input buffer); smoke manual onde captura
  exige hardware/permissão real.

---

# Roadmap (F2–F4) — não detalhado aqui

- **F2 — Auto-zoom no clique:** consumir `metadata.json`; gerar keyframes de
  zoom (ease-in/out) centrados nos cliques; aplicar no export via
  `AVVideoComposition` + Core Image/Metal (transform de escala/posição por
  frame). Editor mínimo em SwiftUI para ajustar intensidade, duração e remover
  zooms.
- **F3 — Webcam overlay:** captura via `AVCaptureDevice`; compor como
  bolha/canto configurável no export (camada extra na `AVVideoComposition`).
- **F4 — Export 9:16 + preview social:** layouts vertical full e split-screen
  (tela + webcam); preview com mock de UI do Instagram/TikTok (área segura,
  comentários, botões) para posicionar elementos antes de postar.

Cada um vira seu próprio spec quando chegar a hora.
