---
title: What it does
weight: 2
---

# What it does

Five examples ship with the engine. Each one is the smallest thing that shows
a particular part of it working.

## teapot

The Utah teapot, from the control points Newell measured off a real one in
1975. 32 Bezier patches, and translucency for looking inside it.

```
cargo run --release --example teapot
```

Drag to turn it, or use the arrows. Q and E roll it, T makes it see-through,
scroll moves closer, R puts it back, space locks the cursor.

![The Utah teapot in white, casting a teapot shaped shadow](/media/teapot.png)

![The same teapot, translucent](/media/teapot-glass.png)

## klein

A Klein bottle, drawn as a wire mesh so the neck is visible where it passes
through the wall. Parametric surfaces, and two-sided geometry for a surface
with no outside.

```
cargo run --release --example klein
```

The teapot's controls, plus M to swap the mesh for the solid surface.

![A Klein bottle as a wire mesh, its neck curving over and back down into its
body, casting a lattice shadow](/media/klein.png)

![The same bottle, translucent, the neck carrying on inside the body after it
passes through the wall](/media/klein-glass.png)

## tunnel

Flying down the inside of a twisting tunnel, which is the one place a surface
with no visible outside is all you ever look at. It is also the hardest thing
you can ask of mipmaps: a checker receding to a vanishing point.

```
cargo run --release --example tunnel
```

Steer with the mouse. It flies itself, faster the further you get, and brushing
the wall costs you speed.

![Looking down a tunnel of dark and light checks receding to a vanishing point,
with a gold ring hanging off centre](/media/tunnel.png)

## cubes

The lighting. A sun, two spots going round, and a lamp between them. L takes
you through them one kind at a time.

The sun is a direction with no position, so there is nothing to draw for it,
and turning it off is the only way to see what it was doing. The spots cast
down their cones. The lamp casts every way at once, which takes six
projections rather than one.

```
cargo run --release --example cubes
```

![Four cubes on a dark checkered floor lit by one lamp, each throwing its
shadow off in a different direction](/media/cubes.png)

## rolling

The collision. A ball rolled around a walled room: sliding along walls,
settling in corners, and never passing through anything.

```
cargo run --release --example rolling
```

WASD or the arrows roll the ball and the camera follows it.
