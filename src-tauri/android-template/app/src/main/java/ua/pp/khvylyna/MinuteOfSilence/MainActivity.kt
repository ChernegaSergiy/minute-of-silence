package ua.pp.khvylyna.MinuteOfSilence

import android.app.KeyguardManager
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.view.View
import android.view.WindowManager
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  private var ceremonyWebView: WebView? = null
  private lateinit var ceremonyDevice: CeremonyDevice
  private var readyOverlayId: String? = null

  override fun onNewIntent(intent: Intent) {
    setIntent(intent)
    super.onNewIntent(intent)
    if (intent.getStringExtra(CeremonyNotifications.EXTRA_OVERLAY_ID) != ceremonyDevice.activeOverlayId) {
      setLockScreenOverlayVisible(false)
    }
    showReadyOverlay()
  }

  override fun onWebViewCreate(webView: WebView) {
    ceremonyWebView = webView
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    ceremonyDevice = CeremonyDevice.install(this)
    ceremonyDevice.attach(this)
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    val content = findViewById<View>(android.R.id.content)
    val safeAreaTypes = WindowInsetsCompat.Type.systemBars() or
      WindowInsetsCompat.Type.displayCutout()

    ViewCompat.setOnApplyWindowInsetsListener(content) { view, windowInsets ->
      val safeArea = windowInsets.getInsets(safeAreaTypes)
      view.setPadding(safeArea.left, safeArea.top, safeArea.right, safeArea.bottom)

      WindowInsetsCompat.Builder(windowInsets)
        .setInsets(safeAreaTypes, Insets.NONE)
        .build()
    }
    ViewCompat.requestApplyInsets(content)
  }

  override fun onDestroy() {
    ceremonyDevice.detach(this)
    super.onDestroy()
  }

  fun markCeremonyOverlayReady(ceremonyId: String) {
    readyOverlayId = ceremonyId
    showReadyOverlay()
  }

  fun endCeremonyOverlay(ceremonyId: String) {
    if (readyOverlayId == ceremonyId) readyOverlayId = null
    if (intent.getStringExtra(CeremonyNotifications.EXTRA_OVERLAY_ID) != ceremonyId) return
    intent.removeExtra(CeremonyNotifications.EXTRA_OVERLAY_ID)
    setLockScreenOverlayVisible(false)
    ceremonyWebView?.settings?.offscreenPreRaster = false
  }

  private fun canRevealOverlay(ceremonyId: String): Boolean =
    ceremonyDevice.activeOverlayId == ceremonyId &&
      readyOverlayId == ceremonyId &&
      intent.getStringExtra(CeremonyNotifications.EXTRA_OVERLAY_ID) == ceremonyId &&
      isKeyguardLocked()

  private fun isKeyguardLocked(): Boolean =
    getSystemService(KeyguardManager::class.java)?.isKeyguardLocked == true

  private fun showReadyOverlay() {
    val ceremonyId = ceremonyDevice.activeOverlayId ?: return
    if (!canRevealOverlay(ceremonyId)) return
    val webView = ceremonyWebView ?: return
    webView.settings.offscreenPreRaster = true
    webView.postVisualStateCallback(0L, object : WebView.VisualStateCallback() {
      override fun onComplete(requestId: Long) {
        if (canRevealOverlay(ceremonyId)) setLockScreenOverlayVisible(true)
      }
    })
  }

  private fun setLockScreenOverlayVisible(showOverlay: Boolean) {
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O_MR1) {
      setShowWhenLocked(showOverlay)
      setTurnScreenOn(showOverlay)
    } else {
      val flags = WindowManager.LayoutParams.FLAG_SHOW_WHEN_LOCKED or
        WindowManager.LayoutParams.FLAG_TURN_SCREEN_ON
      if (showOverlay) window.addFlags(flags) else window.clearFlags(flags)
    }
  }
}
