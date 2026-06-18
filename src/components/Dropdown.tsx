import { useEffect, useRef, useState } from "react";

export interface DropdownOption {
  id: string;
  label: string;
}

export interface DropdownGroup {
  label?: string;
  options: DropdownOption[];
}

export function Dropdown(props: {
  groups: DropdownGroup[];
  value: string | null;
  onChange: (id: string) => void;
  placeholder?: string;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const all = props.groups.flatMap((g) => g.options);
  const selected = all.find((o) => o.id === props.value);

  return (
    <div className="dd" ref={ref}>
      <button type="button" className="dd-btn" onClick={() => setOpen((v) => !v)}>
        <span className="dd-btn-label">
          {selected ? selected.label : props.placeholder ?? "Selecionar..."}
        </span>
        <span className="dd-caret" aria-hidden="true">▾</span>
      </button>
      {open && (
        <div className="dd-menu" role="listbox">
          {props.groups.map((g, gi) => (
            <div className="dd-group" key={gi}>
              {g.label && <div className="dd-group-label">{g.label}</div>}
              {g.options.length === 0 && <div className="dd-empty">—</div>}
              {g.options.map((o) => (
                <div
                  key={o.id}
                  role="option"
                  aria-selected={o.id === props.value}
                  className={"dd-item" + (o.id === props.value ? " selected" : "")}
                  onClick={() => {
                    props.onChange(o.id);
                    setOpen(false);
                  }}
                >
                  <span className="dd-check">{o.id === props.value ? "✓" : ""}</span>
                  <span className="dd-item-label">{o.label}</span>
                </div>
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
