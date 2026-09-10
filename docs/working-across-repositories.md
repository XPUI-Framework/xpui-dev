# Working across the repositories

[`../README.md`](../README.md) is the front page; making a change that crosses
repositories is [contributing.md](contributing.md).

What the umbrella is for, what each of its checks catches, and what it leaves
to the nine.

## What it is for

Each repository proves its own change with its own gate. What none of them
can prove is that a change to a public item still builds what depends on it,
because every dependency between them is a `git` dependency on `main` — a
pushed revision, not the working tree beside it. `Cargo.toml`'s `[patch]`
table redirects every one of those at the sibling checkout on disk, so one
build here compiles the working trees rather than the pushed revisions.

**It does not reach everything.** The build graph is what `gate/` names:
`xpui`, `xpui-chrome`, the four board crates, both backends,
`xpui-screenshot`, `xpui-simulator` and `xpui-gallery`. The two firmwares and
`xpui-cpp` are outside it — nothing depends on them — and the full run does
not close the gap either, because each sibling's own gate resolves its `git`
dependencies against pushed `main`. **A local, unpushed change to `xpui` is
never compiled against a firmware or the C++ host in any mode here.** Push
`xpui` first, then run those two repositories' own gates.

## The trap in `[patch]`

Cargo matches a patch to a dependency **by URL string**. A trailing `.git`,
`http` for `https`, or a different case, and the patch silently does not
apply — the build succeeds against the pushed revision, and the local change
is not tested at all. `Cargo.lock` is where to check: every `xpui*` crate
should have no `source` line.

A sibling that is *missing* is not silent: a `[patch]` path that does not
exist is a hard cargo error before any stage runs.
`every repository is checked out beside this one` is first among the
cross-repository stages so that the three unpatched repositories — the two
firmwares and `xpui-cpp` — are named too, in one message rather than nine.

## The lock file that does not travel

`Cargo.lock` here is resolved against the patched paths, so it records what
the stack looked like on the machine that last ran the gate rather than what
a fresh clone of the nine would resolve to. Treat it as a local artifact.
`every lock file agrees about the shared crates` reads this one **and** every
sibling's, the monorepo's included, so a local lock that drifts fails the
stage like any other: `embedded-graphics`, `embedded-graphics-core`,
`critical-section` and `u8g2-fonts` cross repository boundaries as **types**,
and `DrawTarget` from 0.8.1 is not `DrawTarget` from 0.8.2 — the compiler
names the same path twice in one error and blames a trait rather than a
version.

## What is shared, and how

Nothing is published and there is no submodule, so a file that every
repository needs is **copied**, and a copy nobody compares is a fork with a
delay on it. Two kinds are compared:

- **Files**, byte for byte, each across the repositories that hold it:

  | | |
  |---|---|
  | `LICENSE`, `clippy.toml` | all eleven — the nine, this one, and the monorepo while it exists |
  | `SECURITY.md`, `CODE_OF_CONDUCT.md`, both `.github/ISSUE_TEMPLATE/*.yml`, `.github/PULL_REQUEST_TEMPLATE.md`, `.github/dependabot.yml`, both `.claude/agents/*.md` | the ten |
  | ten `xtask` modules | the nine: reading a markdown fence, a manifest, a path and a comment is the same job everywhere |
  | `.gitignore` | nine of the ten; `xpui-cpp`'s is a superset with the PlatformIO lines, and there is no superset mode |
  | `build-and-test.sh` | six |
  | `xtask/src/cpp.rs` | the two that hold C++ |
  | `xtask/src/cargo.rs` | in **three shapes** — `xpui-rp2040` adds `host_triple` because its cargo config targets the board, and `xpui-simulator` and `xpui-cpp` drop `target_installed` because neither has a bare-metal lint |
  | `rust-toolchain.toml`'s `channel` line | all eleven, so a warning means the same thing everywhere |

  Each repository's `xtask/src/main.rs` is deliberately *not* compared; it is
  that repository's own list of checks. The FreeInk SDK revision is compared
  by a stage of its own.
- **Sections**, as text: the `## Where it sits` diagram in every README,
  apart from the `style` line that bolds the repository you are in, and the
  `[workspace.lints]` table in every workspace root — twelve of those, not
  ten: `xpui-rp2040`'s `docs-test/` and `xtask/` are workspaces themselves.

A change to any of them is made in all ten in one sitting, or the umbrella
fails on the first push; [contributing.md](contributing.md) says how.

## What each check catches

| | |
|---|---|
| every repository is checked out beside this one | names every missing one in a single message, before nine other stages fail one at a time with worse ones. A patched sibling that is gone is a hard cargo error; one present but not a git checkout would pass silently |
| every shared file is one file | twenty-five paths in twenty-seven comparisons — the table above, each across the repositories that hold it |
| every shared section is one section | the diagram across the ten READMEs, and the `[workspace.lints]` table across **twelve** workspace roots — `xpui-rp2040`'s `docs-test/` and `xtask/` are two of their own |
| both repositories pin the same SDK revision | `xpui-backends` compiles the shim against the SDK's headers and `xpui-cpp` links it; a revision written down twice is one that will disagree with itself |
| every organisation URL names a file that is there | each repository's `documented paths resolve` reads relative links and says so; nothing else reads a `github.com/XPUI-Framework/…` URL, and this resolves its `blob/main` and `tree/main` links against the sibling's pushed `origin/main` |
| every lock file agrees about the shared crates | the four crates above |
| every repository gates itself | `all` only: each sibling's `./build-and-test.sh`, from its own root |
| every crate, from local paths | `cargo build --workspace` and `cargo test --workspace` through the `[patch]` table, so what is tested is what is on disk |

## What `cross` skips

`cross` is what CI runs, because each repository's own workflow has already
run its gate, and running all nine again would pay twice for the same checks.
So `cross` does not run:

- **any sibling's gate** — its lint for a bare-metal target, its comment
  checks, its README order, its doctests;
- **anything in C++** — the format stage, the compiled `cpp` fences, the
  symbol check, the shim, the host and its `ctest` cases. CI here installs
  neither clang-format nor the FreeInk SDK, and needs neither;
- **any firmware image** — the `all` mode of `xpui-rp2040` and `xpui-esp32`.
  Nor does the full run: `every repository gates itself` invokes each
  sibling's script with no argument, which is its `check` mode, and both
  firmwares put their link stages behind `all`. **No image is linked here in
  either mode.**

A change that touches any of those is proven in its own repository first,
then here. `./build-and-test.sh` with no argument runs the nine gates too,
which is the twenty-minute version; iterate on `cross`.
