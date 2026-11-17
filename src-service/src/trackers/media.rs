//! Media player title cleanup (Phase 3).
//!
//! Several media players leave a generic title ("VLC media player") or append
//! branding to the playing filename. This normalizes the stored window title so
//! reports show the actual movie/video name.

/// Known media players and the branding suffix they append.
const MEDIA_BRANDING: &[(&str, &str)] = &[
    ("vlc.exe", "- VLC media player"),
    ("mpc-hc.exe", "- Media Player Classic"),
    ("mpc-be.exe", "- Media Player Classic - BE"),
    ("potplayer.exe", "- PotPlayer"),
    ("wmplayer.exe", "- Windows Media Player"),
    ("mpv.exe", ""), // mpv uses the file path in the title
];

/// Strip media-player branding from a window title.
pub fn clean_media_title(process_name: &str, title: &str) -> String {
    let p = process_name.to_ascii_lowercase();
    let mut cleaned = title.trim().to_string();
    for (name, suffix) in MEDIA_BRANDING {
        if p.ends_with(name) && !suffix.is_empty() {
            if let Some(stripped) = cleaned.strip_suffix(suffix) {
                cleaned = stripped.trim().to_string();
                break;
            }
        }
    }
    // mpv: strip the full path, keep the file stem.
    if p.ends_with("mpv.exe") {
        if let Some(file) = cleaned.rsplit(['/', '\\']).next() {
            cleaned = file.trim().to_string();
        }
        // strip extension
        if let Some(dot) = cleaned.rfind('.') {
            cleaned.truncate(dot);
        }
    }
    if cleaned.is_empty() {
        title.to_string()
    } else {
        cleaned
    }
}
