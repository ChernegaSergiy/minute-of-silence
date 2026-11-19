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
  mergeClasses,
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
  bundleIcon,
  Settings24Filled,
  Settings24Regular,
  CalendarMonth24Filled,
  CalendarMonth24Regular,
  Info24Filled,
  Info24Regular,
  DocumentBulletList24Filled,
  DocumentBulletList24Regular,
} from "@fluentui/react-icons";

import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import {
  bringWindowToFront,
  getInitialState,
  getStatus,
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

const MOBILE_NAV_TABS = [
  { value: "settings", label: "tabs.settings", Icon: bundleIcon(Settings24Filled, Settings24Regular) },
  { value: "personal_dates", label: "tabs.personal_dates", Icon: bundleIcon(CalendarMonth24Filled, CalendarMonth24Regular) },
  { value: "about", label: "tabs.about", Icon: bundleIcon(Info24Filled, Info24Regular) },
  { value: "changelog", label: "tabs.changelog", Icon: bundleIcon(DocumentBulletList24Filled, DocumentBulletList24Regular) },
] as const;
const MOBILE_NAV_ICON_HEIGHT = "32px";
const mobileNavHighlight = {
  "--mobile-nav-icon-highlight-background": `var(--mobile-nav-icon-highlight-color, ${tokens.colorNeutralBackground2Hover})`,
};
const mobileNavMotion = {
  transitionDuration: tokens.durationFast,
  transitionTimingFunction: tokens.curveDecelerateMin,
  "@media (prefers-reduced-motion: reduce)": {
    transitionDuration: "0ms",
  },
};

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
      backgroundColor: tokens.colorNeutralBackground2,
    },
  },
  mobileNavList: {
    display: "flex",
  },
  mobileNavItem: {
    flex: 1,
    minWidth: 0,
    gridTemplateColumns: "minmax(0, 1fr)",
    height: "72px",
    transitionProperty: "flex-grow",
    ...mobileNavMotion,
    padding: "10px 0 12px",
    "&::after": { display: "none" },
    "&::before": { display: "none" },
    "&:active:enabled": mobileNavHighlight,
    "@media (hover: hover)": {
      "&:enabled:hover": mobileNavHighlight,
    },
  },
  mobileNavItemSelected: {
    flexGrow: 1.8,
    "--mobile-nav-icon-highlight-color": tokens.colorBrandBackground2Hover,
  },
  mobileNavContent: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    gap: "4px",
    width: "100%",
    transform: `translateY(calc((100% - ${MOBILE_NAV_ICON_HEIGHT}) / 2))`,
    transitionProperty: "transform",
    ...mobileNavMotion,
  },
  mobileNavContentSelected: {
    transform: "translateY(0)",
  },
  mobileNavIcon: {
    position: "relative",
    isolation: "isolate",
    display: "grid",
    placeItems: "center",
    width: "64px",
    height: MOBILE_NAV_ICON_HEIGHT,
    flexShrink: 0,
    borderRadius: "16px",
    color: tokens.colorNeutralForeground2,
    transitionProperty: "background-color, color",
    ...mobileNavMotion,
    "& svg": { width: "24px", height: "24px" },
    "&::before": {
      content: '""',
      position: "absolute",
      inset: 0,
      borderRadius: "inherit",
      zIndex: -1,
      backgroundColor: "var(--mobile-nav-icon-highlight-background, transparent)",
    },
  },
  mobileNavIconSelected: {
    backgroundColor: tokens.colorBrandBackground2,
    color: tokens.colorBrandForeground2,
  },
  mobileNavLabel: {
    fontSize: tokens.fontSizeBase200,
    fontWeight: tokens.fontWeightSemibold,
    lineHeight: tokens.lineHeightBase200,
    color: tokens.colorNeutralForeground1,
    letterSpacing: 0,
    whiteSpace: "nowrap",
    maxWidth: "100%",
    overflow: "hidden",
    textOverflow: "ellipsis",
    opacity: 0,
    transform: "translateY(4px)",
    transitionProperty: "opacity, transform",
    ...mobileNavMotion,
    // Preview only: the locale strings are upper case, a sentence-case label needs new strings.
    textTransform: "lowercase",
    "&::first-letter": { textTransform: "uppercase" },
  },
  mobileNavLabelSelected: {
    opacity: 1,
    transform: "translateY(0)",
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
  const [supportsVisualOverlay, setSupportsVisualOverlay] = useState(false);
  const [supportsMediaResumption, setSupportsMediaResumption] = useState(false);
  const [showOverlay, setShowOverlay] = useState(false);
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
        setSupportsVisualOverlay(init.supportsVisualOverlay);
        setSupportsMediaResumption(init.supportsMediaResumption);
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
          refresh();
        }),
        await onCeremonyEnd(() => {
          setTestRunning(false);
          setShowOverlay(false);
          setCeremonyDurationMs(undefined);
          setIsTestCeremony(false);
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

  const overlayVisible = supportsVisualOverlay && !!settings?.showVisualOverlay && (showOverlay || status.ceremonyActive);
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
                <div key={selectedNav} className={styles.scroll}>
                  {selectedNav === "settings" ? (
                    <SettingsTab
                      settings={settings}
                      supportsVisualOverlay={supportsVisualOverlay}
                      supportsMediaResumption={supportsMediaResumption}
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
              {MOBILE_NAV_TABS.map(({ value, label, Icon }) => {
                const selected = selectedNav === value;
                return (
                  <Tab
                    key={value}
                    value={value}
                    className={mergeClasses(styles.mobileNavItem, selected && styles.mobileNavItemSelected)}
                  >
                    <span className={mergeClasses(styles.mobileNavContent, selected && styles.mobileNavContentSelected)}>
                      <span
                        aria-hidden="true"
                        className={mergeClasses(styles.mobileNavIcon, selected && styles.mobileNavIconSelected)}
                      >
                        <Icon filled={selected} />
                      </span>
                      <span className={mergeClasses(styles.mobileNavLabel, selected && styles.mobileNavLabelSelected)}>
                        {t(label)}
                      </span>
                    </span>
                  </Tab>
                );
              })}
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

      {supportsVisualOverlay && (
        <Overlay
          show={overlayVisible}
          durationSeconds={ceremonyDurationMs ? ceremonyDurationMs / 1000 : undefined}
          personalDates={personalDates}
          settings={settings}
          isTest={isTestCeremony || status.ceremonyIsTest}
        />
      )}
    </>
  );
}
