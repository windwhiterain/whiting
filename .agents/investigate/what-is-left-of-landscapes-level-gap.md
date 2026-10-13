# What is left of landscape's level gap once the colour space and the output encoding are both accounted for

Measured 2026-10-13, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`, with
`e75a18e` in the tree. Probes: `out/probe-output-encoding.py` (this file's measurement),
`out/probe-landscape-sky.py`, `out/probe-world-colorspace.py`. Frames:
`out/cs-landscape.png` `e1c3b08c5ab3e6cd` (traced, with the colour space named),
`out/rr-landscape.png` `f7c00ad9cc65d009` (rasterised),
`out/air.png` `d4420f30350830c9`.

## The question

`a-low-correlation-measures-the-bounce-and-the-samples.md` left `landscape` with a median level
ratio of **0.788** against the rasterised frame and called it the largest disagreement in the
gallery. The colour space defect closed most of it — see
[the-air-image-reaches-cycles-with-no-colour-space](the-air-image-reaches-cycles-with-no-colour-space.md)
— and moved the whole frame from 0.831 to 0.993. What is left over is a real disagreement or an
artefact of comparing two files that were written differently?

## It was two different things, and one of them is not a disagreement

**The tracer's file and the rasteriser's file are not encoded the same way, and now that can be
stated as a measurement rather than an assumption.** Cycles reports `Color management disabled` and
copies radiance straight into the eight bit file: a document whose world is written at radiance
**0.5** and which contains nothing else renders a frame that is **0.50196** everywhere — the eight bit
rounding of 128. With a 0.4545 gamma applied it would have read 0.72976.

**The rasterised frame is tone mapped.** `crates/gallery/src/landscape/mod.rs:1034-1041` is
Reinhard `c/(c+1)`, then `pow(0.4545)`, saturation 1.18, an S curve at 0.26 and a 0.30 vignette.

Putting the traced frame through that scene's **own** curve and comparing again:

| rows | traced raw | traced graded | rasterised | graded/rasterised |
|---|---|---|---|---|
| 0–128 | 0.3817 | 0.5176 | 0.4407 | 1.175 |
| 128–256 | 0.3921 | 0.5389 | 0.4782 | 1.127 |
| 256–384 | 0.4051 | 0.5563 | 0.4838 | 1.150 |
| 384–512 | 0.4210 | 0.5695 | 0.5152 | 1.105 |
| 512–640 | 0.4633 | 0.5856 | 0.5592 | 1.047 |
| 640–768 | 0.7616 | 0.6674 | 0.6518 | 1.024 |
| 768–896 | 0.8156 | 0.6612 | 0.6588 | 1.004 |
| 896–1024 | 0.7260 | 0.6309 | 0.6102 | 1.034 |
| **whole frame** | 0.5458 | 0.5909 | 0.5497 | **1.075** |

**The ground agrees.** The three rows below the horizon come out 1.004, 1.024 and 1.034 — inside
three and a half per cent. Every figure that said the traced ground was long, 1.18 to 1.36 on the raw
files, was the tone curve and nothing else.

**The sky is now the one that is high**, 1.05 to 1.18, and it was not before. That is a change of
sign, and it is explained in kind by
[the-raster-sky-band-is-not-the-tracers-sky-band](the-raster-sky-band-is-not-the-tracers-sky-band.md):
`landscape` paints three ranges of silhouette, a set of crepuscular shafts and a cirrus deck into the
pixels no geometry reached. The rasterised sky is a sky plus three painted things and the traced sky
is a sky. Grading cannot reconcile two pictures.

**So `landscape` no longer has a level disagreement to explain.** It has a composition difference in
its upper half that has been documented since 2026-10-12, and a ground that agrees to three per cent
once each file is read the way its own writer wrote it.

## Two earlier readings this file overturns

**`the-tone-curve-was-masking-a-ground-brightness-gap.md` said every sky agreed after grading** —
landscape 115.32 against 112.73. That measurement was taken with the environment map being read as
scene linear, and the sky was therefore short by roughly the amount the curve lifts it. The two errors
were cancelling, and the agreement was the cancellation, not the sky. With the colour space named the
traced sky grades to 0.5176 against a rasterised 0.4407.

**The same file, and `the-raster-sky-band-is-not-the-tracers-sky-band.md`, said a compressing curve
could not account for the raster side being the brighter one.** That was true of the numbers they had,
and it no longer is: the raw comparison still has the raster side darker in the sky, but graded, the
traced side is the brighter one. The conclusion they drew from it — that something other than the
curve was wrong with the sky — was right, and what it was is the colour space.

## Not measured

- **`dunes` and `terraces` under either change.** Both carry skies and neither was re-rendered.
  `dunes` is the one scene in the gallery whose sky is not painted into, so it is the scene where this
  file's method would give a clean sky number, and that number does not exist.
- **Whether the ground agreement holds at the pixel level rather than per row band.** A row band mean
  hides structure, and `dunes`' ground correlating at +0.9893 while sitting forty nine levels away is
  the standing reminder that shape and level are separate questions.
- **Anything on the four scenes that declare no environment**, whose sky the tracer draws black.

## Sources

- The writer's behaviour: `out/encode-calibration.cycles.xml` and its frame, rendered at radiance 0.5
  with no geometry and no texture.
- The curve: `crates/gallery/src/landscape/mod.rs`, `GRADE`, lines 1034 to 1041, read from the source
  rather than from the earlier note's table of it.
- The painted sky: the same `GRADE`, lines 993 to 1031.
- What the exporter says it cannot carry: the `render_cycles` run's dropped-field report, which names
  `ShadowingShader` and `DepthShader` as stages of the rasterised pipeline.