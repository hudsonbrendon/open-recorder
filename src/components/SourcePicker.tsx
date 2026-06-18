import type { SourceOption } from "../lib/api";

export function SourcePicker(props: {
  displays: SourceOption[]; windows: SourceOption[];
  value: string | null; onChange: (id: string) => void;
}) {
  return (
    <label className="field">
      <span>Fonte</span>
      <select value={props.value ?? ""} onChange={(e) => props.onChange(e.target.value)}>
        <optgroup label="Telas">
          {props.displays.map((d) => <option key={d.id} value={d.id}>{d.name}</option>)}
        </optgroup>
        <optgroup label="Janelas">
          {props.windows.map((w) => <option key={w.id} value={w.id}>{w.name}</option>)}
        </optgroup>
      </select>
    </label>
  );
}
