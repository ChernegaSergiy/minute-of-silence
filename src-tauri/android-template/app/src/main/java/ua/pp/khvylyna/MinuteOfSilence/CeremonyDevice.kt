package ua.pp.khvylyna.MinuteOfSilence

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Build
import androidx.annotation.Keep
import org.json.JSONObject
import kotlin.math.roundToInt

/** Application-scoped platform operations used by the Rust ceremony runner. */
@Keep
class CeremonyDevice private constructor(private val context: Context) {
  private val audioManager: AudioManager
    get() = context.getSystemService(Context.AUDIO_SERVICE) as AudioManager

  /** The focus held while other players are paused, API 26 and newer. */
  private var focusRequest: AudioFocusRequest? = null
  private var holdsFocus = false

  private val focusListener = AudioManager.OnAudioFocusChangeListener { }

  private val audioPlayer = CeremonyAudioPlayer(context)

  private external fun connectNative()

  init {
    connectNative()
  }

  fun callFromNative(command: String, payload: String): String {
    val args = JSONObject(payload)
    val result = JSONObject()
    when (command) {
      "getVolume" -> {
        val max = audioManager.getStreamMaxVolume(AudioManager.STREAM_MUSIC)
        val level = audioManager.getStreamVolume(AudioManager.STREAM_MUSIC)
        result.put("level", if (max > 0) (level * 100f / max).roundToInt() else 0)
      }
      "setVolume" -> {
        val max = audioManager.getStreamMaxVolume(AudioManager.STREAM_MUSIC)
        val index = (args.getInt("level").coerceIn(0, 100) * max / 100f).roundToInt()
        audioManager.setStreamVolume(AudioManager.STREAM_MUSIC, index, 0)
      }
      "isMuted" -> result.put("muted", audioManager.isStreamMute(AudioManager.STREAM_MUSIC))
      "setMute" -> {
        val direction = if (args.getBoolean("mute")) AudioManager.ADJUST_MUTE else AudioManager.ADJUST_UNMUTE
        audioManager.adjustStreamVolume(AudioManager.STREAM_MUSIC, direction, 0)
      }
      "pauseMedia" -> result.put("granted", requestExclusiveFocus())
      "resumeMedia" -> abandonFocus()
      "enqueueAudio" -> audioPlayer.enqueue(args.getString("name"), args.getDouble("volume").toFloat())
      "isAudioIdle" -> result.put("idle", audioPlayer.isIdle())
      "stopAudio" -> audioPlayer.stop()
      "getAudioDuration" -> result.put("durationMs", audioPlayer.durationMs(args.getString("name")))
      else -> error("Unknown ceremony command: $command")
    }
    return if (result.length() == 0) "null" else result.toString()
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

  companion object {
    init {
      System.loadLibrary("minute_of_silence_lib")
    }

    private var device: CeremonyDevice? = null

    @Synchronized
    fun install(context: Context): CeremonyDevice =
      device ?: CeremonyDevice(context.applicationContext).also { device = it }
  }
}
