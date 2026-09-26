# blitzkit

In which I develop a wrapper around wgpu-rs to create a graphics development engine in rust.

2d _and_ 3d. I've got five games built on it so far.

Feel free to follow along!

## Games built on it

- [pong](https://github.com/jvalol/pong)
- [snake](https://github.com/jvalol/snake)
- [tetris](https://github.com/jvalol/tetris)
- [marble](https://github.com/jvalol/marble) first 3d game here
- [slider](https://github.com/jvalol/slider) second. fly through a tunnel, try to thread through the rings

The examples below live in this repo. The games are their own repos.

## Examples

Run any of them with `cargo run --release --example <name>`.

**teapot**, the Utah teapot, from the points Newell measured off a real one in
1975. Pressing T makes it translucent.

![The Utah teapot in white, casting a teapot shaped shadow](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/teapot.png)

![The same teapot but translucent](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/teapot-glass.png)

**klein**, a Klein bottle you can spin any way you like. M for the solid surface, T for glass.

![A Klein bottle drawn as a wire mesh, its neck curving over and back down into
its body, casting a lattice shadow on the floor](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/klein.png)

![The same bottle but translucent, the neck visible carrying on down inside the body
after it passes through the wall](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/klein-glass.png)

**tunnel**, a checkered tunnel that wanders, and rings to fly through.

![Looking down a tunnel of dark and light checks receding to a vanishing point,
with a gold ring hanging off centre partway down it](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/tunnel.png)

**cubes**, the lighting. One sun and two lamps going round. L switches between
them. The sun is a direction with no position, so there's nothing to draw for
it, and turning it off is the only way to _see_ what it was doing.

**rolling**, the collision. Roll a ball round a walled room with WASD or the arrow keys. It slides
along the walls instead of going through them.

## License

MIT or Apache-2.0, whichever suits you.

The font, Press Start 2P, isn't mine. It's under the SIL Open Font License 1.1,
and that license travels with it in `res/fonts/OFL.txt`.

---

I asked AI to draft this for me. I've edited it. Any surviving AI smells are my oversight.
