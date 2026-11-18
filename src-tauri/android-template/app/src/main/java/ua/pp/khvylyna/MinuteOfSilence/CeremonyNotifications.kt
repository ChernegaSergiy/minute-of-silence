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

/** Notifications the ceremony needs: the wake-up one and the one of the service. */
object CeremonyNotifications {
  const val CHANNEL_ID = "ceremony"
  const val SERVICE_CHANNEL_ID = "ceremony_service"
  const val WAKE_NOTIFICATION_ID = 1
  const val SERVICE_NOTIFICATION_ID = 2
  const val EXTRA_CEREMONY_OVERLAY_LAUNCH = "ceremony_overlay_launch"

  /**
   * Posts a notification with a full-screen intent that opens the app. When the
   * screen is off or locked Android starts the activity right away, otherwise
   * the notification pops up and a tap opens the app.
   */
  fun showWakeUp(context: Context) {
    ensureChannel(context)
    val builder = NotificationCompat.Builder(context, CHANNEL_ID)
      .setSmallIcon(R.drawable.ic_stat_ceremony)
      .setContentTitle(context.getString(R.string.ceremony_notification_title))
      .setContentText(context.getString(R.string.ceremony_notification_start))
      .setCategory(NotificationCompat.CATEGORY_ALARM)
      .setPriority(NotificationCompat.PRIORITY_HIGH)
      .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
      .setAutoCancel(true)
      .setContentIntent(openApp(context))
    if (CeremonySchedule.shouldShowVisualOverlay(context)) {
      if (!CeremonySchedule.isNextSkipped(context)) {
        builder.setFullScreenIntent(openApp(context, true), true)
      }
    }
    manager(context).notify(WAKE_NOTIFICATION_ID, builder.build())
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

  private fun openApp(context: Context, overlayLaunch: Boolean = false): PendingIntent {
    val intent = Intent(context, MainActivity::class.java).apply {
      flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP
      if (overlayLaunch) putExtra(EXTRA_CEREMONY_OVERLAY_LAUNCH, true)
    }
    return PendingIntent.getActivity(
      context,
      if (overlayLaunch) 1 else 0,
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
