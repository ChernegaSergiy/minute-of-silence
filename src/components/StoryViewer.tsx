import React, { useState, useEffect } from "react";
import {
  Dialog,
  DialogSurface,
  DialogBody,
  DialogContent,
  makeStyles,
  tokens,
  Button,
  Avatar,
  Text,
} from "@fluentui/react-components";
import { Dismiss24Regular, ChevronLeft24Regular, ChevronRight24Regular } from "@fluentui/react-icons";

const useStyles = makeStyles({
  dialogSurface: {
    maxWidth: "calc(90vh * 9 / 16)",
    width: "100%",
    height: "90vh",
    padding: 0,
    margin: "auto",
    backgroundColor: tokens.colorNeutralBackgroundStatic,
    borderRadius: tokens.borderRadiusLarge,
    display: "flex",
    flexDirection: "column",
    overflow: "hidden",
  },
  dialogContent: {
    display: "flex",
    flexDirection: "column",
    height: "100%",
    position: "relative",
  },
  header: {
    position: "absolute",
    top: 0,
    left: 0,
    right: 0,
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: tokens.spacingVerticalM,
    background: "linear-gradient(to bottom, rgba(0,0,0,0.6) 0%, rgba(0,0,0,0) 100%)",
    zIndex: 10,
  },
  headerLeft: {
    display: "flex",
    alignItems: "center",
    gap: tokens.spacingHorizontalS,
  },
  headerText: {
    display: "flex",
    flexDirection: "column",
  },
  progressContainer: {
    position: "absolute",
    top: tokens.spacingVerticalS,
    left: tokens.spacingHorizontalM,
    right: tokens.spacingHorizontalM,
    display: "flex",
    gap: "4px",
    zIndex: 11,
  },
  progressSegment: {
    height: "3px",
    flexGrow: 1,
    backgroundColor: "rgba(255, 255, 255, 0.3)",
    borderRadius: "2px",
  },
  progressSegmentActive: {
    backgroundColor: tokens.colorNeutralForegroundInverted,
  },
  mediaContainer: {
    flexGrow: 1,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: tokens.colorNeutralBackgroundStatic,
  },
  navigation: {
    position: "absolute",
    top: 0,
    bottom: 0,
    left: 0,
    right: 0,
    display: "flex",
    zIndex: 5,
  },
  navArea: {
    flexGrow: 1,
    cursor: "pointer",
  },
  closeButton: {
    color: "white",
    "&:hover": {
      color: "white",
    },
    "&:active": {
      color: "white",
    }
  }
});

interface StoryViewerProps {
  isOpen: boolean;
  onClose: () => void;
  authorName?: string;
  publishedAt?: string;
}

export const StoryViewer = ({ isOpen, onClose, authorName = "Автор історії", publishedAt = "Сьогодні" }: StoryViewerProps) => {
  const styles = useStyles();
  const [currentIndex, setCurrentIndex] = useState(0);
  const totalStories = 3;

  // Reset index when opening
  useEffect(() => {
    if (isOpen) setCurrentIndex(0);
  }, [isOpen]);

  const goNext = () => {
    if (currentIndex < totalStories - 1) {
      setCurrentIndex(prev => prev + 1);
    } else {
      onClose(); // Close if it's the last story
    }
  };

  const goPrev = () => {
    if (currentIndex > 0) {
      setCurrentIndex(prev => prev - 1);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={(_, data) => !data.open && onClose()}>
      <DialogSurface className={styles.dialogSurface}>
        <DialogBody style={{ height: "100%" }}>
          <DialogContent className={styles.dialogContent}>
            
            <div className={styles.progressContainer}>
              {Array.from({ length: totalStories }).map((_, idx) => (
                <div 
                  key={idx} 
                  className={`${styles.progressSegment} ${idx <= currentIndex ? styles.progressSegmentActive : ''}`} 
                />
              ))}
            </div>

            <div className={styles.header}>
              <div className={styles.headerLeft}>
                <Avatar name={authorName} size={32} />
                <div className={styles.headerText}>
                  <Text weight="semibold" style={{ color: "white" }}>
                    {authorName}
                  </Text>
                  <Text size={200} style={{ color: "rgba(255,255,255,0.7)" }}>
                    {publishedAt}
                  </Text>
                </div>
              </div>
              <Button 
                icon={<Dismiss24Regular color="white" />} 
                appearance="transparent" 
                className={styles.closeButton}
                onClick={onClose}
              />
            </div>

            <div className={styles.navigation}>
              <div className={styles.navArea} onClick={goPrev} title="Попередня" />
              <div className={styles.navArea} onClick={goNext} title="Наступна" />
            </div>

            <div className={styles.mediaContainer}>
              <Text size={600} style={{ color: "white" }}>
                [Повноекранне Медіа {currentIndex + 1}]
              </Text>
            </div>

          </DialogContent>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
};
