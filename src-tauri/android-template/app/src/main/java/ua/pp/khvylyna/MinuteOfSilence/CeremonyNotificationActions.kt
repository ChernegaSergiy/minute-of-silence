package ua.pp.khvylyna.MinuteOfSilence

/** Holds the Rust process's action channel independently of the activity lifecycle. */
object CeremonyNotificationActions {
  private var handler: ((String, String) -> Unit)? = null

  /** Attaches the handler registered by the Rust backend. */
  @Synchronized
  fun attach(callback: (String, String) -> Unit) {
    handler = callback
  }

  /** Detaches only the matching handler, not a newer registration. */
  @Synchronized
  fun detach(callback: (String, String) -> Unit) {
    if (handler === callback) handler = null
  }

  /** Whether a running Rust backend has registered the action channel. */
  @Synchronized
  fun isAvailable(): Boolean = handler != null

  /** Returns false when the action is unknown or the backend is unavailable. */
  fun dispatch(action: String, targetDate: String): Boolean {
    if (action != "skip" && action != "restore") return false
    val callback = synchronized(this) { handler } ?: return false
    callback(action, targetDate)
    return true
  }
}
