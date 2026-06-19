import type { MicOption } from "../lib/api";
import { Dropdown } from "./Dropdown";

export function MicPicker(props: {
  mics: MicOption[]; value: string | null; onChange: (id: string) => void;
}) {
  return (
    <div className="field">
      <span>Microfone</span>
      <Dropdown
        value={props.value}
        onChange={props.onChange}
        placeholder="Selecionar microfone..."
        groups={[{ options: props.mics.map((m) => ({ id: m.id, label: m.name })) }]}
      />
    </div>
  );
}
