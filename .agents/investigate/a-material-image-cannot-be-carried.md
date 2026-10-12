# A material's image cannot be carried to the tracer in this build

Measured 2026-10-12, against Cycles 5.3.0 at 32 samples on CPU.

## The question

`tiles` is the one gallery scene whose ground is still out by a large factor the other fixes do not
explain — a linear radiance ratio of 1.285, against 0.95 to 1.05 for `sunlight`, `spotlight`,
`dunes` and `landscape`. Its exporter drops the material's `image` and the mesh's `texture`
attribute and reports both, so the tracer draws a flat floor. Is that a limitation of this build or
a gap in `scene::cycles_export`?

## What the exporter says, and whether it is true

`crates/scene/src/cycles_export/material.rs` refuses the field and gives the reason: **a socket
whose name contains a space cannot be connected at all.** Every colour socket on a
`principled_bsdf` is named for its user interface — `Base Color`, not `base_color` — so an image
arriving over a connection has nowhere to land.

That was an assertion in a comment. It is testable, so it was tested: a `mix_color` connected into a
`principled_bsdf`'s base colour, three spellings, everything else held identical.

| target spelling | what Cycles says | what the frame shows |
|---|---|---|
| `Base Color` | `ERROR: Invalid from or to value for connect node` | unaffected |
| `base_color` | **nothing at all** | the BSDF's default colour — the mixed one never arrives |
| `BaseColor` | `ERROR: Unknown input socket name "BaseColor" on "bs"` | unaffected |

**The reason is true.** The space is what the parser splits on, and `Invalid from or to value`
is exactly what a split produces. Removing the space does not get around it — it only gets around
the error, and that is worse: the most natural spelling is the one that fails silently.

## What this settles, and what it does not

`tiles`' flat floor is a real limitation of the target format in this build, not a bug in the
exporter. The exporter refusing the field, and reporting it rather than dropping it, is correct
behaviour and should stay.

It does **not** settle that no route exists. The block is specific to a connection whose **target**
is a colour socket, and every candidate target in Cycles carries a space. A route that never
connects into a colour socket — building the material out of nodes whose outputs are already
`bsdf`, for instance — is **not measured** and is the first thing to try if the question is reopened.

It also does not settle what `tiles` should do. Its 1.285 is now explained as *two different
surfaces* rather than one surface at two brightnesses, which is a better thing to know and is not
the same as being fixed.

## Why it was worth measuring

`cycles-procedural-materials.md` recorded that connect targets are validated and sources are not.
That is incomplete in a way that matters: a target that is a real socket with its space removed is
accepted silently. A writer following the note would write `base_color`, see no error, and ship a
flat material. That bullet is amended in place.

## Sources

- The refusal and its stated reason: `crates/scene/src/cycles_export/material.rs`, the `IMAGE`
  arm and the module documentation above it.
- The measurement: `out/probe-base-colour-socket.py`, which is `out/probe-basecolor.py` with the
  spelling as the only variable. Its four frames are `out/bs-*.png`.
- The scene: `crates/gallery/src/tiles.rs`, which asks for `shaders::textured`.
- The exported XML, confirming the absence directly: `out/gb-tiles-traced.cycles.xml` carries no
  `image_texture`, no `tex_coord` and no `mapping` node at all.
- The ratio: `out/probe-linear-radiance.py` on `out/gb-tiles.png` against `out/gb-tiles-traced.png`.