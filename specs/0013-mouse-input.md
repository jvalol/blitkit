# 0013 Mouse input

**Status:** implemented
**Date:** 2026-09-21

## Goal

Games can be pointed at and looked around with, which keyboard alone cannot do.

## Behavior

Mouse events reach the game the way keyboard events do: engine-owned types, a
trait method with an empty default, and no winit types in sight. There are three
kinds, because they answer different questions.

**Buttons.** `MouseInput` carries a `MouseButton` (left, right, middle, back,
forward, or another by number) and whether it was pressed or released.

**The cursor.** Its position in physical pixels, origin top-left, the same space
as quads and text per spec 0001. A game can compare it against anything it drew
without converting.

**Raw motion.** Relative movement in device units, separate from cursor
position, because a locked cursor stops moving while the mouse keeps going. This
is what turning a camera reads.

**The wheel** reports a scroll in pixels. Some mice report lines instead, which
the engine multiplies by 16 so a game sees one unit.

**Locking the cursor** hides the pointer and pins it, so turning does not stop at
the screen edge. A game asks for it through the renderer, which is what holds the
window, in `Game::before_frame`: a new hook that runs at the start of each frame
for anything needing the renderer rather than the scene. The call returns whether
it worked.

If the platform refuses a lock, the engine tries confining the cursor instead,
and if that fails too it logs and carries on: raw motion still arrives, so
looking around still works, the pointer is simply visible.

Sensitivity, smoothing and inversion are the game's business. The engine reports
what the mouse did.

## Acceptance criteria

- Each winit button maps to its own variant. — `mouse::tests::maps_the_buttons`
- An unnamed button keeps its number. — `mouse::tests::keeps_unnamed_buttons`
- A press carries pressed, a release carries released. — `mouse::tests::carries_button_state`
- A line scroll becomes pixels. — `mouse::tests::line_scrolling_becomes_pixels`
- A pixel scroll passes through unchanged. — `mouse::tests::pixel_scrolling_passes_through`
- The cursor position is in the same pixels as quads and text. — `mouse::tests::the_cursor_is_in_screen_pixels`

### Verified by hand

Run `cargo run --example cubes` in blitkit.

- The cursor position printed under the help line follows the pointer.
- Dragging with the left button turns the camera, and scrolling moves it closer
  and further.
- Space locks the cursor: the pointer disappears and turning keeps going past
  the screen edge. Space again releases it.

Confirmed working on macOS 27 with winit 0.30, which is not a promise about
other platforms: a refused lock leaves the pointer visible and logs a warning,
and dragging still turns the camera.

## Out of scope

Touch, pens, gamepads, multiple cursors, and any sensitivity or smoothing curve.
