package ua.pp.khvylyna.MinuteOfSilence

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import androidx.core.app.NotificationCompat

/** Notifications for the running ceremony and its foreground service. */
object CeremonyNotifications {
  const val CHANNEL_ID = "ceremony"
  const val SERVICE_CHANNEL_ID = "ceremony_service"
  const val SERVICE_NOTIFICATION_ID = 2
  private const val REMINDER_NOTIFICATION_TAG = "ceremony_reminder"

  /** Posts a reminder without opening the app. */
  fun postReminder(context: Context, title: String, body: String) {
    ensureChannel(context)
    val notification = NotificationCompat.Builder(context, CHANNEL_ID)
      .setSmallIcon(R.drawable.ic_stat_ceremony)
      .setContentTitle(title)
      .setContentText(body)
      .setAutoCancel(true)
      .setContentIntent(openApp(context))
      .build()
    manager(context).notify(REMINDER_NOTIFICATION_TAG, System.currentTimeMillis().toInt(), notification)
  }

  /** The quiet ongoing notification of the foreground service. */
  fun buildServiceNotification(context: Context): Notification {
    ensureChannel(context)
    val now = System.currentTimeMillis()
    val skipped = CeremonySchedule.isNextSkipped(context, now)
    val text = if (skipped) {
      R.string.ceremony_notification_skipped
    } else {
      R.string.ceremony_notification_scheduled
    }
    val builder = NotificationCompat.Builder(context, CHANNEL_ID)
      .setSmallIcon(R.drawable.ic_stat_ceremony)
      .setContentTitle(context.getString(R.string.ceremony_notification_title))
      .setContentText(context.getString(text))
      .setOngoing(true)
      .setOnlyAlertOnce(true)
      .setPriority(NotificationCompat.PRIORITY_LOW)
      .setContentIntent(openApp(context))
    if (CeremonyNotificationActions.isAvailable()) {
      val label = if (skipped) {
        R.string.ceremony_notification_restore
      } else {
        R.string.ceremony_notification_skip
      }
      builder.addAction(
        0,
        context.getString(label),
        changeSkip(context, skipped, CeremonySchedule.notificationTargetDate(context, now))
      )
    }
    return builder.build()
  }

  private fun changeSkip(context: Context, skipped: Boolean, targetDate: String): PendingIntent {
    val action = if (skipped) CeremonyService.ACTION_RESTORE else CeremonyService.ACTION_SKIP
    val intent = Intent(context, CeremonyService::class.java).apply {
      this.action = action
      data = Uri.Builder()
        .scheme("ceremony")
        .authority(context.packageName)
        .appendPath(action)
        .appendPath(targetDate)
        .build()
      putExtra(CeremonyService.EXTRA_TARGET_DATE, targetDate)
    }
    val flags = PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      PendingIntent.getForegroundService(context, 0, intent, flags)
    } else {
      PendingIntent.getService(context, 0, intent, flags)
    }
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
