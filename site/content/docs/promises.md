---
title: What it promises
weight: 3
ai_drafted: true
---

# What it promises

Every behaviour in this engine is a written spec, and every acceptance
criterion in a spec names the test that proves it. There are 22 of them, in
[`specs/`](https://github.com/jvalol/blitzkit/tree/main/specs), and they are
kept true: a spec that disagrees with the code is treated as a bug in the spec.

Work happens in that order. The spec comes first, then the tests, then the
code.

## What that buys you

**A promise you can look up.** Shadows, collision, translucency and sound each
have a document saying what the engine does, not how it does it.

**A named test per promise.** An acceptance criterion reads like
`shadow::tests::the_bias_grows_with_the_range`. You can run it.

**Honesty about what is not covered.** Some things need a GPU or a window and
cannot be tested headlessly. Those live under "Verified by hand" in the spec
that needs them, written out as steps, rather than being quietly claimed.

**Scope stated as scope.** Every spec carries what it deliberately does not do.

## Not a substitute for judgement

The specs describe the engine, so they cannot catch a gap between the engine
and its shader. Spec 0022 needed six face directions copied into WGSL by hand,
and four of the six went in wrong. What caught it was a test that reads the
shader source and compares it against the vectors the matrices are built from.
The comment telling the next person to keep the two in step did nothing.

## Built on it

Five games, each its own repository, which is what keeps them honest about
depending on the published crate rather than a local checkout.

- [pong](https://github.com/jvalol/pong)
- [snake](https://github.com/jvalol/snake)
- [tetris](https://github.com/jvalol/tetris)
- [marble](https://github.com/jvalol/marble), the first in 3D
- [slider](https://github.com/jvalol/slider), fly a tunnel and thread the rings
