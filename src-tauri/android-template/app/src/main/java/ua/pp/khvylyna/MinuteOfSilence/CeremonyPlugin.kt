package ua.pp.khvylyna.MinuteOfSilence

import android.app.Activity
import android.content.Context
import android.media.AudioManager
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
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

/**
 * Native half of the ceremony backend. The Rust side registers this class as
 * the `ceremony` plugin and calls its commands through the Tauri bridge.
 */
@TauriPlugin
class CeremonyPlugin(private val activity: Activity) : Plugin(activity) {
  private val audioManager: AudioManager
    get() = activity.getSystemService(Context.AUDIO_SERVICE) as AudioManager

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
}
