import {
  makeStyles,
  tokens,
  Text,
  Card,
  CardHeader,
  CardPreview,
  Avatar
} from "@fluentui/react-components";
import { t } from "../utils/i18n";

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
    height: "200px",
    backgroundColor: tokens.colorNeutralBackground3,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
  },
  content: {
    paddingTop: tokens.spacingVerticalS,
  }
});

export const HomeTab = () => {
  const styles = useStyles();

  return (
    <div className={styles.container}>
      <Text size={500} weight="semibold">
        {t("tabs.home")}
      </Text>

      {/* Горизонтальна стрічка історій */}
      <div className={styles.storiesContainer}>
        {[1, 2, 3, 4, 5, 6].map((i) => (
          <div key={i} className={styles.storyItem}>
            <div className={styles.storyAvatarRing}>
              <Avatar 
                name={`Автор ${i}`} 
                size={56} 
                className={styles.storyAvatar} 
              />
            </div>
            <Text size={200}>Автор {i}</Text>
          </div>
        ))}
      </div>

      {/* Макет публікації (Post) */}
      <Card className={styles.card}>
        <CardHeader
          image={<Avatar name="Офіційне джерело" badge={{ status: "available" }} />}
          header={<Text weight="semibold">Вшанування пам'яті героїв</Text>}
          description={<Text size={200}>Автор: Адміністрація • Сьогодні о 09:00</Text>}
        />
        <CardPreview className={styles.cardPreview}>
          <Text size={400} color="neutralSecondary">[Зображення / Медіа]</Text>
        </CardPreview>
        <div className={styles.content}>
          <Text>Щоденна хвилина мовчання за всіма загиблими у війні. Пам'ятаємо кожного, хто віддав життя за майбутнє.</Text>
        </div>
      </Card>
      
      {/* Ще одна публікація для прикладу скролу */}
      <Card className={styles.card}>
        <CardHeader
          image={<Avatar name="Новини" color="brand" />}
          header={<Text weight="semibold">Оновлення</Text>}
          description={<Text size={200}>Автор: Волонтери • Вчора</Text>}
        />
        <CardPreview className={styles.cardPreview}>
          <Text size={400} color="neutralSecondary">[Зображення / Медіа]</Text>
        </CardPreview>
        <div className={styles.content}>
          <Text>Продовжуємо роботу над платформою. Дякуємо за вашу підтримку.</Text>
        </div>
      </Card>
    </div>
  );
};
