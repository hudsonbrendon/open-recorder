import { useState } from "react";
import { useEditor } from "../state/useEditor";
import { PreviewCanvas } from "./PreviewCanvas";

export function EditorView(props: { videoPath: string; onBack: () => void }) {
  const ed = useEditor(props.videoPath);
  const [, setT] = useState(0);
  return (
    <div className="editor">
      <button className="btn small" onClick={props.onBack}>
        ← Voltar
      </button>
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
