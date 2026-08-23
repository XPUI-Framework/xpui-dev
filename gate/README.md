# `xpui-dev-gate`

> ⚠️ **Under heavy development.** Not production-ready. The API can break
> without notice. Use at your own risk.

A crate with no code. It names every library the dev stack patches, and that is
the whole point.

A `[patch]` table needs something to patch. This crate names the eleven library
crates the six `[patch]` entries cover, so resolving it resolves that whole
stack against the sibling checkouts on disk. `xpui-cpp`'s crates are
deliberately outside it: nothing in the organisation depends on them, so
nothing would be proved by pulling them in.

The lock file that falls out is one of eleven that
[`locks_agree`](../build-and-test.sh) compares — it reads every sibling's, not
just this one, because a disagreement is by definition between two of them. It
checks the third-party crates whose **types** cross a repository boundary.

That last part is not a style question. `DrawTarget` from `embedded-graphics`
0.8.1 is a different type from `DrawTarget` from 0.8.2, and the compiler says
so by naming the same path twice in one error.

`src/lib.rs` is empty on purpose. Adding anything to it would give this crate
an opinion, and its value is that it has none.

## License

MIT — see [LICENSE](../LICENSE). Copyright (c) 2026 Thiago Holanda.
