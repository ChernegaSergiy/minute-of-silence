import { Fragment, useCallback, useEffect, useRef, useState } from "react";
import {
  Badge,
  Button,
  Caption1,
  Card,
  Divider,
  Switch,
  Text,
  Tooltip,
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import {
  ArrowSync20Regular,
  Checkmark20Regular,
  ClipboardCheckmarkRegular,
  ClipboardRegular,
  Desktop20Regular,
  ErrorCircle20Regular,
  Phone20Regular,
  Send20Regular,
  Speaker220Regular,
} from "@fluentui/react-icons";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { DeviceKind, DiscoveredDevice } from "../types";
import {
  discoverDevices,
  getReceivingPairingCode,
  getReceivingSettingsEnabled,
  onPairingCodeChanged,
  onReceivingSettingsStopped,
  sendSettingsToDevice,
  setReceivingSettingsEnabled,
} from "../utils/api";
import { t } from "../utils/i18n";

const useStyles = makeStyles({
  card: {
    marginBottom: tokens.spacingVerticalM,
  },
  headerRow: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    columnGap: tokens.spacingHorizontalM,
    rowGap: tokens.spacingVerticalS,
    flexWrap: "wrap",
    paddingTop: tokens.spacingVerticalSNudge,
    paddingBottom: tokens.spacingVerticalSNudge,
  },
  labelGroup: {
    display: "flex",
    flexDirection: "column",
    rowGap: "2px",
    flex: "1 1 200px",
    minWidth: 0,
  },
  description: {
    fontSize: tokens.fontSizeBase100,
    color: tokens.colorNeutralForeground3,
    marginTop: tokens.spacingVerticalXXS,
  },
  statusText: {
    color: tokens.colorNeutralForeground3,
  },
  errorText: {
    color: tokens.colorPaletteRedForeground1,
  },
  deviceList: {
    display: "flex",
    flexDirection: "column",
    rowGap: tokens.spacingVerticalS,
  },
  deviceRow: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    columnGap: tokens.spacingHorizontalM,
    rowGap: tokens.spacingVerticalS,
    paddingTop: tokens.spacingVerticalXS,
    paddingBottom: tokens.spacingVerticalXS,
    "@media (max-width: 480px)": {
      flexDirection: "column",
      alignItems: "stretch",
    },
  },
  deviceInfo: {
    display: "flex",
    alignItems: "center",
    columnGap: tokens.spacingHorizontalS,
    minWidth: 0,
    flex: "1 1 180px",
    "@media (max-width: 480px)": {
      flex: "0 1 auto",
    },
  },
  deviceIcon: {
    display: "flex",
    color: tokens.colorNeutralForeground2,
    flexShrink: 0,
  },
  deviceMeta: {
    display: "flex",
    flexDirection: "column",
    minWidth: 0,
    overflow: "hidden",
  },
  deviceTitleRow: {
    display: "flex",
    alignItems: "center",
    columnGap: tokens.spacingHorizontalXS,
    rowGap: tokens.spacingVerticalXXS,
    flexWrap: "wrap",
  },
  deviceEndpoint: {
    color: tokens.colorNeutralForeground3,
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  },
  deviceActions: {
    display: "flex",
    alignItems: "center",
    columnGap: tokens.spacingHorizontalXS,
    flexShrink: 0,
    "@media (max-width: 480px)": {
      width: "100%",
    },
  },
  sendButton: {
    "@media (max-width: 480px)": {
      flex: 1,
    },
  },
  spinIcon: {
    animationName: {
      from: { transform: "rotate(0deg)" },
      to: { transform: "rotate(360deg)" },
    },
    animationDuration: "1s",
    animationIterationCount: "infinite",
    animationTimingFunction: "linear",
  },
});

function renderKindIcon(kind: DeviceKind) {
  switch (kind) {
    case "desktop":
      return <Desktop20Regular />;
    case "android":
      return <Phone20Regular />;
    case "hardware":
    case "unknown":
      return <Speaker220Regular />;
  }
}

function formatEndpoint(device: DiscoveredDevice): string {
  return device.port === 80 ? device.ip : `${device.ip}:${device.port}`;
}

type SendState = "idle" | "sending" | "sent" | "error";
type CopyFeedback = { key: string; status: "copied" | "error" };

export default function DeviceDiscoveryCard() {
  const styles = useStyles();
  const [devices, setDevices] = useState<DiscoveredDevice[]>([]);
  const [receivingEnabled, setReceivingEnabled] = useState(false);
  const [receivingBusy, setReceivingBusy] = useState(true);
  const [receivingError, setReceivingError] = useState(false);
  const [pairingCode, setPairingCode] = useState<string | null>(null);
  const [pairingLoading, setPairingLoading] = useState(false);
  const [scanning, setScanning] = useState(false);
  const [hasScanned, setHasScanned] = useState(false);
  const [error, setError] = useState(false);
  const [copyFeedback, setCopyFeedback] = useState<CopyFeedback | null>(null);
  const [sendStates, setSendStates] = useState<Record<string, SendState>>({});
  const copyTimerRef = useRef<number | null>(null);
  const receivingRevisionRef = useRef(0);
  const sendTimersRef = useRef<Record<string, number>>({});

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    const refresh = async (stopped: boolean) => {
      const revision = ++receivingRevisionRef.current;
      try {
        const enabled = await getReceivingSettingsEnabled();
        if (!active || revision !== receivingRevisionRef.current) return;
        setReceivingEnabled(enabled);
        if (stopped && !enabled) setReceivingError(true);
      } catch (error) {
        if (active) setReceivingError(true);
        console.error("Failed to get settings receiving status:", error);
      } finally {
        if (active && revision === receivingRevisionRef.current) setReceivingBusy(false);
      }
    };
    void onReceivingSettingsStopped(() => {
      void refresh(true);
    }).then((stopListening) => {
      if (!active) {
        stopListening();
        return;
      }
      unlisten = stopListening;
      void refresh(false);
    }).catch((error) => {
      if (active) {
        setReceivingError(true);
        setReceivingBusy(false);
      }
      console.error("Failed to listen for settings receiver status:", error);
    });
    return () => {
      active = false;
      receivingRevisionRef.current++;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!receivingEnabled) {
      setPairingCode(null);
      setPairingLoading(false);
      return;
    }
    let active = true;
    let revision = 0;
    let timer: number | undefined;
    let unlisten: (() => void) | undefined;
    const refresh = async () => {
      const request = ++revision;
      const startedAt = performance.now();
      try {
        const info = await getReceivingPairingCode();
        if (!active || request !== revision) return;
        if (timer !== undefined) window.clearTimeout(timer);
        const remaining = info ? info.expiresInMs - (performance.now() - startedAt) : 0;
        setPairingCode(info && remaining > 0 ? info.code : null);
        if (info && remaining > 0) {
          timer = window.setTimeout(() => setPairingCode(null), remaining);
        }
      } catch (error) {
        if (active && request === revision) {
          setPairingCode(null);
          setReceivingError(true);
        }
        console.error("Failed to load pairing code:", error);
      } finally {
        if (active && request === revision) setPairingLoading(false);
      }
    };
    setPairingLoading(true);
    void onPairingCodeChanged(() => { void refresh(); }).then((stopListening) => {
      if (!active) {
        stopListening();
        return;
      }
      unlisten = stopListening;
      void refresh();
    }).catch((error) => {
      if (active) {
        setPairingLoading(false);
        setReceivingError(true);
      }
      console.error("Failed to listen for pairing code updates:", error);
    });
    return () => {
      active = false;
      if (timer !== undefined) window.clearTimeout(timer);
      unlisten?.();
    };
  }, [receivingEnabled]);

  useEffect(() => {
    const timers = sendTimersRef.current;
    return () => {
      if (copyTimerRef.current !== null) {
        window.clearTimeout(copyTimerRef.current);
      }
      Object.values(timers).forEach((id) => window.clearTimeout(id));
    };
  }, []);

  const handleReceivingChange = useCallback(async (enabled: boolean) => {
    const revision = ++receivingRevisionRef.current;
    setReceivingBusy(true);
    setReceivingError(false);
    try {
      const current = await setReceivingSettingsEnabled(enabled);
      if (revision === receivingRevisionRef.current) setReceivingEnabled(current);
    } catch (error) {
      if (revision === receivingRevisionRef.current) setReceivingError(true);
      console.error("Failed to change settings receiving:", error);
    } finally {
      if (revision === receivingRevisionRef.current) setReceivingBusy(false);
    }
  }, []);

  const handleScan = useCallback(async () => {
    setScanning(true);
    setError(false);
    try {
      const found = await discoverDevices();
      setDevices(found);
      setHasScanned(true);
    } catch {
      setError(true);
      setHasScanned(true);
    } finally {
      setScanning(false);
    }
  }, []);

  const handleCopyIp = useCallback(async (device: DiscoveredDevice) => {
    const key = `${device.ip}:${device.port}`;
    let status: CopyFeedback["status"] = "copied";
    try {
      await writeText(device.ip);
    } catch {
      status = "error";
    }
    if (copyTimerRef.current !== null) {
      window.clearTimeout(copyTimerRef.current);
    }
    setCopyFeedback({ key, status });
    copyTimerRef.current = window.setTimeout(() => {
      copyTimerRef.current = null;
      setCopyFeedback((current) => (current?.key === key ? null : current));
    }, 2000);
  }, []);

  const handleSendSettings = useCallback(async (device: DiscoveredDevice) => {
    const key = `${device.ip}:${device.port}`;
    const existingTimer = sendTimersRef.current[key];
    if (existingTimer !== undefined) {
      window.clearTimeout(existingTimer);
      delete sendTimersRef.current[key];
    }
    setSendStates((prev) => ({ ...prev, [key]: "sending" }));
    try {
      await sendSettingsToDevice(device.ip, device.port);
      setSendStates((prev) => ({ ...prev, [key]: "sent" }));
      sendTimersRef.current[key] = window.setTimeout(() => {
        delete sendTimersRef.current[key];
        setSendStates((prev) =>
          prev[key] === "sent" ? { ...prev, [key]: "idle" } : prev,
        );
      }, 3000);
    } catch {
      setSendStates((prev) => ({ ...prev, [key]: "error" }));
      sendTimersRef.current[key] = window.setTimeout(() => {
        delete sendTimersRef.current[key];
        setSendStates((prev) =>
          prev[key] === "error" ? { ...prev, [key]: "idle" } : prev,
        );
      }, 3000);
    }
  }, []);

  return (
    <Card className={styles.card}>
      <Text size={100} weight="semibold" block>
        {t("sections.devices")}
      </Text>
      <div className={styles.headerRow}>
        <div className={styles.labelGroup}>
          <Text weight="medium">{t("controls.device_discovery.receive_label")}</Text>
          <div className={styles.description}>
            {t("controls.device_discovery.receive_description")}
          </div>
        </div>
        <Switch
          aria-label={t("controls.device_discovery.receive_label")}
          checked={receivingEnabled}
          disabled={receivingBusy}
          onChange={(_, data) => { void handleReceivingChange(data.checked); }}
        />
      </div>
      {receivingError && (
        <Caption1 className={styles.errorText}>
          {t("controls.device_discovery.receive_error")}
        </Caption1>
      )}
      {receivingEnabled && !pairingLoading && (
        pairingCode ? (
          <div className={styles.labelGroup}>
            <Text weight="medium">{t("controls.device_discovery.pairing_code")}: {pairingCode}</Text>
            <Caption1 className={styles.statusText}>
              {t("controls.device_discovery.pairing_code_description")}
            </Caption1>
          </div>
        ) : (
          <Caption1 className={styles.statusText}>
            {t("controls.device_discovery.pairing_code_unavailable")}
          </Caption1>
        )
      )}
      <Divider />
      <div className={styles.headerRow}>
        <div className={styles.labelGroup}>
          <Text weight="medium">{t("controls.device_discovery.label")}</Text>
          <div className={styles.description}>
            {t("controls.device_discovery.description")}
          </div>
        </div>
        <Button
          appearance="secondary"
          icon={
            <ArrowSync20Regular className={scanning ? styles.spinIcon : undefined} />
          }
          onClick={handleScan}
          disabled={scanning}
        >
          {scanning
            ? t("controls.device_discovery.scanning")
            : t("controls.device_discovery.scan")}
        </Button>
      </div>

      {hasScanned && !scanning && (
        <>
          <Divider />
          {error ? (
            <Caption1 className={styles.errorText}>
              {t("controls.device_discovery.error")}
            </Caption1>
          ) : devices.length === 0 ? (
            <Caption1 className={styles.statusText}>
              {t("controls.device_discovery.empty")}
            </Caption1>
          ) : (
            <div className={styles.deviceList}>
              {devices.map((device, index) => {
                const key = `${device.ip}:${device.port}`;
                const copyStatus =
                  copyFeedback?.key === key ? copyFeedback.status : "idle";
                const copyLabel =
                  copyStatus === "copied"
                    ? t("controls.device_discovery.copied")
                    : copyStatus === "error"
                    ? t("controls.device_discovery.copy_error")
                    : t("controls.device_discovery.copy_ip");
                const sendState: SendState = sendStates[key] ?? "idle";
                return (
                  <Fragment key={key}>
                    {index > 0 && <Divider appearance="subtle" />}
                    <div className={styles.deviceRow}>
                      <div className={styles.deviceInfo}>
                        <span className={styles.deviceIcon}>
                          {renderKindIcon(device.kind)}
                        </span>
                        <div className={styles.deviceMeta}>
                          <div className={styles.deviceTitleRow}>
                            <Text weight="medium">
                              {device.name ?? device.hostname}
                            </Text>
                            <Badge appearance="tint" size="small">
                              {t(`controls.device_discovery.kind.${device.kind}`)}
                            </Badge>
                          </div>
                          <Caption1 className={styles.deviceEndpoint}>
                            {formatEndpoint(device)} · {device.hostname}
                          </Caption1>
                        </div>
                      </div>
                      <div className={styles.deviceActions}>
                        <Tooltip
                          content={t("controls.device_discovery.send_tooltip")}
                          relationship="description"
                        >
                          <Button
                            className={styles.sendButton}
                            appearance="secondary"
                            icon={
                              sendState === "sent" ? (
                                <Checkmark20Regular />
                              ) : (
                                <Send20Regular />
                              )
                            }
                            disabled={sendState === "sending"}
                            onClick={() => handleSendSettings(device)}
                          >
                            {sendState === "sending"
                              ? t("controls.device_discovery.sending")
                              : sendState === "sent"
                              ? t("controls.device_discovery.sent")
                              : sendState === "error"
                              ? t("controls.device_discovery.send_error")
                              : t("controls.device_discovery.send")}
                          </Button>
                        </Tooltip>
                        <Tooltip content={copyLabel} relationship="label">
                          <Button
                            appearance="subtle"
                            icon={
                              copyStatus === "copied" ? (
                                <ClipboardCheckmarkRegular />
                              ) : copyStatus === "error" ? (
                                <ErrorCircle20Regular />
                              ) : (
                                <ClipboardRegular />
                              )
                            }
                            aria-label={copyLabel}
                            onClick={() => handleCopyIp(device)}
                          />
                        </Tooltip>
                      </div>
                    </div>
                  </Fragment>
                );
              })}
            </div>
          )}
        </>
      )}
    </Card>
  );
}
