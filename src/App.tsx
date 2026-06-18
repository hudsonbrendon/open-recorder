import { useState } from "react";
import { useRecorder } from "./state/useRecorder";
import { SourcePicker } from "./components/SourcePicker";
import { MicPicker } from "./components/MicPicker";
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
    <main className="app">
      <h1>● OpenRecorder</h1>
      <SourcePicker displays={r.displays} windows={r.windows}
                    value={r.selectedId} onChange={r.setSelectedId} />
      <MicPicker mics={r.mics} value={r.selectedMic} onChange={r.setSelectedMic} />
      <RecordControls isRecording={r.isRecording} elapsed={r.elapsed}
                      disabled={!r.selectedId} onStart={r.start} onStop={r.stop} />
      <hr />
      <h2>Gravações</h2>
      <RecordingsList items={r.recordings} onEdit={setEditing} />
      {r.error && <p className="error">{r.error}</p>}
    </main>
  );
}
