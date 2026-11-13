import { describe, expect, it } from "vitest";
import type { PresetBlock, VoiceCapabilities } from "../types";
import {
  blocksToSlots,
  normalizeSlots,
  slotAvailability,
  slotsToBlocks,
  type PresetSlots,
} from "./presetBlocks";

const builtinPresets: Record<string, PresetBlock[]> = {
  voice_metronome: ["announcement", "metronome"],
  metronome_only: ["metronome"],
  voice_silence_bell: ["announcement", "silence", "bell"],
  voice_silence: ["announcement", "silence"],
  voice_metronome_anthem: ["announcement", "metronome", "anthem"],
  voice_metronome_ending: ["announcement", "metronome", "ending"],
  metronome_anthem: ["metronome", "anthem"],
  bell_silence_bell: ["bell", "silence", "bell"],
  bell_metronome_bell: ["bell", "metronome", "bell"],
  silence: ["silence"],
};

const plainVoice: VoiceCapabilities = {
  voice: "bohdan_hdal",
  hasEnding: true,
  announcementIncludesBell: false,
};
const noEndingVoice: VoiceCapabilities = { ...plainVoice, voice: "air_alert", hasEnding: false };
const builtInBellVoice: VoiceCapabilities = {
  ...plainVoice,
  voice: "vinnytsia_city",
  announcementIncludesBell: true,
};

const slots = (overrides: Partial<PresetSlots> = {}): PresetSlots => ({
  openingBell: false,
  announcement: false,
  body: "metronome",
  closing: "none",
  ...overrides,
});

describe("blocksToSlots / slotsToBlocks", () => {
  it.each(Object.entries(builtinPresets))("keeps the blocks of %s unchanged", (_name, blocks) => {
    expect(slotsToBlocks(blocksToSlots(blocks))).toEqual(blocks);
  });

  it("tells an opening bell from a closing bell", () => {
    expect(blocksToSlots(["bell", "silence", "bell"])).toEqual(
      slots({ openingBell: true, body: "silence", closing: "bell" }),
    );
    expect(blocksToSlots(["announcement", "silence", "bell"])).toEqual(
      slots({ announcement: true, body: "silence", closing: "bell" }),
    );
  });

  it("always builds blocks in the canonical order", () => {
    expect(
      slotsToBlocks(slots({ openingBell: true, announcement: true, body: "silence", closing: "anthem" })),
    ).toEqual(["bell", "announcement", "silence", "anthem"]);
  });

  it("falls back to the metronome when there is no main part", () => {
    expect(blocksToSlots([]).body).toBe("metronome");
  });
});

describe("slotAvailability", () => {
  it("allows everything for a plain voice", () => {
    expect(slotAvailability(slots({ announcement: true }), plainVoice)).toEqual({
      openingBell: true,
      ending: true,
    });
  });

  it("disables the ending for a voice without one", () => {
    expect(slotAvailability(slots(), noEndingVoice).ending).toBe(false);
  });

  it("disables the opening bell only together with an announcement that has its own bell", () => {
    expect(slotAvailability(slots({ announcement: true }), builtInBellVoice).openingBell).toBe(false);
    expect(slotAvailability(slots({ announcement: false }), builtInBellVoice).openingBell).toBe(true);
  });

  it("allows everything while the voice is unknown", () => {
    expect(slotAvailability(slots({ announcement: true }))).toEqual({ openingBell: true, ending: true });
  });
});

describe("normalizeSlots", () => {
  it("switches off an ending the voice does not have", () => {
    expect(normalizeSlots(slots({ closing: "ending" }), noEndingVoice).closing).toBe("none");
  });

  it("keeps the other closing blocks for a voice without an ending", () => {
    for (const closing of ["bell", "anthem", "none"] as const) {
      expect(normalizeSlots(slots({ closing }), noEndingVoice).closing).toBe(closing);
    }
  });

  it("switches off the opening bell before an announcement with its own bell", () => {
    const result = normalizeSlots(slots({ openingBell: true, announcement: true }), builtInBellVoice);
    expect(result.openingBell).toBe(false);
    expect(result.announcement).toBe(true);
  });

  it("keeps the opening bell when there is no announcement", () => {
    expect(normalizeSlots(slots({ openingBell: true }), builtInBellVoice).openingBell).toBe(true);
  });

  it("leaves the slots alone for a plain voice", () => {
    const original = slots({ openingBell: true, announcement: true, closing: "ending" });
    expect(normalizeSlots(original, plainVoice)).toEqual(original);
  });

  it("leaves the slots alone while the voice is unknown", () => {
    const original = slots({ openingBell: true, announcement: true, closing: "ending" });
    expect(normalizeSlots(original)).toEqual(original);
  });
});
