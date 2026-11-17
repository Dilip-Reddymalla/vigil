//! Project / workspace extraction from IDE window titles (Phase 6.1).
//!
//! Examples:
//! - `"engine.rs — screentimepc — Visual Studio Code"` → project `screentimepc`
//! - `"Vigil — [c:\repos\vigil] — CLion"` → project `vigil`
//! - `"README.md - myapp - Visual Studio Code"` → project `myapp`

/// Process names that carry a project in their title.
fn is_ide(process: &str) -> bool {
    let p = process.to_ascii_lowercase();
    p.contains("code.exe") || p.contains("cursor.exe") || p.contains("vscodium")
        || p.contains("idea64.exe") || p.contains("idea.exe")
        || p.contains("clion") || p.contains("pycharm") || p.contains("rider")
        || p.contains("webstorm") || p.contains("goland") || p.contains("sublime")
        || p.contains("vim") || p.contains("neovim") || p.contains("notepad++")
}

/// Split a title on the common segment separators.
fn segments(title: &str) -> Vec<String> {
    title
        .split(|c: char| c == '—' || c == '–' || c == '-' || c == '|' || c == '·')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Extract the project name from an IDE title.
pub fn project_from_title(process: &str, title: &str) -> Option<String> {
    if !is_ide(process) {
        return None;
    }
    let parts = segments(title);
    if parts.is_empty() {
        return None;
    }
    // JetBrains style: "Project – file - IDEA" → project is the first segment.
    let lower = title.to_ascii_lowercase();
    let is_jetbrains =
        lower.contains("clion") || lower.contains("pycharm") || lower.contains("rider")
            || lower.contains("webstorm") || lower.contains("goland") || lower.contains("idea");
    if is_jetbrains {
        let first = &parts[0];
        // Skip trailing path-ish tokens; keep the first meaningful name.
        let candidate = first.trim_matches(['[', ']']);
        return if candidate.is_empty() { None } else { Some(candidate.to_string()) };
    }

    // VS Code / Sublime style: "file — project — <app>" → project is second-to-last.
    if parts.len() >= 3 {
        return Some(parts[parts.len() - 2].clone());
    }
    // "file - project" (2 parts) → project is the last.
    if parts.len() == 2 {
        return Some(parts[1].clone());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vs_code_extracts_project() {
        assert_eq!(
            project_from_title("Code.exe", "engine.rs — screentimepc — Visual Studio Code"),
            Some("screentimepc".into())
        );
    }

    #[test]
    fn jetbrains_extracts_project() {
        assert_eq!(
            project_from_title("idea64.exe", "Vigil — [c:\\repos\\vigil] — CLion"),
            Some("Vigil".into())
        );
    }

    #[test]
    fn non_ide_returns_none() {
        assert_eq!(project_from_title("brave.exe", "YouTube - Brave"), None);
    }
}
