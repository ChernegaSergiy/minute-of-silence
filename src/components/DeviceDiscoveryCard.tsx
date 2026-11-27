import { useCallback, useState } from "react";
import {
  Badge,
  Button,
  Caption1,
  Card,
  Divider,
  Text,
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import {
  ArrowSync20Regular,
  ClipboardCheckmarkRegular,
  ClipboardRegular,
  Desktop20Regular,
  Phone20Regular,
  Speaker220Regular,
} from "@fluentui/react-icons";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { DeviceKind, DiscoveredDevice } from "../types";
import { discoverDevices } from "../utils/api";
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
    fontSize: tokens.fontSizeBase200,
    color: tokens.colorNeutralForeground3,
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
    rowGap: tokens.spacingVerticalXS,
    flexWrap: "wrap",
  },
  deviceInfo: {
    display: "flex",
    alignItems: "center",
    columnGap: tokens.spacingHorizontalS,
    minWidth: 0,
    flex: "1 1 180px",
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
  },
  deviceTitleRow: {
    display: "flex",
    alignItems: "center",
    columnGap: tokens.spacingHorizontalXS,
    flexWrap: "wrap",
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

export default function DeviceDiscoveryCard() {
  const styles = useStyles();
  const [devices, setDevices] = useState<DiscoveredDevice[]>([]);
  const [scanning, setScanning] = useState(false);
  const [hasScanned, setHasScanned] = useState(false);
  const [error, setError] = useState(false);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

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
    try {
      await writeText(formatEndpoint(device));
      setCopiedKey(key);
      window.setTimeout(() => {
        setCopiedKey((current) => (current === key ? null : current));
      }, 2000);
    } catch {
      // Clipboard write errors are non-fatal
    }
  }, []);

  return (
    <Card className={styles.card}>
      <Text size={100} weight="semibold" block>
        {t("sections.devices")}
      </Text>
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
              {devices.map((device) => {
                const key = `${device.ip}:${device.port}`;
                const isCopied = copiedKey === key;
                return (
                  <div key={key} className={styles.deviceRow}>
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
                        <Caption1 className={styles.statusText}>
                          {formatEndpoint(device)} · {device.hostname}
                        </Caption1>
                      </div>
                    </div>
                    <Button
                      size="small"
                      appearance="subtle"
                      icon={
                        isCopied ? (
                          <ClipboardCheckmarkRegular />
                        ) : (
                          <ClipboardRegular />
                        )
                      }
                      onClick={() => handleCopyIp(device)}
                    >
                      {isCopied
                        ? t("controls.device_discovery.copied")
                        : t("controls.device_discovery.copy_ip")}
                    </Button>
                  </div>
                );
              })}
            </div>
          )}
        </>
      )}
    </Card>
  );
}
