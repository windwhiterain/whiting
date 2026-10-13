# The census, and what each of its two baselines answers

Measured 2026-10-14, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`, at `a5950cd` in the
tree. Probe: `out/probe-grade-matched.py`, with `out/probe-unclipped.py` for the exposure control and
`out/probe-bounce-free.py` for the bounce control. Grade constants are read out of each scene's
`GRADE` in the source, one per scene, and every figure below comes from a frame with **no clipped
pixel**.

## The two baselines, and the question each one answers

A census that compares a direct-only renderer with a path tracer has to choose which of the two it
is measuring, and the choice has to be written down because the numbers are not interchangeable.

| baseline | what it answers | what it is not |
|---|---|---|
| **bounce-free** (`max_bounce = 0`) | **do the two backends read the same scene?** | not what a user sees |
| **as shipped** (whatever the scene declares) | **how much of the difference is the renderer rather than the document?** | not a statement about the scene being read the same |

Both are worth having and neither is the other's baseline. **Every figure in this file is
bounce-free where a bounce-free frame exists**, because that is the baseline that isolates the
document, and the bounce-free renders differ from their bounced ones by 0.002 or less on the scenes
measured, so the two baselines agree wherever both exist.

## How a frame is read, and why each step is there

Comparing two files as they sit on disk answers two questions at once — whether the renderers
computed the same picture, and whether they agree about what a linear radiance looks like at eight
bits. Three steps separate those.

1. **Each frame through its own writer's output.** Cycles reports `Color management disabled` and
   copies radiance straight into the png: an `<emission>` of strength 1.0 reads **1.00000** in the
   file, which is how that is known rather than assumed. The rasterised frame has been through its
   scene's `GRADE`, so the traced frame is put through that same curve before anything is compared.
2. **Nothing clipped.** `<film exposure="0.25" />` and divide the exposure back out. Without it,
   `dunes` has **39.09%** of its pixels with a channel at 255, `landscape` 11.31% and `terraces`
   10.49%, while **no rasterised frame has a single pixel at 255**. A ratio measured between a
   clipped image and an unclipped one is not a ratio between two renderers.
3. **Bounce-free**, where a frame exists.

## The figures

`graded/rasterised` is a whole-frame mean after step 1, and `pearson` is the correlation of luma
between the two.

| scene | graded/rasterised | pearson | comparable |
|---|---|---|---|
| `dunes` | **1.003** | **0.9940** | yes |
| `spotlight` | **1.047** | **0.9904** | yes |
| `sunlight` | **1.081** | **0.9723** | yes |
| `landscape` | **0.981** | 0.9289 | ground only |
| `tiles` | **0.961** | **0.9973** | yes |
| `monoliths` | **0.008** | 0.0969 | **no** |
| `tidewater` | **1.181** | 0.7705 | **no** |
| `terraces` | **1.318** | 0.6496 | **no** |
| `foldyard` | — | — | not measured |

**Five agree, to between one and eight per cent.** `landscape`'s three ground bands are 1.004, 0.999
and 1.006 — inside four per cent — and its whole-frame figure is dragged up only by the upper bands.

## Four scenes are marked not comparable, and why each one is

This is the rule the census applies and the list it applies it to. A scene is **not comparable**
when its rasterised frame contains something the document cannot carry, because the figure then
measures that something rather than the two backends.

| scene | what the document cannot carry | where it is |
|---|---|---|
| `terraces` | the terrain declares **no albedo**; its colour, its water and its Fresnel mirror are computed per fragment in WGSL, and the document carries `base_color = 1, 1, 1` | `crates/gallery/src/terraces.rs:446`, `:809-897` |
| `monoliths` | the scene declares **no environment light** and the rasterised post stage paints a sky anyway — which is also what its near-black mirror ground reflects | `crates/gallery/src/monoliths.rs:271-272` |
| `tidewater` | **aerial perspective on every surface** | `crates/gallery/src/tidewater.rs:818-825` |
| `landscape`, sky bands only | ranges, shafts and cirrus **painted into pixels no geometry reached** | `crates/gallery/src/landscape/mod.rs:993-1031` |

`landscape` is listed with "ground only" rather than as not comparable because its ground does agree,
to within four per cent, and its sky does not. A whole-frame figure for it is a figure about a
mixture and should not be quoted as either.

**The four scenes stay exactly as they are.** Their paintings are the pictures, and a census that
reported only comparable scenes would be a census of the easy half.

## What no figure in this table is

**A disagreement between the two renderers about the same picture.** Five scenes agree to between
one and eight per cent with the encodings and the clipping accounted for, and the four that do not
agree differ by a material, a sky or a fog the other backend was never given.

**Neither backend decodes the environment map, and that is why the comparable ones agree.**
`crates/gallery/src/kit.rs:69` declares every gallery image as `rgba8unorm`, and
`wgpu::TextureFormat::Rgba8Unorm` returns the bytes as they are — no sRGB decode in hardware.
Cycles reads the same bytes as scene linear. **Both sides use the map's sRGB-encoded bytes directly
as radiance, so the world's radiance agrees across the backends.** An earlier note in this directory
treated the undecoded map as a cross-backend defect; it is not one, and that error is what this
method spends its first half undoing.

## Not measured

**`foldyard`**, whose document is sixty four megabytes.

**As-shipped figures for every scene.** Only the bounce-free baseline was run across the gallery;
the two baselines agree to within 0.002 wherever both exist, but that was checked on `dunes` alone.

**Whether any of the four not-comparable scenes would agree if their painting were carried**, which
for `terraces` and `monoliths` is the question
[a-document-level-graph-vocabulary](../proposals/a-document-level-graph-vocabulary.md) is about.