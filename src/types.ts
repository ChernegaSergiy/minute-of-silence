// Mirror of Rust `AnnouncementVoice` enum

export type AnnouncementVoice = "bohdan_hdal" | "sonia_sotnyk" | "dania_khomutovskyi" | "radio_bg" | "air_alert";

// Mirror of Rust `AnthemVoice` enum

export type AnthemVoice = "default" | "mykhailo_khoma" | "oleksandr_ponomarov";

// Mirror of Rust `Settings` struct

export type AudioPreset =
  | "voice_metronome"
  | "metronome_only"
  | "voice_silence_bell"
  | "voice_silence"
  | "voice_metronome_anthem"
  | "voice_metronome_ending"
  | "metronome_anthem"
  | "bell_silence_bell"
  | "bell_metronome_bell"
  | "silence";

export interface Settings {
  /** Enable daily activation at 09:00. */
  ceremonyEnabled: boolean;
  /** Enable app autostart when the system boots. */
  autostartEnabled: boolean;
  /** Run ceremony only on weekdays (Mon-Fri). */
  weekdaysOnly: boolean;
  /** Selected audio preset. */
  preset: AudioPreset;
  volume: number; // 0–100
  /** Pause other media players before the ceremony. */
  pauseOtherPlayers: boolean;
  /** Automatically resume paused players after the ceremony. */
  resumeAfterCeremony: boolean;
  /** Show a visual overlay window when the ceremony starts. */
  showVisualOverlay: boolean;
  /** Show the flag animation window when the ceremony starts. */
  showFlagAnimation: boolean;
  /** Show personal dates on nearby days with weighted proximity. */
  showNearbyPersonalDates: boolean;
  /** Use system time instead of NTP. */
  systemTimeOnly: boolean;
  /** Prioritize app volume over system controls. */
  volumePriority: boolean;
  /** Automatically unmute system if muted during ceremony. */
  autoUnmute: boolean;
  /** NTP server hostname (used when system_time_only is false). */
  ntpServer: string;
  lateStartGraceMinutes: number; // 0–5
  /** Enable reminder notifications. */
  reminderEnabled: boolean;
  /** Minutes before 09:00 to show reminder. 0 = immediately. */
  reminderMinutesBefore: number; // 0–10
  /** Selected announcement voice. */
  announcementVoice: AnnouncementVoice;
  /** Selected anthem voice. */
  anthemVoice: AnthemVoice;
  /** Whether to follow the OS theme. */
  useSystemTheme?: boolean;
  /** Manual UI theme when not using system theme: 'light' | 'dark'. */
  uiTheme?: "light" | "dark";
  /** ID of the last personal date shown via nearby-days algorithm. */
  lastShownNearbyDateId: string | null;
}

// Mirror of Rust `StatusSnapshot` struct

export interface StatusSnapshot {
  ceremonyActive: boolean;
  skipTomorrow: boolean;
  lastActivation: string | null;
  lastNtpSync: string | null;
}

// UI helpers

export const DEFAULT_SETTINGS: Settings = {
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
};

export interface PersonalDate {
  id?: string;
  month: number; // 1-12
  day: number; // 1-31
  label: string;
  year: number;
}
