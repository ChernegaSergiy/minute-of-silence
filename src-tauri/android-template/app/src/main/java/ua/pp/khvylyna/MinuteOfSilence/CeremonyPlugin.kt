package ua.pp.khvylyna.MinuteOfSilence

import android.Manifest
import android.app.Activity
import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Build
import app.tauri.annotation.Command
import app.tauri.PermissionState
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class VolumeArgs {
  var level: Int = 0
}

@InvokeArg
class MuteArgs {
  var mute: Boolean = false
}

@InvokeArg
class EnqueueAudioArgs {
  lateinit var name: String
  var volume: Float = 1f
}

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
class AudioNameArgs {
  lateinit var name: String
}

private const val NOTIFICATIONS_ALIAS = "notifications"

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
  private val audioManager: AudioManager
    get() = activity.getSystemService(Context.AUDIO_SERVICE) as AudioManager

  /** The focus held while other players are paused, API 26 and newer. */
  private var focusRequest: AudioFocusRequest? = null
  private var holdsFocus = false

  // The ceremony does not react to focus changes: it must not be interrupted.
  private val focusListener = AudioManager.OnAudioFocusChangeListener { }

  private val audioPlayer = CeremonyAudioPlayer(activity.applicationContext)

  /** The media stream volume as a percentage of its maximum. */
  @Command
  fun getVolume(invoke: Invoke) {
    val max = audioManager.getStreamMaxVolume(AudioManager.STREAM_MUSIC)
    val current = audioManager.getStreamVolume(AudioManager.STREAM_MUSIC)
    val result = JSObject()
    result.put("level", if (max > 0) Math.round(current * 100f / max) else 0)
    invoke.resolve(result)
  }

  /** Sets the media stream volume from a percentage. */
  @Command
  fun setVolume(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(VolumeArgs::class.java)
      val max = audioManager.getStreamMaxVolume(AudioManager.STREAM_MUSIC)
      val index = Math.round(args.level.coerceIn(0, 100) * max / 100f)
      audioManager.setStreamVolume(AudioManager.STREAM_MUSIC, index, 0)
      invoke.resolve()
    } catch (e: Exception) {
      invoke.reject("Failed to set the volume: ${e.message}")
    }
  }

  /** Whether the media stream is muted. */
  @Command
  fun isMuted(invoke: Invoke) {
    val result = JSObject()
    result.put("muted", audioManager.isStreamMute(AudioManager.STREAM_MUSIC))
    invoke.resolve(result)
  }

  /** Mutes or unmutes the media stream, keeping its volume index. */
  @Command
  fun setMute(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(MuteArgs::class.java)
      val direction = if (args.mute) AudioManager.ADJUST_MUTE else AudioManager.ADJUST_UNMUTE
      audioManager.adjustStreamVolume(AudioManager.STREAM_MUSIC, direction, 0)
      invoke.resolve()
    } catch (e: Exception) {
      invoke.reject("Failed to change the mute state: ${e.message}")
    }
  }

  /**
   * Pauses other players. Android has no call for that, so the ceremony takes
   * the audio focus exclusively for a while. Well-behaved players pause on
   * their own and resume when the focus is released.
   */
  @Command
  fun pauseMedia(invoke: Invoke) {
    val result = JSObject()
    try {
      result.put("granted", requestExclusiveFocus())
      invoke.resolve(result)
    } catch (e: Exception) {
      invoke.reject("Failed to take the audio focus: ${e.message}")
    }
  }

  /** Releases the audio focus so that the paused players can resume. */
  @Command
  fun resumeMedia(invoke: Invoke) {
    try {
      abandonFocus()
      invoke.resolve()
    } catch (e: Exception) {
      invoke.reject("Failed to release the audio focus: ${e.message}")
    }
  }

  private fun requestExclusiveFocus(): Boolean {
    abandonFocus()
    val granted: Int
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      val attributes = AudioAttributes.Builder()
        .setUsage(AudioAttributes.USAGE_MEDIA)
        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
        .build()
      val request = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN_TRANSIENT_EXCLUSIVE)
        .setAudioAttributes(attributes)
        .setOnAudioFocusChangeListener(focusListener)
        .build()
      focusRequest = request
      granted = audioManager.requestAudioFocus(request)
    } else {
      @Suppress("DEPRECATION")
      granted = audioManager.requestAudioFocus(
        focusListener,
        AudioManager.STREAM_MUSIC,
        AudioManager.AUDIOFOCUS_GAIN_TRANSIENT_EXCLUSIVE
      )
    }
    holdsFocus = granted == AudioManager.AUDIOFOCUS_REQUEST_GRANTED
    return holdsFocus
  }

  private fun abandonFocus() {
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      focusRequest?.let { audioManager.abandonAudioFocusRequest(it) }
      focusRequest = null
    } else if (holdsFocus) {
      @Suppress("DEPRECATION")
      audioManager.abandonAudioFocus(focusListener)
    }
    holdsFocus = false
  }

  /** Queues a bundled recording for playback. */
  @Command
  fun enqueueAudio(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(EnqueueAudioArgs::class.java)
      audioPlayer.enqueue(args.name, args.volume)
      invoke.resolve()
    } catch (e: Exception) {
      invoke.reject("Failed to queue the recording: ${e.message}")
    }
  }

  /** Whether the queue is empty and nothing is playing. */
  @Command
  fun isAudioIdle(invoke: Invoke) {
    val result = JSObject()
    result.put("idle", audioPlayer.isIdle())
    invoke.resolve(result)
  }

  /** Stops the playback and drops the queue. */
  @Command
  fun stopAudio(invoke: Invoke) {
    audioPlayer.stop()
    invoke.resolve()
  }

  /** The length of a bundled recording. */
  @Command
  fun getAudioDuration(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(AudioNameArgs::class.java)
      val result = JSObject()
      result.put("durationMs", audioPlayer.durationMs(args.name))
      invoke.resolve(result)
    } catch (e: Exception) {
      invoke.reject("Failed to read the recording length: ${e.message}")
    }
  }

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
   * hidden. Answers whether the permission is granted.
   */
  @Command
  fun requestNotificationPermission(invoke: Invoke) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
      getPermissionState(NOTIFICATIONS_ALIAS) == PermissionState.GRANTED
    ) {
      resolveNotificationPermission(invoke, true)
      return
    }
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
}
