package ua.pp.khvylyna.MinuteOfSilence

import java.util.Calendar
import java.util.GregorianCalendar
import java.util.Locale
import java.util.TimeZone

/**
 * When the app wakes up before the ceremony. It has no Android dependencies so
 * that unit tests can run it on a plain JVM.
 */
object CeremonyWakeTime {
  private const val CEREMONY_HOUR = 9

  /** The app wakes a minute early to start and sync the time before 09:00. */
  const val WAKE_LEAD_MS = 60_000L

  /**
   * The next moment to wake the app, in epoch milliseconds, or null when no
   * ceremony is due within the next days.
   *
   * The ceremony is at 09:00 local time of [zone]. [skipDate] is the date of a
   * skipped ceremony as `yyyy-MM-dd`.
   */
  fun nextWakeMillis(
    now: Long,
    weekdaysOnly: Boolean,
    skipDate: String?,
    zone: TimeZone = TimeZone.getDefault()
  ): Long? {
    for (offset in 0..8) {
      val day = GregorianCalendar(zone).apply {
        timeInMillis = now
        add(Calendar.DAY_OF_YEAR, offset)
        set(Calendar.HOUR_OF_DAY, CEREMONY_HOUR)
        set(Calendar.MINUTE, 0)
        set(Calendar.SECOND, 0)
        set(Calendar.MILLISECOND, 0)
      }
      val wakeAt = day.timeInMillis - WAKE_LEAD_MS
      if (wakeAt <= now) continue

      val dayOfWeek = day.get(Calendar.DAY_OF_WEEK)
      if (weekdaysOnly && (dayOfWeek == Calendar.SATURDAY || dayOfWeek == Calendar.SUNDAY)) continue
      if (skipDate != null && isoDate(day) == skipDate) continue

      return wakeAt
    }
    return null
  }

  private fun isoDate(day: Calendar): String = String.format(
    Locale.ROOT,
    "%04d-%02d-%02d",
    day.get(Calendar.YEAR),
    day.get(Calendar.MONTH) + 1,
    day.get(Calendar.DAY_OF_MONTH)
  )
}
