import type { RecordingResult } from "../lib/api";
import { fileName, formatElapsed } from "../lib/format";
import { revealInFolder } from "../lib/api";

export function RecordingsList(props: {
  items: RecordingResult[];
  onEdit: (p: string) => void;
}) {
  if (props.items.length === 0) return <p className="muted">Nenhuma gravação ainda.</p>;
  return (
    <ul className="recordings">
      {props.items.map((r) => (
        <li key={r.video_path}>
          <span>{fileName(r.video_path)}</span>
          <span className="muted">{formatElapsed(r.duration_ms)}</span>
          <button className="btn small" onClick={() => revealInFolder(r.video_path)}>Mostrar</button>
          <button className="btn small" onClick={() => props.onEdit(r.video_path)}>Editar</button>
        </li>
      ))}
    </ul>
  );
}
