# blitzkit

In which I develop a wrapper around wgpu-rs to create a graphics development engine in rust.

2d _and_ 3d. I built four games using it as the engine to prove it out.

Feel free to follow along!

## Games built on it

- [pong](https://github.com/jvalol/pong)
- [snake](https://github.com/jvalol/snake)
- [tetris](https://github.com/jvalol/tetris)
- [marble](https://github.com/jvalol/marble) first 3d game here

There are also examples in this repo. Sibling repos have actual game implementations.

## Examples

`cargo run --release --example teapot` is the Utah teapot, from the control
points Martin Newell measured off a real one in 1975. Press T to
make it translucent.

![The Utah teapot in white, spout to the left and handle to the right, casting a
teapot shaped shadow](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/teapot.png)

![The same teapot in glass, its far wall, the underside of its lid and its handle
all showing through the near wall](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/teapot-glass.png)

`cargo run --release --example klein` is a Klein bottle you can turn any way you
drag it. M swaps the wire mesh for the solid surface and T to make it translucent.

![A Klein bottle drawn as a wire mesh, its neck curving over and back down into
its body, casting a lattice shadow on the floor](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/klein.png)

![The same bottle in glass, the neck visible carrying on down inside the body
after it passes through the wall](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/klein-glass.png)

```
cargo run --release --example tunnel
```

![Looking down a tunnel of dark and light checks receding to a vanishing point,
with a gold ring hanging off centre partway down it](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/tunnel.png)

## License

The code is under MIT or Apache-2.0, whichever you prefer.

The font that ships with it, Press Start 2P, is not mine and is licensed
separately under the SIL Open Font License 1.1. That license travels with it in
`res/fonts/OFL.txt`.
