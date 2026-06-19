export interface WebcamOverlay {
  enabled: boolean;
  shape: "circle" | "rounded";
  x: number;
  y: number;
  size: number;
  border_width: number;
  border_color: string;
  mirror: boolean;
}

export function geometry(
  ov: WebcamOverlay,
  outW: number,
  outH: number
): { s: number; x: number; y: number } {
  const clamp = (v: number, lo: number, hi: number) =>
    Math.min(hi, Math.max(lo, v));
  const maxS = Math.min(outW, outH);
  const s = clamp(Math.round(ov.size * outW), 1, maxS);
  const x = clamp(Math.round(ov.x * outW), 0, outW - s);
  const y = clamp(Math.round(ov.y * outH), 0, outH - s);
  return { s, x, y };
}
