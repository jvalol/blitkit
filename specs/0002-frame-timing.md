# 0002 Frame timing

**Status:** implemented
**Date:** 2026-09-19

## Goal

Games move at the same speed on any display, and a slow frame doesn't teleport
anything across the screen.

## Behavior

The engine redraws continuously and calls `Game::update` once per frame with `dt`,
the seconds since the previous update. Presentation is vsynced, so the frame rate
follows the display: 60 fps gives about 0.0167, and 120 fps about 0.0083.

`dt` is capped at `MAX_DELTA_TIME`, 0.05 seconds. Below 20 fps the game slows down
instead of taking one large step. That keeps a stall, a window drag, or a
breakpoint from moving anything far enough to pass through what it should hit.

Games express speeds per second and multiply by `dt`.

## Acceptance criteria

- A normal frame time is passed through unchanged. — `tests::clamp_delta_time_passes_normal_frames`
- A long frame is capped at `MAX_DELTA_TIME`. — `tests::clamp_delta_time_caps_long_frames`
- The cap is a fifth of a second's worth of frames, so 20 fps is the floor. — `tests::max_delta_time_is_a_twentieth_of_a_second`

### Verified by hand

- The game does not speed up on a 120 Hz display. — run pong on a ProMotion
  display and compare with a 60 Hz one.

## Out of scope

A fixed timestep, frame rate limiting, and any way to ask for a present mode other
than vsync.
