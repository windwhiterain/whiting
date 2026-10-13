# Sunlight agrees and monoliths is a painted sky, and both were previously open

Measured 2026-10-14, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`, at `855929b` and later
in the tree. Probe: `out/probe-grade-matched.py`, grade constants read out of each scene's `GRADE` in
the source. Traced frames at film exposure 0.25 with the exposure divided back out, so nothing is
clipped; rasterised frames re-rendered from the same build.

| scene | whole frame | per band | pearson |
|---|---|---|---|
| `sunlight` | **1.081** | 1.009 – 1.236 | **0.9723** |
| `monoliths` | **0.008** | 0.000 – 0.042 | 0.0969 |

## `sunlight` is closed, and it closes an eleven per cent gap recorded two days ago

`a-low-correlation-measures-the-bounce-and-the-samples.md` left `sunlight` with a level ratio of
0.886 that "does not move with the block size" and called it open. Re-measured on the shipped build
with each frame read the way its own writer wrote it and nothing clipped, it is **1.081** at a
pearson of **0.9723**.

`sunlight` carries no `depth >= 1.0` block and its grade is Reinhard and a gamma and nothing else
(`crates/gallery/src/sunlight.rs:288-289`). Its top band is black on both sides — it names no
environment light, so the tracer's world is black and the rasteriser draws nothing there either,
which is the behaviour `crates/gallery/src/shaders.rs:2052-2076` records as deliberately restored.

## `monoliths` is a painted sky, and the painting is what the water reflects

The traced frame is black to the eye apart from a warm rim on a few spires; the rasterised one has a
blue sky, a blue water plane and lit monoliths. **The geometry is in frame in both**, so this is not
a camera or a projection fault, and the light is not missing: the document carries

```xml
<state shader="sun_emission">
    <transform translate="-60, -48, 30" rotate="90, 1, 0, 0">
        <light name="sun" light_type="point" strength="754037.8, 542907.216, 339317.01" />
    </transform>
</state>
```

with the scene's `power` of 754037.8 arriving intact and its position carried through the axis
conversion. **The scene declares no `Environment` light**, so the tracer's world background is black.
The rasterised side draws a sky anyway: `crates/gallery/src/monoliths.rs:271-272` puts
`sky_at(ray_through(in.uv))` into every pixel no geometry reached.

**And that painted sky is most of the picture.** The ground is `surface_reflecting([0.018, 0.020,
0.026], 0.0, 0.06, [0.0; 3], 1.0)` — near black, and a mirror at 0.06 roughness. What the rasterised
frame shows as a blue plane is that ground reflecting the sky the renderer drew for itself, and a
tracer with a black world has nothing for it to reflect.

So `monoliths` is the same class as `landscape` and `terraces` and a different class from `terraces`
in one respect: **its materials do carry albedo**, and every `principled_bsdf` in the document has a
real `base_color`. The whole difference is the painted sky and the reflection that painting feeds.
`the-raster-sky-band-is-not-the-tracers-sky-band.md` already says a sky-band figure is only
meaningful for a scene that paints nothing into its sky, and lists what paints; `monoliths` was
missing from that list.

## What a scene that wants a sky has to say

The stand-in hemisphere that used to light nine of the twelve scenes was removed, and
`shaders.rs:2054-2056` is explicit that a scene naming no `Environment` light now gets black. That
removal was about the **lighting** term. **The post stage's own sky block is a separate thing and is
still present in nine scenes.** So a scene can declare no environment light, get black from the
lighting path, and still have a painted sky from the post stage — and the two backends then disagree
about a scene rather than about a renderer, with the exporter reporting nothing, because there was
nothing dropped.

## Not measured

**Whether `monoliths` would agree if it declared an `Environment` light.** That is the experiment
that would separate "the scene is incomplete for a tracer" from "the rasteriser invents a sky", and
it needs an edit to `monoliths.rs` that was not made.

**`tidewater`** carries a further term this method does not model: grain, a dither the eight bit
file it was written to has already lost, the same situation `cycles-raster-mapping-is-calibrated.md`
records for `foldyard`. Its white balance is per-pixel on the pixel's own luma and is reproduced.

**`foldyard`** has never been measured under this method — its document is sixty four megabytes.

**The remaining unmeasured scenes are `tidewater` and `foldyard`.** `monoliths`, `sunlight`, `tiles`,
`spotlight`, `dunes`, `landscape` and `terraces` all have figures now.