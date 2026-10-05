import { useState, useEffect } from "react";
import {
  makeStyles,
  tokens,
  Text,
  Card,
  CardHeader,
  Avatar,
  Button,
  Skeleton,
  SkeletonItem
} from "@fluentui/react-components";
import { Feed48Regular, WifiOff48Regular, ArrowClockwise20Regular } from "@fluentui/react-icons";
import { t } from "../utils/i18n";
import { StoryViewer } from "./StoryViewer";
import { prefetchMedia, gcMediaCache } from "../utils/mediaCache";
import { CachedAvatar } from "./CachedAvatar";
import { PostMediaCarousel } from "./PostMediaCarousel";
import { PostContent } from "./PostContent";
import { CmsFeed, CmsStory } from "../types";
import { getFeedCache, saveFeedCache } from "../utils/api";
import { BASE_URL, FEED_URL } from "../utils/constants";

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
  storyAvatarRingViewed: {
    borderRadius: "50%",
    padding: "2px",
    background: tokens.colorNeutralStroke1,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
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
});

// Group stories by author
const groupStoriesByAuthor = (stories: CmsStory[], viewedIds: Set<string>) => {
  const grouped: Record<string, CmsStory[]> = {};
  
  stories.forEach(story => {
    if (!grouped[story.author]) {
      grouped[story.author] = [];
    }
    grouped[story.author].push(story);
  });

  return Object.entries(grouped)
    .map(([author, authorStories]) => {
      // Sort author's stories from oldest to newest (viewing order)
      const sortedStories = [...authorStories].sort((a, b) => 
        new Date(a.publishedAt).getTime() - new Date(b.publishedAt).getTime()
      );
      
      return {
        author,
        stories: sortedStories,
        hasUnviewed: sortedStories.some(s => !viewedIds.has(s.id)),
        // Timestamp of the newest story for sorting avatar rings
        latestStoryDate: new Date(sortedStories[sortedStories.length - 1].publishedAt).getTime()
      };
    })
    // Unviewed authors first, then by newest story date
    .sort((a, b) => {
      if (a.hasUnviewed && !b.hasUnviewed) return -1;
      if (!a.hasUnviewed && b.hasUnviewed) return 1;
      return b.latestStoryDate - a.latestStoryDate;
    });
};

export const HomeTab = () => {
  const styles = useStyles();
  const [selectedStoryAuthor, setSelectedStoryAuthor] = useState<string | null>(null);
  const [feed, setFeed] = useState<CmsFeed | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [viewedIds, setViewedIds] = useState<Set<string>>(() => {
    try {
      const saved = localStorage.getItem("viewedStories");
      return saved ? new Set(JSON.parse(saved)) : new Set();
    } catch {
      return new Set();
    }
  });

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
        
        // Extract all media URLs for prefetching and GC
        const activeMediaUrls = new Set<string>();
        data.posts.forEach(p => {
          activeMediaUrls.add(`${BASE_URL}avatars/${p.author}.png`);
          if (p.media) activeMediaUrls.add(`${BASE_URL}${p.media}`);
        });
        data.stories.forEach(s => {
          activeMediaUrls.add(`${BASE_URL}avatars/${s.author}.png`);
          s.media.forEach(m => activeMediaUrls.add(`${BASE_URL}${m}`));
        });
        
        const urlsArray = Array.from(activeMediaUrls);
        
        // Fire and forget media prefetch and GC
        prefetchMedia(urlsArray).catch(console.error);
        gcMediaCache(urlsArray).catch(console.error);

        
        // Garbage Collection for viewed stories
        const activeIds = new Set(data.stories.map(s => s.id));
        setViewedIds(prev => {
          const newSet = new Set<string>();
          let changed = false;
          prev.forEach(id => {
            if (activeIds.has(id)) {
              newSet.add(id);
            } else {
              changed = true; // Found an obsolete ID
            }
          });
          
          if (changed) {
            localStorage.setItem("viewedStories", JSON.stringify(Array.from(newSet)));
            return newSet;
          }
          return prev;
        });

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

  const markStoryViewed = (id: string) => {
    setViewedIds(prev => {
      if (prev.has(id)) return prev;
      const next = new Set(prev);
      next.add(id);
      localStorage.setItem("viewedStories", JSON.stringify(Array.from(next)));
      return next;
    });
  };

  const groupedStories = feed ? groupStoriesByAuthor(feed.stories, viewedIds) : [];

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
          <Button onClick={() => fetchFeed(true)} appearance="primary">
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
              <div className={group.hasUnviewed ? styles.storyAvatarRing : styles.storyAvatarRingViewed}>
                <CachedAvatar name={group.author} size={56} className={styles.storyAvatar} imageUrl={`${BASE_URL}avatars/${group.author}.png`} />
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
                image={
                  <CachedAvatar name={post.author} badge={{ status: "available" }} imageUrl={`${BASE_URL}avatars/${post.author}.png`} />
                }
                header={<Text weight="semibold">{post.title}</Text>}
                description={<Text size={200}>{t("feed.author")}: {post.author} • {date}</Text>}
              />
              <PostMediaCarousel mediaUrls={post.media.map(m => BASE_URL + m)} />
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
        stories={groupedStories.find(g => g.author === selectedStoryAuthor)?.stories || []}
        onStoryViewed={markStoryViewed}
      />
    </div>
  );
};
