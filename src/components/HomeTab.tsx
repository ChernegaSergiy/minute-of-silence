import { useState, useEffect } from "react";
import {
  makeStyles,
  tokens,
  Text,
  Card,
  CardHeader,
  CardPreview,
  Avatar,
  Spinner
} from "@fluentui/react-components";
import { t } from "../utils/i18n";
import { StoryViewer } from "./StoryViewer";
import { CmsFeed, CmsPost, CmsStory } from "../types";

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
  loadingContainer: {
    padding: tokens.spacingVerticalXXL,
    display: "flex",
    justifyContent: "center",
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
    "-ms-overflow-style": "none",
    "&::-webkit-scrollbar": {
      display: "none"
    }
  },
  mediaImage: {
    minWidth: "100%",
    width: "100%",
    height: "auto",
    maxHeight: "500px",
    objectFit: "cover",
    scrollSnapAlign: "center",
  },
  content: {
    paddingTop: tokens.spacingVerticalS,
    whiteSpace: "pre-wrap",
  }
});

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

  useEffect(() => {
    const fetchFeed = async () => {
      try {
        const response = await fetch(FEED_URL);
        if (response.ok) {
          const data: CmsFeed = await response.json();
          setFeed(data);
        }
      } catch (error) {
        console.error("Failed to fetch feed:", error);
      } finally {
        setLoading(false);
      }
    };
    fetchFeed();
  }, []);

  if (loading) {
    return (
      <div className={styles.loadingContainer}>
        <Spinner size="large" label="Завантаження..." />
      </div>
    );
  }

  const groupedStories = feed ? groupStoriesByAuthor(feed.stories) : [];

  return (
    <div className={styles.container}>
      <Text size={500} weight="semibold">
        {t("tabs.home")}
      </Text>

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
      {feed?.posts.map((post) => {
        const date = new Date(post.publishedAt).toLocaleDateString();
        return (
          <Card key={post.id} className={styles.card}>
            <CardHeader
              image={<Avatar name={post.author} badge={{ status: "available" }} />}
              header={<Text weight="semibold">{post.title}</Text>}
              description={<Text size={200}>Автор: {post.author} • {date}</Text>}
            />
            {post.media && post.media.length > 0 && (
              <CardPreview className={styles.cardPreview}>
                {post.media.map((imgSrc, idx) => (
                  <img 
                    key={idx}
                    src={`${BASE_URL}${imgSrc}`} 
                    alt={`Post media ${idx + 1}`} 
                    className={styles.mediaImage}
                  />
                ))}
              </CardPreview>
            )}
            <div className={styles.content}>
              <Text>{post.content}</Text>
            </div>
          </Card>
        );
      })}

      <StoryViewer 
        isOpen={selectedStoryAuthor !== null} 
        onClose={() => setSelectedStoryAuthor(null)}
        authorName={selectedStoryAuthor || undefined}
      />
    </div>
  );
};
