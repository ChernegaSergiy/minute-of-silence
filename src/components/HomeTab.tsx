import { useState, useEffect } from "react";
import {
  makeStyles,
  tokens,
  Text,
  Card,
  CardHeader,
  CardPreview,
  Avatar,
  Button,
  Skeleton,
  SkeletonItem
} from "@fluentui/react-components";
import { Feed48Regular, WifiOff48Regular, ArrowClockwise20Regular } from "@fluentui/react-icons";
import { t } from "../utils/i18n";
import { StoryViewer } from "./StoryViewer";
import { CmsFeed, CmsStory } from "../types";
import { getFeedCache, saveFeedCache } from "../utils/api";

const FEED_URL = "https://feed.khvylyna.pp.ua/feed.json";
const BASE_URL = "https://feed.khvylyna.pp.ua/";

const useStyles = makeStyles({
  container: {
    padding: tokens.spacingVerticalM,
    paddingBottom: tokens.spacingVerticalXXL,
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalL,
    maxWidth: "600px",
    margin: "0 auto",
  },
  header: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
  },
  stateCard: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    gap: tokens.spacingVerticalM,
    padding: tokens.spacingVerticalXXL,
    textAlign: "center",
    backgroundColor: tokens.colorNeutralBackground2,
    marginTop: tokens.spacingVerticalM,
  },
  skeletonStories: {
    display: "flex",
    gap: tokens.spacingHorizontalM,
    overflow: "hidden",
  },
  skeletonCard: {
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalM,
  },
  storiesContainer: {
    display: "flex",
    gap: tokens.spacingHorizontalM,
    overflowX: "auto",
    paddingBottom: tokens.spacingVerticalS,
    scrollbarWidth: "none",
    "-ms-overflow-style": "none",
    "&::-webkit-scrollbar": {
      display: "none"
    }
  },
  storyItem: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: tokens.spacingVerticalXS,
    cursor: "pointer",
    minWidth: "72px",
  },
  storyAvatarRing: {
    borderRadius: "50%",
    padding: "2px",
    background: tokens.colorBrandBackground,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
  },
  storyAvatar: {
    border: `2px solid ${tokens.colorNeutralBackground1}`,
  },
  card: {
    width: "100%",
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
  carouselWrapper: {
    position: "relative",
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
  content: {
    paddingTop: tokens.spacingVerticalS,
    whiteSpace: "pre-wrap",
  },
  readMoreButton: {
    color: tokens.colorBrandForeground1,
    cursor: "pointer",
    fontWeight: tokens.fontWeightSemibold,
    marginTop: tokens.spacingVerticalXS,
    display: "inline-block",
    "&:hover": {
      textDecoration: "underline",
    }
  }
});

const PostContent = ({ content }: { content: string }) => {
  const styles = useStyles();
  const [isExpanded, setIsExpanded] = useState(false);
  
  const MAX_LENGTH = 150;
  const paragraphs = content.split('\n');
  const isLong = content.length > MAX_LENGTH || paragraphs.length > 3;

  let displayContent = content;
  if (!isExpanded && isLong) {
    if (paragraphs.length > 3) {
      displayContent = paragraphs.slice(0, 3).join('\n');
    }
    if (displayContent.length > MAX_LENGTH) {
      displayContent = displayContent.slice(0, MAX_LENGTH);
      const lastSpace = displayContent.lastIndexOf(" ");
      if (lastSpace > 0) {
        displayContent = displayContent.slice(0, lastSpace);
      }
    }
    displayContent += "...";
  }

  const displayParagraphs = displayContent.split('\n');

  return (
    <div className={styles.content}>
      {displayParagraphs.map((line, idx) => (
        line.trim() === '' ? (
          <br key={idx} />
        ) : (
          <Text key={idx} as="p" block style={{ margin: 0, marginBottom: "8px" }}>
            {line}
          </Text>
        )
      ))}
      {isLong && (
        <Text 
          className={styles.readMoreButton} 
          onClick={() => setIsExpanded(!isExpanded)}
        >
          {isExpanded ? "Сховати" : "Більше"}
        </Text>
      )}
    </div>
  );
};

const PostMediaCarousel = ({ media }: { media: string[] }) => {
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

  if (!media || media.length === 0) return null;

  return (
    <div className={styles.carouselWrapper}>
      <CardPreview className={styles.cardPreview} onScroll={handleScroll}>
        {media.map((imgSrc, idx) => (
          <img 
            key={idx}
            src={`${BASE_URL}${imgSrc}`} 
            alt={`Post media ${idx + 1}`} 
            className={styles.mediaImage}
            style={{ aspectRatio, objectFit: "cover" }}
            onLoad={(e) => handleImageLoad(e, idx)}
          />
        ))}
      </CardPreview>
      {media.length > 1 && (
        <div className={styles.indicatorContainer}>
          {media.map((_, idx) => (
            <div 
              key={idx} 
              className={`${styles.indicatorDot} ${idx === activeIndex ? styles.indicatorDotActive : ""}`} 
            />
          ))}
        </div>
      )}
    </div>
  );
};

// Group stories by author
const groupStoriesByAuthor = (stories: CmsStory[]) => {
  const grouped: Record<string, CmsStory[]> = {};
  stories.forEach(story => {
    if (!grouped[story.author]) {
      grouped[story.author] = [];
    }
    grouped[story.author].push(story);
  });
  return Object.entries(grouped).map(([author, authorStories]) => ({
    author,
    stories: authorStories
  }));
};

export const HomeTab = () => {
  const styles = useStyles();
  const [selectedStoryAuthor, setSelectedStoryAuthor] = useState<string | null>(null);
  const [feed, setFeed] = useState<CmsFeed | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);

  const fetchFeed = async (forceRefresh = false) => {
    setLoading(true);
    setError(false);
    
    const minLoadingTime = 600; // ms
    const startTime = Date.now();
    let isShowingSkeleton = false;

    try {
      // 1. Instantly load from cache if available (skip if forcing refresh)
      let cachedData = null;
      if (!forceRefresh) {
        cachedData = await getFeedCache<CmsFeed>();
        if (cachedData) {
          setFeed(cachedData);
          setLoading(false); // Hide immediately since we have data
        } else {
          isShowingSkeleton = true;
        }
      } else {
        isShowingSkeleton = true;
      }

      // 2. Fetch fresh data in the background
      const response = await fetch(FEED_URL);
      
      // Prevent nanosecond flashing if we are showing the skeleton
      if (isShowingSkeleton) {
        const elapsed = Date.now() - startTime;
        if (elapsed < minLoadingTime) {
          await new Promise(r => setTimeout(r, minLoadingTime - elapsed));
        }
      }

      if (response.ok) {
        const data: CmsFeed = await response.json();
        setFeed(data);
        await saveFeedCache(data);
      } else if (!cachedData) {
        setError(true);
      }
    } catch (err) {
      console.error("Failed to fetch feed:", err);
      
      // Delay error appearance as well if we were showing the skeleton
      if (isShowingSkeleton) {
        const elapsed = Date.now() - startTime;
        if (elapsed < minLoadingTime) {
          await new Promise(r => setTimeout(r, minLoadingTime - elapsed));
        }
      }

      // Check current feed via functional state update to avoid stale closures
      setFeed(prevFeed => {
        if (!prevFeed) {
          setError(true);
        }
        return prevFeed;
      });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchFeed();

    const handleKeyDown = (e: KeyboardEvent) => {
      // Refresh on F5 or Ctrl/Cmd + R
      if (e.key === "F5" || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "r")) {
        e.preventDefault();
        fetchFeed(true);
      }
    };
    
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const groupedStories = feed ? groupStoriesByAuthor(feed.stories) : [];

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <Text size={500} weight="semibold">
          {t("tabs.home")}
        </Text>
        <Button 
          icon={<ArrowClockwise20Regular />} 
          appearance="transparent" 
          onClick={() => fetchFeed(true)}
          disabled={loading}
          title={t("feed.refresh")}
        />
      </div>

      {loading && !feed && (
        <Skeleton animation="pulse" style={{ display: "flex", flexDirection: "column", gap: "24px" }}>
          <div className={styles.skeletonStories}>
            {[1, 2, 3, 4, 5].map(i => (
              <div key={i} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
                <SkeletonItem shape="circle" size={56} />
                <SkeletonItem shape="rectangle" style={{ width: '40px', height: '12px' }} />
              </div>
            ))}
          </div>
          {[1, 2].map(i => (
            <Card key={i} className={styles.card}>
              <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
                <SkeletonItem shape="circle" size={40} />
                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1 }}>
                  <SkeletonItem shape="rectangle" style={{ width: '40%', height: '16px' }} />
                  <SkeletonItem shape="rectangle" style={{ width: '20%', height: '12px' }} />
                </div>
              </div>
              <SkeletonItem shape="rectangle" style={{ width: '100%', height: '200px', marginTop: '12px' }} />
              <SkeletonItem shape="rectangle" style={{ width: '80%', height: '12px', marginTop: '12px' }} />
            </Card>
          ))}
        </Skeleton>
      )}

      {error && (
        <Card className={styles.stateCard}>
          <WifiOff48Regular style={{ color: tokens.colorNeutralForeground4 }} />
          <Text size={400} weight="medium">
            {t("feed.errorTitle")}
          </Text>
          <Text size={300} style={{ color: tokens.colorNeutralForeground3 }}>
            {t("feed.errorSubtitle")}
          </Text>
          <Button onClick={fetchFeed} appearance="primary">
            {t("feed.retryButton")}
          </Button>
        </Card>
      )}

      {/* Horizontal stories feed */}
      {groupedStories.length > 0 && (
        <div className={styles.storiesContainer}>
          {groupedStories.map((group, index) => (
            <div 
              key={index} 
              className={styles.storyItem}
              onClick={() => setSelectedStoryAuthor(group.author)}
            >
              <div className={styles.storyAvatarRing}>
                <Avatar 
                  name={group.author} 
                  size={56} 
                  className={styles.storyAvatar} 
                />
              </div>
              <Text size={200}>{group.author}</Text>
            </div>
          ))}
        </div>
      )}

      {/* Posts feed */}
      {feed?.posts && feed.posts.length > 0 && (
        feed.posts.map((post) => {
          const date = new Date(post.publishedAt).toLocaleDateString();
          return (
            <Card key={post.id} className={styles.card}>
              <CardHeader
                image={<Avatar name={post.author} badge={{ status: "available" }} />}
                header={<Text weight="semibold">{post.title}</Text>}
                description={<Text size={200}>Автор: {post.author} • {date}</Text>}
              />
              <PostMediaCarousel media={post.media} />
              <PostContent content={post.content} />
            </Card>
          );
        })
      )}
      
      {feed && feed.posts && feed.posts.length === 0 && (
        <Card className={styles.stateCard}>
          <Feed48Regular style={{ color: tokens.colorNeutralForeground4 }} />
          <Text size={400} weight="medium">
            {t("feed.emptyTitle")}
          </Text>
          <Text size={300} style={{ color: tokens.colorNeutralForeground3 }}>
            {t("feed.emptySubtitle")}
          </Text>
        </Card>
      )}

      <StoryViewer 
        isOpen={selectedStoryAuthor !== null} 
        onClose={() => setSelectedStoryAuthor(null)}
        authorName={selectedStoryAuthor || undefined}
      />
    </div>
  );
};
