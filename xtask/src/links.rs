//! Absolute links into the organisation, checked against the branch they name.
//!
//! Each repository's own gate reads its *relative* links; nothing else reads
//! a `github.com/XPUI-Framework/…` URL.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::SIBLINGS;

const PREFIX: &str = "https://github.com/XPUI-Framework/";

/// Every organisation URL names a file that is on that repository's `main`.
///
/// Against `origin/main` rather than the working tree: the URL is what a
/// reader on GitHub follows, so what matters is whether it resolves *there*.
/// A file added locally and not yet pushed is a broken link until it is.
pub fn org_links_resolve() -> Result<String, String> {
    let urls = collect();
    if urls.is_empty() {
        // No URLs at all means the search broke, not that the tree is clean.
        return Err("no organisation URLs found anywhere, which cannot be right".into());
    }

    let mut broken = Vec::new();
    let mut trees = std::collections::BTreeMap::new();
    for url in &urls {
        let rest = &url[PREFIX.len()..];
        let Some((repo, path)) = rest.split_once("/main/") else {
            continue;
        };
        let repo = repo.trim_end_matches("/blob").trim_end_matches("/tree");
        // The profile repository is what GitHub shows on the organisation
        // page. It links out to the ten; nothing links into it.
        if repo == ".github" {
            broken.push(format!(
                "  {url}\n      the organisation profile is never linked to, only from"
            ));
            continue;
        }
        // The organisation's `xpui-framework` is the framework crate, and it
        // is checked out here as `xpui`. The directory of that name beside it
        // is the monorepo, on a different remote entirely.
        let dir = if repo == "xpui-framework" {
            "xpui"
        } else {
            repo
        };
        if !Path::new("..").join(dir).join(".git").exists() {
            broken.push(format!(
                "  {url}\n      no checkout of {repo} to check it against"
            ));
            continue;
        }
        let tree = trees.entry(dir.to_string()).or_insert_with(|| {
            let out = Command::new("git")
                .args([
                    "-C",
                    &format!("../{dir}"),
                    "ls-tree",
                    "-r",
                    "--name-only",
                    "origin/main",
                ])
                .output();
            out.map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default()
        });
        let found = tree
            .lines()
            .any(|entry| entry == path || entry.starts_with(&format!("{path}/")));
        if !found {
            broken.push(format!("  {url}"));
        }
    }

    if broken.is_empty() {
        Ok(format!("{} URLs, all resolved", urls.len()))
    } else {
        Err(format!(
            "{}\n\n{} URL(s) above name nothing on that repository's main branch. A\n\
             404 in a README is worse than a relative path, because nothing but\n\
             this line reads it.",
            broken.join("\n"),
            broken.len()
        ))
    }
}

/// Every `blob`/`tree` URL into the organisation, across the nine and this
/// repository.
fn collect() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for repo in SIBLINGS.iter().chain(std::iter::once(&"xpui-dev")) {
        walk(&Path::new("..").join(repo), &mut found);
    }
    found
}

fn walk(dir: &Path, found: &mut BTreeSet<String>) {
    const PRUNED: [&str; 4] = ["target", ".git", ".pio", "freeink-sdk"];
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if PRUNED.iter().any(|p| entry.file_name() == *p) {
            continue;
        }
        if path.is_dir() {
            walk(&path, found);
            continue;
        }
        let interesting = path
            .extension()
            .is_some_and(|x| x == "md" || x == "rs" || x == "toml");
        if !interesting {
            continue;
        }
        for url in urls_in(&fs::read_to_string(&path).unwrap_or_default()) {
            found.insert(url);
        }
    }
}

/// The organisation URLs in one file's text.
///
/// A URL ends at whitespace, a closing parenthesis or a `#` anchor: a
/// markdown link's `)` is not part of the address, and an anchor is a claim
/// about a heading rather than a file.
fn urls_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(PREFIX) {
        let tail = &rest[at..];
        let end = tail
            .find([')', '#', ' ', '\n', '\t', '"', '`', '<', '>'])
            .unwrap_or(tail.len());
        let url = &tail[..end];
        if url.contains("/blob/main/") || url.contains("/tree/main/") {
            out.push(url.trim_end_matches(['.', ',']).to_string());
        }
        rest = &tail[end.max(1)..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_markdown_link_stops_at_the_parenthesis() {
        let found = urls_in(
            "see [it](https://github.com/XPUI-Framework/xpui-chrome/blob/main/src/lib.rs) here",
        );
        assert_eq!(
            found,
            ["https://github.com/XPUI-Framework/xpui-chrome/blob/main/src/lib.rs"]
        );
    }

    #[test]
    fn an_anchor_is_a_claim_about_a_heading_and_is_cut() {
        let found = urls_in("https://github.com/XPUI-Framework/xpui/blob/main/README.md#install");
        assert_eq!(
            found,
            ["https://github.com/XPUI-Framework/xpui/blob/main/README.md"]
        );
    }

    #[test]
    fn a_repository_root_url_is_not_a_file_claim() {
        // `github.com/XPUI-Framework/xpui-chrome` names a repository, not a
        // path on a branch, and there is nothing to resolve.
        assert!(urls_in("https://github.com/XPUI-Framework/xpui-chrome").is_empty());
    }

    #[test]
    fn two_urls_on_one_line_are_both_found() {
        // Built at run time: this file is walked too, and a literal here would
        // be a claim about a repository that does not exist.
        let text = format!("{PREFIX}a/blob/main/x.rs and {PREFIX}b/tree/main/y");
        assert_eq!(urls_in(&text).len(), 2);
    }
}
