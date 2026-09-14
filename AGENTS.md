# `xpui-dev`

## What this is, and what it may not become

The umbrella: the nine repositories checked out side by side and built as
one through a `[patch]` table, plus the checks no single repository can make
— that every sibling is present, that every copy of a shared file and
section is still one, that both FreeInk SDK pins agree, that every
organisation URL names a file on the sibling's pushed `main`, that the lock
files agree about the crates whose types cross a boundary, and that every
library the `[patch]` table covers builds from local paths, with the `gate`
crate's tests run against them. **It
does not reach everything**: neither firmware nor `xpui-cpp` is in that
graph, so a green run here says nothing about them.

**No cross-repository check exists anywhere else, and there is almost no code
here.** `format`, `lint` and `rustdoc links resolve` check this repository's
own two crates, as every sibling checks its own; everything after them is
what none of the nine can see. `gate/` is a crate with nothing in it, kept
that way by its one test. The `xtask/` here is a different program from the
nine's — `files.rs`, `links.rs`, `locks.rs`, `sections.rs` — and shares no
module with them; it carries neither their four documentation checks nor
their two comment checks, so this file and the README are held to the
standard by convention and by review. Nothing here publishes, formats a
sibling, or opens a window.

## The gate

```bash
./build-and-test.sh cross    # everything below but the line marked `+` — what CI runs
./build-and-test.sh          # everything, including every sibling's own gate
```

```text
format · lint · rustdoc links resolve · every repository is checked out beside this one · every shared file is one file · every shared section is one section · both repositories pin the same SDK revision · every organisation URL names a file that is there · every lock file agrees about the shared crates · every patched crate, from local paths
+ every repository gates itself, before the last stage
```

Nothing compares this list to what runs — there is no `the gate is
documented` here — so copy it from what `./build-and-test.sh cross` prints
when a stage is added. `git fetch --all` in every sibling before running it:
the URL check reads each `origin/main`, and a stale remote is a stale check.
The full run takes about twenty minutes; iterate on `cross`.

## What only this repository checks

Everything after `rustdoc links resolve`. The nine cannot see each other;
this reads all of them from `..`.

## Style that bites here

- **Every path in `Cargo.toml` is relative**, and that is load-bearing: an
  absolute one makes the repository meaningless anywhere but the machine that
  wrote it.
- **A `[patch]` entry is keyed by the exact URL the manifests name.** A
  trailing `.git`, `http` for `https`, or a different case, and the patch
  silently does not apply. `Cargo.lock` is where to check: every `xpui*`
  crate has no `source` line.
- **`Cargo.lock` here is a local artifact**, resolved against the patched
  paths — but `locks_agree` still reads it, alongside every sibling's. A drift
  here fails the stage.
- **`gate/src/lib.rs` stays empty.** Its test counts the lines above
  `#[cfg(test)]` and fails on one.
- **The shared `xtask` module lists are two constants in `files.rs`**:
  `XTASK`, the thirteen every sibling carries, and `GROUPS`, for `cpp.rs` and
  `cargo.rs`'s three shapes. Add a module to every sibling, then to whichever
  of the two it belongs in, in the same sitting.
- **The precision pass applies to all six source files by hand** — the four
  above plus `xtask/src/main.rs` and `gate/src/lib.rs`. No comment check runs
  here, so no gate phrase, no run over ten lines, no header over fifteen.

## Where the documentation lives, and what proves each piece

| Document | Proven by |
|---|---|
| [`README.md`](README.md), [`gate/README.md`](gate/README.md) | two of this repository's own stages read the front page — the shared-section compare and the URL check — but **nothing checks a heading's order or name.** Read that against the standard by hand; a mangled heading has already shipped here once |
| [`docs/README.md`](docs/README.md) | the index of `docs/`; nothing here checks its links, so read it against `docs/` by hand |
| [`docs/working-across-repositories.md`](docs/working-across-repositories.md) | its check table is one row per cross-repository stage; `format`, `lint` and `rustdoc links resolve` are not in it |
| [`docs/contributing.md`](docs/contributing.md) | the push order is `SIBLINGS`' order in `xtask/src/main.rs` |
| `AGENTS.md` | the stage list above, against what `./build-and-test.sh cross` prints |
| every `///` and `//!` | `rustdoc links resolve`, with warnings denied |

## Git

Never stage, never commit, never push without being asked, each time. The
index is the reviewer's queue; leave new work unstaged. No self-attribution
in a commit message. Never rewrite a commit that exists; a correction is a new
commit. A change that crosses repositories is pushed in dependency order and
this repository last; the order, the rules that apply to all ten, and the
five review steps are in
[`xpui`'s `docs/orientation.md`](https://github.com/XPUI-Framework/xpui-framework/blob/main/docs/orientation.md)
and [`docs/contributing.md`](docs/contributing.md).
