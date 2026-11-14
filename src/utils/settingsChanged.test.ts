import { describe, expect, it } from "vitest";
import type { Settings } from "../types";
import { settingsChanged } from "./settingsChanged";

const base = { volume: 80, blocks: ["announcement", "metronome"] } as unknown as Settings;

describe("settingsChanged", () => {
  it("is false for the very same settings", () => {
    expect(settingsChanged(base, base)).toBe(false);
  });

  it("is false when a list is rebuilt with the same items", () => {
    expect(settingsChanged({ ...base, blocks: ["announcement", "metronome"] }, base)).toBe(false);
  });

  it("is true when a list has other items", () => {
    expect(settingsChanged({ ...base, blocks: ["metronome"] }, base)).toBe(true);
  });

  it("is true when a list has the same items in another order", () => {
    expect(settingsChanged({ ...base, blocks: ["metronome", "announcement"] }, base)).toBe(true);
  });

  it("is true when a plain value changed", () => {
    expect(settingsChanged({ ...base, volume: 50 }, base)).toBe(true);
  });
});
