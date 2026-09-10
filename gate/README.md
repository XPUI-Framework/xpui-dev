# `xpui-dev-gate`

> ⚠️ **Under heavy development.** Not production-ready. The API can break
> without notice. Use at your own risk.

A crate with no code. It names the libraries the dev stack patches, and that
is the whole point.

## Using it

A `[patch]` table needs something to patch. This crate names eleven of the twelve library
crates the six `[patch]` entries cover, so resolving it resolves that stack
against the sibling checkouts on disk. `xpui-cpp`'s crates are
deliberately outside it: nothing in the organisation depends on them, so
nothing would be proved by pulling them in. `xpui-abi-check` is patched but
not named here because nothing would reach it: it is a **dev**-dependency of
`xpui-fui` and of `xpui-cpp`'s `abi`, so depending on either does not build
it.

The lock file that falls out is a local artifact — resolved against the
patched paths, so it records this machine rather than a fresh clone. It is
still read: `locks_agree` in [`xtask/`](../xtask/) compares it with every
sibling's, because a disagreement is by definition between two of them. It checks the
third-party crates whose **types** cross a repository boundary: `DrawTarget`
from `embedded-graphics` 0.8.1 is a different type from `DrawTarget` from
0.8.2, and the compiler says so by naming the same path twice in one error.

`src/lib.rs` is empty on purpose. Adding anything to it would give this crate
an opinion, and its value is that it has none.

## Checking it

The gate is the repository's; run `./build-and-test.sh cross` from the root.
This crate's own test is one of the two it runs:

```bash
cargo test -p xpui-dev-gate
```

It reads its own source and fails if anything above the test module is code.

## License

MIT — see [LICENSE](../LICENSE). Copyright (c) 2026 Thiago Holanda.
