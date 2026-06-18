import { describe, it, expect } from "vitest";
import { msToX, xToMs } from "./timeline";

describe("timeline mapping", () => {
  it("maps ms to x and back", () => {
    expect(msToX(0, 10000, 500)).toBe(0);
    expect(msToX(10000, 10000, 500)).toBe(500);
    expect(msToX(5000, 10000, 500)).toBe(250);
    expect(Math.round(xToMs(250, 10000, 500))).toBe(5000);
  });
  it("clamps out of range", () => {
    expect(msToX(20000, 10000, 500)).toBe(500);
    expect(xToMs(-10, 10000, 500)).toBe(0);
  });
});
