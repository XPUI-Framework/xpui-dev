//! The cross-repository gate.
//!
//! Nine repositories that were one, and everything after `rustdoc links
//! resolve` below is what none of them can check alone. Each has its own `xtask/` holding its
//! own list; no cross-repository check is in any of them.
//!
//! ```text
//! ./build-and-test.sh          everything, including each repository's gate
//! ./build-and-test.sh cross    only the cross-repository half — what CI runs
//! ```
//!
//! CI uses `cross` because each repository's own workflow has already run its
//! gate, and running all nine again would pay twice for the same checks out of
//! a private repository's minutes.

mod files;
mod links;
mod locks;
mod sections;

use std::process::{Command, ExitCode};

/// The nine, in dependency order.
const SIBLINGS: [&str; 9] = [
    "xpui",
    "xpui-chrome",
    "xpui-boards",
    "xpui-backends",
    "xpui-simulator",
    "xpui-gallery",
    "xpui-rp2040",
    "xpui-esp32",
    "xpui-cpp",
];

fn main() -> ExitCode {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/..");
    std::env::set_current_dir(root).expect("this repository's root");

    let cross_only = match std::env::args().nth(1).as_deref() {
        None | Some("all") => false,
        Some("cross") => true,
        Some(other) => {
            eprintln!("unknown argument `{other}`\nusage: ./build-and-test.sh [all|cross]");
            return ExitCode::from(2);
        }
    };

    let mut gate: Vec<(&str, Box<dyn Fn() -> Result<String, String>>)> = vec![
        // This repository's own code first.
        (
            "format",
            Box::new(|| run(env!("CARGO"), &["fmt", "--all", "--check"]).map(|()| "clean".into())),
        ),
        (
            "lint",
            Box::new(|| {
                run(
                    env!("CARGO"),
                    &[
                        "clippy",
                        "--workspace",
                        "--all-targets",
                        "--",
                        "-D",
                        "warnings",
                    ],
                )
                .map(|()| "host".into())
            }),
        ),
        ("rustdoc links resolve", Box::new(rustdoc)),
        (
            "every repository is checked out beside this one",
            Box::new(siblings_are_present),
        ),
        (
            "every shared file is one file",
            Box::new(files::shared_files_agree),
        ),
        (
            "every shared section is one section",
            Box::new(sections::shared_sections_agree),
        ),
        (
            "both repositories pin the same SDK revision",
            Box::new(files::sdk_revisions_agree),
        ),
        (
            "every organisation URL names a file that is there",
            Box::new(links::org_links_resolve),
        ),
        (
            "every lock file agrees about the shared crates",
            Box::new(locks::locks_agree),
        ),
    ];
    if !cross_only {
        gate.push((
            "every repository gates itself",
            Box::new(every_repository_gates),
        ));
    }
    gate.push((
        "every crate, from local paths",
        Box::new(the_whole_stack_builds),
    ));

    let mut failed = 0;
    for (name, check) in gate.drain(..) {
        println!("\n==> {name}");
        match check() {
            Ok(note) if note.is_empty() => println!("    ok"),
            Ok(note) => println!("    {}", note.replace('\n', "\n    ")),
            Err(why) => {
                println!("{why}");
                eprintln!("FAILED: {name}");
                failed += 1;
            }
        }
    }

    if failed > 0 {
        eprintln!("\n{failed} check(s) failed.");
        return ExitCode::FAILURE;
    }
    if cross_only {
        println!("\nThe cross-repository checks pass. Each gate runs in its own CI.");
    } else {
        println!("\nEverything passes.");
    }
    ExitCode::SUCCESS
}

/// Every sibling is on disk.
///
/// First, because everything else here reads them. A missing one would
/// otherwise resolve from GitHub through the `[patch]` section, and a local
/// change would go untested with the build green.
fn siblings_are_present() -> Result<String, String> {
    let missing: Vec<&str> = SIBLINGS
        .into_iter()
        .filter(|repo| !std::path::Path::new("..").join(repo).join(".git").exists())
        .collect();
    if missing.is_empty() {
        Ok(format!("{} repositories", SIBLINGS.len()))
    } else {
        Err(format!(
            "  not a git checkout: {}\n\nClone them from github.com/XPUI-Framework.",
            missing.join(", ")
        ))
    }
}

/// Each repository's own gate, run from its own root.
fn every_repository_gates() -> Result<String, String> {
    let mut failures = Vec::new();
    for repo in SIBLINGS {
        println!("\n--- {repo}");
        let ok = Command::new("./build-and-test.sh")
            .current_dir(format!("../{repo}"))
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            failures.push(repo);
        }
    }
    if failures.is_empty() {
        Ok(format!("{} gates", SIBLINGS.len()))
    } else {
        Err(format!("  failed: {}", failures.join(", ")))
    }
}

/// The whole stack, from the paths on disk rather than from GitHub.
///
/// `[patch]` redirects every git dependency at `../<repo>`, so what is built
/// here is what is in the working trees — including changes nobody has pushed.
fn the_whole_stack_builds() -> Result<String, String> {
    run(env!("CARGO"), &["build", "--workspace"])?;
    run(env!("CARGO"), &["test", "--workspace"])?;
    Ok("built and tested".into())
}

/// This workspace's own docs, with every rustdoc warning an error.
///
/// The `[workspace.lints]` table denies `rustdoc::unescaped_backticks`, and a
/// lint nothing runs is decoration. The nine siblings do this from a shared
/// `cargo.rs`; this repository has none, so the command lives here.
fn rustdoc() -> Result<String, String> {
    let status = Command::new(env!("CARGO"))
        .env("RUSTDOCFLAGS", "-D warnings")
        .args(["doc", "--workspace", "--no-deps"])
        .status()
        .map_err(|e| format!("could not run cargo: {e}"))?;
    if status.success() {
        Ok("cargo doc --workspace --no-deps".into())
    } else {
        Err(
            "cargo doc --workspace --no-deps failed. An intra-doc link that\n\
             does not resolve is a link nothing else in the build reads."
                .into(),
        )
    }
}

/// One command, failing with its own name.
pub fn run(program: &str, arguments: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(arguments)
        .status()
        .map_err(|e| format!("{program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed", arguments.join(" ")))
    }
}
