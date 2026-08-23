# `xpui-dev`

> ⚠️ **Under heavy development.** Not production-ready. The API can break
> without notice. Use at your own risk.

The umbrella. Nine repositories, checked out side by side, built as one.

**This repository is never pushed.** Its `[patch]` table points at directories
on one machine, and a lock file resolved against them means nothing anywhere
else. `./build-and-test.sh` refuses to run if a remote is configured.

## What it is for

Before the split, `./build-and-test.sh` proved a change across the framework,
three backends, the boards and five examples at once, and CI ran it as one
step. A `Chrome` trait change was one commit.

Afterwards it is a change in one repository and a `cargo update` in five
others, and nothing says the five still work. This is what says so — while the
change is still local, before any of it is pushed.

```bash
git clone git@github.com:XPUI-Framework/xpui-framework.git xpui
git clone git@github.com:XPUI-Framework/xpui-chrome.git
git clone git@github.com:XPUI-Framework/xpui-boards.git
git clone git@github.com:XPUI-Framework/xpui-backends.git
git clone git@github.com:XPUI-Framework/xpui-simulator.git
git clone git@github.com:XPUI-Framework/xpui-gallery.git
git clone git@github.com:XPUI-Framework/xpui-rp2040.git
git clone git@github.com:XPUI-Framework/xpui-esp32.git
git clone git@github.com:XPUI-Framework/xpui-cpp.git

cd xpui-dev && ./build-and-test.sh
```

The framework's directory is `xpui`, matching the crate; its repository is
called `xpui-framework`.

## What it checks

| | |
|---|---|
| every sibling is present | a missing one would otherwise resolve from GitHub, and a local change would go untested with the build green |
| this repository has no remote | see above |
| every lock file agrees | `embedded-graphics`, `embedded-graphics-core`, `critical-section` and `u8g2-fonts` cross repository boundaries as *types*. `DrawTarget` from 0.8.1 is not `DrawTarget` from 0.8.2, and the error blames a trait rather than a version |
| the whole stack builds and tests | from local paths, so what is tested is what is on disk |

Each repository still gates itself. This gates only what no single one can see.

## The trap in `[patch]`

Cargo matches a patch to a dependency **by URL string**. A trailing `.git`,
`http` for `https`, or a different case, and the patch silently does not
apply — the build succeeds against the pushed revision, and the local change
is not tested at all. `Cargo.lock` is where to check: every `xpui*` crate
should have no `source` line.
