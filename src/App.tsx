import { useState } from "react";
import { useRecorder } from "./state/useRecorder";
import { SourceSelect } from "./components/SourceSelect";
import { DevicePill } from "./components/DevicePill";
import { RecordControls } from "./components/RecordControls";
import { RecordingsList } from "./components/RecordingsList";
import { EditorView } from "./components/EditorView";
import "./App.css";

export default function App() {
  const r = useRecorder();
  const [editing, setEditing] = useState<string | null>(null);

  if (editing !== null) {
    return <EditorView videoPath={editing} onBack={() => setEditing(null)} />;
  }

  return (
    <main className="recorder">
      <header className="recorder-head">
        <span className="brand-dot" />
        <h1>OpenRecorder</h1>
      </header>

      <section className="recorder-card">
        <span className="ctl-label">Fonte</span>
        <SourceSelect
          kind={r.sourceKind}
          onKindChange={r.setSourceKind}
          displays={r.displays}
          windows={r.windows}
          value={r.selectedId}
          onChange={r.setSelectedId}
        />

        <div className="pills">
          <DevicePill
            icon="🎙"
            title="Microfone"
            on={r.micOn}
            onToggle={r.setMicOn}
            groups={[{ options: r.mics.map((m) => ({ id: m.id, label: m.name })) }]}
            value={r.selectedMic}
            onChange={r.setSelectedMic}
          />
          <DevicePill
            icon="📷"
            title="Câmera"
            on={r.camOn}
            onToggle={r.setCamOn}
            groups={[{ options: r.cameras.map((c) => ({ id: c.id, label: c.name })) }]}
            value={r.selectedCamera}
            onChange={r.setSelectedCamera}
          />
        </div>

        <RecordControls
          isRecording={r.isRecording}
          elapsed={r.elapsed}
          disabled={!r.selected}
          onStart={r.start}
          onStop={r.stop}
        />
      </section>

      <section className="recordings-section">
        <h2 className="section-title">Gravações</h2>
        <RecordingsList items={r.recordings} onEdit={setEditing} />
      </section>

      {r.error && <p className="error">{r.error}</p>}
    </main>
  );
}
