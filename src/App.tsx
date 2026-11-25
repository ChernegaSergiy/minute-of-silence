import { Suspense, lazy, useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
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
  getPlatform,
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
  { value: "settings", label: "tabs.mobile.settings", Icon: bundleIcon(Settings24Filled, Settings24Regular) },
  { value: "personal_dates", label: "tabs.mobile.personal_dates", Icon: bundleIcon(CalendarMonth24Filled, CalendarMonth24Regular) },
  { value: "about", label: "tabs.mobile.about", Icon: bundleIcon(Info24Filled, Info24Regular) },
  { value: "changelog", label: "tabs.mobile.changelog", Icon: bundleIcon(DocumentBulletList24Filled, DocumentBulletList24Regular) },
] as const;
const MOBILE_NAV_ICON_HEIGHT = "32px";

type NavMode = "bottom" | "rail" | "drawer" | "compact-drawer";

const NAV_QUERIES = {
  touch: "(pointer: coarse)",
  lowHeight: "(max-height: 480px)",
  narrow: "(max-width: 600px)",
};

function subscribeNavMode(onChange: () => void) {
  const lists = Object.values(NAV_QUERIES).map((q) => window.matchMedia(q));
  lists.forEach((l) => l.addEventListener("change", onChange));
  return () => lists.forEach((l) => l.removeEventListener("change", onChange));
}

function getNavMode(platform: "android" | "desktop" | null): NavMode {
  const m = (q: string) => window.matchMedia(q).matches;
  if (platform === "desktop") return m(NAV_QUERIES.narrow) ? "compact-drawer" : "drawer";
  if (m(NAV_QUERIES.lowHeight)) return "rail";
  if (m(NAV_QUERIES.narrow)) return "bottom";
  return "rail";
}

function useNavMode(platform: "android" | "desktop" | null): NavMode {
  return useSyncExternalStore(subscribeNavMode, () => getNavMode(platform));
}

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
  ceremonyId: null,
  ceremonyElapsedMs: null,
  ceremonyDurationMs: null,
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
  compactDesktopNavItem: {
    width: "44px",
    minWidth: "44px",
    paddingInline: tokens.spacingHorizontalMNudge,
  },
  desktopNavDrawer: {
    flexShrink: 0,
    transitionProperty: "width",
    transitionDuration: tokens.durationNormal,
    transitionTimingFunction: tokens.curveDecelerateMin,
    "@media (prefers-reduced-motion: reduce)": {
      transitionDuration: "0ms",
    },
  },
  desktopNavItem: {
    transitionProperty: "width",
    transitionDuration: tokens.durationNormal,
    transitionTimingFunction: tokens.curveDecelerateMin,
    "@media (prefers-reduced-motion: reduce)": {
      transitionDuration: "0ms",
    },
  },
  desktopNavLabel: {
    display: "inline-block",
    maxWidth: "200px",
    overflow: "hidden",
    whiteSpace: "nowrap",
    opacity: 1,
    transitionProperty: "max-width, opacity",
    transitionDuration: tokens.durationNormal,
    transitionTimingFunction: tokens.curveDecelerateMin,
    "@media (prefers-reduced-motion: reduce)": {
      transitionDuration: "0ms",
    },
  },
  compactDesktopNavLabel: {
    maxWidth: 0,
    opacity: 0,
  },
  navItemIcon: {
    pointerEvents: "none",
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
  bottomBar: {
    borderTop: `1px solid ${tokens.colorNeutralStroke2}`,
    backgroundColor: tokens.colorNeutralBackground2,
  },
  rail: {
    display: "flex",
    flexDirection: "column",
    justifyContent: "flex-start",
    flexShrink: 0,
    width: "96px",
    overflowY: "auto",
    paddingTop: "12px",
    borderRight: `1px solid ${tokens.colorNeutralStroke2}`,
    backgroundColor: tokens.colorNeutralBackground2,
  },
  railList: {
    width: "100%",
    alignItems: "stretch",
  },
  railItem: {
    flex: "0 0 auto",
    height: "auto",
    padding: "6px 0",
    justifyContent: "center",
  },
  railItemSelected: {
    "--mobile-nav-icon-highlight-color": tokens.colorBrandBackground2Hover,
  },
  railIcon: {
    width: "56px",
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
  const [platform, setPlatform] = useState<"android" | "desktop" | null>(null);
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
        const [init, runtimePlatform] = await Promise.all([getInitialState(), getPlatform()]);
        setPlatform(runtimePlatform);
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
          setStatus((current) => ({
            ...current,
            ceremonyActive: true,
            ceremonyIsTest: isTest,
            ceremonyId: payload.ceremony_id ?? current.ceremonyId,
            ceremonyElapsedMs: payload.elapsed_ms ?? current.ceremonyElapsedMs,
            ceremonyDurationMs: payload.duration_ms ?? current.ceremonyDurationMs,
          }));
          refresh();
        }),
        await onCeremonyEnd(() => {
          setTestRunning(false);
          setShowOverlay(false);
          setCeremonyDurationMs(undefined);
          setIsTestCeremony(false);
          setStatus((current) => ({
            ...current,
            ceremonyActive: false,
            ceremonyIsTest: false,
            ceremonyId: null,
            ceremonyElapsedMs: null,
            ceremonyDurationMs: null,
          }));
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
      refresh();
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

  const navMode = useNavMode(platform);

  const renderMobileNav = (variant: "bottom" | "rail") => {
    const isRail = variant === "rail";
    return (
      <nav className={isRail ? styles.rail : styles.bottomBar}>
        <TabList
          vertical={isRail}
          selectedValue={selectedNav}
          onTabSelect={(_, data) => setSelectedNav(data.value as string)}
          className={mergeClasses(styles.mobileNavList, isRail && styles.railList)}
        >
          {MOBILE_NAV_TABS.map(({ value, label, Icon }) => {
            const selected = selectedNav === value;
            const showLabel = selected || isRail;
            return (
              <Tab
                key={value}
                value={value}
                className={mergeClasses(
                  styles.mobileNavItem,
                  isRail && styles.railItem,
                  selected && (isRail ? styles.railItemSelected : styles.mobileNavItemSelected)
                )}
              >
                <span className={mergeClasses(styles.mobileNavContent, showLabel && styles.mobileNavContentSelected)}>
                  <span
                    aria-hidden="true"
                    className={mergeClasses(
                      styles.mobileNavIcon,
                      isRail && styles.railIcon,
                      selected && styles.mobileNavIconSelected
                    )}
                  >
                    <Icon filled={selected} />
                  </span>
                  <span className={mergeClasses(styles.mobileNavLabel, showLabel && styles.mobileNavLabelSelected)}>
                    {t(label)}
                  </span>
                </span>
              </Tab>
            );
          })}
        </TabList>
      </nav>
    );
  };

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
            {(navMode === "drawer" || navMode === "compact-drawer") && (
            <NavDrawer
              className={styles.desktopNavDrawer}
              style={{ width: navMode === "compact-drawer" ? "64px" : "260px" }}
              selectedValue={selectedNav}
              onNavItemSelect={(_, data) => setSelectedNav(data.value as string)}
              open
              type="inline"
              density="small"
            >
              <NavDrawerBody key={selectedNav}>
                <NavItem
                  value="settings"
                  icon={{ children: <Settings20Regular />, className: styles.navItemIcon }}
                  className={mergeClasses(
                    styles.desktopNavItem,
                    navMode === "compact-drawer" && styles.compactDesktopNavItem
                  )}
                  aria-label={t("tabs.desktop.settings")}
                  title={t("tabs.desktop.settings")}
                >
                  <span className={mergeClasses(
                    styles.desktopNavLabel,
                    navMode === "compact-drawer" && styles.compactDesktopNavLabel
                  )}>{t("tabs.desktop.settings")}</span>
                </NavItem>
                <NavItem
                  value="personal_dates"
                  icon={{ children: <CalendarMonth20Regular />, className: styles.navItemIcon }}
                  className={mergeClasses(
                    styles.desktopNavItem,
                    navMode === "compact-drawer" && styles.compactDesktopNavItem
                  )}
                  aria-label={t("tabs.desktop.personal_dates")}
                  title={t("tabs.desktop.personal_dates")}
                >
                  <span className={mergeClasses(
                    styles.desktopNavLabel,
                    navMode === "compact-drawer" && styles.compactDesktopNavLabel
                  )}>{t("tabs.desktop.personal_dates")}</span>
                </NavItem>
                <NavItem
                  value="about"
                  icon={{ children: <Info20Regular />, className: styles.navItemIcon }}
                  className={mergeClasses(
                    styles.desktopNavItem,
                    navMode === "compact-drawer" && styles.compactDesktopNavItem
                  )}
                  aria-label={t("tabs.desktop.about")}
                  title={t("tabs.desktop.about")}
                >
                  <span className={mergeClasses(
                    styles.desktopNavLabel,
                    navMode === "compact-drawer" && styles.compactDesktopNavLabel
                  )}>{t("tabs.desktop.about")}</span>
                </NavItem>
                <NavItem
                  value="changelog"
                  icon={{ children: <DocumentBulletList20Regular />, className: styles.navItemIcon }}
                  className={mergeClasses(
                    styles.desktopNavItem,
                    navMode === "compact-drawer" && styles.compactDesktopNavItem
                  )}
                  aria-label={t("tabs.desktop.changelog")}
                  title={t("tabs.desktop.changelog")}
                >
                  <span className={mergeClasses(
                    styles.desktopNavLabel,
                    navMode === "compact-drawer" && styles.compactDesktopNavLabel
                  )}>{t("tabs.desktop.changelog")}</span>
                </NavItem>
              </NavDrawerBody>
            </NavDrawer>
            )}
            {navMode === "rail" && renderMobileNav("rail")}

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
                className={platform === "android" ? styles.buttonBarButton : undefined}
                icon={testRunning ? <Stop20Regular /> : <Play20Regular />}
                appearance="secondary"
                onClick={testRunning ? stopTestCeremony : triggerCeremonyNow}
                disabled={!hydrated}
              >
                {testRunning ? t("buttons.stop") : t("buttons.test")}
              </Button>
              <Button
                className={platform === "android" ? styles.buttonBarButton : undefined}
                icon={<Save20Regular />}
                appearance="primary"
                disabled={!hydrated || !isDirty}
                onClick={handleSave}
              >
                {isDirty ? t("buttons.save_dirty") : t("buttons.save")}
              </Button>
            </div>
          )}
          {navMode === "bottom" && renderMobileNav("bottom")}
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
