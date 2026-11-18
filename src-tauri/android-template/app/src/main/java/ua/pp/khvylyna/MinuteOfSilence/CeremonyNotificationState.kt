package ua.pp.khvylyna.MinuteOfSilence

import java.util.Calendar
import java.util.Locale
import java.util.TimeZone

/** Resolves the next eligible ceremony for display, without changing its schedule. */
object CeremonyNotificationState {
  /** The next 09:00, excluding weekends when required but not excluding a skipped ceremony. */
  fun nextCeremonyMillis(
    now: Long,
    weekdaysOnly: Boolean,
    zone: TimeZone = TimeZone.getDefault()
  ): Long {
    val day = Calendar.getInstance(zone).apply {
      timeInMillis = now
      set(Calendar.HOUR_OF_DAY, 9)
      set(Calendar.MINUTE, 0)
      set(Calendar.SECOND, 0)
      set(Calendar.MILLISECOND, 0)
      if (timeInMillis <= now) add(Calendar.DAY_OF_YEAR, 1)
    }
    if (weekdaysOnly) {
      while (day.get(Calendar.DAY_OF_WEEK) == Calendar.SATURDAY ||
        day.get(Calendar.DAY_OF_WEEK) == Calendar.SUNDAY) {
        day.add(Calendar.DAY_OF_YEAR, 1)
      }
    }
    return day.timeInMillis
  }

  /** The next eligible ceremony date in the ISO format accepted by Rust action requests. */
  fun nextCeremonyDate(
    now: Long,
    weekdaysOnly: Boolean,
    zone: TimeZone = TimeZone.getDefault()
  ): String {
    val day = Calendar.getInstance(zone).apply {
      timeInMillis = nextCeremonyMillis(now, weekdaysOnly, zone)
    }
    return String.format(
      Locale.ROOT,
      "%04d-%02d-%02d",
      day.get(Calendar.YEAR),
      day.get(Calendar.MONTH) + 1,
      day.get(Calendar.DAY_OF_MONTH)
    )
  }

  /** Whether the stored skip date belongs to the next eligible ceremony. */
  fun isNextSkipped(
    now: Long,
    weekdaysOnly: Boolean,
    skipDate: String?,
    zone: TimeZone = TimeZone.getDefault()
  ): Boolean = skipDate != null && skipDate == nextCeremonyDate(now, weekdaysOnly, zone)
}
