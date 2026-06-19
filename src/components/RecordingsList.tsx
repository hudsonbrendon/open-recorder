import type { RecordingResult } from "../lib/api";
import { fileName, formatElapsed } from "../lib/format";
import { revealInFolder } from "../lib/api";

export function RecordingsList(props: { items: RecordingResult[] }) {
  if (props.items.length === 0) return <p className="muted">Nenhuma gravação ainda.</p>;
  return (
    <ul className="recordings">
      {props.items.map((r) => (
        <li key={r.video_path}>
          <span>{fileName(r.video_path)}</span>
          <span className="muted">{formatElapsed(r.duration_ms)}</span>
          <button className="btn small" onClick={() => revealInFolder(r.video_path)}>Mostrar</button>
        </li>
      ))}
    </ul>
  );
}
