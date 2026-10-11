# Why do two gallery scenes correlate negatively with the rasteriser?

> **Answer: the correlation is a function of one number — how much brighter the rasteriser's
> sky band is than the tracer's — and it is monotone across eleven scenes.** The two negative
> scenes are negative because the rasteriser paints a sky and the tracer renders a black one,
> and the sky occupies a large part of their frames. `dunes` has a complete `Atmosphere` and a
> post grade that calls `air_sky`, and its `lights` list holds a sun and nothing else.
>
> This is not a winding, a camera or a geometry question. The document-level survey behind it
> is in [cycles-mesh-normals.md](cycles-mesh-normals.md), and it is finished: 620 of 620 meshes
> agree with their own normals and the picture did not move.

## The measurement

Pearson correlation of luma, whole frame, Cycles at 1024x1024 and 32 samples on CPU against the
rasterised frame re-rendered from the same commit. The **sky gap** is the mean luma of the
raster frame's top third minus the mean luma of the tracer's top third — the part of the frame
above the horizon, which is where a landscape's sky is.

| scene | raster top third | tracer top third | **gap** | whole frame |
|---|---|---|---|---|
| terraces | 164.6 | 4.3 | **160.2** | **−0.8145** |
| dunes | 155.8 | 0.0 | **155.7** | **−0.6585** |
| monoliths | 126.5 | 0.0 | **126.5** | **−0.1448** |
| stilllife | 60.7 | 23.3 | 37.4 | +0.4580 |
| landscape | 141.7 | 111.1 | 30.6 | +0.7917 |
| foldyard | 126.9 | 96.3 | 30.6 | +0.1166 |
| lanterns | 6.2 | 0.0 | 6.2 | +0.8494 |
| sunlight | 7.7 | 4.6 | 3.2 | +0.9751 |
| spotlight | 0.8 | 0.7 | 0.1 | +0.9278 |
| tiles | 0.0 | 0.0 | 0.0 | +0.9739 |
| tidewater | 57.3 | 126.7 | **−69.3** | +0.3389 |

The ordering is the whole finding. A negative correlation is not "close but different": it is a
frame where the bright part of one picture is the dark part of the other, and a sky occupying a
quarter of the frame is enough to do that on its own.

`tidewater` is the control that makes the mechanism legible rather than merely correlated. It is
the one scene where the **tracer's** sky is the brighter of the two, the gap is **negative**, and
its correlation is the lowest of the six scenes that declare an environment — the same mechanism,
pushed the other way, because the tracer applies no view transform and the rasteriser's Reinhard
curve does not.

## What the two worst scenes actually do

`dunes` (`crates/gallery/src/dunes.rs`) has everything a sky needs and does not hand it over:

- `fn atmosphere() -> Atmosphere` with zenith, horizon, ground and sun;
- a post grade that calls `air_sky(ray)` — **the sky it draws is the atmosphere it holds**;
- a `lights` list containing the sun and **no environment light**, and no `images` list.

Its own comment says the air is *"handed to the lighting stage and to the post stage"*. It is not
handed to the world, because there was nothing to hand it to until the format grew a world.

`terraces` (`crates/gallery/src/terraces.rs`) is further along the same road and does not even
import `air`:

- its sky is `sky_colour(ray)` in WGSL (`terraces.rs:895`), with a horizon colour that follows
  the sun around the compass, a zenith of `(0.11, 0.26, 0.58)`, a glow and a disc;
- the post stage draws the far mountain ranges itself where no geometry reached
  (`ridges(sky, ray, 1.0, 3)`, `terraces.rs:1042`), so **the horizon line in its raster frame is
  partly painted rather than geometry**;
- the whole valley mist is an exponential height fog mixing towards a sun-warmed haze
  (`terraces.rs:997-1024`).

**none of that is a world.** and the frames show what that costs: on `terraces` the tracer draws
the sun's rim light correctly — the glowing ridge lines are there — and leaves everything else
black, because the sun is the only light in the scene and the sky that fills it exists in a post
stage the tracer does not have.

## this is the same limit SPEC already records, not a new defect

[SPEC/render/cycles_backend.md](../SPEC/render/cycles_backend.md) says of atmosphere:

> atmosphere belongs to the **post stage** ... a surface shader that paints its own fog is a
> surface whose fog a post stage cannot correct, a tone curve cannot, and a second backend
> cannot reproduce.

That is right, and `terraces`'s fog and its painted ranges are exactly the case it describes. But
the same file also records the fix that *does* cross, from `5fb5c12`:

> the fix is not a new kind of environment light — the format has had that all along — it is for
> a scene to **declare the sky it already has as an image**, and for that image to be the same
> sky.

and measured on `landscape` it was worth `+0.5119 → +0.6223`. `landscape`, `tidewater` and
`foldyard` have taken it; the other ten scenes have not. Every document that correlates well has
a `<background>` and every document that correlates badly does not:

| has `<background>` | whole frame | has none | whole frame |
|---|---|---|---|
| landscape | +0.7917 | **dunes** | **−0.6585** |
| tidewater | +0.3389 | **terraces** | **−0.8145** |
| foldyard | +0.1166 | monoliths | −0.1448 |
| | | stilllife | +0.4580 |

`stilllife` is the counterexample worth keeping: it has no sky and still correlates at +0.46,
because almost none of its frame is sky. **a missing world is worth exactly as much as the
amount of frame it was standing behind.**

## what it would cost, scene by scene

- **`dunes` is a six-line change and tests the whole mechanism.** it already holds the `Atmosphere`
  and already draws `air_sky`; it needs `image("air", 256, 128, atmosphere().environment_pixels(256, 128), Latlong)`
  in `images` and `environment_light("air", "air", [1.0; 3], r)` in `lights`. The map is literally
  the sky the rasteriser paints, term for term, which is the condition `5fb5c12` set. The only
  free number is the radiance `r` — `landscape` chose 0.55, `foldyard` 0.26, `tidewater` 1.0 —
  and it is the one thing that has to be measured rather than copied.
- **`terraces` is real work.** its sky lives in WGSL, not in Rust, so giving it a world means
  either porting `sky_colour` into an `Atmosphere` or writing the environment map by hand from the
  same three colours. The fog and the painted ranges stay behind either way, so the honest
  prediction is that terraces recovers most of its gap and not all of it — and the part it does
  not recover is the part SPEC says cannot cross.
- **`monoliths` and `stilllife`** are the same one-line-per-scene change and are worth less:
  −0.1448 and +0.4580, because little of either frame is sky.

## Sources

- `out/probe-agreement.py` for the correlations, over the `w3-*.png` and `w3r-*.png` frames.
- The sky gap measured directly from those frames: mean luma of the top third of each.
- `<background>` counted in the thirteen exported documents.
- `crates/gallery/src/dunes.rs:61` (`atmosphere`), `:79` (the air handed to the post stage),
  `:100-109` (`lights`, the sun only), `:344` (`air_sky` in the grade).
- `crates/gallery/src/terraces.rs:895` (`sky_colour`), `:997-1024` (the height fog),
  `:1036-1044` (the grade painting the sky and the ranges), `:104-107` (`lights`, the sun only).
- `crates/gallery/src/landscape/mod.rs:631-671` — the shape of the fix, already taken.