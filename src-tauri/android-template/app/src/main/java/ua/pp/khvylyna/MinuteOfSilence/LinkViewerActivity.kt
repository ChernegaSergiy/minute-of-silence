package ua.pp.khvylyna.MinuteOfSilence

import android.annotation.SuppressLint
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.net.Uri
import android.net.http.SslError
import android.os.Bundle
import android.view.Gravity
import android.view.ViewGroup
import android.webkit.SslErrorHandler
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.TextView
import androidx.activity.enableEdgeToEdge
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AppCompatActivity
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class LinkViewerActivity : AppCompatActivity() {
  private lateinit var webView: WebView
  private lateinit var pageTitle: TextView
  private lateinit var initialUri: Uri
  private var fallbackStarted = false

  @SuppressLint("SetJavaScriptEnabled")
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    val initialUrl = intent.getStringExtra(EXTRA_URL)
    if (initialUrl.isNullOrBlank()) {
      finish()
      return
    }
    val uri = Uri.parse(initialUrl)
    if (uri.scheme != "https" || uri.host.isNullOrBlank()) {
      finish()
      return
    }
    initialUri = uri

    try {
      val root = LinearLayout(this).apply {
        orientation = LinearLayout.VERTICAL
        setBackgroundColor(Color.WHITE)
      }
      val toolbar = LinearLayout(this).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(8), 0, dp(16), 0)
        setBackgroundColor(Color.WHITE)
      }
      val closeButton = ImageButton(this).apply {
        setImageResource(android.R.drawable.ic_menu_close_clear_cancel)
        contentDescription = getString(android.R.string.cancel)
        setBackgroundColor(Color.TRANSPARENT)
        scaleType = android.widget.ImageView.ScaleType.CENTER
        layoutParams = LinearLayout.LayoutParams(dp(48), dp(56))
        setOnClickListener { finish() }
      }
      pageTitle = TextView(this).apply {
        text = uri.host
        textSize = 16f
        setTypeface(null, Typeface.BOLD)
        setTextColor(Color.BLACK)
        maxLines = 1
        ellipsize = android.text.TextUtils.TruncateAt.END
        gravity = Gravity.CENTER_VERTICAL
        layoutParams = LinearLayout.LayoutParams(0, dp(56), 1f)
      }
      toolbar.addView(closeButton)
      toolbar.addView(pageTitle)
      root.addView(toolbar, LinearLayout.LayoutParams.MATCH_PARENT, dp(56))

      webView = WebView(this)
      webView.settings.apply {
        javaScriptEnabled = true
        domStorageEnabled = true
        allowFileAccess = false
        allowContentAccess = false
        mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
          safeBrowsingEnabled = true
        }
        setSupportMultipleWindows(false)
        javaScriptCanOpenWindowsAutomatically = false
      }
      webView.webChromeClient = object : WebChromeClient() {
        override fun onReceivedTitle(view: WebView?, title: String?) {
          pageTitle.text = title?.takeIf(String::isNotBlank) ?: initialUri.host
        }
      }
      webView.webViewClient = object : WebViewClient() {
        override fun shouldOverrideUrlLoading(view: WebView?, request: WebResourceRequest): Boolean {
          val requestUri = request.url
          if (requestUri.scheme == "https") return false
          openInBrowser(requestUri)
          return true
        }

        override fun onReceivedError(
          view: WebView?,
          request: WebResourceRequest,
          error: android.webkit.WebResourceError
        ) {
          if (request.isForMainFrame) openInBrowser(request.url)
        }

        override fun onReceivedHttpError(
          view: WebView?,
          request: WebResourceRequest,
          response: WebResourceResponse
        ) {
          if (request.isForMainFrame && response.statusCode >= 400) {
            openInBrowser(request.url)
          }
        }

        override fun onReceivedSslError(view: WebView?, handler: SslErrorHandler, error: SslError) {
          handler.cancel()
          openInBrowser(error.url?.let(Uri::parse) ?: initialUri)
        }

        override fun onRenderProcessGone(view: WebView?, detail: android.webkit.RenderProcessGoneDetail): Boolean {
          openInBrowser(initialUri)
          return true
        }
      }
      root.addView(webView, LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        0,
        1f
      ))
      setContentView(root)

      val safeAreaTypes = WindowInsetsCompat.Type.systemBars() or
        WindowInsetsCompat.Type.displayCutout()
      ViewCompat.setOnApplyWindowInsetsListener(root) { view, windowInsets ->
        val safeArea = windowInsets.getInsets(safeAreaTypes)
        view.setPadding(safeArea.left, safeArea.top, safeArea.right, safeArea.bottom)
        WindowInsetsCompat.Builder(windowInsets)
          .setInsets(safeAreaTypes, Insets.NONE)
          .build()
      }
      ViewCompat.requestApplyInsets(root)

      onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
        override fun handleOnBackPressed() {
          if (::webView.isInitialized && webView.canGoBack()) webView.goBack() else finish()
        }
      })
      webView.loadUrl(initialUri.toString())
    } catch (_: Exception) {
      openInBrowser(initialUri)
    }
  }

  override fun onDestroy() {
    if (::webView.isInitialized) {
      webView.stopLoading()
      webView.loadUrl("about:blank")
      webView.removeAllViews()
      webView.destroy()
    }
    super.onDestroy()
  }

  private fun openInBrowser(uri: Uri) {
    if (fallbackStarted || isFinishing) return
    fallbackStarted = true
    try {
      startActivity(Intent(Intent.ACTION_VIEW, uri))
    } catch (_: Exception) {
    } finally {
      finish()
    }
  }

  private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

  companion object {
    const val EXTRA_URL = "link_url"
  }
}
