//! Which version of it this is. `Cargo.toml` holds the number; the page's metadata, its
//! structured data, `llms.txt` and the changelog repeat it, and the tests below fail when one
//! of them falls behind. `scripts/version.sh` moves them all at once.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn page_says_the_same_version() {
        let page = include_str!("../web/index.html");
        assert!(page.contains(&format!("<meta name=\"version\" content=\"{VERSION}\">")), "web/index.html meta version");
        assert!(page.contains(&format!("\"softwareVersion\": \"{VERSION}\"")), "web/index.html softwareVersion");
        assert!(include_str!("../web/llms.txt").contains(&format!("Version {VERSION}.")), "web/llms.txt");
    }

    #[test]
    fn changelog_leads_with_this_version() {
        let log = include_str!("../CHANGELOG.md");
        let first = log.lines().find(|l| l.starts_with("## [") && !l.starts_with("## [Unreleased]"));
        assert_eq!(first.and_then(|l| l.split(['[', ']']).nth(1)), Some(VERSION), "CHANGELOG.md newest entry");
        assert!(log.contains(&format!("\n[{VERSION}]: ")), "CHANGELOG.md link for {VERSION}");
    }
}
