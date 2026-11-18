package ua.pp.khvylyna.MinuteOfSilence

import android.app.NotificationManager
import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import androidx.core.content.ContextCompat

/**
 * Keeps the app process alive while the ceremony is switched on, so that the
 * scheduler keeps running and the ceremony can start and play with the screen
 * off.
 */
class CeremonyService : Service() {
  private val handler = Handler(Looper.getMainLooper())
  private val refreshNotification = Runnable { updateNotification() }
  private val timeReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
      updateNotification()
    }
  }

  override fun onCreate() {
    super.onCreate()
    ContextCompat.registerReceiver(
      this,
      timeReceiver,
      IntentFilter().apply {
        addAction(Intent.ACTION_TIME_CHANGED)
        addAction(Intent.ACTION_TIMEZONE_CHANGED)
        addAction(Intent.ACTION_DATE_CHANGED)
        addAction(Intent.ACTION_SCREEN_ON)
      },
      ContextCompat.RECEIVER_NOT_EXPORTED
    )
  }

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
    scheduleNotificationUpdate()
    return START_STICKY
  }

  override fun onDestroy() {
    handler.removeCallbacks(refreshNotification)
    unregisterReceiver(timeReceiver)
    super.onDestroy()
  }

  private fun updateNotification() {
    val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    manager.notify(
      CeremonyNotifications.SERVICE_NOTIFICATION_ID,
      CeremonyNotifications.buildServiceNotification(this)
    )
    scheduleNotificationUpdate()
  }

  private fun scheduleNotificationUpdate() {
    handler.removeCallbacks(refreshNotification)
    val delay = CeremonySchedule.nextNotificationUpdateMillis(this) - System.currentTimeMillis()
    handler.postDelayed(refreshNotification, delay.coerceAtLeast(1L))
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
