import { describe, it, expect } from "vitest";
import { smoothstep, zoomAt, type ZoomModel } from "./zoom";

const fixture: ZoomModel = {
  version: 1,
  segments: [{
    start_ms: 0, end_ms: 2000, ease_in_ms: 300, ease_out_ms: 400,
    scale: 2.0, targets: [{ t_ms: 0, x: 0.25, y: 0.75 }],
  }],
};
const close = (a: number, b: number) => Math.abs(a - b) < 1e-6;

describe("smoothstep", () => {
  it("endpoints, mid, clamp", () => {
    expect(close(smoothstep(0), 0)).toBe(true);
    expect(close(smoothstep(1), 1)).toBe(true);
    expect(close(smoothstep(0.5), 0.5)).toBe(true);
    expect(close(smoothstep(-3), 0)).toBe(true);
    expect(close(smoothstep(7), 1)).toBe(true);
  });
});

describe("zoomAt parity", () => {
  it("outside -> identity", () => {
    const z = zoomAt(fixture, 2500);
    expect(close(z.scale, 1) && close(z.cx, 0.5) && close(z.cy, 0.5)).toBe(true);
  });
  it("ease-in mid", () => {
    const z = zoomAt(fixture, 150);
    expect(close(z.scale, 1.5)).toBe(true);
    expect(close(z.cx, 1 / 3)).toBe(true);
    expect(close(z.cy, 2 / 3)).toBe(true);
  });
  it("plateau", () => {
    const z = zoomAt(fixture, 1000);
    expect(close(z.scale, 2) && close(z.cx, 0.25) && close(z.cy, 0.75)).toBe(true);
  });
  it("ease-out mid", () => {
    expect(close(zoomAt(fixture, 1800).scale, 1.5)).toBe(true);
  });
});
