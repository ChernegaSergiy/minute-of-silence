package ua.pp.khvylyna.MinuteOfSilence

import java.util.Calendar
import java.util.TimeZone
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** Covers display status boundaries independently of the wake-up alarm. */
class CeremonyNotificationStateTest {
  private val kyiv = TimeZone.getTimeZone("Europe/Kiev")

  private fun at(year: Int, month: Int, day: Int, hour: Int, minute: Int = 0, second: Int = 0): Long =
    Calendar.getInstance(kyiv).apply {
      clear()
      set(year, month - 1, day, hour, minute, second)
    }.timeInMillis

  @Test
  fun actionTargetsTodayUntilNineAndTomorrowAfterwards() {
    assertEquals("2026-11-18", CeremonyNotificationState.nextCeremonyDate(at(2026, 11, 18, 8, 59, 59), false, kyiv))
    assertEquals("2026-11-19", CeremonyNotificationState.nextCeremonyDate(at(2026, 11, 18, 9), false, kyiv))
    assertEquals("2026-11-19", CeremonyNotificationState.nextCeremonyDate(at(2026, 11, 18, 9, 0, 1), false, kyiv))
  }

  @Test
  fun actionTargetsRespectTheWeekdaysSetting() {
    val now = at(2026, 11, 20, 10)
    assertEquals("2026-11-21", CeremonyNotificationState.nextCeremonyDate(now, false, kyiv))
    assertEquals("2026-11-23", CeremonyNotificationState.nextCeremonyDate(now, true, kyiv))
    assertEquals("2026-11-23", CeremonyNotificationState.nextCeremonyDate(at(2026, 11, 22, 6), true, kyiv))
  }

  @Test
  fun actionTargetsArePaddedAcrossMonthAndYearBoundaries() {
    assertEquals("2027-01-01", CeremonyNotificationState.nextCeremonyDate(at(2026, 12, 31, 23), false, kyiv))
    assertEquals("2026-03-05", CeremonyNotificationState.nextCeremonyDate(at(2026, 3, 4, 20), false, kyiv))
  }

  @Test
  fun actionTargetsUseTheLocalTimeZone() {
    val now = at(2026, 11, 18, 9)
    assertEquals("2026-11-19", CeremonyNotificationState.nextCeremonyDate(now, false, kyiv))
    assertEquals("2026-11-18", CeremonyNotificationState.nextCeremonyDate(now, false, TimeZone.getTimeZone("UTC")))
  }

  @Test
  fun actionTargetsAndSkipStatusReferToTheSameDateAcrossClockChanges() {
    for ((month, day) in listOf(3 to 29, 10 to 25)) {
      val now = at(2026, month, day, 8, 59, 59)
      val target = CeremonyNotificationState.nextCeremonyDate(now, false, kyiv)
      assertTrue(CeremonyNotificationState.isNextSkipped(now, false, target, kyiv))
      assertFalse(CeremonyNotificationState.isNextSkipped(at(2026, month, day, 9), false, target, kyiv))
    }
  }

  @Test
  fun reflectsSkippingAndRestoringTheNextCeremony() {
    val now = at(2026, 11, 18, 6)
    assertFalse(CeremonyNotificationState.isNextSkipped(now, false, null, kyiv))
    assertTrue(CeremonyNotificationState.isNextSkipped(now, false, "2026-11-18", kyiv))
    assertFalse(CeremonyNotificationState.isNextSkipped(now, false, "2026-11-17", kyiv))
    assertFalse(CeremonyNotificationState.isNextSkipped(now, false, "2026-11-19", kyiv))
  }

  @Test
  fun keepsTodaysSkipUntilNineNotUntilTheWakeUpTime() {
    val now = at(2026, 11, 18, 8, 59, 59)
    assertTrue(CeremonyNotificationState.isNextSkipped(now, false, "2026-11-18", kyiv))
    assertEquals(at(2026, 11, 18, 9), CeremonyNotificationState.nextCeremonyMillis(now, false, kyiv))
  }

  @Test
  fun advancesExactlyAtNine() {
    for (now in listOf(at(2026, 11, 18, 9), at(2026, 11, 18, 9, 0, 1))) {
      assertFalse(CeremonyNotificationState.isNextSkipped(now, false, "2026-11-18", kyiv))
      assertTrue(CeremonyNotificationState.isNextSkipped(now, false, "2026-11-19", kyiv))
      assertEquals(at(2026, 11, 19, 9), CeremonyNotificationState.nextCeremonyMillis(now, false, kyiv))
    }
  }

  @Test
  fun skipsWeekendsOnlyWhenRequired() {
    for (now in listOf(at(2026, 11, 20, 9), at(2026, 11, 21, 6), at(2026, 11, 22, 6))) {
      assertTrue(CeremonyNotificationState.isNextSkipped(now, true, "2026-11-23", kyiv))
      assertFalse(CeremonyNotificationState.isNextSkipped(now, true, "2026-11-21", kyiv))
    }
    assertTrue(CeremonyNotificationState.isNextSkipped(at(2026, 11, 21, 6), false, "2026-11-21", kyiv))
  }

  @Test
  fun handlesTheYearBoundaryAndPaddedDates() {
    assertTrue(CeremonyNotificationState.isNextSkipped(at(2026, 12, 31, 23), false, "2027-01-01", kyiv))
    assertFalse(CeremonyNotificationState.isNextSkipped(at(2026, 12, 31, 23), false, "2027-1-1", kyiv))
  }

  @Test
  fun keepsLocalNineAcrossClockChanges() {
    for ((month, day) in listOf(3 to 29, 10 to 25)) {
      val now = at(2026, month, day - 1, 10)
      assertEquals(at(2026, month, day, 9), CeremonyNotificationState.nextCeremonyMillis(now, false, kyiv))
    }
  }
}
