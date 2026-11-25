# The Rust side finds the plugin class by name and the bridge reads its annotations.
-keep @app.tauri.annotation.TauriPlugin class * { *; }
-keepclassmembers class * {
    @app.tauri.annotation.Command <methods>;
}
-keep @app.tauri.annotation.InvokeArg class * { *; }
