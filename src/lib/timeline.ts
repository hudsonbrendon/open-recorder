const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

export function msToX(ms: number, durationMs: number, widthPx: number): number {
  if (durationMs <= 0) return 0;
  return clamp((ms / durationMs) * widthPx, 0, widthPx);
}

export function xToMs(x: number, durationMs: number, widthPx: number): number {
  if (widthPx <= 0) return 0;
  return clamp((x / widthPx) * durationMs, 0, durationMs);
}
