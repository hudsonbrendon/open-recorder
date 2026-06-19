import type { MicOption } from "../lib/api";

export function MicPicker(props: {
  mics: MicOption[]; value: string | null; onChange: (id: string) => void;
}) {
  return (
    <label className="field">
      <span>Microfone</span>
      <select value={props.value ?? ""} onChange={(e) => props.onChange(e.target.value)}>
        {props.mics.map((m) => <option key={m.id} value={m.id}>{m.name}</option>)}
      </select>
    </label>
  );
}
