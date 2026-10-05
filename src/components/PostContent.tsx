import { useState } from "react";
import { makeStyles, tokens, Text } from "@fluentui/react-components";
import { t } from "../utils/i18n";

const useStyles = makeStyles({
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

export const PostContent = ({ content }: { content: string }) => {
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
          {isExpanded ? t("feed.showLess") : t("feed.showMore")}
        </Text>
      )}
    </div>
  );
};
