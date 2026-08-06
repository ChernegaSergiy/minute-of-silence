import { useEffect, useRef, useState, useMemo } from "react";
import {
  makeStyles,
  shorthands,
  tokens,
  Title1,
  Subtitle1,
  FluentProvider,
  webDarkTheme,
  mergeClasses,
} from "@fluentui/react-components";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../utils/i18n";
import { saveSettings } from "../utils/api";
import { isLeapYear, selectNearbyDate } from "../utils/nearbyDateSelection";
import type { PersonalDate, Settings } from "../types";

type UpdateSetting = <K extends keyof Settings>(key: K, value: Settings[K]) => void;

interface OverlayProps {
  show: boolean;
  durationSeconds?: number;
  personalDates?: PersonalDate[];
  settings?: Settings;
  onUpdateSetting?: UpdateSetting;
  isTest?: boolean;
}

const candleUrl = "/img/candle_circle.png";
const ringUrl   = "/img/progress_ring.png";

const RING_SIZE   = 260;
const CANDLE_SIZE = RING_SIZE;

const useStyles = makeStyles({
  container: {
    display: "flex",
    position: "fixed",
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    backgroundColor: "rgb(8, 8, 8)", // Stable deep solid black base
    zIndex: 9999,
    justifyContent: "center",
    alignItems: "center",
    flexDirection: "column",
    overflow: "hidden",
    userSelect: "none",
    opacity: 0,
    pointerEvents: "none",
    transition: "opacity 1200ms cubic-bezier(0.25, 1, 0.5, 1)",
    fontFamily: tokens.fontFamilyBase,
  },
  containerVisible: {
    opacity: 1,
    pointerEvents: "auto",
  },
  inner: {
    textAlign: "center",
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    ...shorthands.gap("48px"), // Balanced Fluent UI spacing
    zIndex: 2,
    transform: "scale(0.96)",
    opacity: 0,
    transition: "transform 1400ms cubic-bezier(0.16, 1, 0.3, 1), opacity 1200ms ease-in-out",
  },
  innerVisible: {
    transform: "scale(1)",
    opacity: 1,
  },
  mediaWrapper: {
    position: "relative",
    width: `${RING_SIZE}px`,
    height: `${RING_SIZE}px`,
  },
  canvas: {
    position: "absolute",
    inset: 0,
    width: "100%",
    height: "100%",
    zIndex: 1,
  },
  candle: {
    position: "absolute",
    inset: 0,
    width: "100%",
    height: "100%",
    objectFit: "contain",
    zIndex: 0,
  },
  title: {
    color: tokens.colorNeutralForeground1,
    textTransform: "uppercase",
    letterSpacing: "0.3em",
    fontWeight: tokens.fontWeightSemibold,
    fontSize: "24px",
    margin: 0,
  },
  subtitle: {
    color: tokens.colorNeutralForeground4,
    textTransform: "uppercase",
    letterSpacing: "0.5em",
    fontSize: "13px",
    margin: 0,
  },
  subtitleContainer: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    ...shorthands.gap("8px"),
  },
  personalName: {
    color: tokens.colorNeutralForeground1,
    fontSize: "13px",
    fontWeight: tokens.fontWeightSemibold,
    letterSpacing: "0.4em",
    textTransform: "uppercase",
    textAlign: "center",
    maxWidth: "600px",
    lineHeight: "1.5",
    opacity: 0,
    transform: "translateY(4px)",
    transition: "opacity 500ms ease-in-out, transform 500ms cubic-bezier(0.25, 1, 0.5, 1)",
  },
  personalNameVisible: {
    opacity: 1,
    transform: "translateY(0)",
  }
});

interface ApngInfo {
  width: number;
  height: number;
  /** Each entry is a base64-encoded raw RGBA pixel buffer of the composited canvas. */
  frames: string[];
}

/**
 * Decode APNG frames via the Rust backend.
 *
 * Returns HTMLCanvasElement[] for use with drawImage(), which goes through the
 * WebKit compositing pipeline — unlike putImageData which bypasses it.
 * Using a hidden canvas buffer is the most reliable cross-platform method for WebKit.
 */
async function loadApngFrames(
  src: string,
): Promise<{ frames: HTMLCanvasElement[]; width: number; height: number }> {
  const resp = await fetch(src);
  const arrayBuf = await resp.arrayBuffer();
  const bytes = Array.from(new Uint8Array(arrayBuf));

  const info = await invoke<ApngInfo>("decode_apng_frames", { data: bytes });

  const frames: HTMLCanvasElement[] = info.frames.map((b64) => {
    const binaryStr = atob(b64);
    const len = binaryStr.length;
    const buf = new ArrayBuffer(len);
    const view = new Uint8Array(buf);
    for (let i = 0; i < len; i++) {
      view[i] = binaryStr.charCodeAt(i);
    }
    const imageData = new ImageData(new Uint8ClampedArray(buf), info.width, info.height);
    
    const canvas = document.createElement("canvas");
    canvas.width = info.width;
    canvas.height = info.height;
    const ctx = canvas.getContext("2d");
    if (ctx) ctx.putImageData(imageData, 0, 0);
    
    return canvas;
  });

  return { frames, width: info.width, height: info.height };
}

function useApngPlayer(
  canvasRef: React.RefObject<HTMLCanvasElement | null>,
  src: string,
  durationSeconds: number,
  active: boolean,
) {
  useEffect(() => {
    if (!active) return;
    let rafId: number;
    let frames: HTMLCanvasElement[] = [];
    let startTime: number | null = null;
    let isCancelled = false;

    const run = async () => {
      try {
        const data = await loadApngFrames(src);
        if (isCancelled) {
          // No manual cleanup needed for canvas elements
          return;
        }
        frames = data.frames;

        const canvas = canvasRef.current;
        if (!canvas || frames.length === 0) {
          return;
        }

        // Size canvas to match the native APNG dimensions.
        canvas.width = data.width;
        canvas.height = data.height;

        const ctx = canvas.getContext("2d")!;

        const tick = (now: number) => {
          if (!startTime) startTime = now;
          const elapsed = (now - startTime) / 1000;
          const progress = Math.min(elapsed / durationSeconds, 1);
          const frameIdx = Math.min(Math.floor(progress * frames.length), frames.length - 1);

          ctx.clearRect(0, 0, canvas.width, canvas.height);
          ctx.drawImage(frames[frameIdx], 0, 0);

          if (progress < 1) {
            rafId = requestAnimationFrame(tick);
          } else {
            startTime = null;
            rafId = requestAnimationFrame(tick);
          }
        };
        rafId = requestAnimationFrame(tick);
      } catch (e) {
        console.error("APNG decode failed:", e);
      }
    };

    run();

    return () => {
      isCancelled = true;
      cancelAnimationFrame(rafId);
    };
  }, [active, src, durationSeconds, canvasRef]);
}

export default function Overlay({
  show,
  durationSeconds = 60,
  personalDates = [],
  settings,
  onUpdateSetting,
  isTest = false,
}: OverlayProps) {
  const styles = useStyles();
  const ringCanvasRef = useRef<HTMLCanvasElement>(null);
  const lastSavedNearbyIdRef = useRef<string | null>(null);

  // Manage mounting delay for smooth transitions
  const [shouldRender, setShouldRender] = useState(show);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (show) {
      setShouldRender(true);
      const t = setTimeout(() => setVisible(true), 50);
      return () => clearTimeout(t);
    } else {
      setVisible(false);
      const t = setTimeout(() => setShouldRender(false), 1200); // Wait for transition to finish
      return () => clearTimeout(t);
    }
  }, [show]);

  useApngPlayer(ringCanvasRef, ringUrl, durationSeconds, shouldRender);

  // Get active personal dates matching today
  const activeDates = useMemo(() => {
    const today = new Date();
    const currentMonth = today.getMonth() + 1;
    const currentDay = today.getDate();
    const currentYear = today.getFullYear();

    let active = personalDates.filter(
      (d) => d.month === currentMonth && d.day === currentDay
    );

    if (currentMonth === 2 && currentDay === 28 && !isLeapYear(currentYear)) {
      const feb29Events = personalDates.filter((d) => d.month === 2 && d.day === 29);
      active = [...active, ...feb29Events];
    }
    return active;
  }, [personalDates]);

  // Nearby date selection when no exact match and setting is enabled
  const nearbyDate = useMemo(() => {
    if (activeDates.length > 0) return null;
    if (!settings?.showNearbyPersonalDates) return null;
    if (personalDates.length === 0) return null;
    return selectNearbyDate(personalDates, new Date(), settings.lastShownNearbyDateId ?? null);
    // `show` is intentionally in deps though unused in the body —
    // it forces a fresh weighted draw each time the overlay reopens.
  }, [activeDates, personalDates, settings?.showNearbyPersonalDates, show]);

  const displayDates = activeDates.length > 0 ? activeDates : (nearbyDate ? [nearbyDate] : []);
  const hasActiveDates = displayDates.length > 0;

  // Save last shown nearby date ID (skip for test ceremonies)
  useEffect(() => {
    const newId = nearbyDate?.id ?? null;
    if (nearbyDate && show && settings && onUpdateSetting && !isTest && lastSavedNearbyIdRef.current !== newId) {
      lastSavedNearbyIdRef.current = newId;
      onUpdateSetting("lastShownNearbyDateId", newId);
      saveSettings({ ...settings, lastShownNearbyDateId: newId }).catch(() => {});
    }
  }, [nearbyDate, show, settings, onUpdateSetting, isTest]);

  // Name carousel/slider state
  const [currentNameIndex, setCurrentNameIndex] = useState(0);
  const [nameFadeState, setNameFadeState] = useState(true);

  // Reset index when active dates or overlay state changes
  useEffect(() => {
    setCurrentNameIndex(0);
    setNameFadeState(true);
  }, [displayDates, show]);

  // Rotator effect for multiple names
  useEffect(() => {
    if (displayDates.length <= 1 || !show) return;

    const interval = setInterval(() => {
      setNameFadeState(false); // Start fade-out animation

      setTimeout(() => {
        setCurrentNameIndex((prev) => (prev + 1) % displayDates.length);
        setNameFadeState(true); // Start fade-in animation
      }, 500);
    }, 4000); // 4 seconds total interval for each slide

    return () => clearInterval(interval);
  }, [displayDates, show]);

  const currentCommemorationName = displayDates[currentNameIndex]?.label || "";

  if (!shouldRender) return null;

  return (
    <FluentProvider theme={webDarkTheme}>
      <div className={mergeClasses(styles.container, visible && styles.containerVisible)}>
        <div className={mergeClasses(styles.inner, visible && styles.innerVisible)}>
          <Title1 className={styles.title}>{t("overlay.title")}</Title1>
          
          <div className={styles.mediaWrapper}>
            <img
              src={candleUrl}
              alt=""
              aria-hidden="true"
              className={styles.candle}
              width={CANDLE_SIZE}
              height={CANDLE_SIZE}
            />
            <canvas
              ref={ringCanvasRef}
              className={styles.canvas}
              aria-hidden="true"
            />
          </div>
          
          <div className={styles.subtitleContainer}>
            <Subtitle1 className={styles.subtitle}>
              {hasActiveDates ? t("overlay.personal_subtitle") : t("overlay.subtitle")}
            </Subtitle1>
            {hasActiveDates && currentCommemorationName && (
              <div
                className={mergeClasses(
                  styles.personalName,
                  nameFadeState && styles.personalNameVisible
                )}
              >
                {currentCommemorationName}
              </div>
            )}
          </div>
        </div>
      </div>
    </FluentProvider>
  );
}
