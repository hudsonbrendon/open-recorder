import type { SourceOption } from "../lib/api";
import type { SourceKind } from "../state/useRecorder";
import { Dropdown } from "./Dropdown";

export function SourceSelect(props: {
  kind: SourceKind;
  onKindChange: (k: SourceKind) => void;
  displays: SourceOption[];
  windows: SourceOption[];
  value: string | null;
  onChange: (id: string) => void;
}) {
  const list = props.kind === "display" ? props.displays : props.windows;
  return (
    <div className="source-select">
      <div className="seg">
        <button
          type="button"
          className={"seg-btn" + (props.kind === "display" ? " active" : "")}
          onClick={() => props.onKindChange("display")}
        >
          Tela
        </button>
        <button
          type="button"
          className={"seg-btn" + (props.kind === "window" ? " active" : "")}
          onClick={() => props.onKindChange("window")}
        >
          Janela
        </button>
      </div>
      <Dropdown
        groups={[{ options: list.map((s) => ({ id: s.id, label: s.name })) }]}
        value={props.value}
        onChange={props.onChange}
        placeholder={props.kind === "display" ? "Selecionar tela..." : "Selecionar janela..."}
      />
    </div>
  );
}
