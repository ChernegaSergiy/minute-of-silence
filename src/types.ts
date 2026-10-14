import type { Settings } from "./types.gen";

export * from "./types.gen";

// UI helpers

export const DEFAULT_SETTINGS: Settings = {
  version: 1,
  ceremonyEnabled: true,
  autostartEnabled: true,
  weekdaysOnly: false,
  preset: "voice_metronome",
  volume: 80,
  pauseOtherPlayers: true,
  resumeAfterCeremony: false,
  showVisualOverlay: true,
  showFlagAnimation: false,
  showNearbyPersonalDates: false,
  systemTimeOnly: false,
  volumePriority: false,
  autoUnmute: false,
  ntpServer: "pool.ntp.org",
  lateStartGraceMinutes: 1,
  reminderEnabled: false,
  reminderMinutesBefore: 5,
  announcementVoice: "bohdan_hdal",
  anthemVoice: "default",
  useSystemTheme: true,
  uiTheme: "light",
  lastShownNearbyDateId: null,
  skipDate: null,
};

export interface PersonalDate {
  id?: string;
  month: number; // 1-12
  day: number; // 1-31
  label: string;
  year: number;
}
