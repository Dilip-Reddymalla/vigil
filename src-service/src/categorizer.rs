//! Process-name → category mapping.
//!
//! Phase 1 ships a small static heuristic table. Phase 2+ will add the
//! user-overridable `app_categories` table as the source of truth.

/// Predefined application categories.
pub const CATEGORY_PRODUCTIVITY: &str = "productivity";
pub const CATEGORY_DEVELOPMENT: &str = "development";
pub const CATEGORY_COMMUNICATION: &str = "communication";
pub const CATEGORY_ENTERTAINMENT: &str = "entertainment";
pub const CATEGORY_SOCIAL_MEDIA: &str = "social_media";
pub const CATEGORY_BROWSING: &str = "browsing";
pub const CATEGORY_SYSTEM: &str = "system";
pub const CATEGORY_OTHER: &str = "other";

/// Map a process name to a category (lowercased, best-effort).
pub fn categorize(process_name: &str) -> &'static str {
    let p = process_name.to_ascii_lowercase();
    let known: &[(&str, &str)] = &[
        ("code.exe", CATEGORY_DEVELOPMENT),
        ("devenv.exe", CATEGORY_DEVELOPMENT),
        ("idea64.exe", CATEGORY_DEVELOPMENT),
        ("pycharm64.exe", CATEGORY_DEVELOPMENT),
        ("rider64.exe", CATEGORY_DEVELOPMENT),
        ("docker desktop.exe", CATEGORY_DEVELOPMENT),
        ("git bash.exe", CATEGORY_DEVELOPMENT),
        ("powershell.exe", CATEGORY_DEVELOPMENT),
        ("cmd.exe", CATEGORY_DEVELOPMENT),
        ("outlook.exe", CATEGORY_COMMUNICATION),
        ("teams.exe", CATEGORY_COMMUNICATION),
        ("ms-teams.exe", CATEGORY_COMMUNICATION),
        ("slack.exe", CATEGORY_COMMUNICATION),
        ("discord.exe", CATEGORY_COMMUNICATION),
        ("zoom.exe", CATEGORY_COMMUNICATION),
        ("whatsapp.exe", CATEGORY_COMMUNICATION),
        ("chrome.exe", CATEGORY_BROWSING),
        ("msedge.exe", CATEGORY_BROWSING),
        ("firefox.exe", CATEGORY_BROWSING),
        ("opera.exe", CATEGORY_BROWSING),
        ("brave.exe", CATEGORY_BROWSING),
        ("spotify.exe", CATEGORY_ENTERTAINMENT),
        ("vlc.exe", CATEGORY_ENTERTAINMENT),
        ("netflix.exe", CATEGORY_ENTERTAINMENT),
        ("steam.exe", CATEGORY_ENTERTAINMENT),
        ("spotify.exe", CATEGORY_ENTERTAINMENT),
        ("instagram.exe", CATEGORY_SOCIAL_MEDIA),
        ("facebook.exe", CATEGORY_SOCIAL_MEDIA),
        ("tiktok.exe", CATEGORY_SOCIAL_MEDIA),
        ("x.exe", CATEGORY_SOCIAL_MEDIA),
        ("twitter.exe", CATEGORY_SOCIAL_MEDIA),
        ("linkedin.exe", CATEGORY_SOCIAL_MEDIA),
        ("winword.exe", CATEGORY_PRODUCTIVITY),
        ("excel.exe", CATEGORY_PRODUCTIVITY),
        ("powerpnt.exe", CATEGORY_PRODUCTIVITY),
        ("onenote.exe", CATEGORY_PRODUCTIVITY),
        ("notion.exe", CATEGORY_PRODUCTIVITY),
        ("obsidian.exe", CATEGORY_PRODUCTIVITY),
        ("adobe acrobat.exe", CATEGORY_PRODUCTIVITY),
        ("explorer.exe", CATEGORY_SYSTEM),
    ];

    for (name, category) in known {
        if p == *name || p.ends_with(name) {
            return category;
        }
    }
    CATEGORY_OTHER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_processes() {
        assert_eq!(categorize("Code.exe"), CATEGORY_DEVELOPMENT);
        assert_eq!(categorize("chrome.exe"), CATEGORY_BROWSING);
        assert_eq!(categorize("WINWORD.EXE"), CATEGORY_PRODUCTIVITY);
        assert_eq!(categorize("explorer.exe"), CATEGORY_SYSTEM);
    }

    #[test]
    fn defaults_to_other() {
        assert_eq!(categorize("someapp.exe"), CATEGORY_OTHER);
    }
}
