# 0003 Keyboard input

**Status:** implemented
**Date:** 2026-09-20

## Goal

Games read the keyboard without knowing winit exists.

## Behavior

Every key press and release reaches `Game::process_keyboard` as a `KeyboardInput`
carrying the key, whether it was pressed or released, and whether the press came
from the operating system's key repeat.

Keys are identified by physical position, not by the letter printed on the cap.
W is the same key on QWERTY and QWERTZ. On a German layout, Y and Z are the only
letters that report swapped.

`KeyboardKey` has one variant per winit `KeyCode`, and `KeyboardKey::from_key_code`
maps them. It returns `None` for a key it doesn't know, which can happen because
winit may add key codes in a later version. Unknown keys are dropped, and the game
never hears about them.

Games that want an action to happen once per press ignore events where `repeat` is
true. Games that track whether a key is held don't need to, since holding a key
produces one press, repeats, then one release.

## Acceptance criteria

- A letter key maps to its variant. — `keyboard::tests::maps_letter_keys`
- Named keys map to the engine's names for them. — `keyboard::tests::maps_named_keys`
- Every winit key code the engine knows maps to a distinct variant. — `keyboard::tests::mapping_is_one_to_one`
- A press carries `Pressed` and a release carries `Released`. — `keyboard::tests::carries_key_state`
- A repeat press is marked as a repeat. — `keyboard::tests::carries_repeat`

### Verified by hand

- Holding Escape in pong returns to the menu without quitting from it. — run pong,
  start a game, hold Escape.

## Out of scope

Text input, modifier state, key mapping by layout rather than position, and
mouse or gamepad input.
