//! Parts of files that exist in more than one repository and have to agree,
//! where the whole file legitimately differs.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::SIBLINGS;

/// A named slice of a file: the lines from the first one `starts` accepts up
/// to the first one `ends` accepts, minus those `skip` accepts.
struct Section {
    file: &'static str,
    name: &'static str,
    /// Roots beyond the nine and this repository that carry the file.
    extra: &'static [&'static str],
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
        extra: &[],
        starts: |l| l == "## Where it sits",
        ends: |l| l.starts_with("## ") && l != "## Where it sits",
        skip: |l| l.trim_start().starts_with("style "),
    },
    // The lint table every workspace root carries — twelve roots, because two
    // directories in `xpui-rp2040` are workspaces of their own.
    Section {
        file: "Cargo.toml",
        name: "[workspace.lints]",
        extra: &["xpui-rp2040/docs-test", "xpui-rp2040/xtask"],
        starts: |l| l.starts_with("[workspace.lints"),
        ends: |l| l.starts_with('[') && !l.starts_with("[workspace.lints"),
        skip: |l| l.trim().is_empty() || l.trim_start().starts_with('#'),
    },
];

/// Every copy of each shared section is the same section.
pub fn shared_sections_agree() -> Result<String, String> {
    let mut notes = Vec::new();
    let mut problems = Vec::new();
    for section in &SECTIONS {
        let mut roots: Vec<&str> = SIBLINGS.to_vec();
        roots.push("xpui-dev");
        roots.extend(section.extra);

        let mut by_content: BTreeMap<String, Vec<&str>> = BTreeMap::new();
        let mut missing = Vec::new();
        for root in &roots {
            let path = PathBuf::from("..").join(root).join(section.file);
            match extract(section, &fs::read_to_string(path).unwrap_or_default()) {
                Some(body) => by_content.entry(body).or_default().push(root),
                None => missing.push(*root),
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
        notes.push(format!("{label:<30} one section, {} copies", roots.len()));
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
