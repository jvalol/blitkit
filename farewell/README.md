# blitkit

This crate is now [blitzkit](https://crates.io/crates/blitzkit). Same engine,
name I prefer saying.

```toml
[dependencies]
blitzkit = "0.3"
```

blitzkit 0.3.0 is blitkit 0.2.9 with a new name, so swapping the dependency and
the `use` lines is the whole migration. This version is empty: it exists to say
where the engine went.

The code, the specs and the games are at
[github.com/jvalol/blitzkit](https://github.com/jvalol/blitzkit).

---

This lives inside the blitzkit repo but is not part of the blitzkit crate: its
`Cargo.toml` excludes this folder. Publish it from here with `cargo publish`,
using a token scoped to `blitkit` rather than `blitzkit`.
