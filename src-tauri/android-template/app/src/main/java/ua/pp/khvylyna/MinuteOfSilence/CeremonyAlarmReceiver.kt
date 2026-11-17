package ua.pp.khvylyna.MinuteOfSilence

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.PowerManager

/** Receives the wake-up alarm, keeps the device awake and opens the app. */
class CeremonyAlarmReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) {
    // The next alarm first, so that a failure below cannot cost the days after.
    CeremonySchedule.schedule(context)

    val powerManager = context.getSystemService(Context.POWER_SERVICE) as PowerManager
    val wakeLock = powerManager.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, WAKE_LOCK_TAG)
    wakeLock.acquire(WAKE_LOCK_TIMEOUT_MS)

    CeremonyNotifications.showWakeUp(context)
    // Keeps the process alive from here on, once the app is up.
    CeremonyService.start(context)
  }

  private companion object {
    const val WAKE_LOCK_TAG = "MinuteOfSilence:ceremony"

    // Long enough to cover the ceremony and the late-start window.
    const val WAKE_LOCK_TIMEOUT_MS = 5 * 60 * 1000L
  }
}
