import {
  makeStyles,
  tokens,
  Text,
  Card,
  CardHeader,
  CardPreview,
  Avatar,
  Button
} from "@fluentui/react-components";
import { Share20Regular, Heart20Regular } from "@fluentui/react-icons";
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
  footer: {
    display: "flex",
    gap: tokens.spacingHorizontalS,
    marginTop: tokens.spacingVerticalS,
  }
});

export const HomeTab = () => {
  const styles = useStyles();

  return (
    <div className={styles.container}>
      <Text size={500} weight="semibold">
        {t("tabs.home")}
      </Text>

      {/* Макет публікації 1 */}
      <Card className={styles.card}>
        <CardHeader
          image={<Avatar name="Minute of Silence" badge={{ status: "available" }} />}
          header={<Text weight="semibold">Вшанування пам'яті</Text>}
          description={<Text size={200}>Сьогодні о 09:00</Text>}
        />
        <CardPreview className={styles.cardPreview}>
          <Text size={400} color="neutralSecondary">Зображення / Медіа</Text>
        </CardPreview>
        <Text>Щоденна хвилина мовчання за всіма загиблими у війні. Пам'ятаємо кожного героя.</Text>
        <div className={styles.footer}>
          <Button icon={<Heart20Regular />} appearance="subtle">Пам'ятаю</Button>
          <Button icon={<Share20Regular />} appearance="subtle">Поділитися</Button>
        </div>
      </Card>

      {/* Макет публікації 2 */}
      <Card className={styles.card}>
        <CardHeader
          image={<Avatar name="Оновлення" color="brand" />}
          header={<Text weight="semibold">Нова архітектура проєкту</Text>}
          description={<Text size={200}>Вчора</Text>}
        />
        <Text>Ми розпочали роботу над новою архітектурою доставки контенту через Cloudflare Pages. Це дозволить зробити застосунок ще більш швидким і незалежним.</Text>
        <div className={styles.footer}>
          <Button icon={<Heart20Regular />} appearance="subtle">Підтримати</Button>
        </div>
      </Card>
    </div>
  );
};
