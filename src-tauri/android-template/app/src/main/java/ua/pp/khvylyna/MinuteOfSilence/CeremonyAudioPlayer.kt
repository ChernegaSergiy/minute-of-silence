package ua.pp.khvylyna.MinuteOfSilence

import android.content.Context
import android.content.res.AssetFileDescriptor
import android.media.AudioAttributes
import android.media.MediaMetadataRetriever
import android.media.MediaPlayer
import android.util.Log
import java.io.IOException
import java.util.ArrayDeque

/**
 * Plays the bundled ceremony recordings one after another.
 *
 * The Rust side decides what plays and when: it queues files, waits until the
 * player is idle and stops it. The files are read from the APK assets.
 */
class CeremonyAudioPlayer(private val context: Context) {
  private val lock = Any()
  private val queue = ArrayDeque<String>()
  private var player: MediaPlayer? = null
  private var gain = 1f

  /** Queues a recording behind the ones already queued and starts playing if idle. */
  fun enqueue(name: String, volume: Float) {
    synchronized(lock) {
      gain = volume.coerceIn(0f, 1f)
      queue.addLast(name)
      if (player == null) {
        playNext()
      }
    }
  }

  /** True when nothing is playing and nothing is queued. */
  fun isIdle(): Boolean = synchronized(lock) { player == null && queue.isEmpty() }

  /** Drops the queue and stops the current recording. */
  fun stop() {
    synchronized(lock) {
      queue.clear()
      player?.let { release(it) }
      player = null
    }
  }

  /** The length of a recording in milliseconds. */
  fun durationMs(name: String): Long {
    val retriever = MediaMetadataRetriever()
    try {
      open(name).use { fd ->
        retriever.setDataSource(fd.fileDescriptor, fd.startOffset, fd.length)
      }
      val value = retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)
      return value?.toLongOrNull() ?: throw IOException("No duration for $name")
    } finally {
      retriever.release()
    }
  }

  // Callers hold the lock.
  private fun playNext() {
    while (true) {
      val name = queue.pollFirst() ?: return
      try {
        val next = MediaPlayer()
        try {
          next.setAudioAttributes(
            AudioAttributes.Builder()
              .setUsage(AudioAttributes.USAGE_MEDIA)
              .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
              .build()
          )
          open(name).use { fd ->
            next.setDataSource(fd.fileDescriptor, fd.startOffset, fd.length)
          }
          next.setVolume(gain, gain)
          next.setOnCompletionListener { finished(it) }
          next.setOnErrorListener { finishedPlayer, _, _ ->
            finished(finishedPlayer)
            true
          }
          next.prepare()
          next.start()
        } catch (e: Exception) {
          release(next)
          throw e
        }
        player = next
        return
      } catch (e: Exception) {
        // A recording that cannot play must not block the ones behind it.
        Log.e(TAG, "Failed to play $name", e)
      }
    }
  }

  private fun finished(finishedPlayer: MediaPlayer) {
    synchronized(lock) {
      release(finishedPlayer)
      if (player === finishedPlayer) {
        player = null
        playNext()
      }
    }
  }

  private fun release(target: MediaPlayer) {
    try {
      target.setOnCompletionListener(null)
      target.setOnErrorListener(null)
      target.release()
    } catch (e: Exception) {
      Log.w(TAG, "Failed to release the player", e)
    }
  }

  private fun open(name: String): AssetFileDescriptor {
    require(name.isNotEmpty() && !name.contains('/') && !name.contains("..")) {
      "Invalid recording name: $name"
    }
    return try {
      context.assets.openFd("audio/$name")
    } catch (e: IOException) {
      context.assets.openFd(name)
    }
  }

  private companion object {
    const val TAG = "CeremonyAudioPlayer"
  }
}
