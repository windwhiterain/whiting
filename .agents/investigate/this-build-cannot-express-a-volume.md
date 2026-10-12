# This build cannot express a volume, so a scene's painted haze has no counterpart

Measured 2026-10-12, against Cycles 5.3.0 at 32 samples on CPU.

## The question

`foldyard` and `tidewater` paint aerial perspective in their post stage, which the SPEC requires of
them, and their grounds sit at 0.274 and 0.699 of what the tracer draws. The SPEC's rule moves
atmosphere out of the surface bodies and into the post stage, so the tracer has nothing to fog
through. Is that a design choice — should the exporter carry a volume — or is the target format
incapable of one?

## The measurement

The same closed box, looked into from outside, with a different volume graph each time. A black
volume at a density of 8.0 is opaque: if any of this reached the renderer the frame would be black,
so "no effect" cannot be mistaken for "a small effect".

| variant | what Cycles says | pixels changed of 360000 |
|---|---|---|
| no volume | — | 0 (the baseline) |
| `volume_absorption`, density 0.35 | `ERROR: Unknown shader node "volume_absorption"` | 0 |
| `volume_scatter`, density 0.30 | `ERROR: Unknown shader node "volume_scatter"` | 0 |
| `principled_volume`, density 0.30 | silent | 0 |
| **`principled_volume`, density 8.0, opaque** | **silent** | **0** |
| `principled_volume` via the long connect form | `ERROR: Invalid from or to value for connect node` | 0 |

**The frames are bit-identical.** The two nodes that would carry a volume do not exist in this
build, and the one that is registered is inert: it parses, it connects, and it changes nothing.

## What this settles

There is no route, so it is not a decision to be taken. A scene that paints haze in its grade has
painted something the tracer cannot be given, and the exporter cannot close the gap by carrying a
volume because the format has nowhere to put one.

That leaves the two scenes with a permanent difference of 0.274 and 0.699 on the ground, which is
the right thing to record rather than the right thing to fix. The alternative — dropping the post
stage's haze — would make the two frames agree and make both pictures worse, and the SPEC's rule
that atmosphere belongs to the post stage is there for the same reason.

## The pattern this is the second instance of

`principled_volume` accepts a connection and does nothing, which is the same failure as
`base_color` as a connect target: **a name the parser accepts and the graph ignores.** Both were
found by rendering, both would have shipped silently, and neither raises anything. Two instances is
enough to call it a property of this build rather than a coincidence.

The long connect form — `from_node`/`from_socket`/`to_node`/`to_socket` — fails on a socket named
`volume`, so the space-free form is not usable here either.

## Not measured

- Whether the material's own `Volume` output could be reached by a route that never names the word
  `volume` — no socket name for it was found.
- Whether another build registers the two missing nodes. The claim is about **this** executable.

## Sources

- The measurement: `out/probe-volume-fog.py`, whose frames are `out/vol-*.png`; identity checked
  with `out/compare-frames.py`.
- The exporter's silence on volumes: `crates/scene/src/cycles_export` carries no `volume` string
  at all, which is why this was open rather than known.
- The ratios: `out/probe-linear-radiance.py` on `out/sky-foldyard-camera0.png` and
  `out/gb-tidewater-fixed-camera0.png` against their traced frames.
- The rule that put the haze in the post stage: `.agents/SPEC/render/cycles_backend.md`, *the
  geometry pipeline ships geometry, and the detail is baked later*.