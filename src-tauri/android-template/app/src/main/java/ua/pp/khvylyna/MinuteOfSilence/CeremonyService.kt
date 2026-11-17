package ua.pp.khvylyna.MinuteOfSilence

import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.content.ContextCompat

/**
 * Keeps the app process alive while the ceremony is switched on, so that the
 * scheduler keeps running and the ceremony can start and play with the screen
 * off.
 */
class CeremonyService : Service() {
  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    val notification = CeremonyNotifications.buildServiceNotification(this)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
      startForeground(
        CeremonyNotifications.SERVICE_NOTIFICATION_ID,
        notification,
        ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
      )
    } else {
      startForeground(CeremonyNotifications.SERVICE_NOTIFICATION_ID, notification)
    }
    return START_STICKY
  }

  companion object {
    /** Starts the service. Call it while the app is in the foreground or from the alarm. */
    fun start(context: Context) {
      ContextCompat.startForegroundService(context, Intent(context, CeremonyService::class.java))
    }

    /** Stops the service. */
    fun stop(context: Context) {
      context.stopService(Intent(context, CeremonyService::class.java))
    }
  }
}
