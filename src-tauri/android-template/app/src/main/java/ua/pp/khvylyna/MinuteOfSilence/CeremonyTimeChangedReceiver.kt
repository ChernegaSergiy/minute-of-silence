package ua.pp.khvylyna.MinuteOfSilence

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/** Reschedules the next ceremony alarm when the system clock or timezone changes. */
class CeremonyTimeChangedReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) {
    when (intent.action) {
      Intent.ACTION_TIME_CHANGED,
      Intent.ACTION_TIMEZONE_CHANGED -> CeremonySchedule.schedule(context)
    }
  }
}
