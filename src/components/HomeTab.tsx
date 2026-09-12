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

      {/* Макет історії (Story) - без заголовку і тексту, лише медіа та автор */}
      <Card className={styles.card}>
        <CardHeader
          image={<Avatar name="Новини" color="brand" />}
          header={<Text weight="semibold">Історія (Story)</Text>}
          description={<Text size={200}>Автор: Волонтери • Вчора</Text>}
        />
        <CardPreview className={styles.cardPreview}>
          <Text size={400} color="neutralSecondary">[Вертикальне Медіа / Відео]</Text>
        </CardPreview>
      </Card>
    </div>
  );
};
