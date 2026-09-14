//! The files that exist in more than one repository, and have to agree.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::SIBLINGS;

/// Carried by the nine and this repository, and compared across all ten — a
/// lint setting that drifts in the umbrella is still drift.
const SHARED: [&str; 2] = ["LICENSE", "clippy.toml"];

/// The thirteen modules every sibling's `xtask` carries.
///
/// Not `main.rs`, which is each repository's own list of checks and is meant
/// to differ. These hold the reading of a markdown fence, a manifest, a path
/// and a comment, which is the same job everywhere.
///
/// The nine only: this repository's gate is the cross-repository half and
/// shares no checks with them.
const XTASK: [&str; 13] = [
    "xtask/src/agents.rs",
    "xtask/src/commands.rs",
    "xtask/src/comments.rs",
    "xtask/src/docs.rs",
    "xtask/src/faults.rs",
    "xtask/src/fences.rs",
    "xtask/src/pages.rs",
    "xtask/src/paths.rs",
    "xtask/src/prose.rs",
    "xtask/src/readme.rs",
    "xtask/src/reference.rs",
    "xtask/src/rustdoc.rs",
    "xtask/src/tree.rs",
];

/// Files carried by a named subset, and the subset that carries each.
///
/// A file may be listed more than once with disjoint subsets: `cargo.rs` has
/// three shapes, and each shape is one file across the repositories that hold
/// it.
const GROUPS: &[(&str, &[&str])] = &[
    // The C++ half of the gate, in the two repositories that hold C++.
    ("xtask/src/cpp.rs", &["xpui-backends", "xpui-cpp"]),
    (
        "xtask/src/cargo.rs",
        &[
            "xpui",
            "xpui-chrome",
            "xpui-boards",
            "xpui-backends",
            "xpui-gallery",
            "xpui-esp32",
        ],
    ),
    // With `host_triple`: `.cargo/config.toml` there targets the board.
    ("xtask/src/cargo.rs", &["xpui-rp2040"]),
    // Without `target_installed`: neither has a bare-metal lint.
    ("xtask/src/cargo.rs", &["xpui-simulator", "xpui-cpp"]),
    // `xpui-cpp`'s is a superset with the PlatformIO lines, and there is no
    // superset mode here, so it is left out.
    (
        ".gitignore",
        &[
            "xpui",
            "xpui-chrome",
            "xpui-boards",
            "xpui-backends",
            "xpui-simulator",
            "xpui-gallery",
            "xpui-rp2040",
            "xpui-esp32",
            "xpui-dev",
        ],
    ),
    // The three `all`-mode repositories and this one take other arguments.
    (
        "build-and-test.sh",
        &[
            "xpui",
            "xpui-chrome",
            "xpui-boards",
            "xpui-backends",
            "xpui-simulator",
            "xpui-gallery",
        ],
    ),
];

/// The community files and the two reviewer agents, carried by the nine and
/// this repository.
const TEN_ONLY: [&str; 8] = [
    "SECURITY.md",
    "CODE_OF_CONDUCT.md",
    ".github/ISSUE_TEMPLATE/bug.yml",
    ".github/ISSUE_TEMPLATE/feature.yml",
    ".github/PULL_REQUEST_TEMPLATE.md",
    ".github/dependabot.yml",
    ".claude/agents/code-reviewer.md",
    ".claude/agents/docs-reviewer.md",
];

/// Every repository carries the same copy of each shared file.
///
/// There is no submodule and nothing is published, so these files are
/// **copied**. A copy nobody compares is a fork with a delay on it.
pub fn shared_files_agree() -> Result<String, String> {
    let roots: Vec<String> = SIBLINGS.iter().map(|s| s.to_string()).collect();
    let nine: Vec<String> = SIBLINGS.iter().map(|s| s.to_string()).collect();
    let mut ten = nine.clone();
    ten.push("xpui-dev".into());

    let mut notes = Vec::new();
    let mut problems = Vec::new();

    let mut compare = |file: &str, roots: &[String], named: bool| {
        let mut here = roots.to_vec();
        here.sort();
        let mut by_content: BTreeMap<Vec<u8>, Vec<String>> = BTreeMap::new();
        let mut missing = Vec::new();
        for root in &here {
            match fs::read(PathBuf::from("..").join(root).join(file)) {
                Ok(bytes) => by_content.entry(bytes).or_default().push(root.clone()),
                Err(_) => missing.push(root.clone()),
            }
        }
        if !missing.is_empty() {
            problems.push(format!("  {file} is absent from: {}", missing.join(", ")));
        } else if by_content.len() > 1 {
            problems.push(format!(
                "  {file} has {} different versions:\n{}",
                by_content.len(),
                by_content
                    .values()
                    .map(|group| format!("      {}", group.join(", ")))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        } else if named {
            notes.push(format!("{file:<22} one file across {}", here.join(", ")));
        } else {
            notes.push(format!("{file:<22} one file, {} copies", here.len()));
        }
    };

    for file in SHARED {
        compare(file, &ten, false);
    }
    for file in XTASK {
        compare(file, &nine, false);
    }
    for (file, only) in GROUPS {
        let only: Vec<String> = only.iter().map(|s| (*s).to_string()).collect();
        let repeated = GROUPS.iter().filter(|(name, _)| name == file).count() > 1;
        compare(file, &only, repeated);
    }
    for file in TEN_ONLY {
        compare(file, &ten, false);
    }

    // The toolchain: the same compiler everywhere, whatever targets each
    // installs. Only the `channel` line has to agree — a `targets` list
    // legitimately differs per repository.
    let mut channels: Vec<(String, String)> = Vec::new();
    for root in roots.iter().chain(std::iter::once(&"xpui-dev".to_string())) {
        let text = fs::read_to_string(PathBuf::from("..").join(root).join("rust-toolchain.toml"))
            .unwrap_or_default();
        // Root-qualified when it is missing. A bare "MISSING" makes ten
        // absent files collapse into one distinct value, and the check then
        // reports agreement about a file that exists nowhere.
        let channel = match text.lines().find(|l| l.trim_start().starts_with("channel")) {
            Some(line) => line.trim().to_string(),
            None => format!("MISSING in {root}"),
        };
        channels.push((channel, root.clone()));
    }
    let distinct: Vec<&String> = {
        let mut seen: Vec<&String> = channels.iter().map(|(c, _)| c).collect();
        seen.sort();
        seen.dedup();
        seen
    };
    if distinct.len() == 1 {
        notes.push(format!("{:<22} {}", "rust-toolchain.toml", distinct[0]));
    } else {
        problems.push(format!(
            "  rust-toolchain.toml names more than one channel:\n{}",
            channels
                .iter()
                .map(|(c, r)| format!("      {r}: {c}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    if problems.is_empty() {
        Ok(notes.join("\n"))
    } else {
        Err(format!(
            "{}\n\nEach is one file with a copy per repository; edit one and copy it\n\
             to the rest.",
            problems.join("\n")
        ))
    }
}

/// Both repositories pin the same FreeInk SDK revision.
///
/// `xpui-backends` compiles the shim against the SDK's headers and `xpui-cpp`
/// links it. A revision written down twice is one that will disagree with
/// itself, and the error surfaces at the link with no hint that a revision is
/// the cause.
pub fn sdk_revisions_agree() -> Result<String, String> {
    const PINS: [&str; 2] = [
        "../xpui-backends/fui/freeink-sdk.rev",
        "../xpui-cpp/cpp_host/freeink-sdk.rev",
    ];
    let mut read = Vec::new();
    for pin in PINS {
        match fs::read_to_string(pin) {
            Ok(text) => read.push((pin, text.trim().to_string())),
            Err(_) => {
                return Err(format!(
                    "  {pin} is not there\n\nA repository that compiles against the SDK and pins no revision\n\
                     would have its CI clone whatever main happens to be."
                ));
            }
        }
    }
    if read[0].1 != read[1].1 {
        return Err(format!(
            "  {}: {}\n  {}: {}\n\nThe shim is syntax-checked against one SDK and linked against\n\
             another, and the error surfaces at the link.",
            read[0].0, read[0].1, read[1].0, read[1].1
        ));
    }
    Ok(read[0].1.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_absent_everywhere_is_not_agreement() {
        // The value `shared_files_agree` records for a missing `channel` line.
        let absent: Vec<String> = ["a", "b", "c"]
            .into_iter()
            .map(|root| format!("MISSING in {root}"))
            .collect();
        let mut distinct = absent.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            3,
            "three absences must read as three, not one"
        );
    }

    #[test]
    fn the_lists_name_no_file_twice() {
        // A file in two lists would be compared against two different sets of
        // roots, and the second answer would be the one printed.
        for file in XTASK.iter().chain(TEN_ONLY.iter()) {
            assert!(!SHARED.contains(file), "{file} is in two lists");
            assert!(
                !GROUPS.iter().any(|(name, _)| name == file),
                "{file} is in two lists"
            );
        }
    }

    #[test]
    fn a_file_in_several_groups_is_in_each_root_once() {
        // `cargo.rs` in three shapes: a root in two of them would be held to
        // two different files.
        for (file, roots) in GROUPS {
            for root in *roots {
                let holders = GROUPS
                    .iter()
                    .filter(|(name, others)| name == file && others.contains(root))
                    .count();
                assert_eq!(holders, 1, "{root} carries {file} in {holders} groups");
            }
        }
    }
}
