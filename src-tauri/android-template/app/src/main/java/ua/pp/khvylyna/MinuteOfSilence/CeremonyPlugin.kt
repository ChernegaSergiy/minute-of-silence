package ua.pp.khvylyna.MinuteOfSilence

import android.app.Activity
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Plugin

/**
 * Native half of the ceremony backend. The Rust side registers this class as
 * the `ceremony` plugin and calls its commands through the Tauri bridge.
 */
@TauriPlugin
class CeremonyPlugin(private val activity: Activity) : Plugin(activity)
