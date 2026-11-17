//! Domain-level category intelligence (Phase 5).
//!
//! Assigns a category to browser domains so productivity/focus metrics reflect the
//! *actual site* (GitHub → development, Kick/YouTube → entertainment) rather than
//! blanket browser time. Seeded defaults + user overrides in `domain_categories`.

use rusqlite::Connection;

pub const CAT_DEVELOPMENT: &str = "development";
pub const CAT_PRODUCTIVITY: &str = "productivity";
pub const CAT_COMMUNICATION: &str = "communication";
pub const CAT_EDUCATION: &str = "education";
pub const CAT_ENTERTAINMENT: &str = "entertainment";
pub const CAT_SOCIAL: &str = "social_media";
pub const CAT_BROWSING: &str = "browsing";
pub const CAT_OTHER: &str = "other";

/// Categories that count toward the productivity score.
pub const PRODUCTIVE_CATEGORIES: &[&str] = &[
    CAT_DEVELOPMENT,
    CAT_PRODUCTIVITY,
    CAT_COMMUNICATION,
    CAT_EDUCATION,
];

/// Default domain → category map.
const SEED: &[(&str, &str)] = &[
    ("github.com", CAT_DEVELOPMENT),
    ("gitlab.com", CAT_DEVELOPMENT),
    ("stackoverflow.com", CAT_DEVELOPMENT),
    ("crates.io", CAT_DEVELOPMENT),
    ("leetcode.com", CAT_DEVELOPMENT),
    ("docs.rs", CAT_DEVELOPMENT),
    ("docs.google.com", CAT_PRODUCTIVITY),
    ("notion.so", CAT_PRODUCTIVITY),
    ("figma.com", CAT_PRODUCTIVITY),
    ("linear.app", CAT_PRODUCTIVITY),
    ("trello.com", CAT_PRODUCTIVITY),
    ("coursera.org", CAT_EDUCATION),
    ("udemy.com", CAT_EDUCATION),
    ("wikipedia.org", CAT_EDUCATION),
    ("arxiv.org", CAT_EDUCATION),
    ("youtube.com", CAT_ENTERTAINMENT),
    ("kick.com", CAT_ENTERTAINMENT),
    ("twitch.tv", CAT_ENTERTAINMENT),
    ("netflix.com", CAT_ENTERTAINMENT),
    ("spotify.com", CAT_ENTERTAINMENT),
    ("hulu.com", CAT_ENTERTAINMENT),
    ("disneyplus.com", CAT_ENTERTAINMENT),
    ("x.com", CAT_SOCIAL),
    ("reddit.com", CAT_SOCIAL),
    ("instagram.com", CAT_SOCIAL),
    ("linkedin.com", CAT_SOCIAL),
    ("facebook.com", CAT_SOCIAL),
    ("whatsapp.com", CAT_COMMUNICATION),
    ("discord.com", CAT_COMMUNICATION),
    ("t.me", CAT_COMMUNICATION),
    ("telegram.org", CAT_COMMUNICATION),
    ("outlook.com", CAT_COMMUNICATION),
    ("mail.google.com", CAT_COMMUNICATION),
];

/// Insert default mappings if the table is empty.
pub fn ensure_seeded(conn: &Connection) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM domain_categories", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    for (domain, category) in SEED {
        conn.execute(
            "INSERT OR IGNORE INTO domain_categories (domain, category) VALUES (?1, ?2)",
            rusqlite::params![domain, category],
        )?;
    }
    Ok(())
}

/// Look up a domain's category (exact, then suffix match). Defaults to
/// `browsing`. `web.`/`www.` prefixes are normalized by the caller already.
pub fn category_for_domain(conn: &Connection, domain: &str) -> String {
    let lower = domain.to_ascii_lowercase();
    let mut map: Vec<(String, String)> = Vec::new();
    if let Ok(mut seed) = conn.prepare("SELECT domain, category FROM domain_categories") {
        if let Ok(pairs) = seed.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        }) {
            for p in pairs.flatten() {
                map.push(p);
            }
        }
    }
    for (d, c) in &map {
        if lower == *d || lower.ends_with(&format!(".{d}")) {
            return c.clone();
        }
    }
    CAT_BROWSING.to_string()
}

/// Sum of productive browser time (ms) for a day, using domain categories.
pub fn productive_browser_ms(conn: &Connection, local_date: &str) -> i64 {
    let mut total: i64 = 0;
    let Ok(mut stmt) = conn.prepare(
        "SELECT b.normalized_domain, COALESCE(SUM(s.duration_ms),0)
           FROM browser_visits b
           LEFT JOIN app_sessions s ON s.id = b.session_id
          WHERE b.local_date = ?1
          GROUP BY b.normalized_domain",
    ) else {
        return 0;
    };
    let Ok(rows) = stmt.query_map([local_date], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    }) else {
        return 0;
    };
    for row in rows.flatten() {
        let cat = category_for_domain(conn, &row.0);
        if PRODUCTIVE_CATEGORIES.contains(&cat.as_str()) {
            total += row.1;
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrations::apply(&c).unwrap();
        c
    }

    #[test]
    fn seeds_and_looks_up() {
        let conn = mem();
        ensure_seeded(&conn).unwrap();
        assert_eq!(category_for_domain(&conn, "github.com"), CAT_DEVELOPMENT);
        assert_eq!(category_for_domain(&conn, "kick.com"), CAT_ENTERTAINMENT);
        assert_eq!(category_for_domain(&conn, "reddit.com"), CAT_SOCIAL);
        assert_eq!(category_for_domain(&conn, "unknown.example.net"), CAT_BROWSING);
    }

    #[test]
    fn suffix_match_works() {
        let conn = mem();
        ensure_seeded(&conn).unwrap();
        assert_eq!(category_for_domain(&conn, "docs.github.com"), CAT_DEVELOPMENT);
    }
}
