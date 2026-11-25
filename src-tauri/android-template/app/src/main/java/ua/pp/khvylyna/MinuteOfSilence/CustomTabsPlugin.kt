package ua.pp.khvylyna.MinuteOfSilence

import android.app.Activity
import android.net.Uri
import android.widget.Toast
import androidx.browser.customtabs.CustomTabsIntent
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@InvokeArg
class OpenLinkArgs {
  lateinit var url: String
}

@TauriPlugin
class CustomTabsPlugin(private val activity: Activity) : Plugin(activity) {
  @Command
  fun openLink(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(OpenLinkArgs::class.java)
      val uri = Uri.parse(args.url)
      require(uri.scheme == "https" && !uri.host.isNullOrBlank()) {
        "Only secure web links can be opened"
      }

      activity.runOnUiThread {
        try {
          CustomTabsIntent.Builder()
            .setShowTitle(true)
            .build()
            .launchUrl(activity, uri)
          invoke.resolve()
        } catch (error: Exception) {
          Toast.makeText(activity, R.string.about_link_open_failed, Toast.LENGTH_SHORT).show()
          invoke.reject("Failed to open the link: ${error.message}")
        }
      }
    } catch (error: Exception) {
      invoke.reject("Invalid web link: ${error.message}")
    }
  }
}
