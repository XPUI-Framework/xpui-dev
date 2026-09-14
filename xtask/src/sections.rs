//! Parts of files that exist in more than one repository and have to agree,
//! where the whole file legitimately differs.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::SIBLINGS;

/// A named slice of a file: the lines from the first one `starts` accepts up
/// to the first one `ends` accepts, minus those `skip` accepts.
struct Section {
    file: &'static str,
    name: &'static str,
    /// Whether a directory below a repository root that declares a
    /// `[workspace]` of its own carries this file too.
    nested: bool,
    starts: fn(&str) -> bool,
    ends: fn(&str) -> bool,
    skip: fn(&str) -> bool,
}

const SECTIONS: [Section; 2] = [
    // The dependency diagram in every README. The `style` line bolds the
    // repository you are in and is the one line meant to differ.
    Section {
        file: "README.md",
        name: "## Where it sits",
        nested: false,
        starts: |l| l == "## Where it sits",
        ends: |l| l.starts_with("## ") && l != "## Where it sits",
        skip: |l| l.trim_start().starts_with("style "),
    },
    // The lint table every workspace root carries. A nested workspace cannot
    // inherit it and needs a copy of its own, so the roots are found rather
    // than listed: a list is a second copy of what the manifests already say,
    // and the copy is what goes stale.
    Section {
        file: "Cargo.toml",
        name: "[workspace.lints]",
        nested: true,
        starts: |l| l.starts_with("[workspace.lints"),
        ends: |l| l.starts_with('[') && !l.starts_with("[workspace.lints"),
        skip: |l| l.trim().is_empty() || l.trim_start().starts_with('#'),
    },
];

/// Whether a manifest declares a workspace of its own.
///
/// The bare table, not `[workspace.lints]` or `[workspace.package]`: those are
/// keys *of* a workspace and appear in a root that has already declared one.
fn declares_a_workspace(text: &str) -> bool {
    text.lines().any(|l| l.trim() == "[workspace]")
}

/// Every directory below one of the ten that is a workspace of its own.
///
/// Found rather than listed. A nested workspace nobody listed is a root
/// nothing compares, and its members then lint under cargo's defaults with
/// every gate green — which a list can never catch, because the root it misses
/// is the root it does not name. `git ls-files` rather than a walk, so
/// `target/` and an untracked scratch clone are excluded for free, and only
/// the ten are read.
fn nested_roots() -> Vec<String> {
    let mut found = Vec::new();
    for repo in SIBLINGS.iter().chain(std::iter::once(&"xpui-dev")) {
        let dir = PathBuf::from("..").join(repo);
        let Ok(listed) = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["ls-files", "*Cargo.toml"])
            .output()
        else {
            continue;
        };
        for rel in String::from_utf8_lossy(&listed.stdout).lines() {
            let Some(parent) = rel.strip_suffix("/Cargo.toml") else {
                continue; // the repository's own root, already counted
            };
            if declares_a_workspace(&fs::read_to_string(dir.join(rel)).unwrap_or_default()) {
                found.push(format!("{repo}/{parent}"));
            }
        }
    }
    found.sort();
    found
}

/// Every copy of each shared section is the same section.
pub fn shared_sections_agree() -> Result<String, String> {
    let mut notes = Vec::new();
    let mut problems = Vec::new();
    let nested = nested_roots();
    for section in &SECTIONS {
        let mut roots: Vec<String> = SIBLINGS.iter().map(|s| (*s).to_string()).collect();
        roots.push("xpui-dev".to_string());
        if section.nested {
            roots.extend(nested.iter().cloned());
        }

        let mut by_content: BTreeMap<String, Vec<&str>> = BTreeMap::new();
        let mut missing = Vec::new();
        for root in &roots {
            let path = PathBuf::from("..").join(root).join(section.file);
            match extract(section, &fs::read_to_string(path).unwrap_or_default()) {
                Some(body) => by_content.entry(body).or_default().push(root.as_str()),
                None => missing.push(root.as_str()),
            }
        }
        let label = format!("{} {}", section.file, section.name);
        if !missing.is_empty() {
            problems.push(format!("  {label} is absent from: {}", missing.join(", ")));
            continue;
        }
        if by_content.len() > 1 {
            problems.push(format!(
                "  {label} has {} different versions:\n{}",
                by_content.len(),
                by_content
                    .values()
                    .map(|group| format!("      {}", group.join(", ")))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
            continue;
        }
        // The nested roots are named, not just counted: a count nobody can
        // check is how the twelfth root went missing.
        let found = if section.nested && !nested.is_empty() {
            format!(", including {}", nested.join(", "))
        } else {
            String::new()
        };
        notes.push(format!(
            "{label:<30} one section, {} copies{found}",
            roots.len()
        ));
    }
    if problems.is_empty() {
        Ok(notes.join("\n"))
    } else {
        Err(format!(
            "{}\n\nEach is one passage with a copy per repository; edit one and copy it\n\
             to the rest.",
            problems.join("\n")
        ))
    }
}

/// The section's lines, or `None` when the file does not have it.
fn extract(section: &Section, text: &str) -> Option<String> {
    let mut lines = text.lines().skip_while(|l| !(section.starts)(l)).peekable();
    lines.peek()?;
    let kept: Vec<&str> = lines
        .take_while(|l| !(section.ends)(l))
        .filter(|l| !(section.skip)(l))
        .collect();
    let end = kept
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .map_or(0, |i| i + 1);
    Some(kept[..end].join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_style_line_is_the_one_difference_a_readme_may_have() {
        let a = "# a\n\n## Where it sits\n\ngraph\n  style a stroke\n```\n\n## License\nMIT\n";
        let b = "# b\n\n## Where it sits\n\ngraph\n  style b stroke\n```\n\n## License\nMIT\n";
        let c = "# c\n\n## Where it sits\n\ngraph\n  c --> a\n```\n\n## License\nMIT\n";
        let readme = &SECTIONS[0];
        assert_eq!(extract(readme, a), extract(readme, b));
        assert_ne!(extract(readme, a), extract(readme, c));
        assert_eq!(extract(readme, "# no such section\n"), None);
    }

    #[test]
    fn a_workspace_key_is_not_a_workspace_declaration() {
        // The trap the discovery rests on: every root carries
        // `[workspace.lints.clippy]`, and a member carries `[lints]
        // workspace = true`. Neither makes a directory a workspace, and
        // counting either would make every crate in the stack a root.
        assert!(declares_a_workspace("[workspace]\nmembers = []\n"));
        assert!(declares_a_workspace(
            "[package]\nname = \"p\"\n\n[workspace]\n"
        ));
        assert!(!declares_a_workspace(
            "[package]\nname = \"p\"\n\n[lints]\nworkspace = true\n"
        ));
        assert!(!declares_a_workspace(
            "[workspace.lints.clippy]\nx = \"deny\"\n"
        ));
        assert!(!declares_a_workspace(
            "[workspace.package]\nversion = \"0\"\n"
        ));
    }

    #[test]
    fn every_nested_root_found_carries_the_table_it_was_found_for() {
        // The discovery and the compare have to agree about what a root is:
        // a directory found here but without the section would be reported
        // as a missing copy rather than as a root nobody should have counted.
        let lints = &SECTIONS[1];
        for root in nested_roots() {
            let path = PathBuf::from("..").join(&root).join(lints.file);
            let text = fs::read_to_string(&path).unwrap_or_default();
            assert!(
                declares_a_workspace(&text),
                "{root} was found but declares no workspace"
            );
            assert!(
                extract(lints, &text).is_some(),
                "{root} is a workspace of its own and carries no {}",
                lints.name
            );
        }
    }

    #[test]
    fn the_lint_table_is_read_without_its_neighbours() {
        let root = "[workspace]\nmembers = []\n\n[workspace.lints.clippy]\nx = \"deny\"\n\n\
                    [workspace.lints.rustdoc]\ny = \"deny\"\n\n# about the profile\n[profile.release]\nlto = true\n";
        let package_first = "[package]\nname = \"p\"\n\n[workspace.lints.clippy]\nx = \"deny\"\n\
                             [workspace.lints.rustdoc]\ny = \"deny\"\n[package.metadata]\n";
        let lints = &SECTIONS[1];
        assert_eq!(extract(lints, root), extract(lints, package_first));
        assert_eq!(
            extract(lints, root).as_deref(),
            Some("[workspace.lints.clippy]\nx = \"deny\"\n[workspace.lints.rustdoc]\ny = \"deny\"")
        );
    }
}
