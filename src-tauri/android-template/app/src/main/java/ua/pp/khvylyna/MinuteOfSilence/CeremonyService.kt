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
import android.util.Log
import androidx.core.content.ContextCompat

/**
 * Starts and keeps the ceremony scheduler running without an Activity.
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
    if (!CeremonySchedule.isEnabled(this)) {
      stopSelf(startId)
      return START_NOT_STICKY
    }
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
    CeremonyDevice.install(this).startSchedulerNative()
    scheduleNotificationUpdate()
    handleNotificationAction(intent)
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

  private fun handleNotificationAction(intent: Intent?) {
    val actionIntent = intent ?: return
    val action = when (actionIntent.action) {
      ACTION_SKIP -> "skip"
      ACTION_RESTORE -> "restore"
      else -> return
    }
    val targetDate = actionIntent.getStringExtra(EXTRA_TARGET_DATE)
    if (targetDate == null) {
      Log.w("CeremonyService", "Notification action has no target date")
      return
    }
    try {
      if (CeremonyNotificationActions.dispatch(action, targetDate)) return

      val result = CeremonyDevice.install(this)
        .handleNotificationActionNative(action, targetDate)
      val settings = org.json.JSONObject(result)
      val skipDate = if (settings.isNull("skipDate")) null else settings.getString("skipDate")
      val ceremonyEnabled = settings.getBoolean("ceremonyEnabled")
      val weekdaysOnly = settings.getBoolean("weekdaysOnly")
      if (settings.getBoolean("appStateHandled")) {
        CeremonySchedule.updateStoredSettings(this, ceremonyEnabled, weekdaysOnly, skipDate)
      } else {
        CeremonySchedule.configure(this, ceremonyEnabled, weekdaysOnly, skipDate)
      }
      updateNotification()
    } catch (error: Exception) {
      Log.w("CeremonyService", "Failed to apply notification action", error)
    }
  }

  companion object {
    const val ACTION_SKIP = "ua.pp.khvylyna.MinuteOfSilence.action.SKIP_CEREMONY"
    const val ACTION_RESTORE = "ua.pp.khvylyna.MinuteOfSilence.action.RESTORE_CEREMONY"
    const val EXTRA_TARGET_DATE = "targetDate"

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
