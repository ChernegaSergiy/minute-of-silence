package ua.pp.khvylyna.MinuteOfSilence

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.core.app.NotificationCompat

/** Notifications the ceremony needs: the wake-up one and the one of the service. */
object CeremonyNotifications {
  const val CHANNEL_ID = "ceremony"
  const val SERVICE_CHANNEL_ID = "ceremony_service"
  const val WAKE_NOTIFICATION_ID = 1
  const val SERVICE_NOTIFICATION_ID = 2

  /**
   * Posts a notification with a full-screen intent that opens the app. When the
   * screen is off or locked Android starts the activity right away, otherwise
   * the notification pops up and a tap opens the app.
   */
  fun showWakeUp(context: Context) {
    ensureChannel(context)
    val notification = NotificationCompat.Builder(context, CHANNEL_ID)
      .setSmallIcon(context.applicationInfo.icon)
      .setContentTitle(context.getString(R.string.ceremony_notification_title))
      .setContentText(context.getString(R.string.ceremony_notification_start))
      .setCategory(NotificationCompat.CATEGORY_ALARM)
      .setPriority(NotificationCompat.PRIORITY_HIGH)
      .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
      .setAutoCancel(true)
      .setContentIntent(openApp(context))
      .setFullScreenIntent(openApp(context), true)
      .build()
    manager(context).notify(WAKE_NOTIFICATION_ID, notification)
  }

  /** The quiet ongoing notification of the foreground service. */
  fun buildServiceNotification(context: Context): Notification {
    ensureChannel(context)
    return NotificationCompat.Builder(context, CHANNEL_ID)
      .setSmallIcon(context.applicationInfo.icon)
      .setContentTitle(context.getString(R.string.ceremony_notification_title))
      .setContentText(context.getString(R.string.ceremony_notification_scheduled))
      .setOngoing(true)
      .setOnlyAlertOnce(true)
      .setPriority(NotificationCompat.PRIORITY_LOW)
      .setContentIntent(openApp(context))
      .build()
  }

  private fun openApp(context: Context): PendingIntent {
    val intent = Intent(context, MainActivity::class.java).apply {
      flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP
    }
    return PendingIntent.getActivity(
      context,
      0,
      intent,
      PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
    )
  }

  private fun ensureChannel(context: Context) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
    val name = context.getString(R.string.ceremony_channel_name)
    // The wake-up notification must pop up, the one of the service must stay quiet.
    manager(context).createNotificationChannel(
      NotificationChannel(CHANNEL_ID, name, NotificationManager.IMPORTANCE_HIGH)
    )
    manager(context).createNotificationChannel(
      NotificationChannel(SERVICE_CHANNEL_ID, name, NotificationManager.IMPORTANCE_LOW)
    )
  }

  private fun manager(context: Context) =
    context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
}
