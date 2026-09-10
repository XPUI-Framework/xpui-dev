//! One version of each crate whose types cross a repository boundary.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::SIBLINGS;

/// Crates that appear in more than one repository's public types.
///
/// A `DrawTarget` from one major version is not the `DrawTarget` from the
/// next, and the compiler's error blames a trait rather than a version — so a
/// skew here is a type mismatch that reads like a bug in the code. It has to
/// be a semver-incompatible pair to bite: cargo unifies 0.8.1 with 0.8.2 and
/// only one of them survives into the graph.
const SHARED_CRATES: [&str; 4] = [
    "embedded-graphics",
    "embedded-graphics-core",
    "critical-section",
    "u8g2-fonts",
];

/// Every lock file agrees about the shared crates.
pub fn locks_agree() -> Result<String, String> {
    // This repository's own lock, then every sibling's — and the monorepo's,
    // which resolves the same crates and can skew like any other. Named
    // rather than read out of `..`: a directory beside the ten is not part of
    // the stack, and `20` sends the author to clone one there to follow a
    // tutorial from clean. `../xpui-dev/Cargo.lock` is this file under another
    // name, and a file compared with itself reads as two repositories
    // agreeing.
    let mut locks = vec!["Cargo.lock".to_string()];
    let mut roots: Vec<String> = SIBLINGS.iter().map(|s| (*s).to_string()).collect();
    if Path::new("../xpui-framework/.git").exists() {
        roots.push("xpui-framework".into());
    }
    for root in roots {
        let lock = Path::new("..").join(&root).join("Cargo.lock");
        if lock.is_file() {
            locks.push(lock.to_string_lossy().to_string());
        }
    }
    locks.sort();

    let mut problems = Vec::new();
    let mut agreed = Vec::new();
    for crate_name in SHARED_CRATES {
        let mut by_version: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for lock in &locks {
            if let Some(version) = version_in(lock, crate_name) {
                by_version.entry(version).or_default().push(lock.clone());
            }
        }
        match by_version.len() {
            // A crate no lock file mentions is a name that has stopped being
            // real — a typo in the list, or a dependency dropped everywhere.
            // Reporting nothing reads as agreement about a version nobody
            // has.
            0 => problems.push(format!(
                "  {crate_name} appears in no lock file, so nothing was compared"
            )),
            1 => agreed.push(format!(
                "{crate_name} {}",
                by_version.keys().next().expect("exactly one")
            )),
            n => problems.push(format!(
                "  {crate_name} is resolved {n} different ways:\n{}",
                by_version
                    .iter()
                    .map(|(v, where_from)| format!("      {v}: {}", where_from.join(", ")))
                    .collect::<Vec<_>>()
                    .join("\n")
            )),
        }
    }
    if problems.is_empty() {
        Ok(agreed.join(", "))
    } else {
        Err(format!(
            "{}\n\nTheir types cross repository boundaries, so a skew is a type\n\
             mismatch rather than a warning. `cargo update -p <crate>` in the\n\
             ones behind.",
            problems.join("\n")
        ))
    }
}

/// The version one lock file resolves a crate to.
///
/// `[[package]]` entries are `name` then `version`, so the line after the
/// name is the answer. Deliberately not a TOML parser: this reads two keys.
fn version_in(lock: &str, crate_name: &str) -> Option<String> {
    let text = fs::read_to_string(lock).ok()?;
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line.trim() != format!("name = \"{crate_name}\"") {
            continue;
        }
        for next in lines.by_ref() {
            if let Some(value) = next.trim().strip_prefix("version = ") {
                return Some(value.trim_matches('"').to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str, body: &str) -> String {
        let path = std::env::temp_dir().join(format!("xpui-lock-{name}.toml"));
        fs::write(&path, body).expect("scratch lock");
        path.to_string_lossy().to_string()
    }

    #[test]
    fn the_version_read_is_the_one_belonging_to_that_name() {
        // A lock file lists dozens of packages and the first `version = ` in
        // the file belongs to none of them. The one wanted is the one after
        // the matching `name`.
        let lock = scratch(
            "ordering",
            "[[package]]\nname = \"other\"\nversion = \"9.9.9\"\n\n\
             [[package]]\nname = \"embedded-graphics\"\nversion = \"0.8.2\"\n",
        );
        assert_eq!(
            version_in(&lock, "embedded-graphics").as_deref(),
            Some("0.8.2")
        );
        let _ = fs::remove_file(lock);
    }

    #[test]
    fn a_crate_that_is_not_locked_here_is_not_a_disagreement() {
        // A repository that does not depend on a crate says nothing about its
        // version, and must not be counted as a third opinion.
        let lock = scratch(
            "absent",
            "[[package]]\nname = \"other\"\nversion = \"1.0.0\"\n",
        );
        assert_eq!(version_in(&lock, "embedded-graphics"), None);
        let _ = fs::remove_file(lock);
    }

    #[test]
    fn a_name_that_only_contains_the_crate_is_not_the_crate() {
        // `embedded-graphics-core` must not answer for `embedded-graphics`.
        let lock = scratch(
            "prefix",
            "[[package]]\nname = \"embedded-graphics-core\"\nversion = \"0.4.1\"\n",
        );
        assert_eq!(version_in(&lock, "embedded-graphics"), None);
        let _ = fs::remove_file(lock);
    }
}
