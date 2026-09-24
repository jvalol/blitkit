# blitzkit

In which I develop a wrapper around wgpu-rs to create a graphics development engine in rust.

2d and 3d. I built four games using it as the engine to prove it out.

Feel free to follow along!

## Games built on it

- [pong](https://github.com/jvalol/pong)
- [snake](https://github.com/jvalol/snake)
- [tetris](https://github.com/jvalol/tetris)
- [marble](https://github.com/jvalol/marble) first 3d game here

There are also examples in this repo. Not games, just demonstrations.

## Examples

`cargo run --release --example teapot` is the Utah teapot, from the control
points Martin Newell measured off a real one in 1975. Thirty-two Bezier patches
the engine tessellates from the formula rather than a model file it loads. T
turns it to glass.

![The Utah teapot in white, spout to the left and handle to the right, casting a
teapot shaped shadow](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/teapot.png)

![The same teapot in glass, its far wall, the underside of its lid and its handle
all showing through the near wall](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/teapot-glass.png)

`cargo run --release --example klein` is a Klein bottle you can turn any way you
drag it. The surface has no outside, so both sides of it are drawn. M swaps the
wire mesh for the solid surface and T turns that to glass.

![A Klein bottle drawn as a wire mesh, its neck curving over and back down into
its body, casting a lattice shadow on the floor](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/klein.png)

![The same bottle in glass, the neck visible carrying on down inside the body
after it passes through the wall](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/klein-glass.png)

`cubes` and `rolling` are the other two: lit textured geometry, and a ball with
collision and shadows.

## License

The code is under MIT or Apache-2.0, whichever you prefer.

The font that ships with it, Press Start 2P, is not mine and is licensed
separately under the SIL Open Font License 1.1. That license travels with it in
`res/fonts/OFL.txt`.
