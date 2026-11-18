package ua.pp.khvylyna.MinuteOfSilence

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import app.tauri.annotation.Command
import app.tauri.PermissionState
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Channel
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class AutostartArgs {
  var enabled: Boolean = false
}

@InvokeArg
class ScheduleArgs {
  var enabled: Boolean = false
  var weekdaysOnly: Boolean = false
  var skipDate: String? = null
}

@InvokeArg
class NotificationRequestArgs {
  var onlyOnce: Boolean = false
}

@InvokeArg
class NotificationActionHandlerArgs {
  lateinit var handler: Channel
}

private const val NOTIFICATIONS_ALIAS = "notifications"
private const val PERMISSION_PREFS = "ceremony_permissions"
private const val KEY_NOTIFICATIONS_ASKED = "notificationsAsked"

/**
 * Native half of the ceremony backend. The Rust side registers this class as
 * the `ceremony` plugin and calls its commands through the Tauri bridge.
 */
@TauriPlugin(
  permissions = [
    Permission(strings = [Manifest.permission.POST_NOTIFICATIONS], alias = NOTIFICATIONS_ALIAS)
  ]
)
class CeremonyPlugin(private val activity: Activity) : Plugin(activity) {
  /** Stores the ceremony schedule, sets the wake-up alarm and runs the service while it is on. */
  @Command
  fun configureSchedule(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(ScheduleArgs::class.java)
      CeremonySchedule.configure(
        activity.applicationContext,
        args.enabled,
        args.weekdaysOnly,
        args.skipDate
      )
      if (args.enabled) {
        CeremonyService.start(activity.applicationContext)
      } else {
        CeremonyService.stop(activity.applicationContext)
      }
      invoke.resolve()
    } catch (e: Exception) {
      invoke.reject("Failed to set the wake-up alarm: ${e.message}")
    }
  }

  /** Connects notification actions to the Rust settings owner. */
  @Command
  fun registerNotificationActionHandler(invoke: Invoke) {
    val args = invoke.parseArgs(NotificationActionHandlerArgs::class.java)
    val callback: (String, String) -> Unit = { action, targetDate ->
      val payload = JSObject()
      payload.put("action", action)
      payload.put("targetDate", targetDate)
      args.handler.sendObject(payload)
    }
    CeremonyNotificationActions.attach(callback)
    invoke.resolve()
  }

  /** Switches the alarm re-arming after a reboot on or off. */
  @Command
  fun setAutostart(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(AutostartArgs::class.java)
      BootReceiver.setEnabled(activity.applicationContext, args.enabled)
      invoke.resolve()
    } catch (e: Exception) {
      invoke.reject("Failed to change the autostart: ${e.message}")
    }
  }

  /**
   * Asks the user to allow notifications, which Android 13 and newer require.
   * Without them the wake-up notification and the one of the service stay
   * hidden. With `onlyOnce` it asks only if the app never asked before. Answers
   * whether the permission is granted.
   */
  @Command
  fun requestNotificationPermission(invoke: Invoke) {
    val args = invoke.parseArgs(NotificationRequestArgs::class.java)
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
      getPermissionState(NOTIFICATIONS_ALIAS) == PermissionState.GRANTED
    ) {
      resolveNotificationPermission(invoke, true)
      return
    }

    val prefs = activity.getSharedPreferences(PERMISSION_PREFS, Context.MODE_PRIVATE)
    if (args.onlyOnce && prefs.getBoolean(KEY_NOTIFICATIONS_ASKED, false)) {
      resolveNotificationPermission(invoke, false)
      return
    }
    prefs.edit().putBoolean(KEY_NOTIFICATIONS_ASKED, true).apply()
    requestPermissionForAlias(NOTIFICATIONS_ALIAS, invoke, "notificationPermissionResult")
  }

  @PermissionCallback
  private fun notificationPermissionResult(invoke: Invoke) {
    val granted = getPermissionState(NOTIFICATIONS_ALIAS) == PermissionState.GRANTED
    resolveNotificationPermission(invoke, granted)
  }

  private fun resolveNotificationPermission(invoke: Invoke, granted: Boolean) {
    val result = JSObject()
    result.put("granted", granted)
    invoke.resolve(result)
  }

  /**
   * The state of the notification permission: `granted`, `prompt` when the
   * system dialog can still be shown, or `denied` when it was refused for good.
   */
  @Command
  fun getNotificationPermission(invoke: Invoke) {
    val state = if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
      "granted"
    } else {
      when (getPermissionState(NOTIFICATIONS_ALIAS)) {
        PermissionState.GRANTED -> "granted"
        PermissionState.DENIED -> "denied"
        else -> "prompt"
      }
    }
    val result = JSObject()
    result.put("state", state)
    invoke.resolve(result)
  }

  /**
   * Opens the system notification settings of the app. The permission dialog
   * no longer shows after it was refused for good, so this is the way back.
   */
  @Command
  fun openNotificationSettings(invoke: Invoke) {
    try {
      val intent = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
        Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS)
          .putExtra(Settings.EXTRA_APP_PACKAGE, activity.packageName)
      } else {
        Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS)
          .setData(Uri.fromParts("package", activity.packageName, null))
      }
      activity.startActivity(intent)
      invoke.resolve()
    } catch (e: Exception) {
      invoke.reject("Failed to open the notification settings: ${e.message}")
    }
  }
}
