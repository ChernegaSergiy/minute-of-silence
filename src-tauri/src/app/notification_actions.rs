//! Validated schedule changes requested by Android notification actions.

use chrono::{DateTime, Local, NaiveDate};
use serde::Deserialize;

use super::next_skip_date;
use crate::Settings;

/// An explicit action, rather than a toggle that could reverse a repeated tap.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum NotificationAction {
    Skip,
    Restore,
}

/// A notification action tied to the ceremony date displayed by its button.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NotificationActionRequest {
    action: NotificationAction,
    target_date: NaiveDate,
}

impl NotificationActionRequest {
    /// Changes only the skip date, ignoring disabled or outdated schedules.
    pub(crate) fn apply(&self, settings: &mut Settings, now: DateTime<Local>) -> bool {
        if !settings.ceremony_enabled
            || self.target_date != next_skip_date(now, settings.weekdays_only)
        {
            return false;
        }

        let skip_date = match self.action {
            NotificationAction::Skip => Some(self.target_date),
            NotificationAction::Restore => {
                if settings.skip_date != Some(self.target_date) {
                    return false;
                }
                None
            }
        };
        if settings.skip_date == skip_date {
            return false;
        }
        settings.skip_date = skip_date;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(day: u32, hour: u32, minute: u32, second: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 11, day, hour, minute, second)
            .single()
            .expect("unambiguous test time")
    }

    fn request(action: &str, target_date: &str) -> NotificationActionRequest {
        serde_json::from_value(serde_json::json!({
            "action": action,
            "targetDate": target_date,
        }))
        .expect("valid test action")
    }

    #[test]
    fn skip_and_restore_change_only_the_skip_date() {
        let mut settings = Settings::default();
        let original = settings.clone();
        let now = at(18, 8, 30, 0);
        assert!(request("skip", "2026-11-18").apply(&mut settings, now));
        let mut expected = original.clone();
        expected.skip_date = NaiveDate::from_ymd_opt(2026, 11, 18);
        assert_eq!(settings, expected);
        assert!(request("restore", "2026-11-18").apply(&mut settings, now));
        assert_eq!(settings, original);
    }

    #[test]
    fn repeated_actions_are_idempotent() {
        let mut settings = Settings::default();
        let now = at(18, 8, 30, 0);
        let skip = request("skip", "2026-11-18");
        let restore = request("restore", "2026-11-18");
        assert!(skip.apply(&mut settings, now));
        assert!(!skip.apply(&mut settings, now));
        assert!(restore.apply(&mut settings, now));
        assert!(!restore.apply(&mut settings, now));
    }

    #[test]
    fn ignores_disabled_ceremonies() {
        let mut settings = Settings {
            ceremony_enabled: false,
            skip_date: NaiveDate::from_ymd_opt(2026, 11, 18),
            ..Settings::default()
        };
        let original = settings.clone();
        for action in ["skip", "restore"] {
            assert!(!request(action, "2026-11-18").apply(&mut settings, at(18, 8, 0, 0)));
            assert_eq!(settings, original);
        }
    }

    #[test]
    fn rejects_todays_button_exactly_at_nine_and_afterwards() {
        for now in [at(18, 9, 0, 0), at(18, 9, 0, 1)] {
            let mut settings = Settings {
                skip_date: NaiveDate::from_ymd_opt(2026, 11, 18),
                ..Settings::default()
            };
            let original = settings.clone();
            for action in ["skip", "restore"] {
                assert!(!request(action, "2026-11-18").apply(&mut settings, now));
                assert_eq!(settings, original);
            }
            assert!(request("skip", "2026-11-19").apply(&mut settings, now));
        }
    }

    #[test]
    fn accepts_todays_button_one_second_before_nine() {
        let mut settings = Settings::default();
        assert!(request("skip", "2026-11-18").apply(&mut settings, at(18, 8, 59, 59)));
    }

    #[test]
    fn respects_weekdays_when_validating_the_target_date() {
        let mut settings = Settings {
            weekdays_only: true,
            ..Settings::default()
        };
        for now in [at(20, 9, 0, 0), at(21, 12, 0, 0), at(22, 12, 0, 0)] {
            assert!(!request("skip", "2026-11-21").apply(&mut settings, now));
            request("skip", "2026-11-23").apply(&mut settings, now);
            assert_eq!(settings.skip_date, NaiveDate::from_ymd_opt(2026, 11, 23));
        }
    }

    #[test]
    fn restore_does_not_clear_another_dates_skip() {
        let mut settings = Settings {
            skip_date: NaiveDate::from_ymd_opt(2026, 11, 19),
            ..Settings::default()
        };
        let original = settings.clone();
        assert!(!request("restore", "2026-11-18").apply(&mut settings, at(18, 8, 0, 0)));
        assert_eq!(settings, original);
    }

    #[test]
    fn rejects_unknown_actions_and_invalid_dates() {
        for payload in [
            r#"{"action":"toggle","targetDate":"2026-11-18"}"#,
            r#"{"action":"skip","targetDate":"2026-02-30"}"#,
            r#"{"action":"skip"}"#,
            r#"{"action":"skip","targetDate":"2026-11-18","extra":true}"#,
        ] {
            assert!(serde_json::from_str::<NotificationActionRequest>(payload).is_err());
        }
    }
}
