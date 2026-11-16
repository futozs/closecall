---
status: accepted
date: 2026-11-16
decision-makers: futozs
---

# The logo: `close()`

## Context and Problem Statement

The first icon (a ring with a red top half and a bar through it) read as a Pokéball. The owner asked
for a mark as good as the name. It has to work as a macOS app icon that fills the Tahoe squircle, as
a 16 px Windows and Linux icon, and as a one-colour menu-bar template with visible states.

## Considered Options

Five designers each explored one direction, three judges scored them (meaning, craft at small sizes,
originality and look-alike risk), and the winner was refined:

- `close()`: a paper "(" and a signal-red ")" with nearly touching tips (21 points)
- A roller shutter stopping just above a browser tab (17)
- A curtain drawn across a screen (17)
- A heavy C closing on a red window (16)
- A laptop lid left open a crack (12)

## Decision Outcome

Chosen option: `close()`, because it draws the name itself. `close()` is a call that closes; the red
closing paren is the one that acts; the tips almost meet, which is the near miss, around an empty
middle where nothing is left on screen. It avoids the shield, lock and eye clichés of privacy apps.

- Sources live in [design/brand/](../../design/brand/): the 1024 master, hand-hinted 16 and 32 px
  drawings, the bare mark for light and dark backgrounds, three menu-bar template states (watching
  `( )`, share active `()`, paused), the favicon and the DMG background.
- Small sizes use the hinted drawings, not downscales: `32x32.png`, and the 16 and 32 px entries of
  `icon.icns` and `icon.ico`.
- The DMG window frames the install as `close( Close Call → Applications )`.

### Consequences

- Good, because the mark is two strokes and survives 16 px and the menu bar.
- Bad, because a paren pair could be read as "code tool" by people who do not program; the name and
  the README carry the meaning for them.
- Keep the gap at the tips at least 2 px at 32 px and the middle wider than one stroke, so the pair
  never closes into a two-tone ring.

## More Information

The concept sheets and judge notes were produced in the logo design session on 2026-11-16 and are
summarised here; the brand files are the source of truth.
