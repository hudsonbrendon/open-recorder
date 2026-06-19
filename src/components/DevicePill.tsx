import { Dropdown, type DropdownGroup } from "./Dropdown";

export function DevicePill(props: {
  icon: string;
  title: string;
  on: boolean;
  onToggle: (on: boolean) => void;
  groups: DropdownGroup[];
  value: string | null;
  onChange: (id: string) => void;
}) {
  return (
    <div className={"pill" + (props.on ? "" : " off")}>
      <button
        type="button"
        className="pill-head"
        onClick={() => props.onToggle(!props.on)}
        aria-pressed={props.on}
        title={props.on ? "Desligar" : "Ligar"}
      >
        <span className="pill-icon">{props.icon}</span>
        <span className="pill-title">{props.title}</span>
        <span className={"pill-switch" + (props.on ? " on" : "")}>
          <span className="pill-knob" />
        </span>
      </button>
      {props.on && (
        <Dropdown
          groups={props.groups}
          value={props.value}
          onChange={props.onChange}
          placeholder="Selecionar..."
        />
      )}
    </div>
  );
}
