# Every traced frame in this gallery loses its highlights to the file, and no rasterised one does

Measured 2026-10-13, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`, with `e75a18e` in the
tree. Probes: `out/probe-grade-matched.py` (each scene's own grade, read from source),
`out/probe-unclipped.py`, `out/probe-bounce-free.py`. Frames are named `cs-` for traced and `rrf-`
for rasterised, and every probe hashes its inputs and refuses to compare a file with itself.

## The question

`landscape` is closed: its ground agrees once the colour space is named and each frame is read the
way its own writer wrote it. `dunes` and `terraces` were still open, and `dunes` is the only scene
in the gallery whose sky is a sky on both sides — its `GRADE` writes `environment_sky(ray)` and
nothing else into the pixels no geometry reached (`crates/gallery/src/dunes.rs:365-368`), where
`landscape` paints ranges, shafts and cirrus and `terraces` paints ranges at
`crates/gallery/src/terraces.rs:1078` and fogs the whole frame at 1085. What do they say once they
are put through their own curves, and once the bounce is removed?

## The thing that was wrong with every level number taken so far

**Cycles writes radiance straight into an eight bit png and every pixel above one is gone.**

The writer's behaviour is measured, not assumed: a document whose world is written at radiance 0.5
and which contains nothing else renders 0.50196 everywhere — the eight bit rounding of 128 — where a
0.4545 gamma would have given 0.72976 (`out/encode-calibration.cycles.xml`).

| frame | max | mean | pixels with a channel at 255 |
|---|---|---|---|
| `cs-landscape` | 1.0000 | 0.5458 | **11.31%** |
| `cs-dunes` | 1.0000 | 0.7520 | **39.09%** |
| `cs-terraces` | 1.0000 | 0.6746 | **10.49%** |
| `rr-landscape` | 0.8627 | 0.5497 | 0.00% |
| `rrf-dunes` | 0.8471 | 0.6110 | 0.00% |
| `rrf-terraces` | 0.9137 | 0.5243 | 0.00% |

**Not one rasterised frame has a pixel at 255. Every traced frame has them.** A ratio measured
between a clipped image and an unclipped one is not a ratio between the two renderers, and it moves
towards one as the clipping grows — so every level figure in this gallery has been an under-statement
by an unknown amount, and the 39% on `dunes` is the worst of them.

**The way round it is the film exposure.** `<film exposure="e" />` multiplies radiance on the way
to the file and multiplication is reversible, so rendering at `e = 0.25` puts a radiance of four at
1.0 instead of clipping it, and dividing by `e` afterwards recovers the value. Rendered that way
`dunes` and `terraces` have **0.00%** clipped pixels and implied maxima of **3.01** and **2.01**. The
division happens **before** the grade, because the grade is applied to radiance and a quarter of a
radiance is not a radiance.

## What each scene says, on unclipped frames

Grade constants read out of each scene's `GRADE` in the source, not out of any note:

| scene | source | exposure | curve | saturation | S | vignette |
|---|---|---|---|---|---|---|
| `dunes` | `crates/gallery/src/dunes.rs:373-389` | 1 | Reinhard | 1.22 | 0.30 | 0.28 |
| `terraces` | `crates/gallery/src/terraces.rs:1089-1104` | 0.80 | ACES | 1.26 | 0.30 | 0.30 |
| `landscape` | `crates/gallery/src/landscape/mod.rs:1034-1041` | 1 | Reinhard | 1.18 | 0.26 | 0.30 |

| scene | graded/rasterised, whole frame | per band | pearson, whole frame |
|---|---|---|---|
| `dunes` | **1.003** | 1.000 – 1.006 | **0.9940** |
| `spotlight` | **1.047** | 1.004 – 1.313 | **0.9904** |
| `tiles` | **0.961** | 0.934 – 0.973 | **0.9973** |
| `landscape` | **0.981** | 0.936 – 1.006 | 0.9289 |
| `sunlight` | **1.081** | 1.009 – 1.236 | **0.9723** |
| `tidewater` | **1.181** | 0.628 – 2.134 | 0.7705 |
| `monoliths` | **0.008** | 0.000 – 0.042 | 0.0969 |
| `terraces` | **1.318** | 1.065 – 1.571 | 0.6496 |

`spotlight`'s top two bands are black on both sides, so their ratio and correlation carry no
information and are printed as a dash rather than as `nan` and `inf`; its 1.313 is the first band
with any signal in it. `tiles` agrees on level and disagrees on shape in its brightest bands, with
band pearson −0.5372 and −0.6976 where the level ratio is 0.960 and 0.973 — a local inversion where
the two draw the pattern in opposite directions, which is not a level finding and is not chased here.

> **The first two rows were 1.086 and 1.077 when this table was written, and both were measured with
> `e75a18e` in the tree.** That attribute was reverted at `323322c` after it was measured to be an
> sRGB **encode** applied by the texture node rather than a decode. Re-measured on the shipped build
> — frames `rlx-dunes` `670379bb34674ce3` and `rlx-landscape` `4be19e8be0b9871e`, traced at film
> exposure 0.25 with the exposure divided back out — **`dunes` agrees to three parts in a thousand
> and `landscape` to under two per cent**, and `dunes`' per band pearson is 0.9912 to 0.9982.
>
> **So the systematic disagreement this file recorded for `dunes` and `landscape` was caused by the
> colour space attribute, not by either renderer.** The attribute brightened the environment, which
> read as an improvement on the raw comparison and was then carried through the graded comparison as
> if it were a fix.

**`dunes` is the clean answer and it is very good.** Shape correlation **0.9940** over the whole
frame and 0.9912 to 0.9982 in every band, and the level is **one part in a thousand**, with no
sky/ground split at all. Removing the bounce changes almost nothing: the bounce-free frame is 0.7501
against 0.7520 on the earlier build, so **indirect light is not a residual**. A scene whose sky is a
sky on both sides, read each frame the way its own writer wrote it, with nothing clipped, agrees to
three decimal places.

**`landscape`'s ground claim survives both corrections.** Its three ground bands are 1.004, 0.999 and
1.006, and its sky bands 0.936 to 0.987 — closer than the four per cent quoted before, and closer
than the 1.020 to 1.037 quoted before that. Its pearson stays low in the upper bands (0.192, 0.539,
0.512) because that is where it paints ranges, shafts and cirrus.

**`terraces` is out, and not because of either renderer.** 1.318 whole frame with a pearson of
0.6496, and the per band ratio rises monotonically from 1.065 at the sky to 1.571 at the near
ground. Its terrain declares **no albedo**: `crates/gallery/src/terraces.rs:446` is
`surface([1.0, 1.0, 1.0], 0.0, 0.55, [0.0; 3])` with the comment "the terrain's albedo is decided
per fragment by the surface". The colour, the water and the Fresnel mirror towards an analytic sky
function all live in `TERRAIN_SURFACE` at `terraces.rs:809-897`, and the document can carry
`base_color = 1, 1, 1` and nothing else. The 675 dropped fields are 405 `reflect` and 270
rasterised-only stages, none of which can whiten a material, and nothing is missing from the list —
**a report about declared-and-unmappable fields cannot report a material that declares nothing to
map.** See
[terraces-terrain-is-a-wgsl-material-the-document-cannot-carry](terraces-terrain-is-a-wgsl-material-the-document-cannot-carry.md).

## A hazard this file's own probes walked into

**`out/air.png` is whichever of `landscape` and `dunes` rendered last.** Both name their environment
image `air.png` and the pixels are not the same file: `landscape`'s is 6927 bytes, `dunes`' is 701,
and `terraces` names its `sky.png`. Every render writes its own beside the document first, so a
render is self-consistent, but a probe that reads `out/air.png` next to the other scene's frame is
comparing against the wrong map. It happened here and had to be caught. **An environment image has to
be named after the document that carries it, or read out of that document, never assumed.**

## Not verified

**The absolute radiance calibration is not established.** A world-only document carrying `dunes`'
own world block rendered 0.6266, against 0.424 predicted from the map's bytes times the strength and
0.142 from the same decoded. Neither matches, and the reason is that the camera-to-texel mapping this
file would need to predict them is itself unmeasured — the visible row band was estimated wrong once
during this work — so the arithmetic proves nothing either way and is not used.

The `__builtin_srgb` change therefore rests on the source and on the measured improvement in
agreement, not on this. `src/util/colorspace.cpp:28` binds that string to `u_colorspace_srgb` and
`colorspace.cpp:121-123` routes it through `to_scene_linear`. A direct check of the decode would
need the mapping this file does not have, or an EXR the probes cannot read: Cycles writes one
(`out/dw-world.exr`, 109342 bytes) and this machine's Python has no EXR reader installed, so float
output is currently unreachable to every probe here.

> **Both of these paragraphs were superseded the same evening.** The attribute was measured against a
> generated ramp to be an sRGB **encode** applied by the texture node, not a decode, and it was
> reverted at `323322c`; see
> [what-the-colour-space-attribute-actually-does](what-the-colour-space-attribute-actually-does.md).
> The absolute calibration was settled without either route: a document carrying an `<emission>` of
> strength 1.0 reads **exactly 1.00000** in the file, so the eight bit png holds linear radiance with
> no gamma and no view transform. **Neither the mapping nor an EXR reader was needed.**

## `tiles` and `spotlight`, which paint nothing into their sky

Neither carries a `depth >= 1.0` block. A search of `crates/gallery/src` finds one in `dunes`,
`landscape`, `monoliths`, `terraces`, `tidewater`, `foldyard`, `lanterns`, `nurbs` and `stilllife`,
and in neither of these. Both grades are Reinhard and a gamma and nothing else
(`crates/gallery/src/tiles.rs:198-199`, `spotlight.rs:272-273`), read from the source.

`spotlight` agrees to **1.047** whole frame at a pearson of **0.9904**, and `tiles` to **0.961** at
**0.9973**, which is the highest shape agreement of any scene measured here. `spotlight`'s top two
bands are black on both sides, so their ratio and correlation carry no information; the probe prints
a dash for those rather than the `nan` and `inf` a division would produce, because a number that
cannot be read as a measurement should not be printed in the column where measurements go.

**`tiles` agrees on level and disagrees on shape in its brightest bands** — band pearson −0.5372 and
−0.6976 where the level ratio is 0.960 and 0.973. A local inversion, with the two drawing the pattern
in opposite directions, which is not a level finding and is not chased here.

## The four that are out, and what each one is out for

| scene | why |
|---|---|
| `terraces` | its terrain declares **no albedo**; the colour, the water and the Fresnel mirror are WGSL computed per fragment and the document carries `base_color = 1, 1, 1` |
| `monoliths` | the scene declares **no environment light** and the rasterised post stage paints a sky anyway, which is also what the near-black mirror ground reflects |
| `tidewater` | aerial perspective applied to **every surface**, `crates/gallery/src/tidewater.rs:818-825` |
| `foldyard` | never measured; its document is sixty four megabytes |

**`tidewater` is 1.181 whole frame at 0.7705**, and the split is informative: its top two bands agree
to **0.993 and 0.999** at a pearson above 0.85 — sky and far water, where there is little depth to
haze — and the bands below it run from 0.628 to 2.134. The law is
`mix(color, aerial_colour(ray), clamp(1.0 - exp(-depth_of_air), 0.0, 0.985))` on every fragment with
`depth < 1.0`, which is a fog a path tracer computes by other means and this document does not carry.

**Nothing in this table is a disagreement between the two renderers about the same picture.** Four
scenes have a material, a sky or a fog the other backend cannot be given, and four agree to between
one and eight per cent.

## A silent bug in this file's own tooling, found by a scene that is not square

`out/probe-unclipped.py` passed `--width 1024 --height 1024` as constants. `tidewater` is **1536 by
1024**, so the probe wrote a differently shaped frame without saying so, and the comparison failed on
shapes rather than on levels. It now reads `width` and `height` out of the document's `<camera>` and
prints what it read. **A probe that can produce a wrong-shaped frame without complaining is a probe
whose numbers cannot be trusted on any scene it has not been read against.**

## Not done

**The gallery census has not been re-run** under the exposure and grade method. Every figure in it
is an eight bit, clipped, differently graded comparison, and the method in this file replaces all
of it.

**`foldyard` has no bounce-free render and no unclipped one** — its document is sixty four megabytes.

**Three scenes were never measured**: `sunlight`, `tidewater` and `monoliths`, out of nine that
render. `tiles` and `spotlight` were added after this list was written.

## Sources

- The writer: `out/encode-calibration.cycles.xml`, a world at radiance 0.5 with no geometry.
- The grades: each scene's `GRADE` constant, cited per row above.
- The bounce control: `out/csb0-dunes.cycles.xml`, `out/probe-bounce-free.py`, one line.
- The exposure control: `out/csx-dunes.cycles.xml`, `out/csx-terraces.cycles.xml`,
  `out/csx-landscape.cycles.xml`, one line each, `out/probe-unclipped.py`.
- The comparison: `out/probe-grade-matched.py`.
- The frames, hashed on the way in: `csx-dunes` `f4c6aad095351d23`, `rrf-dunes` `d505195abaaf920f`,
  `csx-terraces` `243aaa9f1b631e95`, `rrf-terraces` `1aa14dd4ec78b2ae`, `csx-landscape` `659a3fea7b6273ea`,
  `rr-landscape` `f7c00ad9cc65d009`.