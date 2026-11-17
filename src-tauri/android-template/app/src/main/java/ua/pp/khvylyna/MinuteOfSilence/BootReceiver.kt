package ua.pp.khvylyna.MinuteOfSilence

import android.content.BroadcastReceiver
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager

/**
 * Sets the wake-up alarm again after a reboot or an app update, because
 * Android drops all alarms then. The receiver is disabled unless the autostart
 * setting is on.
 */
class BootReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) {
    when (intent.action) {
      Intent.ACTION_BOOT_COMPLETED,
      Intent.ACTION_MY_PACKAGE_REPLACED -> CeremonySchedule.schedule(context)
    }
  }

  companion object {
    /** Switches the receiver on or off. */
    fun setEnabled(context: Context, enabled: Boolean) {
      val state = if (enabled) {
        PackageManager.COMPONENT_ENABLED_STATE_ENABLED
      } else {
        PackageManager.COMPONENT_ENABLED_STATE_DISABLED
      }
      context.packageManager.setComponentEnabledSetting(
        ComponentName(context, BootReceiver::class.java),
        state,
        PackageManager.DONT_KILL_APP
      )
    }

    /** Whether the receiver is switched on. */
    fun isEnabled(context: Context): Boolean {
      val state = context.packageManager.getComponentEnabledSetting(
        ComponentName(context, BootReceiver::class.java)
      )
      return state == PackageManager.COMPONENT_ENABLED_STATE_ENABLED
    }
  }
}
