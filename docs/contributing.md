# Contributing across the repositories

This repository is the one whose changes are about the other nine, so this
guide is about a change that crosses them. A change inside one repository is
that repository's `docs/contributing.md`. What each cross-repository check
catches is [working-across-repositories.md](working-across-repositories.md);
the front page is [`../README.md`](../README.md).

## Building it

From this repository's root, one directory up from here:

```bash
./build-and-test.sh cross    # this repository's own checks; what CI runs
./build-and-test.sh          # the above, plus every sibling's gate
```

The ten side by side, [SDL2](https://www.libsdl.org/), and a fetch in every sibling first, as
[`../README.md`](../README.md)'s `## Requirements` says. There is nothing to
format here but `xtask/` and `gate/`, and `format` checks them.

## A change that crosses repositories

1. **Make it in the repository that owns the item**, and pass that gate.
2. **Run `cross` here.** It builds every crate in its graph from the
   working trees, so a `Chrome` trait change that breaks the simulator fails
   now rather than after a push. It reaches neither firmware nor `xpui-cpp`;
   for those, push the crate you changed and run their gates.
3. **Fix the dependents in their own repositories**, each against its own
   gate, and run `cross` again.
4. **Push in dependency order** — `xpui`, then `xpui-chrome`, `xpui-boards`,
   `xpui-backends`, the simulator, the gallery, the two firmwares,
   `xpui-cpp` — because every dependency is a `git` dependency on `main`,
   and a dependent pushed first fails its own CI against the old revision.
   Each dependent needs a `cargo update -p <crate>` to move its lock file to
   the new revision; commit the lock file with the change.
5. **Push this repository last**, if it changed at all. It usually has not.

## A shared file, in every copy at once

A copied file is compared byte for byte, and **each one across a different
set of repositories**: `LICENSE` and `clippy.toml` across the ten; the community files
across the ten; the `xtask` modules across the nine; the `## Where it sits`
diagram across the ten READMEs; the `[workspace.lints]` table across every
workspace root, of which there are more than there are repositories.
[working-across-repositories.md](working-across-repositories.md) has the
table. A change to one is a change to every copy, made in one sitting. Edit
the copy in one repository, copy it to the rest, run each gate, and run
`cross` here before pushing any of them; the first push of a half-done
change fails every sibling's CI, not only this one.

The `xtask` modules are the ones to be careful with: a fix to the fence
scanner is a fix in nine places, and each repository's `main.rs` is its own
and stays different.

## The review

Five steps, in order, none skipped:

1. `cross` passes here, and each touched repository's gate passes there,
   with the real exit codes read.
2. The [code-reviewer](../.claude/agents/code-reviewer.md) agent reviews the
   change — every finding resolved, not noted.
3. The [docs-reviewer](../.claude/agents/docs-reviewer.md) agent reviews the
   prose, last: it runs every command a document gives and resolves every
   snippet against the API.
4. The author reviews it, and tests it in the simulator or on a board where
   the change reaches one. That is their step.
5. They say commit — in each repository, in the order above.

## Commits

The subject says what was done — imperative, under fifty characters, one
concern. The body says what changed and why, in under about ten lines,
carrying the fact that is not in the diff. Nothing about how the bug was
found. No self-attribution. A change that crosses repositories is one commit
per repository, each saying which sibling it follows.
