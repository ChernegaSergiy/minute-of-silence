package ua.pp.khvylyna.MinuteOfSilence

import android.os.Bundle
import android.view.View
import androidx.activity.enableEdgeToEdge
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    CeremonyDevice.install(this)
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    val content = findViewById<View>(android.R.id.content)
    val safeAreaTypes = WindowInsetsCompat.Type.systemBars() or
      WindowInsetsCompat.Type.displayCutout()
    val imeType = WindowInsetsCompat.Type.ime()

    ViewCompat.setOnApplyWindowInsetsListener(content) { view, windowInsets ->
      val safeArea = windowInsets.getInsets(safeAreaTypes)
      val imeBottom = windowInsets.getInsets(imeType).bottom
      view.setPadding(
        safeArea.left,
        safeArea.top,
        safeArea.right,
        maxOf(safeArea.bottom, imeBottom)
      )

      WindowInsetsCompat.Builder(windowInsets)
        .setInsets(safeAreaTypes, Insets.NONE)
        .build()
    }
    ViewCompat.requestApplyInsets(content)
  }
}
