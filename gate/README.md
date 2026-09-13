[![CI](https://github.com/XPUI-Framework/xpui-dev/actions/workflows/ci.yml/badge.svg)](https://github.com/XPUI-Framework/xpui-dev/actions/workflows/ci.yml) [![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)

# `xpui-dev-gate`

A crate with no code. It names the libraries the dev stack patches, and that
is the whole point.

## Using it

A `[patch]` table needs something to patch. This crate names every library
crate the `[patch]` entries cover, so resolving it resolves that stack
against the sibling checkouts on disk. `xpui-abi-check` is named even though
nothing here would otherwise reach it — it is a **dev**-dependency of
`xpui-fui` and of `xpui-cpp`'s `abi`, and a dev-dependency of a git
dependency is not resolved — because a patched crate outside the graph is a
patch cargo warns about on every command and a crate the umbrella never
builds from the working tree. `xpui-cpp`'s own crates are deliberately
outside it: nothing in the organisation depends on them, so nothing would be
proved by pulling them in.

The lock file that falls out is a local artifact — resolved against the
patched paths, so it records this machine rather than a fresh clone. It is
still read: `locks_agree` in [`xtask/`](../xtask/) compares it with every
sibling's, because a disagreement is by definition between two of them. It
checks the third-party crates whose **types** cross a repository boundary: a
`DrawTarget` from one major version of `embedded-graphics-core` is a
different type from the next one's, and the compiler says so by naming the
same path twice in one error. It takes a semver-incompatible pair — cargo
unifies 0.8.1 with 0.8.2 and only one reaches the graph.

`src/lib.rs` is empty on purpose. Adding anything to it would give this crate
an opinion, and its value is that it has none.

## Checking it

The gate is the repository's; run `./build-and-test.sh cross` from the root,
which runs this crate's one test beside `xtask`'s. To run it alone:

```bash
cargo test -p xpui-dev-gate
```

It reads its own source and fails if anything above the test module is code.

## License

MIT — see [LICENSE](../LICENSE). Copyright (c) 2026 Thiago Holanda.
