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
  private var requestedOverlayId: String? = null
  private var readyOverlayId: String? = null

  override fun onNewIntent(intent: Intent) {
    setIntent(intent)
    super.onNewIntent(intent)
    if (intent.getStringExtra(CeremonyNotifications.EXTRA_OVERLAY_ID) != requestedOverlayId) {
      setLockScreenOverlayVisible(false)
    }
    showReadyOverlay()
  }

  override fun onWebViewCreate(webView: WebView) {
    ceremonyWebView = webView
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    CeremonyDevice.install(this)
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

  fun requestCeremonyOverlay(ceremonyId: String) {
    if (!isKeyguardLocked()) return
    requestedOverlayId = ceremonyId
    ceremonyWebView?.settings?.offscreenPreRaster = true
    CeremonyNotifications.showCeremonyOverlay(this, ceremonyId)
  }

  fun markCeremonyOverlayReady(ceremonyId: String) {
    readyOverlayId = ceremonyId
    showReadyOverlay()
  }

  fun endCeremonyOverlay(ceremonyId: String) {
    if (readyOverlayId == ceremonyId) readyOverlayId = null
    if (requestedOverlayId != ceremonyId) return
    requestedOverlayId = null
    intent.removeExtra(CeremonyNotifications.EXTRA_OVERLAY_ID)
    setLockScreenOverlayVisible(false)
    ceremonyWebView?.settings?.offscreenPreRaster = false
    CeremonyNotifications.finishCeremonyOverlay(this)
  }

  private fun canRevealOverlay(ceremonyId: String): Boolean =
    requestedOverlayId == ceremonyId &&
      readyOverlayId == ceremonyId &&
      intent.getStringExtra(CeremonyNotifications.EXTRA_OVERLAY_ID) == ceremonyId &&
      isKeyguardLocked()

  private fun isKeyguardLocked(): Boolean =
    getSystemService(KeyguardManager::class.java)?.isKeyguardLocked == true

  private fun showReadyOverlay() {
    val ceremonyId = requestedOverlayId ?: return
    if (!canRevealOverlay(ceremonyId)) return
    ceremonyWebView?.postVisualStateCallback(0L, object : WebView.VisualStateCallback() {
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
