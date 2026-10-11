# Why landscape agreement fell from +0.6223 to +0.2339

measured 2026-10-11 on `d993bc7` (main). probe: `out/probe-agreement.py`, 1024x1024, 32
samples on CPU. sources: `git merge-base`, the branch tip's own file contents, and the A/B
below.

## The question

`landscape` agreed with itself across the two backends at **+0.6223** when the sky work
landed. After `d993bc7` it agrees at **+0.2339**. What changed?

## The answer

`e447f85` ("Shade the massif as a plain grey diffuse, and author it as a tree") gave the
massif a **per-fragment surface body**, `mountain_shade`. It is the only change to a
shading law in that commit; everything else in it is geometry and a material tint.

That body is what costs the 0.37, and nothing else in the commit is. Geometry and body
were separated by A/B, with the geometry held fixed:

| raster massif | whole frame | lower 62% | lower 38% |
|---|---|---|---|
| `mountain_shade` (main, as committed) | **+0.2339** | +0.0740 | +0.0508 |
| empty body, new geometry tree | **+0.6029** | +0.5159 | +0.5854 |

The tree rewrite is therefore worth about **0.02**, which is what remains between +0.6029
and the +0.6223 measured before it. The body is worth about **0.37**.

The measure is `git checkout --`-clean: the second row was taken by editing only the one
`stage_set("massif", ...)` argument and restoring the file afterwards.

## The body's stated premise is false on main

`mountain_shade`'s own documentation gives the reason it exists:

> The shared lighting stage's ambient term is fed by the scene's environment light, and
> this scene names none — which the stage answers with **black**, by design and on
> purpose.

The scene names one. `crates/gallery/src/landscape/mod.rs:671` declares

```rust
environment_light("air", "air", [1.0; 3], 0.55)
```

and that line arrived in `5fb5c12` ("Give the landscape scene a sky the path tracer can
read"), which is **not** an ancestor of `e447f85`:

```
$ git merge-base 5fb5c12 e447f85
77b3fc2fe6d8dc34b3f2a6d90ad338cfd3b8e9e4
$ git show e447f85:crates/gallery/src/landscape/mod.rs | grep -c environment_light
0
```

So `feature/dissected-plateau` forked at `77b3fc2`, before the sky work, and the massif was
authored against a landscape that had no sky to light it. `d993bc7` merged the two and the
body's justification went with the branch it was written on.

This is a **merge-time premise collision**, not a mistake in either commit. Read on the
branch that wrote it, `mountain_shade` is a reasonable answer to a black shadow side. On
main that shadow side is no longer black, and the body is a law the tracer cannot run.

## The shadow side the body was written for is already grey

Measured on the A/B frame with the body emptied (`out/nb.png`), the massif's shadowed
flanks read 93 mean luma and its lit flanks 188-191: a grey with a lit side, not a
surface with a black side. The dark blue in those flanks is the environment light the
body's comment says is absent.

## What the body costs, in kind rather than in correlation

The body is `albedo * (sky_term + sun * max(dot(n, l), 0))` with `sky_term` and `sun`
written as Rust constants. Cycles reads the material's `tint` and builds its own diffuse
from the environment light and the sun. The two are not the same law, and there is no way
to make them one: `MOUNTAIN_SKY` and `MOUNTAIN_SUN` are literals in the raster shader and
`principled_bsdf` has no socket for either.

This is the defect `SPEC/render/cycles_backend.md:396` ("the geometry pipeline ships
geometry, and the detail is baked later") exists to prevent. That decision says
`landscape` draws "**both meshes with no surface body at all**". `e447f85` reverses it for
one of the two.

## A side finding: the striation note in the SPEC no longer describes these frames

`cycles_backend.md:434` records that the stripes along the massif's flanks are geometry
and that **both backends draw them**. That was measured on the old massif with no body.
On current main the striations appear on the **Cycles side only**, and a scanline across
one lit flank (`out/probe-scanline.py`, y=620) separates the two ways the faces read:

| x | 300 | 325 | 350 | 375 | 400 | 425 | 450 | 475 | 500 | 525 |
|---|---|---|---|---|---|---|---|---|---|---|
| raster | 190 | 190 | 191 | 191 | 191 | 191 | 191 | 191 | 191 | 191 |
| cycles | 255 | 243 | 245 | 236 | 242 | 179 | 220 | 238 | 247 | 243 |

Raster reads one value per face across ~275 pixels. Cycles resolves within the face. What
separates them was **not** isolated here: a difference in the interpolated normal field
and the tracer's self-occlusion between the tessellation's slivers both predict this, and
telling them apart needs a controlled render that was not run. Recorded as open rather
than guessed.

Note also that the raster values in that row are **not** clipped — 191 is well under 255 —
so the earlier finding that the eight bit target clips the massif
(`shaded-image-eight-bit-ceiling.md`) is not what hides the Cycles detail here.

## Reproducing

```
cargo run --release -p whiting-device-rasterize --example render -- landscape out\lmr
cargo run --release -p whiting-device-cycles --example render_cycles -- landscape out\lmc.png 32 CPU
python out\probe-agreement.py out\lmr.png out\lmc.png
```

Then change the `stage_set("massif", ...)` body argument in
`crates/gallery/src/landscape/mod.rs` to `""`, rebuild, and run the same three commands.
