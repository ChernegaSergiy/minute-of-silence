package ua.pp.khvylyna.MinuteOfSilence

import android.app.Activity
import android.content.Intent
import android.net.Uri
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
class LinkViewerPlugin(private val activity: Activity) : Plugin(activity) {
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
          activity.startActivity(
            Intent(activity, LinkViewerActivity::class.java)
              .putExtra(LinkViewerActivity.EXTRA_URL, uri.toString())
          )
          invoke.resolve()
        } catch (error: Exception) {
          invoke.reject("Failed to open the in-app web view: ${error.message}")
        }
      }
    } catch (error: Exception) {
      invoke.reject("Invalid web link: ${error.message}")
    }
  }
}
