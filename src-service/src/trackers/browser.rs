//! Browser adapter framework + Chromium best-effort extraction.
//!
//! Full per-tab UI-Automation URL extraction is a later hardening step; this module
//! provides the adapter contract, domain normalization, and a safe title-based
//! heuristic (domain-level by default, per the privacy decision).

/// A normalized browser URL / domain.
#[derive(Debug, Clone)]
pub struct BrowserUrl {
    pub domain: String,
    pub normalized_domain: String,
    pub url: Option<String>,
    pub title: String,
}

/// Normalize a host/domain: strip scheme, path, port, `www.`/`web.`, and reject
/// malformed fragments (e.g. a trailing-dot or one-letter "domain" from titles).
pub fn normalize_domain(input: &str) -> Option<String> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    let rest = match s.find("://") {
        Some(pos) => &s[pos + 3..],
        None => s,
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.split(':').next().unwrap_or(host).trim();
    let host = host.strip_prefix("www.").unwrap_or(host);
    let host = host.strip_prefix("web.").unwrap_or(host);

    // Must look like a real domain: contains a dot, no trailing dot, and the last
    // label is at least 2 chars (rejects fragments like "ft.").
    if !host.contains('.') || host.ends_with('.') {
        return None;
    }
    let last_label = host.rsplit('.').next().unwrap_or("");
    if last_label.len() < 2 {
        return None;
    }
    // Reject numeric/placeholder junk.
    if host.chars().all(|c| !c.is_alphabetic()) {
        return None;
    }

    Some(host.to_lowercase())
}

/// Known site names → canonical domains (used when the title has no literal URL).
/// All entries are unique (deduplicated).
const BRAND_DOMAINS: &[(&str, &str)] = &[
    ("youtube", "youtube.com"),
    ("netflix", "netflix.com"),
    ("spotify", "spotify.com"),
    ("wikipedia", "wikipedia.org"),
    ("reddit", "reddit.com"),
    ("github", "github.com"),
    ("twitch", "twitch.tv"),
    ("facebook", "facebook.com"),
    ("instagram", "instagram.com"),
    ("linkedin", "linkedin.com"),
    ("amazon", "amazon.com"),
    ("stack overflow", "stackoverflow.com"),
    ("google", "google.com"),
    ("gmail", "mail.google.com"),
    ("twitter", "x.com"),
    ("whatsapp", "whatsapp.com"),
    ("discord", "discord.com"),
    ("kick", "kick.com"),
    ("prime video", "primevideo.com"),
    ("hulu", "hulu.com"),
    ("disney", "disneyplus.com"),
    ("crunchyroll", "crunchyroll.com"),
    ("soundcloud", "soundcloud.com"),
    ("medium", "medium.com"),
    ("dev.to", "dev.to"),
    ("mdn", "developer.mozilla.org"),
    ("gitlab", "gitlab.com"),
    ("quora", "quora.com"),
    ("stackexchange", "stackexchange.com"),
    ("coursera", "coursera.org"),
    ("udemy", "udemy.com"),
    ("chatgpt", "chatgpt.com"),
    ("claude", "claude.ai"),
    ("gemini", "gemini.google.com"),
    ("notion", "notion.so"),
    ("figma", "figma.com"),
    ("canva", "canva.com"),
    ("overleaf", "overleaf.com"),
];

/// Try to find a literal domain-like token in the title.
fn literal_domain(title: &str) -> Option<String> {
    for tok in title.split(|c: char| c.is_whitespace() || c == '-' || c == '|') {
        if tok.contains('.') && !tok.contains("..") {
            if let Some(d) = normalize_domain(tok) {
                return Some(d);
            }
        }
    }
    None
}

/// Best-effort domain extraction from a browser window title.
fn extract_domain_from_title(title: &str) -> Option<String> {
    let t = title
        .replace(" - Google Chrome", "")
        .replace(" - Microsoft Edge", "")
        .replace(" - Mozilla Firefox", "");

    if let Some(d) = literal_domain(&t) {
        return Some(d);
    }

    let lower = t.to_lowercase();
    for (brand, domain) in BRAND_DOMAINS {
        if lower.contains(brand) {
            return Some((*domain).to_string());
        }
    }

    None
}

/// A browser adapter handles one or more browser processes.
pub trait BrowserAdapter: Send + Sync {
    fn browser_name(&self) -> &'static str;
    fn process_names(&self) -> &'static [&'static str];

    fn is_browser_process(&self, name: &str) -> bool {
        self.process_names().iter().any(|p| name.eq_ignore_ascii_case(p))
    }

    fn extract_url(&self, title: &str) -> Option<BrowserUrl>;

    /// Extract a URL with the window handle available (lets Chromium read the
    /// address bar via UI Automation, C7). Defaults to title-only extraction.
    fn extract_url_with_hwnd(&self, title: &str, _hwnd: isize) -> Option<BrowserUrl> {
        self.extract_url(title)
    }

    fn is_incognito(&self, title: &str) -> bool;
}

/// Shared Chromium adapter (Chrome + Edge).
pub struct ChromiumAdapter {
    /// Simple LRU cache: `(hwnd, cached_domain)` with a timestamp so we don't
    /// hammer UI Automation on every title change.
    cache: std::sync::Mutex<Option<(isize, String, std::time::Instant)>>,
}

impl Default for ChromiumAdapter {
    fn default() -> Self {
        Self { cache: std::sync::Mutex::new(None) }
    }
}

impl BrowserAdapter for ChromiumAdapter {
    fn browser_name(&self) -> &'static str {
        "chromium"
    }

    fn process_names(&self) -> &'static [&'static str] {
        &["chrome.exe", "msedge.exe", "brave.exe", "opera.exe", "vivaldi.exe"]
    }

    fn extract_url(&self, title: &str) -> Option<BrowserUrl> {
        let domain = extract_domain_from_title(title)?;
        Some(BrowserUrl {
            domain: domain.clone(),
            normalized_domain: domain,
            url: None,
            title: title.to_string(),
        })
    }

    fn extract_url_with_hwnd(&self, title: &str, hwnd: isize) -> Option<BrowserUrl> {
        if hwnd == 0 {
            return self.extract_url(title);
        }
        // LRU cache (500 ms) to avoid repeated UIA queries.
        if let Ok(mut cache) = self.cache.lock() {
            if let Some((hw, domain, at)) = cache.as_ref() {
                if *hw == hwnd && at.elapsed() < std::time::Duration::from_millis(500) {
                    return Some(BrowserUrl {
                        domain: domain.clone(),
                        normalized_domain: domain.clone(),
                        url: None,
                        title: title.to_string(),
                    });
                }
            }
            // Try UIA address-bar extraction first.
            if let Some(raw) = crate::trackers::uia::read_address_bar(
                windows::Win32::Foundation::HWND(hwnd as *mut core::ffi::c_void),
            ) {
                if let Some(domain) = normalize_domain(&raw) {
                    *cache = Some((hwnd, domain.clone(), std::time::Instant::now()));
                    return Some(BrowserUrl {
                        domain: domain.clone(),
                        normalized_domain: domain,
                        url: Some(raw),
                        title: title.to_string(),
                    });
                }
            }
        }
        // Fall back to title heuristic.
        self.extract_url(title)
    }

    fn is_incognito(&self, title: &str) -> bool {
        title.contains("Incognito") || title.contains("InPrivate")
    }
}

/// Pick an adapter for a process name.
pub fn adapter_for_process(name: &str) -> Option<Box<dyn BrowserAdapter>> {
    let adapters: Vec<Box<dyn BrowserAdapter>> = vec![Box::new(ChromiumAdapter::default())];
    adapters.into_iter().find(|a| a.is_browser_process(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_scheme_path_port_www() {
        assert_eq!(normalize_domain("https://www.example.com/path?q=1"), Some("example.com".into()));
        assert_eq!(normalize_domain("http://EXAMPLE.com:8080"), Some("example.com".into()));
        assert_eq!(normalize_domain("example.com"), Some("example.com".into()));
        assert_eq!(normalize_domain("  "), None);
    }

    #[test]
    fn chromium_extracts_domain_from_title() {
        let a = ChromiumAdapter::default();
        // Works when a domain-like token is present in the title.
        let url = a.extract_url("example.com - Home - Google Chrome").unwrap();
        assert_eq!(url.normalized_domain, "example.com");
        // Brand mapping when no literal URL is present.
        assert_eq!(
            a.extract_url("Never Gonna Give You Up - YouTube - Google Chrome").unwrap().normalized_domain,
            "youtube.com"
        );
        assert_eq!(
            a.extract_url("Stack Overflow - Where developers learn - Google Chrome").unwrap().normalized_domain,
            "stackoverflow.com"
        );
        assert!(a.is_incognito("New tab - Incognito - Google Chrome"));
        assert!(!a.is_incognito("Normal tab - Google Chrome"));
    }
}
