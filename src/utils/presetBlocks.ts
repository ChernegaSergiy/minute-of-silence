import type { PresetBlock, VoiceCapabilities } from "../types";

/** The main part of the ceremony: a metronome or one minute of silence. */
export type BodyBlock = "metronome" | "silence";

/** What closes the ceremony after its main part. */
export type ClosingBlock = "none" | "ending" | "bell" | "anthem";

/**
 * The preset constructor as a fixed row of slots. The stored settings keep a
 * plain list of blocks, which is always built from the slots in this order.
 */
export interface PresetSlots {
  openingBell: boolean;
  announcement: boolean;
  body: BodyBlock;
  closing: ClosingBlock;
}

/** Which slots the selected voice allows. */
export interface SlotAvailability {
  openingBell: boolean;
  ending: boolean;
}

const isBody = (block: PresetBlock): block is BodyBlock => block === "metronome" || block === "silence";

/** Reads the slots out of a list of blocks. Blocks that fit no slot are ignored. */
export function blocksToSlots(blocks: readonly PresetBlock[]): PresetSlots {
  const bodyIndex = blocks.findIndex(isBody);
  const before = bodyIndex === -1 ? blocks : blocks.slice(0, bodyIndex);
  const after = bodyIndex === -1 ? [] : blocks.slice(bodyIndex + 1);

  let closing: ClosingBlock = "none";
  if (after.includes("anthem")) closing = "anthem";
  else if (after.includes("ending")) closing = "ending";
  else if (after.includes("bell")) closing = "bell";

  return {
    openingBell: before.includes("bell"),
    announcement: before.includes("announcement"),
    body: bodyIndex === -1 || blocks[bodyIndex] === "metronome" ? "metronome" : "silence",
    closing,
  };
}

/** Builds the list of blocks, in playback order, from the slots. */
export function slotsToBlocks(slots: PresetSlots): PresetBlock[] {
  const blocks: PresetBlock[] = [];
  if (slots.openingBell) blocks.push("bell");
  if (slots.announcement) blocks.push("announcement");
  blocks.push(slots.body);
  if (slots.closing !== "none") blocks.push(slots.closing);
  return blocks;
}

/**
 * Tells which slots can be switched on. Some voices have no ending, and the
 * announcement of some voices already starts with the bell.
 */
export function slotAvailability(slots: PresetSlots, voice?: VoiceCapabilities): SlotAvailability {
  return {
    openingBell: !(slots.announcement && voice?.announcementIncludesBell),
    ending: voice ? voice.hasEnding : true,
  };
}

/** Switches off the slots the voice cannot play. */
export function normalizeSlots(slots: PresetSlots, voice?: VoiceCapabilities): PresetSlots {
  const available = slotAvailability(slots, voice);
  return {
    ...slots,
    openingBell: slots.openingBell && available.openingBell,
    closing: slots.closing === "ending" && !available.ending ? "none" : slots.closing,
  };
}
