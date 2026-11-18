package ua.pp.khvylyna.MinuteOfSilence

import android.app.AlarmManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build

/**
 * Wakes the device shortly before the ceremony.
 *
 * The Rust scheduler still decides whether the ceremony runs. The alarm only
 * brings the app up in time, so it may wake the app on a day the ceremony is
 * skipped but it must never miss a day the ceremony runs.
 */
object CeremonySchedule {
  private const val PREFS = "ceremony_schedule"
  private const val KEY_ENABLED = "enabled"
  private const val KEY_WEEKDAYS_ONLY = "weekdaysOnly"
  private const val KEY_SKIP_DATE = "skipDate"

  /** Stores the schedule and sets the alarm for the next ceremony. */
  fun configure(context: Context, enabled: Boolean, weekdaysOnly: Boolean, skipDate: String?) {
    preferences(context).edit()
      .putBoolean(KEY_ENABLED, enabled)
      .putBoolean(KEY_WEEKDAYS_ONLY, weekdaysOnly)
      .putString(KEY_SKIP_DATE, skipDate)
      .apply()
    schedule(context)
  }

  /** Sets the alarm for the next wake-up, or removes it when the ceremony is off. */
  fun schedule(context: Context) {
    val alarmManager = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
    val operation = alarmIntent(context)
    alarmManager.cancel(operation)

    val prefs = preferences(context)
    if (!prefs.getBoolean(KEY_ENABLED, false)) return

    val wakeAt = CeremonyWakeTime.nextWakeMillis(
      System.currentTimeMillis(),
      prefs.getBoolean(KEY_WEEKDAYS_ONLY, false),
      prefs.getString(KEY_SKIP_DATE, null)
    ) ?: return

    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S && !alarmManager.canScheduleExactAlarms()) {
      alarmManager.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, wakeAt, operation)
    } else {
      alarmManager.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, wakeAt, operation)
    }
  }

  /** Whether a ceremony is switched on in the stored schedule. */
  fun isEnabled(context: Context): Boolean =
    preferences(context).getBoolean(KEY_ENABLED, false)

  /** Whether the next eligible ceremony is marked as skipped. */
  fun isNextSkipped(context: Context, now: Long = System.currentTimeMillis()): Boolean {
    val prefs = preferences(context)
    return CeremonyNotificationState.isNextSkipped(
      now,
      prefs.getBoolean(KEY_WEEKDAYS_ONLY, false),
      prefs.getString(KEY_SKIP_DATE, null)
    )
  }

  /** The date to include in a notification action, even when that ceremony is skipped. */
  fun notificationTargetDate(context: Context, now: Long = System.currentTimeMillis()): String =
    CeremonyNotificationState.nextCeremonyDate(
      now,
      preferences(context).getBoolean(KEY_WEEKDAYS_ONLY, false)
    )

  /** The next time the notification must re-evaluate the skip status. */
  fun nextNotificationUpdateMillis(context: Context): Long =
    CeremonyNotificationState.nextCeremonyMillis(
      System.currentTimeMillis(),
      preferences(context).getBoolean(KEY_WEEKDAYS_ONLY, false)
    )

  private fun alarmIntent(context: Context): PendingIntent = PendingIntent.getBroadcast(
    context,
    0,
    Intent(context, CeremonyAlarmReceiver::class.java),
    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
  )

  private fun preferences(context: Context) =
    context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
