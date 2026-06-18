# OpenRecorder F1 (Fundação de Captura) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Construir um gravador de tela macOS nativo que captura tela/janela/região + microfone, produzindo um `.mp4` cru e um `metadata.json` com eventos de clique/mouse.

**Architecture:** App SwiftUI nativo (macOS). ScreenCaptureKit captura frames de vídeo num `SCStream`; `AVCaptureSession` captura o microfone; ambos alimentam um único `AVAssetWriter` (uma trilha de vídeo + uma de áudio) → `.mp4`. Um `CGEventTap` registra cliques/movimento do mouse em memória, serializados em `metadata.json` no stop. Gravação não-destrutiva: a metadata é gravada mas só consumida na F2.

**Tech Stack:** Swift 5.9+, SwiftUI, ScreenCaptureKit, AVFoundation, Quartz Event Services (CGEventTap). Projeto gerado por XcodeGen. Testes via XCTest (`xcodebuild test`).

## Global Constraints

- Plataforma: **macOS 13.0+** apenas (ScreenCaptureKit exige 12.3+; usamos APIs de 13.0). Nativo, sem cross-platform, sem dependências externas além do XcodeGen (build-time).
- Linguagem: **Swift**, UI em **SwiftUI** (AppKit só onde necessário).
- Sem binários empacotados: **sem ffmpeg, sem Tauri, sem crates**. Encode via AVFoundation.
- Gravação **não-destrutiva**: vídeo cru + `metadata.json`; efeitos só no export (F2+).
- Áudio v1: **apenas microfone**.
- Formato `metadata.json`: **versionado** (`version: 1`); coordenadas relativas ao retângulo da fonte (origem no canto superior-esquerdo).
- Commits: **sem co-author/histórico do Claude**. Mensagens em inglês, formato `tipo: descrição`.
- Bundle ID: `com.openrecorder.app`. Nome do produto: `OpenRecorder`.

## File Structure

```
open-recorder/
├── project.yml                              # XcodeGen: define app + test targets
├── Sources/OpenRecorder/
│   ├── App.swift                            # @main, WindowGroup
│   ├── Info.plist                           # usage descriptions
│   ├── OpenRecorder.entitlements            # hardened runtime, sem sandbox
│   ├── Model/
│   │   ├── RecordingMetadata.swift          # structs Codable do metadata.json
│   │   ├── CaptureSource.swift              # enum/struct da fonte escolhida
│   │   └── SourceCoordinateMapper.swift     # mapeia coords de tela → fonte
│   ├── Capture/
│   │   ├── StreamConfigBuilder.swift        # CaptureSource → SCStreamConfiguration
│   │   ├── SourceEnumerator.swift           # SCShareableContent → listas
│   │   ├── VideoWriter.swift                # wrapper AVAssetWriter
│   │   ├── CaptureEngine.swift              # SCStream → VideoWriter
│   │   ├── AudioRecorder.swift              # AVCaptureSession mic → VideoWriter
│   │   ├── InputRecorder.swift              # protocolo + CGEventTap
│   │   ├── RecordingFinalizer.swift         # escreve metadata.json
│   │   └── RecordingCoordinator.swift       # orquestra start/stop
│   ├── Permissions/
│   │   └── PermissionService.swift          # checa/solicita TCC
│   └── UI/
│       ├── ContentView.swift                # raiz
│       ├── SourcePickerView.swift
│       ├── RecordingControlsView.swift
│       ├── RecordingsListView.swift
│       └── RecorderViewModel.swift          # @MainActor, liga UI ↔ coordinator
├── Tests/OpenRecorderTests/
│   ├── RecordingMetadataTests.swift
│   ├── SourceCoordinateMapperTests.swift
│   ├── StreamConfigBuilderTests.swift
│   ├── InputRecorderTests.swift
│   └── VideoWriterTests.swift
└── docs/
    └── SMOKE-TEST.md                        # checklist manual macOS
```

Cada arquivo tem uma responsabilidade. Lógica pura (Model/, StreamConfigBuilder, InputRecorder buffer, VideoWriter) é testável; partes ligadas a hardware/permissão (CaptureEngine, AudioRecorder, SourceEnumerator) têm smoke manual.

---

## Task 0: Pré-requisitos (executar uma vez, fora do TDD)

**Não é tarefa de código** — verificação de ambiente. O executor roda isto antes da Task 1.

- [ ] **Passo 1: Verificar Xcode e ferramentas**

Run:
```bash
xcodebuild -version && swift --version
```
Expected: imprime versão do Xcode (15+) e do Swift (5.9+). Se faltar, instalar Xcode da App Store + `xcode-select --install`.

- [ ] **Passo 2: Instalar XcodeGen**

Run:
```bash
brew install xcodegen && xcodegen --version
```
Expected: imprime a versão do XcodeGen (2.x). 

---

## Task 1: Scaffold do projeto (app lança + teste roda)

**Files:**
- Create: `project.yml`
- Create: `Sources/OpenRecorder/App.swift`
- Create: `Sources/OpenRecorder/UI/ContentView.swift`
- Create: `Sources/OpenRecorder/Info.plist`
- Create: `Sources/OpenRecorder/OpenRecorder.entitlements`
- Create: `Tests/OpenRecorderTests/RecordingMetadataTests.swift` (placeholder de sanidade)

**Interfaces:**
- Consumes: nada.
- Produces: scheme `OpenRecorder` buildável e testável via `xcodebuild`. App vazio que abre uma janela.

- [ ] **Step 1: Escrever o `project.yml`**

```yaml
name: OpenRecorder
options:
  bundleIdPrefix: com.openrecorder
  deploymentTarget:
    macOS: "13.0"
  createIntermediateGroups: true
settings:
  base:
    SWIFT_VERSION: "5.9"
    MARKETING_VERSION: "0.1.0"
    CURRENT_PROJECT_VERSION: "1"
    PRODUCT_NAME: OpenRecorder
    ENABLE_HARDENED_RUNTIME: YES
    CODE_SIGN_STYLE: Automatic
targets:
  OpenRecorder:
    type: application
    platform: macOS
    sources:
      - Sources/OpenRecorder
    settings:
      base:
        PRODUCT_BUNDLE_IDENTIFIER: com.openrecorder.app
        INFOPLIST_FILE: Sources/OpenRecorder/Info.plist
        CODE_SIGN_ENTITLEMENTS: Sources/OpenRecorder/OpenRecorder.entitlements
        GENERATE_INFOPLIST_FILE: NO
  OpenRecorderTests:
    type: bundle.unit-test
    platform: macOS
    sources:
      - Tests/OpenRecorderTests
    dependencies:
      - target: OpenRecorder
    settings:
      base:
        BUNDLE_LOADER: "$(TEST_HOST)"
        TEST_HOST: "$(BUILT_PRODUCTS_DIR)/OpenRecorder.app/Contents/MacOS/OpenRecorder"
schemes:
  OpenRecorder:
    build:
      targets:
        OpenRecorder: all
        OpenRecorderTests: [test]
    test:
      targets:
        - OpenRecorderTests
```

- [ ] **Step 2: Escrever `Info.plist`**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>OpenRecorder</string>
    <key>CFBundleIdentifier</key>
    <string>com.openrecorder.app</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>NSMicrophoneUsageDescription</key>
    <string>OpenRecorder usa o microfone para gravar narração junto da tela.</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
```

- [ ] **Step 3: Escrever `OpenRecorder.entitlements`**

App Sandbox **desligado**: o `CGEventTap` global e a captura de tela não funcionam sob sandbox para uso geral. Hardened runtime ligado.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <false/>
    <key>com.apple.security.device.audio-input</key>
    <true/>
</dict>
</plist>
```

- [ ] **Step 4: Escrever `App.swift`**

```swift
import SwiftUI

@main
struct OpenRecorderApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
                .frame(minWidth: 720, minHeight: 480)
        }
        .windowResizability(.contentMinSize)
    }
}
```

- [ ] **Step 5: Escrever `ContentView.swift` (placeholder)**

```swift
import SwiftUI

struct ContentView: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "record.circle")
                .font(.system(size: 48))
                .foregroundStyle(.red)
            Text("OpenRecorder")
                .font(.title.bold())
            Text("Fundação de captura")
                .foregroundStyle(.secondary)
        }
        .padding()
    }
}
```

- [ ] **Step 6: Escrever teste de sanidade `RecordingMetadataTests.swift`**

```swift
import XCTest

final class RecordingMetadataTests: XCTestCase {
    func test_sanity() {
        XCTAssertEqual(1 + 1, 2)
    }
}
```

- [ ] **Step 7: Gerar o projeto**

Run:
```bash
cd ~/Github/open-recorder && xcodegen generate
```
Expected: `Created project at .../OpenRecorder.xcodeproj`.

- [ ] **Step 8: Buildar e rodar os testes**

Run:
```bash
cd ~/Github/open-recorder && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: `** TEST SUCCEEDED **`, com `test_sanity` passando.

- [ ] **Step 9: Ignorar artefatos gerados**

Adicionar ao `.gitignore`:
```
/OpenRecorder.xcodeproj
/build
*.xcuserstate
DerivedData/
```
O `.xcodeproj` é gerado pelo XcodeGen — não versionar.

- [ ] **Step 10: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: scaffold macOS app with XcodeGen and test target"
```

---

## Task 2: Modelo de metadata (Codable)

**Files:**
- Create: `Sources/OpenRecorder/Model/RecordingMetadata.swift`
- Test: `Tests/OpenRecorderTests/RecordingMetadataTests.swift` (substitui o placeholder)

**Interfaces:**
- Consumes: nada.
- Produces:
  - `struct RecordingMetadata: Codable, Equatable` com `version: Int`, `recording: RecordingInfo`, `source: SourceInfo`, `events: [InputEvent]`.
  - `struct RecordingInfo: Codable, Equatable` com `width: Int`, `height: Int`, `fps: Int`, `durationMs: Int`.
  - `struct SourceInfo: Codable, Equatable` com `type: String`, `id: String`, `rect: [Int]` (exatamente 4: x,y,w,h).
  - `struct InputEvent: Codable, Equatable` com `tMs: Int`, `type: String`, `x: Int`, `y: Int`, `button: String?`.
  - JSON usa snake_case (`duration_ms`, `t_ms`) via `CodingKeys`.

- [ ] **Step 1: Escrever o teste de round-trip falhando**

```swift
import XCTest
@testable import OpenRecorder

final class RecordingMetadataTests: XCTestCase {
    func test_encodesToSnakeCaseJSON() throws {
        let meta = RecordingMetadata(
            version: 1,
            recording: RecordingInfo(width: 2560, height: 1440, fps: 30, durationMs: 18450),
            source: SourceInfo(type: "display", id: "1", rect: [0, 0, 2560, 1440]),
            events: [
                InputEvent(tMs: 1200, type: "click", x: 840, y: 410, button: "left"),
                InputEvent(tMs: 1200, type: "move", x: 840, y: 410, button: nil),
            ]
        )
        let encoder = JSONEncoder()
        let data = try encoder.encode(meta)
        let json = String(data: data, encoding: .utf8)!
        XCTAssertTrue(json.contains("\"duration_ms\":18450"))
        XCTAssertTrue(json.contains("\"t_ms\":1200"))
        XCTAssertTrue(json.contains("\"version\":1"))
    }

    func test_roundTripPreservesValues() throws {
        let meta = RecordingMetadata(
            version: 1,
            recording: RecordingInfo(width: 100, height: 200, fps: 60, durationMs: 5000),
            source: SourceInfo(type: "window", id: "abc", rect: [10, 20, 30, 40]),
            events: [InputEvent(tMs: 0, type: "click", x: 1, y: 2, button: "right")]
        )
        let data = try JSONEncoder().encode(meta)
        let decoded = try JSONDecoder().decode(RecordingMetadata.self, from: data)
        XCTAssertEqual(decoded, meta)
    }
}
```

- [ ] **Step 2: Rodar o teste pra ver falhar**

Run:
```bash
cd ~/Github/open-recorder && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: FAIL — `cannot find 'RecordingMetadata' in scope`.

- [ ] **Step 3: Implementar o modelo**

```swift
import Foundation

struct RecordingMetadata: Codable, Equatable {
    var version: Int
    var recording: RecordingInfo
    var source: SourceInfo
    var events: [InputEvent]
}

struct RecordingInfo: Codable, Equatable {
    var width: Int
    var height: Int
    var fps: Int
    var durationMs: Int

    enum CodingKeys: String, CodingKey {
        case width, height, fps
        case durationMs = "duration_ms"
    }
}

struct SourceInfo: Codable, Equatable {
    var type: String
    var id: String
    var rect: [Int]
}

struct InputEvent: Codable, Equatable {
    var tMs: Int
    var type: String
    var x: Int
    var y: Int
    var button: String?

    enum CodingKeys: String, CodingKey {
        case tMs = "t_ms"
        case type, x, y, button
    }
}
```

- [ ] **Step 4: Rodar o teste pra ver passar**

Run:
```bash
cd ~/Github/open-recorder && xcodegen generate && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: `** TEST SUCCEEDED **`. (Roda `xcodegen generate` sempre que adicionar/remover arquivos.)

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add versioned RecordingMetadata model with snake_case JSON"
```

---

## Task 3: Mapeamento de coordenadas (tela → fonte)

Eventos do `CGEventTap` chegam em coordenadas globais da tela (origem no canto **inferior**-esquerdo no sistema do Quartz para alguns APIs, mas `CGEvent.location` usa origem no canto **superior**-esquerdo do display principal). Precisamos converter para coordenadas relativas ao retângulo da fonte capturada, com origem no canto superior-esquerdo da fonte. Pontos fora da fonte são descartados.

**Files:**
- Create: `Sources/OpenRecorder/Model/CaptureSource.swift`
- Create: `Sources/OpenRecorder/Model/SourceCoordinateMapper.swift`
- Test: `Tests/OpenRecorderTests/SourceCoordinateMapperTests.swift`

**Interfaces:**
- Consumes: nada.
- Produces:
  - `enum CaptureSourceKind: String { case display, window, region }`
  - `struct CaptureSource: Equatable` com `kind: CaptureSourceKind`, `id: String`, `rect: CGRect` (em coordenadas globais de tela, origem superior-esquerda), `displayID: CGDirectDisplayID`.
  - `struct SourceCoordinateMapper` com `init(sourceRect: CGRect)` e `func map(globalPoint: CGPoint) -> CGPoint?` retornando ponto relativo (origem superior-esquerda da fonte) ou `nil` se fora.

- [ ] **Step 1: Escrever os testes falhando**

```swift
import XCTest
import CoreGraphics
@testable import OpenRecorder

final class SourceCoordinateMapperTests: XCTestCase {
    func test_mapsPointInsideToRelative() {
        let mapper = SourceCoordinateMapper(sourceRect: CGRect(x: 100, y: 50, width: 800, height: 600))
        let result = mapper.map(globalPoint: CGPoint(x: 150, y: 90))
        XCTAssertEqual(result, CGPoint(x: 50, y: 40))
    }

    func test_mapsTopLeftCornerToZero() {
        let mapper = SourceCoordinateMapper(sourceRect: CGRect(x: 100, y: 50, width: 800, height: 600))
        XCTAssertEqual(mapper.map(globalPoint: CGPoint(x: 100, y: 50)), CGPoint(x: 0, y: 0))
    }

    func test_returnsNilWhenOutsideLeft() {
        let mapper = SourceCoordinateMapper(sourceRect: CGRect(x: 100, y: 50, width: 800, height: 600))
        XCTAssertNil(mapper.map(globalPoint: CGPoint(x: 99, y: 90)))
    }

    func test_returnsNilWhenOutsideBottom() {
        let mapper = SourceCoordinateMapper(sourceRect: CGRect(x: 100, y: 50, width: 800, height: 600))
        XCTAssertNil(mapper.map(globalPoint: CGPoint(x: 150, y: 651)))
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run:
```bash
cd ~/Github/open-recorder && xcodegen generate && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: FAIL — `cannot find 'SourceCoordinateMapper' in scope`.

- [ ] **Step 3: Implementar `CaptureSource`**

```swift
import CoreGraphics

enum CaptureSourceKind: String, Equatable {
    case display
    case window
    case region
}

struct CaptureSource: Equatable {
    var kind: CaptureSourceKind
    var id: String
    var rect: CGRect           // global, origem superior-esquerda
    var displayID: CGDirectDisplayID
}
```

- [ ] **Step 4: Implementar `SourceCoordinateMapper`**

```swift
import CoreGraphics

struct SourceCoordinateMapper {
    let sourceRect: CGRect

    init(sourceRect: CGRect) {
        self.sourceRect = sourceRect
    }

    /// Converte um ponto global (origem superior-esquerda do display principal)
    /// para coordenadas relativas à fonte. Retorna nil se cair fora da fonte.
    func map(globalPoint: CGPoint) -> CGPoint? {
        let relX = globalPoint.x - sourceRect.origin.x
        let relY = globalPoint.y - sourceRect.origin.y
        guard relX >= 0, relY >= 0, relX <= sourceRect.width, relY <= sourceRect.height else {
            return nil
        }
        return CGPoint(x: relX, y: relY)
    }
}
```

- [ ] **Step 5: Rodar pra ver passar**

Run:
```bash
cd ~/Github/open-recorder && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: `** TEST SUCCEEDED **`.

- [ ] **Step 6: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add CaptureSource and screen-to-source coordinate mapper"
```

---

## Task 4: Builder de `SCStreamConfiguration`

Isola a lógica pura de transformar uma `CaptureSource` + opções nos valores numéricos da configuração do stream. Testável sem criar stream real.

**Files:**
- Create: `Sources/OpenRecorder/Capture/StreamConfigBuilder.swift`
- Test: `Tests/OpenRecorderTests/StreamConfigBuilderTests.swift`

**Interfaces:**
- Consumes: `CaptureSource` (Task 3).
- Produces:
  - `struct StreamConfigValues: Equatable` com `width: Int`, `height: Int`, `fps: Int`, `sourceRect: CGRect`, `showsCursor: Bool`.
  - `enum StreamConfigBuilder { static func values(for source: CaptureSource, fps: Int, showsCursor: Bool) -> StreamConfigValues }`
  - `func makeConfiguration(_ values: StreamConfigValues) -> SCStreamConfiguration` (aplica os valores num objeto real; não testado por unit).

- [ ] **Step 1: Escrever os testes falhando**

```swift
import XCTest
import CoreGraphics
@testable import OpenRecorder

final class StreamConfigBuilderTests: XCTestCase {
    func test_displayUsesFullRectAndSize() {
        let source = CaptureSource(kind: .display, id: "1",
                                   rect: CGRect(x: 0, y: 0, width: 2560, height: 1440),
                                   displayID: 1)
        let v = StreamConfigBuilder.values(for: source, fps: 30, showsCursor: true)
        XCTAssertEqual(v.width, 2560)
        XCTAssertEqual(v.height, 1440)
        XCTAssertEqual(v.fps, 30)
        XCTAssertEqual(v.sourceRect, CGRect(x: 0, y: 0, width: 2560, height: 1440))
        XCTAssertTrue(v.showsCursor)
    }

    func test_regionUsesRectSizeForOutput() {
        let source = CaptureSource(kind: .region, id: "1",
                                   rect: CGRect(x: 100, y: 100, width: 640, height: 480),
                                   displayID: 1)
        let v = StreamConfigBuilder.values(for: source, fps: 60, showsCursor: false)
        XCTAssertEqual(v.width, 640)
        XCTAssertEqual(v.height, 480)
        XCTAssertEqual(v.fps, 60)
        XCTAssertEqual(v.sourceRect, CGRect(x: 100, y: 100, width: 640, height: 480))
        XCTAssertFalse(v.showsCursor)
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run:
```bash
cd ~/Github/open-recorder && xcodegen generate && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: FAIL — `cannot find 'StreamConfigBuilder' in scope`.

- [ ] **Step 3: Implementar o builder**

```swift
import ScreenCaptureKit
import CoreGraphics

struct StreamConfigValues: Equatable {
    var width: Int
    var height: Int
    var fps: Int
    var sourceRect: CGRect
    var showsCursor: Bool
}

enum StreamConfigBuilder {
    static func values(for source: CaptureSource, fps: Int, showsCursor: Bool) -> StreamConfigValues {
        StreamConfigValues(
            width: Int(source.rect.width),
            height: Int(source.rect.height),
            fps: fps,
            sourceRect: source.rect,
            showsCursor: showsCursor
        )
    }

    static func makeConfiguration(_ v: StreamConfigValues) -> SCStreamConfiguration {
        let config = SCStreamConfiguration()
        config.width = v.width
        config.height = v.height
        config.minimumFrameInterval = CMTime(value: 1, timescale: CMTimeScale(v.fps))
        config.pixelFormat = kCVPixelFormatType_32BGRA
        config.showsCursor = v.showsCursor
        config.queueDepth = 6
        return config
    }
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run:
```bash
cd ~/Github/open-recorder && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: `** TEST SUCCEEDED **`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add SCStreamConfiguration builder with pure value logic"
```

---

## Task 5: InputRecorder (buffer de eventos, testável; CGEventTap real)

A captura real via `CGEventTap` fica atrás de um método de injeção, para testar o buffer/serialização com eventos sintéticos. O tap real só é exercido no smoke manual.

**Files:**
- Create: `Sources/OpenRecorder/Capture/InputRecorder.swift`
- Test: `Tests/OpenRecorderTests/InputRecorderTests.swift`

**Interfaces:**
- Consumes: `SourceCoordinateMapper` (Task 3), `InputEvent` (Task 2).
- Produces:
  - `final class InputRecorder` com:
    - `init(mapper: SourceCoordinateMapper)`
    - `func start(at startTime: CFTimeInterval)` — instala o `CGEventTap` (no-op se não houver permissão).
    - `func stop() -> [InputEvent]` — remove o tap e devolve os eventos.
    - `func ingest(globalPoint: CGPoint, type: String, button: String?, at time: CFTimeInterval)` — caminho testável que aplica o mapper e adiciona ao buffer (usado também internamente pelo callback do tap).
    - `var isTapInstalled: Bool` — false quando sem permissão.

- [ ] **Step 1: Escrever os testes falhando**

```swift
import XCTest
import CoreGraphics
@testable import OpenRecorder

final class InputRecorderTests: XCTestCase {
    func test_ingestStoresMappedEventWithRelativeTime() {
        let mapper = SourceCoordinateMapper(sourceRect: CGRect(x: 100, y: 50, width: 800, height: 600))
        let rec = InputRecorder(mapper: mapper)
        rec.start(at: 1000.0)
        rec.ingest(globalPoint: CGPoint(x: 150, y: 90), type: "click", button: "left", at: 1001.2)
        let events = rec.stop()
        XCTAssertEqual(events.count, 1)
        XCTAssertEqual(events[0].tMs, 1200)            // (1001.2 - 1000.0) * 1000
        XCTAssertEqual(events[0].x, 50)
        XCTAssertEqual(events[0].y, 40)
        XCTAssertEqual(events[0].type, "click")
        XCTAssertEqual(events[0].button, "left")
    }

    func test_ingestDropsEventsOutsideSource() {
        let mapper = SourceCoordinateMapper(sourceRect: CGRect(x: 0, y: 0, width: 100, height: 100))
        let rec = InputRecorder(mapper: mapper)
        rec.start(at: 0)
        rec.ingest(globalPoint: CGPoint(x: 500, y: 500), type: "move", button: nil, at: 0.5)
        XCTAssertEqual(rec.stop().count, 0)
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run:
```bash
cd ~/Github/open-recorder && xcodegen generate && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: FAIL — `cannot find 'InputRecorder' in scope`.

- [ ] **Step 3: Implementar `InputRecorder`**

```swift
import CoreGraphics
import QuartzCore

final class InputRecorder {
    private let mapper: SourceCoordinateMapper
    private var startTime: CFTimeInterval = 0
    private var events: [InputEvent] = []
    private var eventTap: CFMachPort?
    private var runLoopSource: CFRunLoopSource?
    private(set) var isTapInstalled = false

    init(mapper: SourceCoordinateMapper) {
        self.mapper = mapper
    }

    func start(at startTime: CFTimeInterval) {
        self.startTime = startTime
        self.events = []
        installTap()
    }

    func stop() -> [InputEvent] {
        removeTap()
        return events
    }

    /// Caminho testável e também usado pelo callback do tap.
    func ingest(globalPoint: CGPoint, type: String, button: String?, at time: CFTimeInterval) {
        guard let p = mapper.map(globalPoint: globalPoint) else { return }
        let tMs = Int(((time - startTime) * 1000).rounded())
        events.append(InputEvent(tMs: tMs, type: type, x: Int(p.x.rounded()), y: Int(p.y.rounded()), button: button))
    }

    private func installTap() {
        let mask: CGEventMask =
            (1 << CGEventType.leftMouseDown.rawValue) |
            (1 << CGEventType.rightMouseDown.rawValue) |
            (1 << CGEventType.mouseMoved.rawValue) |
            (1 << CGEventType.leftMouseDragged.rawValue)

        let callback: CGEventTapCallBack = { _, type, cgEvent, refcon in
            guard let refcon else { return Unmanaged.passUnretained(cgEvent) }
            let recorder = Unmanaged<InputRecorder>.fromOpaque(refcon).takeUnretainedValue()
            let now = CACurrentMediaTime()
            let loc = cgEvent.location
            switch type {
            case .leftMouseDown:
                recorder.ingest(globalPoint: loc, type: "click", button: "left", at: now)
            case .rightMouseDown:
                recorder.ingest(globalPoint: loc, type: "click", button: "right", at: now)
            case .mouseMoved, .leftMouseDragged:
                recorder.ingest(globalPoint: loc, type: "move", button: nil, at: now)
            default:
                break
            }
            return Unmanaged.passUnretained(cgEvent)
        }

        let refcon = UnsafeMutableRawPointer(Unmanaged.passUnretained(self).toOpaque())
        guard let tap = CGEvent.tapCreate(
            tap: .cgSessionEventTap,
            place: .headInsertEventTap,
            options: .listenOnly,
            eventsOfInterest: mask,
            callback: callback,
            userInfo: refcon
        ) else {
            isTapInstalled = false
            return
        }
        let source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, tap, 0)
        CFRunLoopAddSource(CFRunLoopGetMain(), source, .commonModes)
        CGEvent.tapEnable(tap: tap, enable: true)
        self.eventTap = tap
        self.runLoopSource = source
        self.isTapInstalled = true
    }

    private func removeTap() {
        if let tap = eventTap {
            CGEvent.tapEnable(tap: tap, enable: false)
            CFMachPortInvalidate(tap)
        }
        if let source = runLoopSource {
            CFRunLoopRemoveSource(CFRunLoopGetMain(), source, .commonModes)
        }
        eventTap = nil
        runLoopSource = nil
        isTapInstalled = false
    }
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run:
```bash
cd ~/Github/open-recorder && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: `** TEST SUCCEEDED **`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add InputRecorder with testable ingest and CGEventTap install"
```

---

## Task 6: VideoWriter (wrapper AVAssetWriter) — teste de integração

Recebe `CMSampleBuffer`s e escreve um `.mp4`. Testável de verdade: gera pixel buffers sintéticos, escreve, e verifica que o `.mp4` resultante é um `AVAsset` válido com uma trilha de vídeo.

**Files:**
- Create: `Sources/OpenRecorder/Capture/VideoWriter.swift`
- Test: `Tests/OpenRecorderTests/VideoWriterTests.swift`

**Interfaces:**
- Consumes: nada.
- Produces:
  - `final class VideoWriter` com:
    - `init(outputURL: URL, width: Int, height: Int) throws`
    - `func appendVideo(_ sampleBuffer: CMSampleBuffer)` — ignora se input não está pronto.
    - `func appendAudio(_ sampleBuffer: CMSampleBuffer)` — usado na Task 8.
    - `func finish() async throws` — finaliza a escrita.
  - A trilha de áudio é adicionada sob demanda no primeiro `appendAudio` (lazy), para gravações sem mic não terem trilha vazia.

- [ ] **Step 1: Escrever o teste de integração falhando**

```swift
import XCTest
import AVFoundation
import CoreMedia
@testable import OpenRecorder

final class VideoWriterTests: XCTestCase {
    /// Cria um CMSampleBuffer de vídeo (BGRA) com PTS dado.
    private func makeVideoSample(width: Int, height: Int, ptsSeconds: Double) -> CMSampleBuffer {
        var pixelBuffer: CVPixelBuffer?
        CVPixelBufferCreate(kCFAllocatorDefault, width, height,
                            kCVPixelFormatType_32BGRA, nil, &pixelBuffer)
        let pb = pixelBuffer!
        var formatDesc: CMVideoFormatDescription?
        CMVideoFormatDescriptionCreateForImageBuffer(allocator: kCFAllocatorDefault,
                                                      imageBuffer: pb, formatDescriptionOut: &formatDesc)
        let pts = CMTime(seconds: ptsSeconds, preferredTimescale: 600)
        var timing = CMSampleTimingInfo(duration: CMTime(value: 1, timescale: 30),
                                        presentationTimeStamp: pts,
                                        decodeTimeStamp: .invalid)
        var sample: CMSampleBuffer?
        CMSampleBufferCreateForImageBuffer(allocator: kCFAllocatorDefault,
                                           imageBuffer: pb, dataReady: true,
                                           makeDataReadyCallback: nil, refcon: nil,
                                           formatDescription: formatDesc!,
                                           sampleTiming: &timing, sampleBufferOut: &sample)
        return sample!
    }

    func test_writesValidMP4WithVideoTrack() async throws {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("vw-\(UUID().uuidString).mp4")
        defer { try? FileManager.default.removeItem(at: url) }

        let writer = try VideoWriter(outputURL: url, width: 320, height: 240)
        for i in 0..<10 {
            writer.appendVideo(makeVideoSample(width: 320, height: 240, ptsSeconds: Double(i) / 30.0))
            try await Task.sleep(nanoseconds: 5_000_000) // deixa o input drenar
        }
        try await writer.finish()

        XCTAssertTrue(FileManager.default.fileExists(atPath: url.path))
        let asset = AVURLAsset(url: url)
        let tracks = try await asset.loadTracks(withMediaType: .video)
        XCTAssertEqual(tracks.count, 1)
        let duration = try await asset.load(.duration)
        XCTAssertGreaterThan(duration.seconds, 0)
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run:
```bash
cd ~/Github/open-recorder && xcodegen generate && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: FAIL — `cannot find 'VideoWriter' in scope`.

- [ ] **Step 3: Implementar `VideoWriter`**

```swift
import AVFoundation
import CoreMedia

final class VideoWriter {
    private let writer: AVAssetWriter
    private let videoInput: AVAssetWriterInput
    private var audioInput: AVAssetWriterInput?
    private var sessionStarted = false
    private let lock = NSLock()

    init(outputURL: URL, width: Int, height: Int) throws {
        writer = try AVAssetWriter(outputURL: outputURL, fileType: .mp4)
        let settings: [String: Any] = [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: width,
            AVVideoHeightKey: height,
        ]
        videoInput = AVAssetWriterInput(mediaType: .video, outputSettings: settings)
        videoInput.expectsMediaDataInRealTime = true
        guard writer.canAdd(videoInput) else {
            throw NSError(domain: "VideoWriter", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "Não foi possível adicionar a trilha de vídeo"])
        }
        writer.add(videoInput)
    }

    private func startSessionIfNeeded(_ pts: CMTime) {
        guard !sessionStarted else { return }
        writer.startWriting()
        writer.startSession(atSourceTime: pts)
        sessionStarted = true
    }

    func appendVideo(_ sampleBuffer: CMSampleBuffer) {
        lock.lock(); defer { lock.unlock() }
        let pts = CMSampleBufferGetPresentationTimeStamp(sampleBuffer)
        startSessionIfNeeded(pts)
        if videoInput.isReadyForMoreMediaData {
            videoInput.append(sampleBuffer)
        }
    }

    func appendAudio(_ sampleBuffer: CMSampleBuffer) {
        lock.lock(); defer { lock.unlock() }
        if audioInput == nil {
            let audio = AVAssetWriterInput(mediaType: .audio, outputSettings: [
                AVFormatIDKey: kAudioFormatMPEG4AAC,
                AVNumberOfChannelsKey: 1,
                AVSampleRateKey: 44100,
                AVEncoderBitRateKey: 128_000,
            ])
            audio.expectsMediaDataInRealTime = true
            if writer.canAdd(audio) {
                writer.add(audio)
                audioInput = audio
            }
        }
        let pts = CMSampleBufferGetPresentationTimeStamp(sampleBuffer)
        startSessionIfNeeded(pts)
        if let audioInput, audioInput.isReadyForMoreMediaData {
            audioInput.append(sampleBuffer)
        }
    }

    func finish() async throws {
        videoInput.markAsFinished()
        audioInput?.markAsFinished()
        await writer.finishWriting()
        if writer.status == .failed {
            throw writer.error ?? NSError(domain: "VideoWriter", code: 2)
        }
    }
}
```

> **Nota de áudio (lazy track):** adicionar input ao `AVAssetWriter` após `startWriting()` é proibido. Por isso o áudio na Task 8 precisa ser configurado **antes** do primeiro frame de vídeo. Ajuste na Task 8: o `VideoWriter` recebe um flag `hasAudio` no init quando o mic está ativo, criando a trilha de áudio antes de `startWriting()`. O teste de vídeo-só acima continua válido (sem áudio).

- [ ] **Step 4: Rodar pra ver passar**

Run:
```bash
cd ~/Github/open-recorder && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: `** TEST SUCCEEDED **`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add VideoWriter wrapping AVAssetWriter with integration test"
```

---

## Task 7: VideoWriter com áudio antecipado (corrige ordem de trilha)

Refatora o `VideoWriter` para aceitar `hasAudio` no init, adicionando a trilha de áudio **antes** de `startWriting()` — pré-requisito correto para mux com mic na Task 8.

**Files:**
- Modify: `Sources/OpenRecorder/Capture/VideoWriter.swift`
- Test: `Tests/OpenRecorderTests/VideoWriterTests.swift` (adiciona caso com áudio)

**Interfaces:**
- Consumes: nada.
- Produces: `VideoWriter.init(outputURL:width:height:hasAudio:)` (parâmetro `hasAudio: Bool`, default `false`). Quando `true`, cria a trilha de áudio no init. `appendAudio` deixa de criar trilha lazy.

- [ ] **Step 1: Adicionar teste falhando com áudio**

```swift
    private func makeAudioSample(ptsSeconds: Double) -> CMSampleBuffer {
        var asbd = AudioStreamBasicDescription(
            mSampleRate: 44100, mFormatID: kAudioFormatLinearPCM,
            mFormatFlags: kAudioFormatFlagIsSignedInteger | kAudioFormatFlagIsPacked,
            mBytesPerPacket: 2, mFramesPerPacket: 1, mBytesPerFrame: 2,
            mChannelsPerFrame: 1, mBitsPerChannel: 16, mReserved: 0)
        var formatDesc: CMAudioFormatDescription?
        CMAudioFormatDescriptionCreate(allocator: kCFAllocatorDefault, asbd: &asbd,
                                       layoutSize: 0, layout: nil, magicCookieSize: 0,
                                       magicCookie: nil, extensions: nil, formatDescriptionOut: &formatDesc)
        let frames = 1024
        let byteCount = frames * 2
        var blockBuffer: CMBlockBuffer?
        CMBlockBufferCreateWithMemoryBlock(allocator: kCFAllocatorDefault, memoryBlock: nil,
                                           blockLength: byteCount, blockAllocator: kCFAllocatorDefault,
                                           customBlockSource: nil, offsetToData: 0, dataLength: byteCount,
                                           flags: 0, blockBufferOut: &blockBuffer)
        CMBlockBufferFillDataBytes(with: 0, blockBuffer: blockBuffer!, offsetIntoDestination: 0,
                                   dataLength: byteCount)
        var sample: CMSampleBuffer?
        var timing = CMSampleTimingInfo(duration: CMTime(value: 1, timescale: 44100),
                                        presentationTimeStamp: CMTime(seconds: ptsSeconds, preferredTimescale: 44100),
                                        decodeTimeStamp: .invalid)
        CMSampleBufferCreate(allocator: kCFAllocatorDefault, dataBuffer: blockBuffer, dataReady: true,
                             makeDataReadyCallback: nil, refcon: nil, formatDescription: formatDesc!,
                             sampleCount: frames, sampleTimingEntryCount: 1, sampleTimingArray: &timing,
                             sampleSizeEntryCount: 1, sampleSizeArray: [2], sampleBufferOut: &sample)
        return sample!
    }

    func test_writesMP4WithVideoAndAudioTracks() async throws {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("vwa-\(UUID().uuidString).mp4")
        defer { try? FileManager.default.removeItem(at: url) }

        let writer = try VideoWriter(outputURL: url, width: 320, height: 240, hasAudio: true)
        for i in 0..<10 {
            writer.appendVideo(makeVideoSample(width: 320, height: 240, ptsSeconds: Double(i) / 30.0))
            writer.appendAudio(makeAudioSample(ptsSeconds: Double(i) / 30.0))
            try await Task.sleep(nanoseconds: 5_000_000)
        }
        try await writer.finish()

        let asset = AVURLAsset(url: url)
        let video = try await asset.loadTracks(withMediaType: .video)
        let audio = try await asset.loadTracks(withMediaType: .audio)
        XCTAssertEqual(video.count, 1)
        XCTAssertEqual(audio.count, 1)
    }
```

- [ ] **Step 2: Rodar pra ver falhar**

Run:
```bash
cd ~/Github/open-recorder && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: FAIL — `extra argument 'hasAudio' in call`.

- [ ] **Step 3: Refatorar `VideoWriter`**

Substituir o corpo de `VideoWriter` por:

```swift
import AVFoundation
import CoreMedia

final class VideoWriter {
    private let writer: AVAssetWriter
    private let videoInput: AVAssetWriterInput
    private let audioInput: AVAssetWriterInput?
    private var sessionStarted = false
    private let lock = NSLock()

    init(outputURL: URL, width: Int, height: Int, hasAudio: Bool = false) throws {
        writer = try AVAssetWriter(outputURL: outputURL, fileType: .mp4)

        let videoSettings: [String: Any] = [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: width,
            AVVideoHeightKey: height,
        ]
        videoInput = AVAssetWriterInput(mediaType: .video, outputSettings: videoSettings)
        videoInput.expectsMediaDataInRealTime = true
        guard writer.canAdd(videoInput) else {
            throw NSError(domain: "VideoWriter", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "Não foi possível adicionar a trilha de vídeo"])
        }
        writer.add(videoInput)

        if hasAudio {
            let audio = AVAssetWriterInput(mediaType: .audio, outputSettings: [
                AVFormatIDKey: kAudioFormatMPEG4AAC,
                AVNumberOfChannelsKey: 1,
                AVSampleRateKey: 44100,
                AVEncoderBitRateKey: 128_000,
            ])
            audio.expectsMediaDataInRealTime = true
            if writer.canAdd(audio) {
                writer.add(audio)
                audioInput = audio
            } else {
                audioInput = nil
            }
        } else {
            audioInput = nil
        }
    }

    private func startSessionIfNeeded(_ pts: CMTime) {
        guard !sessionStarted else { return }
        writer.startWriting()
        writer.startSession(atSourceTime: pts)
        sessionStarted = true
    }

    func appendVideo(_ sampleBuffer: CMSampleBuffer) {
        lock.lock(); defer { lock.unlock() }
        startSessionIfNeeded(CMSampleBufferGetPresentationTimeStamp(sampleBuffer))
        if videoInput.isReadyForMoreMediaData {
            videoInput.append(sampleBuffer)
        }
    }

    func appendAudio(_ sampleBuffer: CMSampleBuffer) {
        lock.lock(); defer { lock.unlock() }
        guard let audioInput else { return }
        startSessionIfNeeded(CMSampleBufferGetPresentationTimeStamp(sampleBuffer))
        if audioInput.isReadyForMoreMediaData {
            audioInput.append(sampleBuffer)
        }
    }

    func finish() async throws {
        lock.lock()
        videoInput.markAsFinished()
        audioInput?.markAsFinished()
        lock.unlock()
        await writer.finishWriting()
        if writer.status == .failed {
            throw writer.error ?? NSError(domain: "VideoWriter", code: 2)
        }
    }
}
```

- [ ] **Step 4: Rodar pra ver os dois testes passarem**

Run:
```bash
cd ~/Github/open-recorder && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: `** TEST SUCCEEDED **` (vídeo-só + vídeo+áudio).

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "refactor: VideoWriter creates audio track up front via hasAudio flag"
```

---

## Task 8: SourceEnumerator (lista displays/janelas) — smoke manual

`SCShareableContent` exige permissão de Screen Recording e ambiente gráfico; não é unit-testável de forma confiável. Lógica mínima + verificação manual num pequeno harness de debug.

**Files:**
- Create: `Sources/OpenRecorder/Capture/SourceEnumerator.swift`

**Interfaces:**
- Consumes: `CaptureSource`, `CaptureSourceKind` (Task 3).
- Produces:
  - `struct DisplayOption: Identifiable, Equatable { let id: String; let name: String; let source: CaptureSource }`
  - `struct WindowOption: Identifiable, Equatable { let id: String; let name: String; let source: CaptureSource }`
  - `enum SourceEnumerator { static func displays() async throws -> [DisplayOption]; static func windows() async throws -> [WindowOption] }`

- [ ] **Step 1: Implementar o enumerator**

```swift
import ScreenCaptureKit
import CoreGraphics

struct DisplayOption: Identifiable, Equatable {
    let id: String
    let name: String
    let source: CaptureSource
}

struct WindowOption: Identifiable, Equatable {
    let id: String
    let name: String
    let source: CaptureSource
}

enum SourceEnumerator {
    static func displays() async throws -> [DisplayOption] {
        let content = try await SCShareableContent.excludingDesktopWindows(false,
                                                                           onScreenWindowsOnly: true)
        return content.displays.map { display in
            let rect = CGRect(x: CGFloat(display.frame.origin.x),
                              y: CGFloat(display.frame.origin.y),
                              width: CGFloat(display.width),
                              height: CGFloat(display.height))
            let source = CaptureSource(kind: .display, id: String(display.displayID),
                                       rect: rect, displayID: display.displayID)
            return DisplayOption(id: String(display.displayID),
                                 name: "Tela \(display.displayID) (\(display.width)×\(display.height))",
                                 source: source)
        }
    }

    static func windows() async throws -> [WindowOption] {
        let content = try await SCShareableContent.excludingDesktopWindows(true,
                                                                           onScreenWindowsOnly: true)
        return content.windows.compactMap { window in
            guard let title = window.title, !title.isEmpty,
                  let app = window.owningApplication else { return nil }
            let displayID = CGMainDisplayID()
            let source = CaptureSource(kind: .window, id: String(window.windowID),
                                       rect: window.frame, displayID: displayID)
            return WindowOption(id: String(window.windowID),
                                name: "\(app.applicationName) — \(title)",
                                source: source)
        }
    }
}
```

- [ ] **Step 2: Buildar (sem testes novos)**

Run:
```bash
cd ~/Github/open-recorder && xcodegen generate && xcodebuild build -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -10
```
Expected: `** BUILD SUCCEEDED **`.

- [ ] **Step 3: Smoke manual — adicionar botão de debug temporário**

Em `ContentView.swift`, dentro do `VStack`, adicionar temporariamente:

```swift
            Button("Debug: listar fontes") {
                Task {
                    let displays = (try? await SourceEnumerator.displays()) ?? []
                    let windows = (try? await SourceEnumerator.windows()) ?? []
                    print("Displays: \(displays.map(\.name))")
                    print("Windows: \(windows.count) janelas")
                }
            }
```

- [ ] **Step 4: Rodar o app e verificar**

Run:
```bash
cd ~/Github/open-recorder && xcodebuild build -scheme OpenRecorder -destination 'platform=macOS' -derivedDataPath build 2>&1 | tail -5 && open build/Build/Products/Debug/OpenRecorder.app
```
Clicar "Debug: listar fontes". Na primeira vez, macOS pede permissão de Screen Recording → conceder em Ajustes do Sistema → Privacidade e Segurança → Gravação de Tela → reabrir o app.
Expected (no terminal/Console): imprime ao menos uma tela e uma contagem de janelas > 0.

- [ ] **Step 5: Remover o botão de debug**

Apagar o `Button("Debug: listar fontes")` do `ContentView.swift`.

- [ ] **Step 6: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add SourceEnumerator for displays and windows via ScreenCaptureKit"
```

---

## Task 9: CaptureEngine (SCStream → VideoWriter) — smoke manual

**Files:**
- Create: `Sources/OpenRecorder/Capture/CaptureEngine.swift`

**Interfaces:**
- Consumes: `CaptureSource`, `StreamConfigBuilder`, `VideoWriter`.
- Produces:
  - `final class CaptureEngine: NSObject, SCStreamOutput, SCStreamDelegate`
  - `init(source: CaptureSource, fps: Int, writer: VideoWriter)`
  - `func start() async throws`
  - `func stop() async throws`
  - `var onStreamError: ((Error) -> Void)?`

- [ ] **Step 1: Implementar `CaptureEngine`**

```swift
import ScreenCaptureKit
import AVFoundation

final class CaptureEngine: NSObject, SCStreamOutput, SCStreamDelegate {
    private let source: CaptureSource
    private let fps: Int
    private let writer: VideoWriter
    private var stream: SCStream?
    private let sampleQueue = DispatchQueue(label: "com.openrecorder.capture")
    var onStreamError: ((Error) -> Void)?

    init(source: CaptureSource, fps: Int, writer: VideoWriter) {
        self.source = source
        self.fps = fps
        self.writer = writer
    }

    func start() async throws {
        let content = try await SCShareableContent.excludingDesktopWindows(false,
                                                                           onScreenWindowsOnly: true)
        let filter: SCContentFilter
        switch source.kind {
        case .display, .region:
            guard let display = content.displays.first(where: { $0.displayID == source.displayID }) else {
                throw NSError(domain: "CaptureEngine", code: 1,
                              userInfo: [NSLocalizedDescriptionKey: "Display não encontrado"])
            }
            filter = SCContentFilter(display: display, excludingWindows: [])
        case .window:
            guard let window = content.windows.first(where: { String($0.windowID) == source.id }) else {
                throw NSError(domain: "CaptureEngine", code: 2,
                              userInfo: [NSLocalizedDescriptionKey: "Janela não encontrada"])
            }
            filter = SCContentFilter(desktopIndependentWindow: window)
        }

        let values = StreamConfigBuilder.values(for: source, fps: fps, showsCursor: true)
        let config = StreamConfigBuilder.makeConfiguration(values)
        if source.kind == .region {
            config.sourceRect = source.rect
        }

        let stream = SCStream(filter: filter, configuration: config, delegate: self)
        try stream.addStreamOutput(self, type: .screen, sampleHandlerQueue: sampleQueue)
        try await stream.startCapture()
        self.stream = stream
    }

    func stop() async throws {
        try await stream?.stopCapture()
        stream = nil
    }

    // MARK: SCStreamOutput
    func stream(_ stream: SCStream, didOutputSampleBuffer sampleBuffer: CMSampleBuffer,
                of type: SCStreamOutputType) {
        guard type == .screen, sampleBuffer.isValid else { return }
        // Só frames "complete" carregam imagem útil.
        guard let attachments = CMSampleBufferGetSampleAttachmentsArray(sampleBuffer,
                                                                        createIfNecessary: false) as? [[SCStreamFrameInfo: Any]],
              let statusRaw = attachments.first?[.status] as? Int,
              let status = SCFrameStatus(rawValue: statusRaw), status == .complete else { return }
        writer.appendVideo(sampleBuffer)
    }

    // MARK: SCStreamDelegate
    func stream(_ stream: SCStream, didStopWithError error: Error) {
        onStreamError?(error)
    }
}
```

- [ ] **Step 2: Buildar**

Run:
```bash
cd ~/Github/open-recorder && xcodegen generate && xcodebuild build -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -10
```
Expected: `** BUILD SUCCEEDED **`.

- [ ] **Step 3: Smoke manual via teste integrado (grava 2s da tela principal)**

Adicionar `Tests/OpenRecorderTests/CaptureEngineSmokeTests.swift`:

```swift
import XCTest
import AVFoundation
@testable import OpenRecorder

final class CaptureEngineSmokeTests: XCTestCase {
    // Smoke: exige permissão de Screen Recording. Pulado se não houver displays.
    func test_recordsTwoSecondsOfMainDisplay() async throws {
        let displays = (try? await SourceEnumerator.displays()) ?? []
        try XCTSkipIf(displays.isEmpty, "Sem displays/permite — smoke manual")
        let source = displays[0].source
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("smoke-\(UUID().uuidString).mp4")
        defer { try? FileManager.default.removeItem(at: url) }

        let writer = try VideoWriter(outputURL: url,
                                     width: Int(source.rect.width),
                                     height: Int(source.rect.height))
        let engine = CaptureEngine(source: source, fps: 30, writer: writer)
        try await engine.start()
        try await Task.sleep(nanoseconds: 2_000_000_000)
        try await engine.stop()
        try await writer.finish()

        let asset = AVURLAsset(url: url)
        let tracks = try await asset.loadTracks(withMediaType: .video)
        XCTAssertEqual(tracks.count, 1)
        XCTAssertGreaterThan(try await asset.load(.duration).seconds, 1.0)
    }
}
```

Run (com permissão de Screen Recording concedida):
```bash
cd ~/Github/open-recorder && xcodegen generate && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' -only-testing:OpenRecorderTests/CaptureEngineSmokeTests 2>&1 | tail -20
```
Expected: passa (ou `Skipped` se sem permissão — nesse caso conceder e repetir).

- [ ] **Step 4: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add CaptureEngine bridging SCStream to VideoWriter"
```

---

## Task 10: AudioRecorder (mic → VideoWriter) — smoke manual

**Files:**
- Create: `Sources/OpenRecorder/Capture/AudioRecorder.swift`

**Interfaces:**
- Consumes: `VideoWriter` (com `hasAudio: true`).
- Produces:
  - `final class AudioRecorder: NSObject, AVCaptureAudioDataOutputSampleBufferDelegate`
  - `init(deviceID: String?, writer: VideoWriter)` — `deviceID == nil` usa o mic padrão.
  - `func start() throws`
  - `func stop()`
  - `static func availableMicrophones() -> [(id: String, name: String)]`

- [ ] **Step 1: Implementar `AudioRecorder`**

```swift
import AVFoundation

final class AudioRecorder: NSObject, AVCaptureAudioDataOutputSampleBufferDelegate {
    private let deviceID: String?
    private let writer: VideoWriter
    private let session = AVCaptureSession()
    private let output = AVCaptureAudioDataOutput()
    private let queue = DispatchQueue(label: "com.openrecorder.audio")

    init(deviceID: String?, writer: VideoWriter) {
        self.deviceID = deviceID
        self.writer = writer
    }

    static func availableMicrophones() -> [(id: String, name: String)] {
        let discovery = AVCaptureDevice.DiscoverySession(
            deviceTypes: [.microphone], mediaType: .audio, position: .unspecified)
        return discovery.devices.map { ($0.uniqueID, $0.localizedName) }
    }

    func start() throws {
        let device: AVCaptureDevice?
        if let deviceID {
            device = AVCaptureDevice(uniqueID: deviceID)
        } else {
            device = AVCaptureDevice.default(for: .audio)
        }
        guard let device else {
            throw NSError(domain: "AudioRecorder", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "Microfone não encontrado"])
        }
        let input = try AVCaptureDeviceInput(device: device)
        session.beginConfiguration()
        if session.canAddInput(input) { session.addInput(input) }
        output.setSampleBufferDelegate(self, queue: queue)
        if session.canAddOutput(output) { session.addOutput(output) }
        session.commitConfiguration()
        session.startRunning()
    }

    func stop() {
        session.stopRunning()
    }

    func captureOutput(_ output: AVCaptureOutput, didOutput sampleBuffer: CMSampleBuffer,
                       from connection: AVCaptureConnection) {
        writer.appendAudio(sampleBuffer)
    }
}
```

- [ ] **Step 2: Buildar**

Run:
```bash
cd ~/Github/open-recorder && xcodegen generate && xcodebuild build -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -10
```
Expected: `** BUILD SUCCEEDED **`.

- [ ] **Step 3: Smoke manual (verificado junto na Task 11 via gravação real completa)**

Sem teste automatizado isolado (mic exige permissão + hardware). Verificação acontece no smoke da Task 11 (gravação com áudio gera `.mp4` com trilha de áudio).

- [ ] **Step 4: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add AudioRecorder capturing mic into VideoWriter"
```

---

## Task 11: PermissionService

**Files:**
- Create: `Sources/OpenRecorder/Permissions/PermissionService.swift`
- Test: `Tests/OpenRecorderTests/PermissionServiceTests.swift`

**Interfaces:**
- Consumes: nada.
- Produces:
  - `enum PermissionState: Equatable { case granted, denied, notDetermined }`
  - `enum PermissionService`:
    - `static func screenRecordingState() -> PermissionState`
    - `static func microphoneState() -> PermissionState`
    - `static func inputMonitoringState() -> PermissionState`
    - `static func requestMicrophone() async -> Bool`
    - `static func openSettings(for kind: PermissionKind)` — abre o painel certo.
  - `enum PermissionKind { case screenRecording, microphone, inputMonitoring }` com `var settingsURL: URL`.

- [ ] **Step 1: Escrever teste das URLs de Ajustes (parte testável)**

```swift
import XCTest
@testable import OpenRecorder

final class PermissionServiceTests: XCTestCase {
    func test_settingsURLsArePrivacyPanels() {
        XCTAssertEqual(PermissionKind.screenRecording.settingsURL.absoluteString,
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        XCTAssertEqual(PermissionKind.microphone.settingsURL.absoluteString,
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
        XCTAssertEqual(PermissionKind.inputMonitoring.settingsURL.absoluteString,
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run:
```bash
cd ~/Github/open-recorder && xcodegen generate && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: FAIL — `cannot find 'PermissionKind' in scope`.

- [ ] **Step 3: Implementar `PermissionService`**

```swift
import AVFoundation
import CoreGraphics
import AppKit

enum PermissionState: Equatable {
    case granted, denied, notDetermined
}

enum PermissionKind {
    case screenRecording, microphone, inputMonitoring

    var settingsURL: URL {
        let base = "x-apple.systempreferences:com.apple.preference.security?"
        switch self {
        case .screenRecording: return URL(string: base + "Privacy_ScreenCapture")!
        case .microphone:      return URL(string: base + "Privacy_Microphone")!
        case .inputMonitoring: return URL(string: base + "Privacy_ListenEvent")!
        }
    }
}

enum PermissionService {
    static func screenRecordingState() -> PermissionState {
        CGPreflightScreenCaptureAccess() ? .granted : .denied
    }

    static func microphoneState() -> PermissionState {
        switch AVCaptureDevice.authorizationStatus(for: .audio) {
        case .authorized: return .granted
        case .denied, .restricted: return .denied
        case .notDetermined: return .notDetermined
        @unknown default: return .denied
        }
    }

    static func inputMonitoringState() -> PermissionState {
        CGPreflightListenEventAccess() ? .granted : .denied
    }

    static func requestMicrophone() async -> Bool {
        await AVCaptureDevice.requestAccess(for: .audio)
    }

    static func requestScreenRecording() {
        _ = CGRequestScreenCaptureAccess()
    }

    static func requestInputMonitoring() {
        _ = CGRequestListenEventAccess()
    }

    static func openSettings(for kind: PermissionKind) {
        NSWorkspace.shared.open(kind.settingsURL)
    }
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run:
```bash
cd ~/Github/open-recorder && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: `** TEST SUCCEEDED **`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add PermissionService for screen, mic and input monitoring"
```

---

## Task 12: RecordingFinalizer (escreve metadata.json)

**Files:**
- Create: `Sources/OpenRecorder/Capture/RecordingFinalizer.swift`
- Test: `Tests/OpenRecorderTests/RecordingFinalizerTests.swift`

**Interfaces:**
- Consumes: `RecordingMetadata`, `RecordingInfo`, `SourceInfo`, `InputEvent` (Task 2), `CaptureSource` (Task 3).
- Produces:
  - `enum RecordingFinalizer`:
    - `static func metadata(source: CaptureSource, fps: Int, durationMs: Int, events: [InputEvent]) -> RecordingMetadata`
    - `static func write(_ metadata: RecordingMetadata, to url: URL) throws`

- [ ] **Step 1: Escrever os testes falhando**

```swift
import XCTest
import CoreGraphics
@testable import OpenRecorder

final class RecordingFinalizerTests: XCTestCase {
    func test_buildsMetadataFromSource() {
        let source = CaptureSource(kind: .display, id: "1",
                                   rect: CGRect(x: 0, y: 0, width: 1920, height: 1080),
                                   displayID: 1)
        let meta = RecordingFinalizer.metadata(source: source, fps: 30, durationMs: 5000,
                                               events: [InputEvent(tMs: 10, type: "click", x: 1, y: 2, button: "left")])
        XCTAssertEqual(meta.version, 1)
        XCTAssertEqual(meta.recording, RecordingInfo(width: 1920, height: 1080, fps: 30, durationMs: 5000))
        XCTAssertEqual(meta.source, SourceInfo(type: "display", id: "1", rect: [0, 0, 1920, 1080]))
        XCTAssertEqual(meta.events.count, 1)
    }

    func test_writesJSONFileToDisk() throws {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("meta-\(UUID().uuidString).json")
        defer { try? FileManager.default.removeItem(at: url) }
        let source = CaptureSource(kind: .window, id: "7",
                                   rect: CGRect(x: 5, y: 6, width: 100, height: 200), displayID: 1)
        let meta = RecordingFinalizer.metadata(source: source, fps: 60, durationMs: 1234, events: [])
        try RecordingFinalizer.write(meta, to: url)

        let data = try Data(contentsOf: url)
        let decoded = try JSONDecoder().decode(RecordingMetadata.self, from: data)
        XCTAssertEqual(decoded, meta)
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run:
```bash
cd ~/Github/open-recorder && xcodegen generate && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: FAIL — `cannot find 'RecordingFinalizer' in scope`.

- [ ] **Step 3: Implementar `RecordingFinalizer`**

```swift
import Foundation
import CoreGraphics

enum RecordingFinalizer {
    static func metadata(source: CaptureSource, fps: Int, durationMs: Int,
                         events: [InputEvent]) -> RecordingMetadata {
        RecordingMetadata(
            version: 1,
            recording: RecordingInfo(width: Int(source.rect.width),
                                     height: Int(source.rect.height),
                                     fps: fps, durationMs: durationMs),
            source: SourceInfo(type: source.kind.rawValue, id: source.id,
                               rect: [Int(source.rect.origin.x), Int(source.rect.origin.y),
                                      Int(source.rect.width), Int(source.rect.height)]),
            events: events
        )
    }

    static func write(_ metadata: RecordingMetadata, to url: URL) throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(metadata)
        try data.write(to: url, options: .atomic)
    }
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run:
```bash
cd ~/Github/open-recorder && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: `** TEST SUCCEEDED **`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add RecordingFinalizer building and writing metadata.json"
```

---

## Task 13: RecordingCoordinator (orquestra start/stop)

**Files:**
- Create: `Sources/OpenRecorder/Capture/RecordingCoordinator.swift`
- Test: `Tests/OpenRecorderTests/RecordingCoordinatorTests.swift`

**Interfaces:**
- Consumes: `CaptureEngine`, `AudioRecorder`, `InputRecorder`, `VideoWriter`, `RecordingFinalizer`, `SourceCoordinateMapper`, `CaptureSource`.
- Produces:
  - `struct RecordingResult: Equatable { let videoURL: URL; let metadataURL: URL; let durationMs: Int }`
  - `final class RecordingCoordinator`:
    - `init(outputDirectory: URL)`
    - `func start(source: CaptureSource, micDeviceID: String?, fps: Int) async throws`
    - `func stop() async throws -> RecordingResult`
    - `var isRecording: Bool`
  - Nomes de arquivo: `REC-<timestamp>.mp4` e `REC-<timestamp>.metadata.json` (mesmo timestamp).
  - Helper testável: `static func makeFilenames(timestamp: String) -> (video: String, metadata: String)`.

- [ ] **Step 1: Escrever teste da convenção de nomes**

```swift
import XCTest
@testable import OpenRecorder

final class RecordingCoordinatorTests: XCTestCase {
    func test_filenamesShareTimestamp() {
        let names = RecordingCoordinator.makeFilenames(timestamp: "20260617-153000")
        XCTAssertEqual(names.video, "REC-20260617-153000.mp4")
        XCTAssertEqual(names.metadata, "REC-20260617-153000.metadata.json")
    }
}
```

- [ ] **Step 2: Rodar pra ver falhar**

Run:
```bash
cd ~/Github/open-recorder && xcodegen generate && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: FAIL — `cannot find 'RecordingCoordinator' in scope`.

- [ ] **Step 3: Implementar `RecordingCoordinator`**

```swift
import Foundation
import QuartzCore

struct RecordingResult: Equatable {
    let videoURL: URL
    let metadataURL: URL
    let durationMs: Int
}

final class RecordingCoordinator {
    private let outputDirectory: URL
    private var engine: CaptureEngine?
    private var audio: AudioRecorder?
    private var input: InputRecorder?
    private var writer: VideoWriter?
    private var currentSource: CaptureSource?
    private var currentFps = 30
    private var startMediaTime: CFTimeInterval = 0
    private var videoURL: URL?
    private var metadataURL: URL?
    private(set) var isRecording = false

    init(outputDirectory: URL) {
        self.outputDirectory = outputDirectory
    }

    static func makeFilenames(timestamp: String) -> (video: String, metadata: String) {
        ("REC-\(timestamp).mp4", "REC-\(timestamp).metadata.json")
    }

    private static func timestampNow() -> String {
        let f = DateFormatter()
        f.dateFormat = "yyyyMMdd-HHmmss"
        return f.string(from: Date())
    }

    func start(source: CaptureSource, micDeviceID: String?, fps: Int) async throws {
        try FileManager.default.createDirectory(at: outputDirectory,
                                                withIntermediateDirectories: true)
        let ts = Self.timestampNow()
        let names = Self.makeFilenames(timestamp: ts)
        let videoURL = outputDirectory.appendingPathComponent(names.video)
        let metadataURL = outputDirectory.appendingPathComponent(names.metadata)

        let hasMic = micDeviceID != nil || PermissionService.microphoneState() == .granted
        let writer = try VideoWriter(outputURL: videoURL,
                                     width: Int(source.rect.width),
                                     height: Int(source.rect.height),
                                     hasAudio: hasMic)

        let mapper = SourceCoordinateMapper(sourceRect: source.rect)
        let input = InputRecorder(mapper: mapper)
        let engine = CaptureEngine(source: source, fps: fps, writer: writer)

        startMediaTime = CACurrentMediaTime()
        input.start(at: startMediaTime)
        try await engine.start()

        if hasMic {
            let audio = AudioRecorder(deviceID: micDeviceID, writer: writer)
            try audio.start()
            self.audio = audio
        }

        self.engine = engine
        self.input = input
        self.writer = writer
        self.currentSource = source
        self.currentFps = fps
        self.videoURL = videoURL
        self.metadataURL = metadataURL
        self.isRecording = true
    }

    func stop() async throws -> RecordingResult {
        guard let engine, let input, let writer, let source = currentSource,
              let videoURL, let metadataURL else {
            throw NSError(domain: "RecordingCoordinator", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "Nenhuma gravação ativa"])
        }
        let durationMs = Int(((CACurrentMediaTime() - startMediaTime) * 1000).rounded())
        audio?.stop()
        try await engine.stop()
        let events = input.stop()
        try await writer.finish()

        let meta = RecordingFinalizer.metadata(source: source, fps: currentFps,
                                               durationMs: durationMs, events: events)
        try RecordingFinalizer.write(meta, to: metadataURL)

        self.engine = nil; self.audio = nil; self.input = nil; self.writer = nil
        self.currentSource = nil; self.isRecording = false

        return RecordingResult(videoURL: videoURL, metadataURL: metadataURL, durationMs: durationMs)
    }
}
```

- [ ] **Step 4: Rodar pra ver passar**

Run:
```bash
cd ~/Github/open-recorder && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: `** TEST SUCCEEDED **`.

- [ ] **Step 5: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add RecordingCoordinator orchestrating capture, audio, input"
```

---

## Task 14: ViewModel + UI SwiftUI (fonte, mic, controles, lista)

**Files:**
- Create: `Sources/OpenRecorder/UI/RecorderViewModel.swift`
- Create: `Sources/OpenRecorder/UI/SourcePickerView.swift`
- Create: `Sources/OpenRecorder/UI/RecordingControlsView.swift`
- Create: `Sources/OpenRecorder/UI/RecordingsListView.swift`
- Modify: `Sources/OpenRecorder/UI/ContentView.swift`

**Interfaces:**
- Consumes: `SourceEnumerator`, `AudioRecorder.availableMicrophones`, `RecordingCoordinator`, `PermissionService`, `RecordingResult`.
- Produces: app utilizável fim-a-fim.

- [ ] **Step 1: Implementar `RecorderViewModel`**

```swift
import SwiftUI
import Combine

@MainActor
final class RecorderViewModel: ObservableObject {
    @Published var displays: [DisplayOption] = []
    @Published var windows: [WindowOption] = []
    @Published var microphones: [(id: String, name: String)] = []
    @Published var selectedSource: CaptureSource?
    @Published var selectedMicID: String?
    @Published var isRecording = false
    @Published var elapsedMs = 0
    @Published var recordings: [RecordingResult] = []
    @Published var errorMessage: String?
    @Published var screenPermissionGranted = true

    private let coordinator: RecordingCoordinator
    private var timer: Timer?
    private var startDate: Date?

    init() {
        let dir = FileManager.default.urls(for: .moviesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("OpenRecorder", isDirectory: true)
        self.coordinator = RecordingCoordinator(outputDirectory: dir)
    }

    func refreshSources() async {
        screenPermissionGranted = PermissionService.screenRecordingState() == .granted
        if !screenPermissionGranted {
            PermissionService.requestScreenRecording()
        }
        displays = (try? await SourceEnumerator.displays()) ?? []
        windows = (try? await SourceEnumerator.windows()) ?? []
        microphones = AudioRecorder.availableMicrophones()
        if selectedSource == nil { selectedSource = displays.first?.source }
        if selectedMicID == nil { selectedMicID = microphones.first?.id }
    }

    func toggleRecording() {
        if isRecording { stop() } else { start() }
    }

    private func start() {
        guard let source = selectedSource else { return }
        Task {
            do {
                if PermissionService.microphoneState() == .notDetermined {
                    _ = await PermissionService.requestMicrophone()
                }
                try await coordinator.start(source: source, micDeviceID: selectedMicID, fps: 30)
                isRecording = true
                startDate = Date()
                startTimer()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    private func stop() {
        Task {
            do {
                let result = try await coordinator.stop()
                recordings.insert(result, at: 0)
            } catch {
                errorMessage = error.localizedDescription
            }
            isRecording = false
            stopTimer()
        }
    }

    private func startTimer() {
        timer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { [weak self] _ in
            guard let self, let start = self.startDate else { return }
            Task { @MainActor in self.elapsedMs = Int(Date().timeIntervalSince(start) * 1000) }
        }
    }

    private func stopTimer() {
        timer?.invalidate(); timer = nil; elapsedMs = 0; startDate = nil
    }

    func reveal(_ result: RecordingResult) {
        NSWorkspace.shared.activateFileViewerSelecting([result.videoURL])
    }
}
```

- [ ] **Step 2: Implementar `SourcePickerView`**

```swift
import SwiftUI

struct SourcePickerView: View {
    @ObservedObject var vm: RecorderViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Fonte").font(.headline)
            Picker("Tela", selection: Binding(
                get: { vm.selectedSource?.id ?? "" },
                set: { id in
                    if let d = vm.displays.first(where: { $0.id == id }) { vm.selectedSource = d.source }
                    else if let w = vm.windows.first(where: { $0.id == id }) { vm.selectedSource = w.source }
                })) {
                Section("Telas") {
                    ForEach(vm.displays) { Text($0.name).tag($0.id) }
                }
                Section("Janelas") {
                    ForEach(vm.windows) { Text($0.name).tag($0.id) }
                }
            }
            .labelsHidden()

            Text("Microfone").font(.headline)
            Picker("Mic", selection: Binding(
                get: { vm.selectedMicID ?? "" },
                set: { vm.selectedMicID = $0 })) {
                ForEach(vm.microphones, id: \.id) { Text($0.name).tag($0.id) }
            }
            .labelsHidden()
        }
    }
}
```

- [ ] **Step 3: Implementar `RecordingControlsView`**

```swift
import SwiftUI

struct RecordingControlsView: View {
    @ObservedObject var vm: RecorderViewModel

    private var timeString: String {
        let totalSec = vm.elapsedMs / 1000
        return String(format: "%02d:%02d", totalSec / 60, totalSec % 60)
    }

    var body: some View {
        HStack(spacing: 16) {
            Button(action: vm.toggleRecording) {
                Label(vm.isRecording ? "Parar" : "Gravar",
                      systemImage: vm.isRecording ? "stop.circle.fill" : "record.circle")
                    .font(.title3)
            }
            .buttonStyle(.borderedProminent)
            .tint(vm.isRecording ? .gray : .red)
            .disabled(vm.selectedSource == nil)

            if vm.isRecording {
                Text(timeString).font(.title3.monospacedDigit()).foregroundStyle(.red)
            }
        }
    }
}
```

- [ ] **Step 4: Implementar `RecordingsListView`**

```swift
import SwiftUI

struct RecordingsListView: View {
    @ObservedObject var vm: RecorderViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Gravações").font(.headline)
            if vm.recordings.isEmpty {
                Text("Nenhuma gravação ainda.").foregroundStyle(.secondary)
            } else {
                ForEach(vm.recordings, id: \.videoURL) { rec in
                    HStack {
                        Image(systemName: "film")
                        Text(rec.videoURL.lastPathComponent).lineLimit(1)
                        Spacer()
                        Text(String(format: "%.1fs", Double(rec.durationMs) / 1000))
                            .foregroundStyle(.secondary)
                        Button("Mostrar") { vm.reveal(rec) }
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 5: Reescrever `ContentView`**

```swift
import SwiftUI

struct ContentView: View {
    @StateObject private var vm = RecorderViewModel()

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            HStack {
                Image(systemName: "record.circle").foregroundStyle(.red)
                Text("OpenRecorder").font(.title.bold())
            }

            if !vm.screenPermissionGranted {
                HStack {
                    Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.yellow)
                    Text("Permissão de Gravação de Tela necessária.")
                    Button("Abrir Ajustes") {
                        PermissionService.openSettings(for: .screenRecording)
                    }
                }
            }

            SourcePickerView(vm: vm)
            RecordingControlsView(vm: vm)
            Divider()
            RecordingsListView(vm: vm)
            Spacer()

            if let error = vm.errorMessage {
                Text(error).foregroundStyle(.red).font(.caption)
            }
        }
        .padding(24)
        .task { await vm.refreshSources() }
    }
}
```

- [ ] **Step 6: Buildar e abrir o app**

Run:
```bash
cd ~/Github/open-recorder && xcodegen generate && xcodebuild build -scheme OpenRecorder -destination 'platform=macOS' -derivedDataPath build 2>&1 | tail -10 && open build/Build/Products/Debug/OpenRecorder.app
```
Expected: `** BUILD SUCCEEDED **` e o app abre mostrando seletor de fonte, mic e botão Gravar.

- [ ] **Step 7: Rodar a suíte completa de testes**

Run:
```bash
cd ~/Github/open-recorder && xcodebuild test -scheme OpenRecorder -destination 'platform=macOS' 2>&1 | tail -20
```
Expected: `** TEST SUCCEEDED **` (todos os testes unitários; smoke pode pular sem permissão).

- [ ] **Step 8: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "feat: add SwiftUI UI and view model for end-to-end recording"
```

---

## Task 15: Smoke test manual + README

**Files:**
- Create: `docs/SMOKE-TEST.md`
- Create: `README.md`

**Interfaces:**
- Consumes: tudo.
- Produces: checklist de verificação manual + documentação inicial.

- [ ] **Step 1: Escrever `docs/SMOKE-TEST.md`**

```markdown
# OpenRecorder — Smoke Test Manual (macOS)

Pré-requisito: conceder permissões em Ajustes do Sistema → Privacidade e Segurança:
Gravação de Tela, Microfone, Monitoramento de Entrada. Reabrir o app após conceder.

## Roteiro

1. Abrir o app. O seletor lista pelo menos uma tela e janelas abertas.
2. Selecionar uma tela inteira + um microfone. Clicar "Gravar".
   - O timer começa a contar.
3. Durante ~10s: clicar em vários pontos da tela e mover o mouse.
4. Clicar "Parar".
5. A gravação aparece na lista. Clicar "Mostrar" → abre no Finder.
6. Abrir o `.mp4`: deve reproduzir vídeo + áudio do mic.
7. Abrir o `.metadata.json` ao lado: deve conter `version: 1`, dados de
   `recording`, `source`, e um array `events` com cliques/movimentos
   (coordenadas dentro do tamanho da fonte).

## Casos a verificar

- [ ] Tela inteira grava vídeo + áudio.
- [ ] Janela específica grava só a janela.
- [ ] metadata.json tem eventos de clique com coords plausíveis.
- [ ] Sem permissão de Monitoramento de Entrada: vídeo grava, events fica vazio (degrada).
- [ ] Sem mic selecionado/permitido: vídeo grava sem trilha de áudio.
- [ ] Parar e regravar várias vezes não trava o app.
```

- [ ] **Step 2: Escrever `README.md`**

```markdown
# OpenRecorder

Gravador de tela open source para macOS, com foco em zoom automático no clique
e export em 9:16 para redes sociais. Nativo (Swift/SwiftUI + ScreenCaptureKit +
AVFoundation), leve, sem dependências externas de runtime.

> Status: **F1 (fundação de captura)**. Grava tela/janela/região + microfone →
> `.mp4` + `metadata.json` de cliques. Zoom e export 9:16 vêm nas próximas fases.

## Requisitos

- macOS 13.0+
- Xcode 15+ e [XcodeGen](https://github.com/yonaskolb/XcodeGen) (`brew install xcodegen`)

## Build

```bash
xcodegen generate
xcodebuild build -scheme OpenRecorder -destination 'platform=macOS'
```

## Testes

```bash
xcodebuild test -scheme OpenRecorder -destination 'platform=macOS'
```

## Permissões

OpenRecorder precisa de Gravação de Tela, Microfone e Monitoramento de Entrada
(para registrar cliques). Conceda em Ajustes do Sistema → Privacidade e Segurança.

## Roadmap

- **F2** — Auto-zoom no clique + editor mínimo
- **F3** — Overlay de webcam
- **F4** — Export 9:16 (full / split-screen) + preview Instagram/TikTok

## Licença

MIT (a definir no arquivo LICENSE).
```

- [ ] **Step 3: Executar o smoke test manual**

Seguir `docs/SMOKE-TEST.md` integralmente. Marcar cada caso. Corrigir o que falhar (volta à task correspondente).

- [ ] **Step 4: Commit**

```bash
cd ~/Github/open-recorder && git add -A && \
git -c user.name="Hudson Brendon" -c user.email="contato.hudsonbrendon@gmail.com" \
commit -m "docs: add manual smoke test checklist and README"
```

---

## Self-Review (preenchido pelo autor do plano)

**1. Cobertura do spec:**
- Captura tela/janela/região → Tasks 8, 9 (SourceEnumerator, CaptureEngine; região via `sourceRect`).
- Microfone → Task 10 (AudioRecorder).
- Metadata de cliques/mouse → Tasks 2, 5, 12 (modelo, InputRecorder, finalizer).
- Gravação não-destrutiva (mp4 + metadata.json) → Tasks 6/7 (writer), 12 (metadata), 13 (coordenação).
- Comandos/fluxo UI ↔ core → Tasks 13, 14.
- Permissões macOS → Task 11 + avisos na UI (Task 14).
- Tratamento de erros → `onStreamError`, `errorMessage`, degradação sem input/mic (Tasks 9, 13, 14).
- Testes (unit + integração writer + smoke manual) → Tasks 2–7, 11–13 (unit/integração); 8, 9, 15 (smoke).
- Sem gap identificado para o escopo da F1.

**2. Placeholders:** nenhum "TBD/TODO"; todo passo de código traz o código. Smoke manual é explícito (não é placeholder, é o método de teste correto para captura dependente de hardware).

**3. Consistência de tipos:** `CaptureSource`, `RecordingMetadata`/`RecordingInfo`/`SourceInfo`/`InputEvent`, `VideoWriter(...hasAudio:)`, `RecordingResult`, `InputRecorder.ingest(...)`, `RecordingFinalizer.metadata/write`, `RecordingCoordinator.makeFilenames` — nomes e assinaturas batem entre tarefas produtoras e consumidoras.

**Limitação conhecida (documentada, fora do escopo F1):** sincronização fina de timestamps entre o relógio do ScreenCaptureKit (vídeo) e o do AVCaptureSession (mic) usa o PTS de cada buffer; pequeno drift é aceitável na F1 e será endereçado se necessário em fase futura.
```
