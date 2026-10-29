//! Which version of it this is. `Cargo.toml` holds the number; the page's metadata, its
//! structured data, `llms.txt` and the changelog repeat it, and the tests below fail when one
//! of them falls behind. `scripts/version.sh` moves the numbers all at once; the changelog entry
//! is written by hand. `devlog/` has one post for each day something was released, covering
//! every version that day; the tests fail once a day has gone by without one.

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

    /// From this day on the devlog is one post a day, `devlog/YYYY-MM-DD.md`; before it, one a version.
    const DAILY_FROM: &str = "2026-10-29";
    const DEVLOG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/devlog");
    const MONTHS: [&str; 12] =
        ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"];

    /// Every version in the changelog with the day it was released, newest first.
    fn released() -> Vec<(&'static str, &'static str)> {
        let log = include_str!("../CHANGELOG.md");
        log.lines().filter_map(|l| l.strip_prefix("## [")?.split_once("] - ")).map(|(v, d)| (v, d.trim())).collect()
    }

    /// A daily post's line under its title: `*29 october 2026 · versions 1.11.0 to 1.16.5*`.
    fn post_line(day: &str) -> Option<String> {
        let post = std::fs::read_to_string(format!("{DEVLOG}/{day}.md")).ok()?;
        post.lines().find(|l| l.starts_with('*')).map(str::to_owned)
    }

    /// `2026-10-29` as a post says it: `29 october 2026`.
    fn spoken(day: &str) -> String {
        let n: Vec<usize> = day.split('-').filter_map(|x| x.parse().ok()).collect();
        format!("{} {} {}", n[2], MONTHS[n[1] - 1], n[0])
    }

    #[test]
    fn every_day_before_this_one_has_its_post() {
        let all = released();
        let today = all.iter().find(|r| r.0 == VERSION).map(|r| r.1).expect("CHANGELOG.md has this version");
        let mut days: Vec<&str> = Vec::new();
        for &(v, day) in &all {
            if day < DAILY_FROM || day >= today || days.contains(&day) {
                continue;
            }
            // Newest first, so this is the last version that day: the post has to reach it.
            days.push(day);
            let line = post_line(day).unwrap_or_else(|| panic!("devlog/{day}.md: something was released that day and has no post (devlog/README.md)"));
            assert!(line.ends_with(&format!(" {v}*")), "devlog/{day}.md ends at the wrong version, {v} was the last that day: {line}");
        }
    }

    #[test]
    fn daily_posts_say_their_day_and_versions() {
        let all = released();
        for e in std::fs::read_dir(DEVLOG).unwrap().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(day) = name.strip_suffix(".md").filter(|d| d.len() == 10 && d.as_bytes()[4] == b'-') else { continue };
            let line = post_line(day).unwrap_or_else(|| panic!("devlog/{name} has no *day · versions* line"));
            let rest = line.strip_prefix(&format!("*{} · ", spoken(day))).unwrap_or_else(|| panic!("devlog/{name} names another day: {line}"));
            let named: Vec<&str> = rest.trim_end_matches('*').split(' ').filter(|w| w.starts_with(|c: char| c.is_ascii_digit())).collect();
            assert!(!named.is_empty(), "devlog/{name} names no version: {line}");
            for v in named {
                let r = all.iter().find(|r| r.0 == v).unwrap_or_else(|| panic!("devlog/{name}: {v} is not in CHANGELOG.md"));
                assert!(r.1 <= day, "devlog/{name}: {v} came out after that day");
            }
        }
    }
}
