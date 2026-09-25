# 0004 Sound output

**Status:** implemented
**Date:** 2026-09-20

## Goal

Games play sounds without handling audio device failures themselves.

## Behavior

`SoundSystem` opens the default output device at startup and plays sounds queued
with `queue`, mixing whatever overlaps. `queue_spatial` plays through a separate
spatial output whose emitter position moves per sound. The starting volume is 0.5.
Where the listener stands and which way it faces is spec 0019.

When no device can be opened, whether none exists or the system refuses the one
that does, the engine logs a warning and every queued sound is dropped. Games
call `queue` the same way either way and never check whether sound is available.

Sounds are rodio `Source`s, so the game owns decoding and the engine depends on
rodio with only the playback feature. A game picks the decoders it needs.

## Acceptance criteria

Nothing here can be tested without an audio device, so this spec's criteria are all
verified by hand.

### Verified by hand

- Sounds play. — run pong, bounce the ball off a paddle.
- Overlapping sounds mix rather than cutting each other off. — run pong, score in
  quick succession.
- No device means a warning and silence, not a crash. — run with an output device
  removed or disabled.

## Out of scope

Music, looping, per-sound volume, pausing playback, and choosing an output device.
