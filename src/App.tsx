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
