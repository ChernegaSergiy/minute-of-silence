/**
 * Typed wrappers around Tauri `invoke` calls.
 *
 * All functions in this module correspond 1-to-1 with a `#[tauri::command]`
 * in `src-tauri/src/app/commands.rs`.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type {
  AppInitialState,
  DiscoveredDevice,
  ExportBundle,
  NotificationPermission,
  PersonalDate,
  Settings,
  StatusSnapshot,
} from "../types";

// Initial State

export async function getInitialState(): Promise<AppInitialState> {
  return invoke<AppInitialState>("get_initial_state");
}

export async function getPlatform(): Promise<"android" | "desktop"> {
  return invoke<"android" | "desktop">("get_platform");
}

// Settings

export async function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export async function saveSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>("save_settings", { settings });
}

export async function resetSettings(): Promise<Settings> {
  return invoke<Settings>("reset_settings");
}

export async function setLastShownNearbyDate(id: string | null): Promise<void> {
  return invoke("set_last_shown_nearby_date", { id });
}

export async function exportSettings(): Promise<string> {
  return invoke<string>("export_settings");
}

export async function importSettings(json: string): Promise<ExportBundle> {
  return invoke<ExportBundle>("import_settings", { json });
}

// Personal Dates

export async function getPersonalDates(): Promise<PersonalDate[]> {
  return invoke<PersonalDate[]>("get_personal_dates");
}

export async function savePersonalDates(dates: PersonalDate[]): Promise<void> {
  return invoke("save_personal_dates", { dates });
}

// Status

export async function getStatus(): Promise<StatusSnapshot> {
  return invoke<StatusSnapshot>("get_status");
}

export async function syncNtpNow(): Promise<StatusSnapshot> {
  return invoke<StatusSnapshot>("sync_ntp_now");
}

// Notifications

export async function getNotificationPermission(): Promise<NotificationPermission> {
  return invoke<NotificationPermission>("notification_permission_status");
}

export async function openNotificationSettings(): Promise<void> {
  return invoke<void>("open_notification_settings");
}

export async function getExactAlarmPermission(): Promise<boolean> {
  return invoke<boolean>("exact_alarm_permission_status");
}

export async function openExactAlarmSettings(): Promise<void> {
  return invoke<void>("open_exact_alarm_settings");
}

export async function openAboutLink(url: string): Promise<void> {
  return invoke<void>("open_about_link", { url });
}

// Skip / unskip

export async function skipNext(): Promise<void> {
  return invoke<void>("skip_next");
}

export async function unskipNext(): Promise<void> {
  return invoke<void>("unskip_next");
}

// Manual trigger

export async function triggerCeremonyNow(): Promise<void> {
  return invoke<void>("trigger_ceremony_now");
}

export async function finishCeremonyNow(): Promise<void> {
  return invoke<void>("finish_ceremony_now");
}

export async function stopTestCeremony(): Promise<void> {
  return invoke<void>("stop_test_ceremony");
}

export async function getLogContents(): Promise<string> {
  return invoke<string>("get_log_contents");
}

// Device discovery & peer sync

export async function discoverDevices(): Promise<DiscoveredDevice[]> {
  return invoke<DiscoveredDevice[]>("discover_devices");
}

export async function sendSettingsToDevice(ip: string, port: number): Promise<void> {
  return invoke<void>("send_settings_to_device", { ip, port });
}

export async function getReceivingSettingsEnabled(): Promise<boolean> {
  return invoke<boolean>("get_receiving_settings_enabled");
}

export async function setReceivingSettingsEnabled(enabled: boolean): Promise<boolean> {
  return invoke<boolean>("set_receiving_settings_enabled", { enabled });
}

export function onReceivingSettingsStopped(callback: () => void): Promise<UnlistenFn> {
  return listen("device-receiving-stopped", callback);
}

export const INCOMING_SETTINGS_EVENT = "incoming-settings-offer";

export type IncomingSettingsOffer = {
  id: number;
  senderName: string;
  senderIp: string;
  json: string;
};

export function onIncomingSettingsOffer(
  callback: () => void,
): Promise<UnlistenFn> {
  return listen(INCOMING_SETTINGS_EVENT, callback);
}

export async function getPendingSettingsOffer(): Promise<IncomingSettingsOffer | null> {
  return invoke<IncomingSettingsOffer | null>("get_pending_settings_offer");
}

export async function dismissPendingSettingsOffer(id: number): Promise<void> {
  return invoke<void>("dismiss_pending_settings_offer", { id });
}

// Event listeners

export const CEREMONY_START_EVENT = "ceremony-start";
export const CEREMONY_END_EVENT = "ceremony-end";

export type CeremonyStartPayload = {
  ceremony_id?: number;
  duration_ms?: number;
  elapsed_ms?: number;
  is_test?: boolean;
};

export function onCeremonyStart(callback: (payload: CeremonyStartPayload) => void): Promise<UnlistenFn> {
  return listen(CEREMONY_START_EVENT, (e) => callback((e.payload as unknown) as CeremonyStartPayload));
}

export function onCeremonyEnd(callback: () => void): Promise<UnlistenFn> {
  return listen(CEREMONY_END_EVENT, callback);
}

export async function bringWindowToFront(): Promise<void> {
  const win = getCurrentWindow();
  const isMin = await win.isMinimized();
  if (isMin) {
    await win.unminimize();
  }
  const isVisible = await win.isVisible();
  if (!isVisible) {
    await win.show();
  }
  await win.setFocus();
}
