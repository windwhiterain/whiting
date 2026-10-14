# What accounts for foldyard composing poorly across the two backends

Measured 2026-10-14, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`, at `4cd7d0c` in the
tree. Probes: `out/probe-grade-matched.py` (with `foldyard`'s grade added to its table from source),
`out/probe-unclipped.py` and `out/probe-bounce-free.py`. Grade read out of
`crates/gallery/src/foldyard.rs:1637-1654`. Traced frame `out/rlx-foldyard.png`, rasterised frame
`out/rlf-foldyard-camera0.png`, both 1536 by 1024.

## The question

`.agents/SPEC/render/cycles_backend.md:867-870` records, for a different change:

> `foldyard` composes poorly across the two backends for its own reasons and did before this; it
> is a separate finding, not a regression here.

It names no reasons. This file measures the scene and names them, which is what turns the SPEC's
sentence into something a reader can act on.

## The figures

Both baselines at 32 samples on CPU, both rendered at `<film exposure="0.25" />` and divided back
out, so **no pixel of either traced frame is clipped**: 0.00% have a channel at 255. That is the
control — without it a ratio would be measured against a frame that had already lost its
highlights.

| baseline | whole frame | pearson |
|---|---|---|
| as shipped (`max_bounce = 8`) | **0.938** | **0.8804** |
| bounce free (`max_bounce = 0`) | **0.920** | **0.8706** |

Bands, as shipped, then bounce free:

| rows | as shipped | bounce free | pearson (as shipped) |
|---|---|---|---|
| 0-128 | 0.949 | 0.949 | 0.6592 |
| 128-256 | 0.922 | 0.922 | 0.2897 |
| 256-384 | 0.666 | 0.666 | 0.3972 |
| 384-512 | 0.800 | 0.785 | 0.6163 |
| 512-640 | 0.972 | 0.953 | 0.4930 |
| 640-768 | 1.159 | 1.140 | 0.1465 |
| 768-896 | 0.983 | 0.951 | 0.4314 |
| 896-1024 | 0.917 | 0.892 | 0.4925 |

**The two top bands are identical to four decimals between the baselines, and that is the check
that the rest of the table is about geometry.** A pixel the world fills receives no indirect light
from anything, so removing every bounce cannot move it — and it does not move. The bands that do
move are the ones with surfaces in them.

**The two baselines differ by 0.018 here, against the 0.002 the census quotes for `dunes`.** A yard
lit by a point sun four hundred units off is a scene where indirect light is a large part of the
picture, and bounce-free makes `foldyard` *worse* (0.920 against 0.938), not better. The census's
statement that the two baselines agree wherever both exist was measured on `dunes` alone and does
not hold gallery-wide.

## The two reasons, each from source

### Its atmosphere is in the post stage, and the document has no field for a fog law

`crates/gallery/src/foldyard.rs:1627`:

```wgsl
color = mix(color, air_colour(), air_closure(uniforms.camera.position, world));
```

This runs on **every pixel geometry reached**, keyed on the distance from the eye to that
fragment's own recovered world position (`air_closure`, `crates/gallery/src/air.rs:299-304`). It
mixes towards `AIR_HORIZON`, which is `[0.600, 0.588, 0.560]` (`foldyard.rs:1465`) — about **0.65**
once this scene's own grade has been applied. `foldyard`'s source says how much of the frame this
reaches: the modelled far range stands a hundred and seventy units off "and the closure there is
over sixty percent" (`foldyard.rs:1616-1618`).

**This is compliant with the SPEC, and it is worth being exact about why.**
`cycles_backend.md:520-523` requires atmosphere to be a separate post process, and
`crates/gallery/src/dunes.rs:346-347` words the older form of the rule as *no surface **body** may
reach for `air_colour()` or `air_closure()`*. `foldyard` reaches for both from its **post stage**,
which is where the rule wants them. The 2026-10-13 survey in
[where-atmosphere-is-painted](where-atmosphere-is-painted.md) did not list `foldyard`, and that is
correct rather than an omission: it looked for a surface body computing distance-from-eye, and
`foldyard` has none.

**Compliant and uncorrectable are different facts, and the second one is what the census is
measuring.** `tidewater` was moved to the post stage on this rule, and is still marked not
comparable for exactly this reason. Post-stage atmosphere is invisible to a document.

### It paints three ridge layers into the pixels no geometry reached

`crates/gallery/src/foldyard.rs:1613-1624`: where `depth >= 1.0`, the stage writes
`ridges(environment_sky(ray), ray, RIDGE_SCALE, RIDGE_LAYERS)` with `RIDGE_LAYERS = 3`. Those
layers are in pixels no surface reached, so the tracer has nothing to trace there and draws the
world instead. This is the same fault as `landscape`'s ranges and `monoliths`' sky.

## What this scene is **not** wrong about

**The `reflect` warning on 39 of its materials is noise, and it is not a reason.** The emitter
reports `material '…' declares custom data 'reflect', which has no principled_bsdf socket to fold
into` 39 times on this scene. Every one of those 39 materials declares **`0.0`**: `foldyard` builds
its materials through `Group::new(mesh, tint, roughness, reflect)` and all thirteen groups pass
`0.0` for it, including `far-range` at `foldyard.rs:1249`. Dropping a declared zero loses nothing,
so the report is describing a non-event, 39 times.

The mapping it declines to make is separately settled and separately measured, in
[cycles-emitter-dropped-material-fields](cycles-emitter-dropped-material-fields.md): the document's
`reflect` is how much sky a surface shows **at a grazing angle**, `principled_bsdf`'s
`Specular IOR Level` is the reflectance at **normal incidence**, and writing it made `landscape`
worse on both bands.

## Not measured

**Which reason owns which band.** The two causes above are confirmed from source and the
whole-frame figures are measured, but nothing here decomposes the 0.666 at rows 256-384 from the
1.159 at rows 640-768 into "air" and "painted ridges". Separating them needs a render of a
`foldyard` with the post stage altered, which is a change to the gallery and was not made.

**The other two cameras.** `foldyard` declares three (`foldyard.rs:1336`) and the emitter takes
`cameras.first()` (`cycles_export/scene.rs:308`), so only camera 0 has a traced frame. Cameras 1
and 2 are rasterised only.

## What it is worth

**`foldyard` joins the not-comparable scenes, on two grounds that are already in the census's own
table** — post-stage atmosphere and a painted sky. It is the last scene in the gallery with no row,
and it does not close the census: with `foldyard` in it, **five of nine gallery scenes are not
comparable**, and four of those five are uncorrectable by moving a layer, because the layer is
already where the SPEC requires it to be.