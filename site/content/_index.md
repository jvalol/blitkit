---
title: blitzkit
type: docs
ai_drafted: true
---

# blitzkit

A small 2D and 3D game engine in Rust, over wgpu. A game implements one trait
and calls `start()`, and the engine owns the window, the event loop, drawing,
keyboard and mouse input, and sound.

```
cargo add blitzkit
```

- [Getting started]({{< relref "docs/getting-started" >}}) is the smallest
  program that opens a window.
- [What it does]({{< relref "docs/examples" >}}) is the five examples that ship
  with it.
- [What it promises]({{< relref "docs/promises" >}}) is the part that is
  unusual: every behaviour is a written spec naming the test that proves it.
