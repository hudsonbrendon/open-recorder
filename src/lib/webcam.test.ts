import { describe, it, expect } from "vitest";
import { geometry, type WebcamOverlay } from "./webcam";

const base: WebcamOverlay = {
  enabled: true, shape: "circle", x: 0.5, y: 0.5, size: 0.2,
  border_width: 3, border_color: "#ffffff", mirror: true,
};

describe("webcam geometry", () => {
  it("maps to pixels", () => {
    expect(geometry(base, 1000, 500)).toEqual({ s: 200, x: 500, y: 250 });
  });
  it("clamps within frame", () => {
    expect(geometry({ ...base, x: 0.99 }, 1000, 500)).toEqual({ s: 200, x: 800, y: 250 });
  });
});
