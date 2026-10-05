import { useState } from "react";
import { makeStyles, tokens, CardPreview, mergeClasses } from "@fluentui/react-components";
import { t } from "../utils/i18n";
import { CachedImage } from "./CachedImage";

const useStyles = makeStyles({
  carouselWrapper: {
    position: "relative",
  },
  cardPreview: {
    backgroundColor: tokens.colorNeutralBackground3,
    display: "flex",
    overflowX: "auto",
    scrollSnapType: "x mandatory",
    scrollbarWidth: "none",
    msOverflowStyle: "none",
    "&::-webkit-scrollbar": {
      display: "none"
    }
  },
  mediaImage: {
    minWidth: "100%",
    width: "100%",
    height: "auto",
    objectFit: "cover",
    scrollSnapAlign: "center",
    display: "block",
  },
  indicatorContainer: {
    position: "absolute",
    bottom: tokens.spacingVerticalM,
    left: "50%",
    transform: "translateX(-50%)",
    display: "flex",
    justifyContent: "center",
    gap: "6px",
    padding: "6px 10px",
    backgroundColor: "rgba(0, 0, 0, 0.4)",
    borderRadius: "12px",
    pointerEvents: "none",
  },
  indicatorDot: {
    width: "6px",
    height: "6px",
    borderRadius: "50%",
    backgroundColor: "rgba(255, 255, 255, 0.4)",
    transition: "background-color 0.2s ease",
  },
  indicatorDotActive: {
    backgroundColor: "white",
  },
});

export const PostMediaCarousel = ({ mediaUrls }: { mediaUrls: string[] }) => {
  const styles = useStyles();
  const [activeIndex, setActiveIndex] = useState(0);
  const [aspectRatio, setAspectRatio] = useState<string>("auto");

  const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
    const target = e.currentTarget;
    const index = Math.round(target.scrollLeft / target.clientWidth);
    if (index !== activeIndex) {
      setActiveIndex(index);
    }
  };

  const handleImageLoad = (e: React.SyntheticEvent<HTMLImageElement>, index: number) => {
    if (index === 0) {
      const { naturalWidth, naturalHeight } = e.currentTarget;
      if (naturalWidth && naturalHeight) {
        setAspectRatio(`${naturalWidth} / ${naturalHeight}`);
      }
    }
  };

  if (!mediaUrls || mediaUrls.length === 0) return null;

  return (
    <div className={styles.carouselWrapper}>
      <CardPreview className={styles.cardPreview} onScroll={handleScroll}>
        {mediaUrls.map((url, idx) => (
          <CachedImage 
            key={idx}
            srcUrl={url} 
            alt={`${t("feed.mediaAlt")} ${idx + 1}`} 
            className={styles.mediaImage}
            style={{ aspectRatio, objectFit: "cover" }}
            onLoad={(e) => handleImageLoad(e, idx)}
          />
        ))}
      </CardPreview>
      {mediaUrls.length > 1 && (
        <div className={styles.indicatorContainer}>
          {mediaUrls.map((_, idx) => (
            <div 
              key={idx} 
              className={mergeClasses(
                styles.indicatorDot,
                idx === activeIndex && styles.indicatorDotActive
              )} 
            />
          ))}
        </div>
      )}
    </div>
  );
};
