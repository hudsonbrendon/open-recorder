import type { ZoomModel } from "../lib/zoom";

export function SegmentInspector(props: {
  model: ZoomModel;
  selected: number | null;
  onChange: (m: ZoomModel) => void;
  onDelete: (i: number) => void;
}) {
  if (props.selected === null)
    return <p className="muted">Selecione um zoom na timeline.</p>;
  const seg = props.model.segments[props.selected];
  if (!seg) return null;
  const update = (patch: Partial<typeof seg>) => {
    const segments = props.model.segments.map((s, i) =>
      i === props.selected ? { ...s, ...patch } : s,
    );
    props.onChange({ ...props.model, segments });
  };
  return (
    <div className="inspector">
      <label>
        Zoom (escala)
        <input
          type="range"
          min={1}
          max={4}
          step={0.1}
          value={seg.scale}
          onChange={(e) => update({ scale: Number(e.target.value) })}
        />
        <span>{seg.scale.toFixed(1)}×</span>
      </label>
      <label>
        Início (ms)
        <input
          type="number"
          value={seg.start_ms}
          onChange={(e) => update({ start_ms: Number(e.target.value) })}
        />
      </label>
      <label>
        Fim (ms)
        <input
          type="number"
          value={seg.end_ms}
          onChange={(e) => update({ end_ms: Number(e.target.value) })}
        />
      </label>
      <button
        className="btn small"
        onClick={() => props.onDelete(props.selected!)}
      >
        Deletar zoom
      </button>
    </div>
  );
}
