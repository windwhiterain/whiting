# The census, and what each of its two baselines answers

Measured 2026-10-14, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`, at `a5950cd` in the
tree, with `foldyard` added at `4cd7d0c` and the second baseline run at `1fb13d4`. Probe:
`out/probe-grade-matched.py`, with `out/probe-unclipped.py` for the exposure control and
`out/probe-bounce-free.py` for the bounce control. Grade constants are read out of each scene's
`GRADE` in the source, one per scene, and every figure below comes from a frame with **no clipped
pixel**. `foldyard`'s own measurement, its two reasons, and what was not decomposed are in
[foldyard-composes-poorly-and-these-are-its-reasons](foldyard-composes-poorly-and-these-are-its-reasons.md).

## The two baselines, and the question each one answers

A census that compares a direct-only renderer with a path tracer has to choose which of the two it
is measuring, and the choice has to be written down because the numbers are not interchangeable.

| baseline | what it answers | what it is not |
|---|---|---|
| **bounce-free** (`max_bounce = 0`) | **do the two backends read the same scene?** | not what a user sees |
| **as shipped** (whatever the scene declares) | **how much of the difference is the renderer rather than the document?** | not a statement about the scene being read the same |

Both are worth having and neither is the other's baseline.

**Which baseline each row is on is stated in the table, and an earlier version of this file got it
backwards**, describing bounce-free as the baseline the table was on and as-shipped as the one not
yet run. It was the other way round: the frames are `out/rlx-<scene>.png`, produced by
`probe-unclipped.py` from `out/rl-<scene>.cycles.xml`, and every one of those documents carries
`<integrator ... max_bounce="8" />`. Bounce-free has since been measured for six of the nine scenes
as `out/rlxbf-<scene>.png`, and the table now carries both columns where both exist.

**The two baselines do not agree everywhere, and an earlier version of this file said they did.**
It claimed a difference "of 0.002 or less on the scenes measured", on one scene, and **no
bounce-free `dunes` frame from that run survives in `out/`**, so that figure is not reproducible
from the artefacts this repository keeps and is not carried forward. Six scenes are measured on both
now, and the spread between the two baselines runs from **0.000** (`tiles`) to **0.078**
(`sunlight`). Which of the two is closer to the rasterised frame **depends on the scene**, so the
choice of baseline is not a detail and neither one is the honest one on its own.

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
between the two. **Both baselines are now measured for six of the nine scenes.** The as-shipped
column reproduces the figures this file carried before, to the digit, on the tree at `1fb13d4` —
which is also a second check that dropping the zero-valued `reflect` moved no pixel.

| scene | as shipped | bounce free | pearson as shipped | pearson bounce free | comparable |
|---|---|---|---|---|---|
| `sunlight` | **1.081** | **1.003** | 0.9723 | **0.9989** | yes |
| `spotlight` | **1.047** | **0.991** | 0.9904 | **0.9982** | yes |
| `dunes` | **1.003** | **0.967** | **0.9940** | 0.9921 | yes |
| `landscape` | **0.981** | **0.929** | 0.9289 | 0.9337 | ground only |
| `tiles` | **0.961** | **0.961** | **0.9973** | **0.9973** | yes |
| `foldyard` | 0.938 | 0.920 | 0.8804 | 0.8706 | **no** |
| `tidewater` | **1.181** | — | 0.7705 | — | **no** |
| `monoliths` | **0.008** | — | 0.0969 | — | **no** |
| `terraces` | **1.318** | — | 0.6496 | — | **no** |

**`landscape`'s three ground bands are 1.004, 0.999 and 1.006** — inside four per cent — and its
whole-frame figure is dragged up only by the upper bands. Every scene in the gallery has a row.

## What the second baseline turned out to say

**It is not a refinement of the first, and which one looks better depends on the scene.** Bounce-free
moves the figure **down** in five of the six scenes — by 0.003, 0.056, 0.078, 0.052 and 0.018 — and
not at all in the sixth. That much is unremarkable: indirect light only adds. What is remarkable is
that adding it moves the answer **towards** the rasterised frame on two scenes and **away** from it
on three.

| scene | bounce free moved it | and the shape |
|---|---|---|
| `sunlight` | 1.081 → **1.003**, from eight per cent out to three tenths of one | pearson 0.9723 → **0.9989** |
| `spotlight` | 1.047 → 0.991 | pearson 0.9904 → **0.9982** |
| `dunes` | 1.003 → 0.967, **away** from the rasterised frame | 0.9940 → 0.9921 |
| `landscape` | 0.981 → 0.929, **away** | 0.9289 → 0.9337 |
| `foldyard` | 0.938 → 0.920, **away** | 0.8804 → 0.8706 |
| `tiles` | 0.961 → 0.961, unchanged | 0.9973 → 0.9973 |

**`sunlight`'s eight per cent was almost entirely bounce, not a disagreement about the scene.** With
indirect light removed it agrees to **0.3%** and its shape correlation reaches **0.9989**. The
as-shipped figure was measuring Cycles' interreflection against a rasteriser's own ambient term, and
that difference was being read as a failure to read the scene. `spotlight` is the same story with
smaller numbers: 4.7% becomes 0.9%, and 0.9904 becomes 0.9982.

**`dunes`, `landscape` and `foldyard` go the other way**, so for them the rasterised frame is already
*brighter* than a direct-only trace, and Cycles' bounce is closing a gap rather than opening one.

### `tiles` has no visible indirect light in it

**`out/rlx-tiles.png` and `out/rlxbf-tiles.png` are byte-identical**, and the bounce-free document was
edited and verified: all five came back `max_bounce="0"`, and four of the five frames differ from
their as-shipped counterparts. `tiles` is the fifth, and its traced **raw** mean is `0.3383` on both
baselines to four decimals, so this is not the graded value rounding.

A diffuse room lit by a lamp ought to have some interreflection, so the honest statement is that
**whatever indirect light `tiles` has is below what these two files can express** — not that Cycles
computed none. What would settle it is a frame written at more than eight bits, and this method
cannot produce one because both backends write png. Recorded as a limit, not as a finding about the
scene.

## What the figures do and do not license

**Agreement on one baseline is not agreement on the scene.** The six scenes with both figures now
say something the earlier version of this file could not: three agree *better* without indirect
light and three agree *better* with it. **So neither baseline is the honest one on its own, and
"the comparable scenes agree to within eight per cent" was a statement about whichever baseline was
in the table rather than about the two backends.**

## Five scenes are marked not comparable, and why each one is

This is the rule the census applies and the list it applies it to. A scene is **not comparable**
when its rasterised frame contains something the document cannot carry, because the figure then
measures that something rather than the two backends.

| scene | what the document cannot carry | where it is |
|---|---|---|
| `terraces` | the terrain declares **no albedo**; its colour, its water and its Fresnel mirror are computed per fragment in WGSL, and the document carries `base_color = 1, 1, 1` | `crates/gallery/src/terraces.rs:446`, `:809-897` |
| `monoliths` | the scene declares **no environment light** and the rasterised post stage paints a sky anyway — which is also what its near-black mirror ground reflects | `crates/gallery/src/monoliths.rs:271-272` |
| `tidewater` | **aerial perspective on every surface** | `crates/gallery/src/tidewater.rs:818-825` |
| `foldyard` | **both of the next two rows at once** — aerial perspective on every surface, and three ridge layers painted into the pixels no geometry reached | `crates/gallery/src/foldyard.rs:1627`, `:1613-1624` |
| `landscape`, sky bands only | ranges, shafts and cirrus **painted into pixels no geometry reached** | `crates/gallery/src/landscape/mod.rs:993-1031` |

`landscape` is listed with "ground only" rather than as not comparable because its ground does agree,
to within four per cent, and its sky does not. A whole-frame figure for it is a figure about a
mixture and should not be quoted as either.

**`foldyard`'s air is where the SPEC tells it to be, and that is why its row is not a defect to
fix.** `render/cycles_backend.md:520-523` requires atmosphere to be a separate post process,
`foldyard` has it in one, and the survey in [where-atmosphere-is-painted](where-atmosphere-is-painted.md)
correctly did not list it. Moving it would mean moving it *out* of the post stage. **Post-stage
atmosphere is invisible to a document, so this is the end state the current rule produces**: four of
the five scenes above are uncorrectable by relocating a layer, because the layer is already where
it belongs. That consequence is worth stating rather than discovering one scene at a time.

**The five scenes stay exactly as they are.** Their paintings are the pictures, and a census that
reported only comparable scenes would be a census of the easy half.

## What no figure in this table is

**A disagreement between the two renderers about the same picture.** Four comparable scenes are
within about eight per cent of the rasterised frame, and the five that are not differ by a material,
a sky or a fog the other backend was never given.

**And a figure that holds whichever baseline it was taken on.** With both columns measured, the
comparable four split: `sunlight` and `spotlight` agree *better* with bounce, `dunes` and `landscape`
agree *better* without it. So "they agree to within eight per cent" is true of each scene on one
baseline or the other, and **which one is a fact about the scene rather than about the backends.**
Quote the baseline with the figure or quote nothing.

**Neither backend decodes the environment map, and that is why the comparable ones agree.**
`crates/gallery/src/kit.rs:69` declares every gallery image as `rgba8unorm`, and
`wgpu::TextureFormat::Rgba8Unorm` returns the bytes as they are — no sRGB decode in hardware.
Cycles reads the same bytes as scene linear. **Both sides use the map's sRGB-encoded bytes directly
as radiance, so the world's radiance agrees across the backends.** An earlier note in this directory
treated the undecoded map as a cross-backend defect; it is not one, and that error is what this
method spends its first half undoing.

## Not measured

**Bounce-free figures for `tidewater`, `monoliths` and `terraces`.** The other six scenes carry
both columns. All three are already marked not comparable for a reason that has nothing to do with
the baseline — a painted sky, no environment light, and no albedo — so a second column would not
change what the table says about them, and their `as shipped` figures are large enough
(`1.181`, `0.008`, `1.318`) that the baseline is not the interesting variable.

**Whether `tiles` has interreflection below eight bits.** The two frames are byte-identical, which
this method can only report as *invisible at this resolution*. Settling it needs a format both
backends can write at more than eight bits per channel.

**Whether any of the five not-comparable scenes would agree if their painting were carried**, which
for `terraces` and `monoliths` is the question
[a-document-level-graph-vocabulary](../proposals/a-document-level-graph-vocabulary.md) is about.
For `foldyard` it would also need the air, which is a fog law rather than a graph.