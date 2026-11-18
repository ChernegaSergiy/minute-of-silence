import { Suspense, lazy, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Button,
  FluentProvider,
  NavDrawer,
  NavDrawerBody,
  NavItem,
  Spinner,
  Tab,
  TabList,
  makeStyles,
  shorthands,
  tokens,
  webDarkTheme,
  webLightTheme,
} from "@fluentui/react-components";
import {
  DocumentBulletList20Regular,
  Info20Regular,
  Play20Regular,
  Save20Regular,
  Settings20Regular,
  Stop20Regular,
  CalendarMonth20Regular,
} from "@fluentui/react-icons";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import {
  bringWindowToFront,
  getInitialState,
  getStatus,
  markCeremonyOverlayReady,
  onCeremonyEnd,
  onCeremonyStart,
  saveSettings,
  stopTestCeremony,
  syncNtpNow,
  triggerCeremonyNow,
} from "./utils/api";
import type { PersonalDate, Settings, StatusSnapshot, VoiceCapabilities } from "./types";
import { t } from "./utils/i18n";
import { settingsChanged } from "./utils/settingsChanged";
import AboutTab from "./components/AboutTab";
import Overlay from "./components/Overlay";
import SettingsTab from "./components/SettingsTab";
import PersonalDatesTab from "./components/PersonalDatesTab";
import UpdateDialog, { type UpdateInfo } from "./components/UpdateDialog";
import { useIdle } from "./hooks/useIdle";

const ChangelogTab = lazy(() => import("./components/ChangelogTab"));

const DEFAULT_STATUS: StatusSnapshot = {
  ceremonyActive: false,
  ceremonyIsTest: false,
  skipTomorrow: false,
  lastActivation: null,
  lastNtpSync: null,
};

const useStyles = makeStyles({
  layout: {
    height: "100%",
    display: "flex",
    flexDirection: "column",
  },
  body: {
    display: "flex",
    flex: 1,
    ...shorthands.overflow("hidden"),
  },
  content: {
    flex: 1,
    display: "flex",
    flexDirection: "column",
    ...shorthands.overflow("hidden"),
  },
  scroll: {
    flex: 1,
    overflowY: "auto",
    padding: tokens.spacingVerticalL,
  },
  buttonBar: {
    display: "flex",
    gap: tokens.spacingHorizontalM,
    padding: tokens.spacingVerticalL,
    ...shorthands.borderTop("1px", "solid", tokens.colorNeutralStroke2),
    "@media (max-width: 600px)": {
      gap: tokens.spacingHorizontalS,
    },
  },
  buttonBarButton: {
    "@media (max-width: 600px)": {
      flex: "1 1 0",
      minWidth: 0,
    },
  },
  changelogSpinner: {
    display: "flex",
    justifyContent: "center",
    paddingTop: tokens.spacingVerticalL,
  },
  loadingContainer: {
    display: "flex",
    justifyContent: "center",
    alignItems: "center",
    height: "100%",
  },
  desktopNav: {
    display: "block",
    "@media (max-width: 600px)": {
      display: "none",
    },
  },
  mobileNav: {
    display: "none",
    "@media (max-width: 600px)": {
      display: "block",
      borderTop: `1px solid ${tokens.colorNeutralStroke2}`,
      backgroundColor: tokens.colorNeutralBackground1,
    },
  },
  mobileNavList: {
    display: "flex",
  },
  mobileNavItem: {
    flex: 1,
    minWidth: 0,
    height: "56px",
    padding: "4px 0",
  },
  mobileNavContent: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    gap: "2px",
    width: "100%",
    height: "100%",
  },
  mobileNavIcon: {
    display: "grid",
    placeItems: "center",
    height: "20px",
    "& svg": {
      width: "20px",
      height: "20px",
    },
  },
  mobileNavLabel: {
    fontSize: "9px",
    fontWeight: tokens.fontWeightRegular,
    lineHeight: "12px",
    letterSpacing: 0,
    whiteSpace: "nowrap",
  },
});

export default function App() {
  const styles = useStyles();
  const [selectedNav, setSelectedNav] = useState<string>("settings");
  const [prefersDark, setPrefersDark] = useState(() =>
    typeof window !== "undefined"
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
      : true
  );
  const [settings, setSettings] = useState<Settings | null>(null);
  const [initialSettings, setInitialSettings] = useState<Settings | null>(null);
  const [personalDates, setPersonalDates] = useState<PersonalDate[]>([]);
  const [status, setStatus] = useState<StatusSnapshot>(DEFAULT_STATUS);
  const [version, setVersion] = useState("...");
  const [voices, setVoices] = useState<VoiceCapabilities[]>([]);
  const [showOverlay, setShowOverlay] = useState(false);
  const [overlayCeremonyId, setOverlayCeremonyId] = useState<number>();
  const [ceremonyDurationMs, setCeremonyDurationMs] = useState<number | undefined>(undefined);
  const [isTestCeremony, setIsTestCeremony] = useState(false);
  const [testRunning, setTestRunning] = useState(false);
  const [volumeValue, setVolumeValue] = useState(80);
  const [syncing, setSyncing] = useState(false);
  const [hydrated, setHydrated] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [showUpdateDialog, setShowUpdateDialog] = useState(false);
  const [updateDismissed, setUpdateDismissed] = useState(false);
  const [isInstallingUpdate, setIsInstallingUpdate] = useState(false);
  const isIdle = useIdle(15000); // 15 seconds idle timeout
  const initRef = useRef(false);
  const hasImportedRef = useRef(false);

  const isDirty = useMemo(() => {
    if (!hydrated || !settings || !initialSettings) return false;
    return settingsChanged(settings, initialSettings);
  }, [hydrated, settings, initialSettings]);

  const effectiveDark = (settings?.useSystemTheme ?? true) ? prefersDark : settings?.uiTheme === "dark";

  const handleOverlayReady = useCallback(() => {
    if (overlayCeremonyId === undefined) return;
    markCeremonyOverlayReady(overlayCeremonyId).catch((error) =>
      console.warn("Failed to report ceremony overlay readiness:", error)
    );
  }, [overlayCeremonyId]);

  useEffect(() => {
    if (initRef.current) return;
    initRef.current = true;

    (async () => {
      getCurrentWindow()
        .setTitle(t("app.title"))
        .catch((error) => console.warn("Failed to set localized window title:", error));

      // 1. Atomic startup hydration: fetch all initial state in a single IPC call
      try {
        const init = await getInitialState();
        if (!hasImportedRef.current) {
          setSettings(init.settings);
          setInitialSettings(init.settings);
          setVolumeValue(init.settings.volume);
        }
        setPersonalDates(init.personalDates);
        setStatus(init.status);
        setTestRunning(init.status.ceremonyIsTest);
        setVersion(init.version);
        setVoices(init.voices);
      } catch (error) {
        console.error("Failed to load initial application state:", error);
      } finally {
        setHydrated(true);
      }

      // 2. Non-blocking background network update check
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const update = await invoke<UpdateInfo | null>("check_for_updates").catch(() => null);
        if (update) {
          setUpdateInfo(update);
        }
      } catch {
        // ignore background update check errors
      }
    })();
  }, []);

  useEffect(() => {
    const refresh = async () => {
      try {
        const nextStatus = await getStatus();
        setStatus(nextStatus);
        setTestRunning(nextStatus.ceremonyIsTest);
      } catch {
        // ignore transient status refresh errors
      }
    };

    const unlisteners: (() => void)[] = [];
    (async () => {
      unlisteners.push(
        await onCeremonyStart((payload) => {
          const isTest = payload?.is_test ?? false;
          setTestRunning(isTest);
          setShowOverlay(true);
          setCeremonyDurationMs(payload?.duration_ms);
          setIsTestCeremony(isTest);
          setOverlayCeremonyId(isTest ? undefined : payload?.ceremony_id ?? undefined);
          refresh();
        }),
        await onCeremonyEnd(() => {
          setTestRunning(false);
          setShowOverlay(false);
          setCeremonyDurationMs(undefined);
          setIsTestCeremony(false);
          setOverlayCeremonyId(undefined);
          refresh();
        }),
        await listen("ntp-synced", refresh),
        await listen("status-updated", refresh),
        await listen<UpdateInfo>("update-available", (event) => {
          setUpdateInfo(event.payload);
        }),
        await listen<Settings>("settings-updated", (event) => {
          setSettings((current) =>
            current
              ? { ...current, autostartEnabled: event.payload.autostartEnabled }
              : event.payload
          );
          setInitialSettings((current) =>
            current
              ? { ...current, autostartEnabled: event.payload.autostartEnabled }
              : event.payload
          );
        })
      );
      setInterval(refresh, 60000);
    })();

    return () => unlisteners.forEach((u) => u());
  }, []);

  const overlayVisible = !!settings?.showVisualOverlay && (showOverlay || status.ceremonyActive);
  useEffect(() => {
    if (overlayVisible) void bringWindowToFront().catch(() => {});
  }, [overlayVisible]);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");

    const updateTheme = (event: MediaQueryListEvent) => {
      setPrefersDark(event.matches);
    };

    setPrefersDark(media.matches);
    media.addEventListener("change", updateTheme);

    return () => media.removeEventListener("change", updateTheme);
  }, []);

  // Trigger the update dialog when user is idle
  useEffect(() => {
    if (!updateInfo || updateDismissed) return;

    const ceremonyRunning = status.ceremonyActive || showOverlay;

    if (showUpdateDialog) {
      if (ceremonyRunning && !isInstallingUpdate) {
        setShowUpdateDialog(false);
      }
      return;
    }

    if (isIdle && document.hasFocus() && !ceremonyRunning) {
      setShowUpdateDialog(true);
    }
  }, [updateInfo, updateDismissed, showUpdateDialog, isIdle, status.ceremonyActive, showOverlay, isInstallingUpdate]);

  const updateSetting = useCallback(<K extends keyof Settings>(key: K, value: Settings[K]) => {
    setSettings((prev) => (prev ? { ...prev, [key]: value } : prev));
  }, []);

  const handleSave = useCallback(async () => {
    if (!settings) return;
    await saveSettings(settings);
    setInitialSettings(settings);
  }, [settings]);

  const handleSettingsImported = useCallback((imported: Settings) => {
    hasImportedRef.current = true;
    setSettings(imported);
    void getStatus()
      .then((nextStatus) => {
        setStatus(nextStatus);
        setTestRunning(nextStatus.ceremonyIsTest);
      })
      .catch((error) => console.error("Failed to refresh status after import:", error));
    setInitialSettings(imported);
    setVolumeValue(imported.volume);
  }, []);

  const handleSyncNtp = useCallback(async () => {
    setSyncing(true);
    try {
      const nextStatus = await syncNtpNow();
      setStatus(nextStatus);
      setTestRunning(nextStatus.ceremonyIsTest);
    } finally {
      setSyncing(false);
    }
  }, []);

  const handleManualUpdateCheck = useCallback(async () => {
    setUpdateDismissed(false);
    const { invoke } = await import("@tauri-apps/api/core");
    const update = await invoke<UpdateInfo | null>("check_for_updates");
    return update;
  }, []);

  if (!hydrated) {
    return (
      <FluentProvider
        theme={prefersDark ? webDarkTheme : webLightTheme}
        style={{ height: "100%" }}
      >
        <div className={styles.loadingContainer}>
          <Spinner size="medium" />
        </div>
      </FluentProvider>
    );
  }

  if (!settings) {
    return (
      <FluentProvider
        theme={prefersDark ? webDarkTheme : webLightTheme}
        style={{ height: "100%" }}
      >
        <div className={styles.loadingContainer}>
          <Button appearance="primary" onClick={() => window.location.reload()}>
            {t("buttons.retry")}
          </Button>
        </div>
      </FluentProvider>
    );
  }

  return (
    <>
      <FluentProvider
        theme={effectiveDark ? webDarkTheme : webLightTheme}
        style={{ height: "100%" }}
      >
        <div className={styles.layout}>
          <div className={styles.body}>
            <NavDrawer
              className={styles.desktopNav}
              selectedValue={selectedNav}
              onNavItemSelect={(_, data) => setSelectedNav(data.value as string)}
              open
              type="inline"
              density="small"
            >
              <NavDrawerBody key={selectedNav}>
                <NavItem value="settings" icon={<Settings20Regular />}>
                  {t("tabs.settings")}
                </NavItem>
                <NavItem value="personal_dates" icon={<CalendarMonth20Regular />}>
                  {t("tabs.personal_dates")}
                </NavItem>
                <NavItem value="about" icon={<Info20Regular />}>
                  {t("tabs.about")}
                </NavItem>
                <NavItem value="changelog" icon={<DocumentBulletList20Regular />}>
                  {t("tabs.changelog")}
                </NavItem>
              </NavDrawerBody>
            </NavDrawer>

            <div className={styles.content}>
              {selectedNav === "changelog" ? (
                <Suspense
                  fallback={
                    <div className={styles.changelogSpinner}>
                      <Spinner size="tiny" />
                    </div>
                  }
                >
                  <ChangelogTab />
                </Suspense>
              ) : (
                <div className={styles.scroll}>
                  {selectedNav === "settings" ? (
                    <SettingsTab
                      settings={settings}
                      status={status}
                      voices={voices}
                      volumeValue={volumeValue}
                      syncing={syncing}
                      onUpdateSetting={updateSetting}
                      onVolumeChange={setVolumeValue}
                      onSyncNtp={handleSyncNtp}
                      onSettingsImported={handleSettingsImported}
                      onPersonalDatesImported={setPersonalDates}
                    />
                  ) : selectedNav === "personal_dates" ? (
                    <PersonalDatesTab
                      personalDates={personalDates}
                      onPersonalDatesChange={setPersonalDates}
                    />
                  ) : (
                    <AboutTab
                      version={version}
                      onCheckForUpdates={handleManualUpdateCheck}
                      onUpdateFound={(update) => {
                        setUpdateInfo(update);
                        setShowUpdateDialog(true);
                      }}
                    />
                  )}
                </div>
              )}
            </div>
          </div>

          {selectedNav === "settings" && (
            <div className={styles.buttonBar}>
              <Button
                className={styles.buttonBarButton}
                icon={testRunning ? <Stop20Regular /> : <Play20Regular />}
                appearance="secondary"
                onClick={testRunning ? stopTestCeremony : triggerCeremonyNow}
                disabled={!hydrated}
              >
                {testRunning ? t("buttons.stop") : t("buttons.test")}
              </Button>
              <Button
                className={styles.buttonBarButton}
                icon={<Save20Regular />}
                appearance="primary"
                disabled={!hydrated || !isDirty}
                onClick={handleSave}
              >
                {isDirty ? t("buttons.save_dirty") : t("buttons.save")}
              </Button>
            </div>
          )}
          <nav className={styles.mobileNav}>
            <TabList
              selectedValue={selectedNav}
              onTabSelect={(_, data) => setSelectedNav(data.value as string)}
              className={styles.mobileNavList}
            >
              <Tab className={styles.mobileNavItem} value="settings">
                <span className={styles.mobileNavContent}>
                  <span className={styles.mobileNavIcon}><Settings20Regular /></span>
                  <span className={styles.mobileNavLabel}>{t("tabs.settings")}</span>
                </span>
              </Tab>
              <Tab className={styles.mobileNavItem} value="personal_dates">
                <span className={styles.mobileNavContent}>
                  <span className={styles.mobileNavIcon}><CalendarMonth20Regular /></span>
                  <span className={styles.mobileNavLabel}>{t("tabs.personal_dates")}</span>
                </span>
              </Tab>
              <Tab className={styles.mobileNavItem} value="about">
                <span className={styles.mobileNavContent}>
                  <span className={styles.mobileNavIcon}><Info20Regular /></span>
                  <span className={styles.mobileNavLabel}>{t("tabs.about")}</span>
                </span>
              </Tab>
              <Tab className={styles.mobileNavItem} value="changelog">
                <span className={styles.mobileNavContent}>
                  <span className={styles.mobileNavIcon}><DocumentBulletList20Regular /></span>
                  <span className={styles.mobileNavLabel}>{t("tabs.changelog")}</span>
                </span>
              </Tab>
            </TabList>
          </nav>
        </div>
        <UpdateDialog
          updateInfo={showUpdateDialog ? updateInfo : null}
          onClose={() => {
            setShowUpdateDialog(false);
            setUpdateDismissed(true);
          }}
          onUpdatingChange={setIsInstallingUpdate}
        />
      </FluentProvider>

      <Overlay
        show={overlayVisible}
        durationSeconds={ceremonyDurationMs ? ceremonyDurationMs / 1000 : undefined}
        personalDates={personalDates}
        settings={settings}
        isTest={isTestCeremony || status.ceremonyIsTest}
        onReady={overlayCeremonyId === undefined ? undefined : handleOverlayReady}
      />
    </>
  );
}
