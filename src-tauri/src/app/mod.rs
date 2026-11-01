pub mod apng;
pub mod commands;
pub mod tray;

use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveTime};

pub(crate) fn next_skip_date(now: DateTime<Local>, weekdays_only: bool) -> NaiveDate {
    let ceremony_time = NaiveTime::from_hms_opt(9, 0, 0).unwrap_or_default();
    let mut target = if now.time() < ceremony_time {
        now.date_naive()
    } else {
        (now + chrono::Duration::days(1)).date_naive()
    };

    if weekdays_only {
        while target.weekday().number_from_monday() > 5 {
            target += chrono::Duration::days(1);
        }
    }

    target
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_next_skip_date_daily() {
        // Friday 08:30 (before 09:00) -> Friday
        let fri_morning = Local.with_ymd_and_hms(2026, 10, 30, 8, 30, 0).unwrap();
        assert_eq!(
            next_skip_date(fri_morning, false),
            NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()
        );

        // Friday 09:30 (after 09:00) -> Saturday
        let fri_afternoon = Local.with_ymd_and_hms(2026, 10, 30, 9, 30, 0).unwrap();
        assert_eq!(
            next_skip_date(fri_afternoon, false),
            NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()
        );
    }

    #[test]
    fn test_next_skip_date_weekdays_only() {
        // Friday 08:30 (before 09:00) -> Friday
        let fri_morning = Local.with_ymd_and_hms(2026, 10, 30, 8, 30, 0).unwrap();
        assert_eq!(
            next_skip_date(fri_morning, true),
            NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()
        );

        // Friday 09:30 (after 09:00) -> skips weekend to Monday (Nov 2)
        let fri_afternoon = Local.with_ymd_and_hms(2026, 10, 30, 9, 30, 0).unwrap();
        assert_eq!(
            next_skip_date(fri_afternoon, true),
            NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()
        );

        // Saturday 12:00 -> Monday (Nov 2)
        let sat = Local.with_ymd_and_hms(2026, 10, 31, 12, 0, 0).unwrap();
        assert_eq!(
            next_skip_date(sat, true),
            NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()
        );

        // Sunday 18:00 -> Monday (Nov 2)
        let sun = Local.with_ymd_and_hms(2026, 11, 1, 18, 0, 0).unwrap();
        assert_eq!(
            next_skip_date(sun, true),
            NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()
        );

        // Monday 08:00 -> Monday (Nov 2)
        let mon_morning = Local.with_ymd_and_hms(2026, 11, 2, 8, 0, 0).unwrap();
        assert_eq!(
            next_skip_date(mon_morning, true),
            NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()
        );
    }
}
