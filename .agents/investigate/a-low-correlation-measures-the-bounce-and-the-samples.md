# A low correlation between these two backends measures the bounce count and the sample count

Measured 2026-10-14, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`. Probes:
`out/probe-agreement-vs-averaging.py` (the measurement), `out/probe-frame-content.py` (what is in
a frame before anything is compared), `out/probe-census-both-graded.py` (the census, with both
sides through the same grade). Documents `out/b0-<scene>.cycles.xml` are the census's own exports
with `max_bounce` set to zero, and nothing else changed.

## The question

A correlation between the rasterised frame and the traced frame was low on `sunlight` (0.2588) and
on `terraces` (0.6633 over the whole frame, and **-0.0014** over its lower two thirds). A
correlation near zero between two renderings of one scene is alarming: it says the two draw the same
levels and nothing alike. Is that what is happening?

## It is not, and there are two reasons, and they are separable

**Indirect light.** The rasteriser draws direct light only; the tracer draws light transport.
`out/probe-frame-content.py` says so in one number: the spread among the drawn pixels of the
rasterised `sunlight` frame is **0.0041**, against **0.1067** in the traced one. The rasterised frame
is a piecewise constant image — four constant box faces, a constant floor, black shadows — and a
correlation against a frame with colour bleed in it is low however well the geometry agrees.

**Sampling noise.** A path tracer at 32 to 64 samples is visibly speckled and a rasteriser is not. A
box filter of side `n` divides the standard deviation of independent noise by `n` and leaves a
difference in *what the two draw* intact, so averaging separates them: **noise-dominated
disagreement falls as the block grows, disagreement about the picture does not.**

## What they are, scene by scene

Correlation at block size 1 and at block size 32, against the rasterised frame:

| scene | with bounce, block 1 | no bounce, block 1 | no bounce, block 32 |
|---|---|---|---|
| sunlight | 0.2588 | 0.4400 | **0.9982** |
| spotlight | — | — | **0.9994** |
| tiles | 0.8600 | 0.8600 | 0.9647 |
| landscape | 0.9112 | 0.9079 | 0.9124 |
| dunes | 0.8670 | 0.8381 | 0.8347 |
| terraces | 0.6633 | 0.6471 | 0.7704 |

**`sunlight` is entirely sampling noise.** Removing the bounce leaves only noise, and averaging the
noise away takes the correlation to **0.9982**. The two backends were drawing the same picture all
along, and the census was reading speckle. The level is still out: the traced frame is **0.886** of
the rasterised one, and that does not move with the block size.

**`tiles` is nearly the same story**, 0.9647, and its bounce-free render is **byte identical** to its
bounced one (sha256 `b1e049195544`), so tiles has no indirect contribution to speak of.

**`dunes`, `landscape` and `terraces` do not collapse.** Their correlation is flat or nearly flat as
the block grows, which is the signature of a systematic difference rather than of noise. **These
three are real and none of them has been looked at.**

## The level, which the correlation says nothing about

Median ratio of traced over rasterised, over the pixels both draw, bounce removed:

| scene | ratio |
|---|---|
| tiles | 1.013 |
| terraces | 0.928 |
| sunlight | 0.886 |
| dunes | 0.952 |
| **landscape** | **0.788** |

**`landscape` is out by more than twenty per cent** and nothing above says why. It is the largest
level disagreement in the gallery and, unlike the ones this file has closed, it survives every
control tried here.

> **Closed on 2026-10-13, and not by a renderer.** The environment map was reaching Cycles with no
> colour space, so an eight bit sRGB file was used directly as linear radiance; and the tracer's file
> is raw radiance while this file's rasterised frames are tone mapped. With both accounted for,
> `landscape`'s ground rows agree to within three and a half per cent and its sky band is a
> composition difference, because `landscape` paints ranges, shafts and cirrus into pixels no geometry
> reached. See [what-is-left-of-landscapes-level-gap](what-is-left-of-landscapes-level-gap.md).
> **`dunes` and `terraces` remain open**, and `dunes` is now the scene where this method would give a
> clean sky number, because it is the one that paints nothing into its sky.

## What this does to the census

Every shape figure in `out/probe-census-both-graded.py` is a **block size 1** figure against an
eight-bounce tracer, so every one of them is partly a statement about the bounce count and the
sample count. They are comparable with each other and they are not a statement about the two
backends agreeing.

**A census that compares a direct-only renderer with a path tracer has to choose which of the two it
is measuring**, and the choice has to be written down. Comparing bounce-free to bounce-free answers
"do the backends read the same scene"; comparing as shipped answers "how much of the difference is
the renderer". Both are worth having. Neither is the other's baseline.

## Not done

`dunes`, `landscape` and `terraces` are unexplained and `landscape` carries a twenty per cent level
gap. `foldyard` has no bounce-free render at all — its document is sixty four megabytes and was left
out of this round rather than guessed at. `sunlight`'s eleven per cent level gap is also still open.

No probe here renders, and none of them reads a file's role from its name: every one of them hashes
its inputs and refuses to compare a file with itself. That guard is there because it was already
needed twice.