import { describe, it, expect } from "vitest";
import { formatElapsed, fileName } from "./format";

describe("formatElapsed", () => {
  it("formats milliseconds as MM:SS", () => {
    expect(formatElapsed(0)).toBe("00:00");
    expect(formatElapsed(65000)).toBe("01:05");
    expect(formatElapsed(3599000)).toBe("59:59");
  });
});

describe("fileName", () => {
  it("extracts the last path segment", () => {
    expect(fileName("/Users/x/Movies/OpenRecorder/REC-1.mp4")).toBe("REC-1.mp4");
    expect(fileName("C:\\\\Videos\\\\REC-2.mp4")).toBe("REC-2.mp4");
  });
});
