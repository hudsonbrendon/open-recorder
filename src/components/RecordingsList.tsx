import type { RecordingResult } from "../lib/api";
import { fileName, formatElapsed } from "../lib/format";
import { revealInFolder } from "../lib/api";

export function RecordingsList(props: {
  items: RecordingResult[];
  onEdit: (p: string) => void;
}) {
  if (props.items.length === 0)
    return <p className="rec-empty">Nenhuma gravação ainda.</p>;
  return (
    <div className="rec-list">
      {props.items.map((r) => (
        <div className="rec-card" key={r.video_path}>
          <div className="rec-thumb">
            <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M23 7l-7 5 7 5V7z" />
              <rect x="1" y="5" width="15" height="14" rx="2" ry="2" />
            </svg>
          </div>
          <div className="rec-meta">
            <div className="rec-name">{fileName(r.video_path)}</div>
            <div className="rec-dur">{formatElapsed(r.duration_ms)}</div>
          </div>
          <div className="rec-actions">
            <button className="ghost-btn" onClick={() => revealInFolder(r.video_path)}>
              Mostrar
            </button>
            <button className="ghost-btn primary" onClick={() => props.onEdit(r.video_path)}>
              Editar
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}
