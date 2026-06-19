import type { WebcamOverlay } from "./webcam";

export interface ZoomTarget { t_ms: number; x: number; y: number }
export interface ZoomSegment {
  start_ms: number; end_ms: number; ease_in_ms: number; ease_out_ms: number;
  scale: number; targets: ZoomTarget[];
}
export interface ZoomModel { version: number; segments: ZoomSegment[]; webcam?: WebcamOverlay }
export interface ZoomAt { scale: number; cx: number; cy: number }

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

export function smoothstep(p: number): number {
  const c = clamp(p, 0, 1);
  return c * c * (3 - 2 * c);
}

function targetAt(seg: ZoomSegment, t: number): [number, number] {
  const ts = seg.targets;
  if (ts.length === 1) return [ts[0].x, ts[0].y];
  const first = ts[0].t_ms;
  const last = ts[ts.length - 1].t_ms;
  const tc = clamp(t, first, last);
  for (let i = 0; i < ts.length - 1; i++) {
    const a = ts[i], b = ts[i + 1];
    if (tc >= a.t_ms && tc <= b.t_ms) {
      const span = Math.max(1, b.t_ms - a.t_ms);
      const f = (tc - a.t_ms) / span;
      return [a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f];
    }
  }
  return [ts[ts.length - 1].x, ts[ts.length - 1].y];
}

export function zoomAt(model: ZoomModel, tMs: number): ZoomAt {
  for (const seg of model.segments) {
    if (tMs < seg.start_ms || tMs >= seg.end_ms) continue;
    const rel = tMs - seg.start_ms;
    const dur = seg.end_ms - seg.start_ms;
    const ein = seg.ease_in_ms, eout = seg.ease_out_ms;
    const eIn = ein > 0 ? smoothstep(rel / ein) : 1;
    const eOut = eout > 0 ? smoothstep((dur - rel) / eout) : 1;
    let e: number;
    if (rel < ein && rel > dur - eout) e = Math.min(eIn, eOut);
    else if (rel < ein) e = eIn;
    else if (rel > dur - eout) e = eOut;
    else e = 1;
    const scale = 1 + (seg.scale - 1) * e;
    const [tx, ty] = targetAt(seg, tMs);
    const m = 0.5 / scale;
    return { scale, cx: clamp(tx, m, 1 - m), cy: clamp(ty, m, 1 - m) };
  }
  return { scale: 1, cx: 0.5, cy: 0.5 };
}
