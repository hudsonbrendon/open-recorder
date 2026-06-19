import type { SourceOption } from "../lib/api";
import { Dropdown } from "./Dropdown";

export function SourcePicker(props: {
  displays: SourceOption[]; windows: SourceOption[];
  value: string | null; onChange: (id: string) => void;
}) {
  return (
    <div className="field">
      <span>Fonte</span>
      <Dropdown
        value={props.value}
        onChange={props.onChange}
        placeholder="Selecionar fonte..."
        groups={[
          { label: "Telas", options: props.displays.map((d) => ({ id: d.id, label: d.name })) },
          { label: "Janelas", options: props.windows.map((w) => ({ id: w.id, label: w.name })) },
        ]}
      />
    </div>
  );
}
