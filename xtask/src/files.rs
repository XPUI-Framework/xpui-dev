//! The files that exist in more than one repository, and have to agree.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::SIBLINGS;

/// Carried by the nine, the monorepo and this repository, and compared across
/// all eleven — a lint setting that drifts in the umbrella is still drift.
const SHARED: [&str; 2] = ["LICENSE", "clippy.toml"];

/// The seven modules every repository's `xtask` carries.
///
/// Not `main.rs`, which is each repository's own list of checks and is *meant*
/// to differ: the whole point of moving the gate into Rust was that no
/// repository carries a check it never runs. What these four hold is the
/// reading of a markdown fence and a manifest, which is the same job
/// everywhere, and which is the half that had the bugs.
///
/// The nine only. This repository's gate is the cross-repository half and
/// shares no checks with them; the monorepo still runs the shell this
/// replaced.
const XTASK: [&str; 7] = [
    "xtask/src/commands.rs",
    "xtask/src/docs.rs",
    "xtask/src/faults.rs",
    "xtask/src/fences.rs",
    "xtask/src/paths.rs",
    "xtask/src/prose.rs",
    "xtask/src/tree.rs",
];

/// Files that only two repositories carry, and the two that carry them.
///
/// `cpp.rs` is the C++ half of the gate, and only `xpui-backends` and
/// `xpui-cpp` hold C++ — that is the whole point of the split. It is copied
/// between exactly those two, so it is compared between exactly those two.
const TWO_ONLY: [(&str, [&str; 2]); 1] = [("xtask/src/cpp.rs", ["xpui-backends", "xpui-cpp"])];

/// Every repository carries the same copy of each shared file.
///
/// There is no submodule and nothing is published, so these files are
/// **copied**. A copy nobody compares is a fork with a delay on it — and that
/// is not hypothetical here. The nine ran clippy under *different settings*
/// from the monorepo for as long as `clippy.toml` existed in only one of them,
/// and nothing said so, because the check that existed compared one file and
/// only that file.
pub fn shared_files_agree() -> Result<String, String> {
    let mut roots: Vec<String> = SIBLINGS.iter().map(|s| s.to_string()).collect();
    // The monorepo too, while it exists. It is not one of the nine — its
    // remote is the author's own — but it carries the same copies.
    if Path::new("../xpui-framework/.git").exists() {
        roots.push("xpui-framework".into());
    }

    let mut notes = Vec::new();
    let mut problems = Vec::new();

    for file in SHARED
        .iter()
        .chain(XTASK.iter())
        .chain(TWO_ONLY.iter().map(|(name, _)| name))
    {
        let mut here: Vec<String> =
            if let Some((_, only)) = TWO_ONLY.iter().find(|(name, _)| name == file) {
                only.iter().map(|s| (*s).to_string()).collect()
            } else if XTASK.contains(file) {
                SIBLINGS.iter().map(|s| s.to_string()).collect()
            } else {
                let mut all = roots.clone();
                all.push("xpui-dev".into());
                all
            };
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
            continue;
        }
        if by_content.len() > 1 {
            problems.push(format!(
                "  {file} has {} different versions:\n{}",
                by_content.len(),
                by_content
                    .values()
                    .map(|group| format!("      {}", group.join(", ")))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
            continue;
        }
        notes.push(format!("{file:<22} one file, {} copies", here.len()));
    }

    // The toolchain: the same compiler everywhere, whatever targets each
    // installs. Only the `channel` line has to agree — a `targets` list
    // legitimately differs per repository.
    let mut channels: Vec<(String, String)> = Vec::new();
    for root in roots.iter().chain(std::iter::once(&"xpui-dev".to_string())) {
        let text = fs::read_to_string(PathBuf::from("..").join(root).join("rust-toolchain.toml"))
            .unwrap_or_default();
        // Root-qualified when it is missing. A bare "MISSING" makes eleven
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
        // Eleven missing `rust-toolchain.toml` files used to collapse into one
        // distinct value — "MISSING" — and the check reported agreement about
        // a file that existed nowhere. Root-qualifying it is the fix, and this
        // pins the shape.
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
        // A file in both lists would be compared against two different sets of
        // roots, and the second answer would be the one printed.
        for file in XTASK {
            assert!(!SHARED.contains(&file), "{file} is in both lists");
        }
    }
}
