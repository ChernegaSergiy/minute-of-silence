package ua.pp.khvylyna.MinuteOfSilence

import java.util.GregorianCalendar
import java.util.TimeZone
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Test

/**
 * The wake time decides whether the app is up for the ceremony, so the days
 * that are easy to get wrong are covered: the edges of the wake time, weekends,
 * the skipped day, the change of the clocks and the turn of the year.
 *
 * Every test names its zone and reads the clock only through `now`.
 */
class CeremonyWakeTimeTest {
  private val kyiv: TimeZone = TimeZone.getTimeZone("Europe/Kiev")
  private val utc: TimeZone = TimeZone.getTimeZone("UTC")

  private fun at(
    zone: TimeZone,
    year: Int,
    month: Int,
    day: Int,
    hour: Int,
    minute: Int,
    second: Int = 0
  ): Long = GregorianCalendar(zone).apply {
    clear()
    set(year, month - 1, day, hour, minute, second)
  }.timeInMillis

  private fun wake(
    now: Long,
    weekdaysOnly: Boolean = false,
    skipDate: String? = null,
    zone: TimeZone = kyiv
  ): Long? = CeremonyWakeTime.nextWakeMillis(now, weekdaysOnly, skipDate, zone)

  // 2026-11-18 is a Wednesday.

  @Test
  fun wakesAMinuteBeforeTheCeremonyToday() {
    val now = at(kyiv, 2026, 11, 18, 6, 0)
    assertEquals(at(kyiv, 2026, 11, 18, 8, 59), wake(now))
  }

  @Test
  fun wakesTodayOneSecondBeforeTheWakeTime() {
    val now = at(kyiv, 2026, 11, 18, 8, 58, 59)
    assertEquals(at(kyiv, 2026, 11, 18, 8, 59), wake(now))
  }

  @Test
  fun wakesTomorrowExactlyAtTheWakeTime() {
    val now = at(kyiv, 2026, 11, 18, 8, 59)
    assertEquals(at(kyiv, 2026, 11, 19, 8, 59), wake(now))
  }

  @Test
  fun wakesTomorrowBetweenTheWakeTimeAndTheCeremony() {
    val now = at(kyiv, 2026, 11, 18, 8, 59, 30)
    assertEquals(at(kyiv, 2026, 11, 19, 8, 59), wake(now))
  }

  @Test
  fun wakesTomorrowAfterTheCeremony() {
    val now = at(kyiv, 2026, 11, 18, 10, 0)
    assertEquals(at(kyiv, 2026, 11, 19, 8, 59), wake(now))
  }

  @Test
  fun leadsTheCeremonyByTheWakeLead() {
    val now = at(kyiv, 2026, 11, 18, 6, 0)
    val ceremony = at(kyiv, 2026, 11, 18, 9, 0)
    assertEquals(ceremony - CeremonyWakeTime.WAKE_LEAD_MS, wake(now))
  }

  // 2026-11-20 is a Friday, 21 a Saturday, 22 a Sunday and 23 a Monday.

  @Test
  fun wakesOnWeekendsWhenWeekdaysAreNotRequired() {
    val now = at(kyiv, 2026, 11, 20, 10, 0)
    assertEquals(at(kyiv, 2026, 11, 21, 8, 59), wake(now, weekdaysOnly = false))
  }

  @Test
  fun skipsTheWeekendAfterAFridayCeremony() {
    val now = at(kyiv, 2026, 11, 20, 10, 0)
    assertEquals(at(kyiv, 2026, 11, 23, 8, 59), wake(now, weekdaysOnly = true))
  }

  @Test
  fun skipsTheWeekendOnASaturday() {
    val now = at(kyiv, 2026, 11, 21, 6, 0)
    assertEquals(at(kyiv, 2026, 11, 23, 8, 59), wake(now, weekdaysOnly = true))
  }

  @Test
  fun skipsTheWeekendOnASunday() {
    val now = at(kyiv, 2026, 11, 22, 6, 0)
    assertEquals(at(kyiv, 2026, 11, 23, 8, 59), wake(now, weekdaysOnly = true))
  }

  @Test
  fun stillWakesOnAWeekdayWhenWeekdaysAreRequired() {
    val now = at(kyiv, 2026, 11, 18, 6, 0)
    assertEquals(at(kyiv, 2026, 11, 18, 8, 59), wake(now, weekdaysOnly = true))
  }

  @Test
  fun skipsTheSkippedDay() {
    val now = at(kyiv, 2026, 11, 18, 6, 0)
    assertEquals(at(kyiv, 2026, 11, 19, 8, 59), wake(now, skipDate = "2026-11-18"))
  }

  @Test
  fun ignoresASkipDateOfAnotherDay() {
    val now = at(kyiv, 2026, 11, 18, 6, 0)
    assertEquals(at(kyiv, 2026, 11, 18, 8, 59), wake(now, skipDate = "2026-11-19"))
  }

  @Test
  fun skipsTheSkippedDayAndTheWeekendBehindIt() {
    val now = at(kyiv, 2026, 11, 20, 6, 0)
    assertEquals(
      at(kyiv, 2026, 11, 23, 8, 59),
      wake(now, weekdaysOnly = true, skipDate = "2026-11-20")
    )
  }

  @Test
  fun skipsAMondayThatFollowsTheWeekend() {
    val now = at(kyiv, 2026, 11, 20, 10, 0)
    assertEquals(
      at(kyiv, 2026, 11, 24, 8, 59),
      wake(now, weekdaysOnly = true, skipDate = "2026-11-23")
    )
  }

  @Test
  fun matchesASkipDateWithSingleDigitMonthAndDay() {
    val now = at(kyiv, 2026, 3, 4, 20, 0)
    assertEquals(at(kyiv, 2026, 3, 6, 8, 59), wake(now, skipDate = "2026-03-05"))
  }

  @Test
  fun ignoresAnUnpaddedSkipDate() {
    // The Rust side always sends zero-padded dates, so this one never matches.
    val now = at(kyiv, 2026, 3, 4, 20, 0)
    assertEquals(at(kyiv, 2026, 3, 5, 8, 59), wake(now, skipDate = "2026-3-5"))
  }

  // Kyiv sets the clocks back on 2026-10-25 and forward on 2026-03-29.

  @Test
  fun keepsTheLocalWakeTimeWhenTheClocksGoBack() {
    val now = at(kyiv, 2026, 10, 24, 10, 0)
    val wakeAt = wake(now)
    assertEquals(at(kyiv, 2026, 10, 25, 8, 59), wakeAt)

    val previousDay = at(kyiv, 2026, 10, 24, 8, 59)
    assertEquals(25 * 60 * 60 * 1000L, wakeAt!! - previousDay)
  }

  @Test
  fun keepsTheLocalWakeTimeWhenTheClocksGoForward() {
    val now = at(kyiv, 2026, 3, 28, 10, 0)
    val wakeAt = wake(now)
    assertEquals(at(kyiv, 2026, 3, 29, 8, 59), wakeAt)

    val previousDay = at(kyiv, 2026, 3, 28, 8, 59)
    assertEquals(23 * 60 * 60 * 1000L, wakeAt!! - previousDay)
  }

  @Test
  fun wakesAtTheLocalTimeOnTheDayTheClocksChange() {
    val now = at(kyiv, 2026, 10, 25, 1, 0)
    assertEquals(at(kyiv, 2026, 10, 25, 8, 59), wake(now))
  }

  // Calendar edges.

  @Test
  fun wakesOnNewYearsDay() {
    val now = at(kyiv, 2026, 12, 31, 10, 0)
    assertEquals(at(kyiv, 2027, 1, 1, 8, 59), wake(now))
  }

  @Test
  fun wakesOnALeapDay() {
    val now = at(kyiv, 2028, 2, 28, 10, 0)
    assertEquals(at(kyiv, 2028, 2, 29, 8, 59), wake(now))
  }

  // The zone decides which 09:00 counts.

  @Test
  fun readsTheCeremonyTimeInTheGivenZone() {
    // 05:00 UTC is 07:00 in Kyiv, so Kyiv still has today's wake time ahead,
    // 06:59 UTC. In UTC the same instant is before 08:59 UTC.
    val now = at(utc, 2026, 11, 18, 5, 0)
    assertEquals(at(kyiv, 2026, 11, 18, 8, 59), wake(now, zone = kyiv))
    assertEquals(at(utc, 2026, 11, 18, 8, 59), wake(now, zone = utc))
    assertNotEquals(wake(now, zone = kyiv), wake(now, zone = utc))
  }

  @Test
  fun movesToTomorrowInAZoneWhereTodaysWakeTimePassed() {
    // 07:30 UTC is 09:30 in Kyiv, after the ceremony there, and still before it in UTC.
    val now = at(utc, 2026, 11, 18, 7, 30)
    assertEquals(at(kyiv, 2026, 11, 19, 8, 59), wake(now, zone = kyiv))
    assertEquals(at(utc, 2026, 11, 18, 8, 59), wake(now, zone = utc))
  }
}
