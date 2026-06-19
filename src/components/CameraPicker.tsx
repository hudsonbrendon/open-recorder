import type { CameraOption } from "../lib/api";
import { Dropdown } from "./Dropdown";

export function CameraPicker(props: {
  cameras: CameraOption[];
  value: string | null;
  onChange: (id: string) => void;
}) {
  return (
    <div className="field">
      <span>Câmera</span>
      <Dropdown
        value={props.value ?? ""}
        onChange={(id) => props.onChange(id)}
        placeholder="Nenhuma"
        groups={[
          {
            options: [
              { id: "", label: "Nenhuma" },
              ...props.cameras.map((c) => ({ id: c.id, label: c.name })),
            ],
          },
        ]}
      />
    </div>
  );
}
