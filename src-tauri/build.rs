fn main() {
    // Only run the Tauri build script when the `tauri` feature is enabled;
    // keeps the default (GNU) workspace build independent of MSVC/WebView2.
    #[cfg(feature = "tauri")]
    tauri_build::build();
}
