# How much light the rasterised pipeline loses before its tone curve runs

**Measured 2026-10-10** on the `landscape` scene at 1024×1024, through the rasteriser's own
`--debug grade` channel. No source was changed to take any of these readings; the channel is a
supported one and the scripts are in `out/`.

## The question

The open item said *"no tone curve, so every gallery scene is clipped"*. Both halves of that are
wrong, and the thing it points at is somewhere else. What actually happens to the top of the
range?

## What was measured

### The grade has a tone curve, and the output does not clip

`landscape`'s `GRADE` runs Reinhard (`color / (color + 1)`), a 0.4545 gamma, a saturation lift, a
contrast S-curve and a vignette, in that order — see `crates/gallery/src/landscape/mod.rs:924`.

and the frame it produces has **no clipped pixels at all**:

| measure | value |
|---|---|
| pixels with every channel at or above 250 | **0** |
| pixels with every channel at exactly 255 | **0** |
| brightest channel, 99th percentile of a 4× sample | **192** |
| median channel | 171 |

a highlight at 230 reads as blown out on screen and is not clipped. what was being called
clipping is not clipping.

### The clipping happens **before** the grade, and 19.6% of the surface is under it

`--debug grade` is documented in its own sidecar as *"the radiance the post stage's grade reads at
each pixel, before that grade has run"*, and it says why it cannot be read absolutely:

> The shaded image it copies is an eight bit target, so a light that reached one was clipped
> before the grade saw it: a value at one here may have been one or may have been forty, and
> nothing in this file can tell you which.

measured on `landscape`, over the pixels a surface was actually drawn on (`out/probe-clipping.py`,
separated by the `depth` channel):

| band | pixels | share |
|---|---|---|
| whole frame | 109,205 | 10.4% |
| **of the 556,162 drawn pixels** | 109,205 | **19.6%** |

### and those pixels come out as a plateau

the consequence, which is the part that is visible rather than merely provable. pixels that
arrived at the curve already at or above one carry no radiance above one, so everything
downstream maps them onto nearly the same value:

| region | samples | output range | mean |
|---|---|---|---|
| **clipped before the grade** | 27,314 | **17.0** | 189.7 |
| not clipped | 111,860 | 132.6 | 156.2 |

the sunlit faces of the massif land in a band **17 levels wide out of 255**, while the rest of the
drawn surface spans 133. that is the flat cream plateau the lit faces read as, and it is not a
curve that is wrong — it is a curve being handed a signal that has already lost its top.

## Where the ceiling is

one named constant:

```text
crates/scene/src/standard/pass_schemas.rs:312
pub const COLOR_FORMAT: &str = "rgba8unorm";
```

and the codebase already states the consequence, at the socket that would otherwise be blamed:

```text
crates/scene/src/standard/pass_schemas.rs:713
"It is not a parameter of the curve that follows, and it cannot brighten a value the shaded
image already holds at one."
```

so a scene's `exposure` — the obvious lever — **cannot** reach this. it multiplies a value that has
already been truncated at one. that is why raising the sun to 9.0 made the frame look worse rather
than better: more light arrived, all of it above the ceiling, and none of it distinguishable.

the path tracer has no such target. it carries radiance as floats and only rounds when it writes
the file, which is why the Cycles massif shows gradation across its lit faces and this one does
not.

## What this is not

- **not a missing tone curve.** there is one, and it is applied.
- **not output clipping.** there is none.
- **not fixable by `exposure`**, which the schema says in as many words.
- **not fixable by a better curve.** a curve redistributes a range; this one is given a range whose
  top is already gone. a filmic curve over a clipped signal produces a smoother plateau, not detail.

## The change it implies, and what it cost — **measured 2026-10-11, not made here**

the ceiling is the shaded image's format. **`rgba16float`**, with the frame left alone.

**the frame does not follow, and that is most of why this is cheap.** the present target is a
display image: it is what comes back to the host and what a png is made of, and a host reading
four bytes a pixel cannot be handed eight. widening it together with the shaded image renders
perfectly and then answers

```text
the frame does not encode: the frame is drawn in `rgba16float`, and a frame is read as an 8 bit
rgba image
```

— so the read-back is untouched and the frame keeps a format of its own.

measured on `landscape` at 1024×1024, whole frame:

| | `rgba8unorm` | `rgba16float` |
|---|---|---|
| pre-grade maximum, r/g/b | `1.000 / 1.000 / 1.000` | **`1.344 / 1.077 / 0.896`** |
| pixels at or above 1.0 — red | 188,602 (**18.0%**) | 10,747 (1.0%) |
| — green | 161,394 (15.4%) | 643 |
| — blue | 109,205 (10.4%) | **0** |
| output span over those red pixels | **24.3** of 255 | **127.9** |
| render, steady state, three runs | 74 / 68 ms | 69 / 68 / 80 ms |

**no measurable time cost.** on **cross-backend agreement** the first reading said no effect at all —
`landscape` `+0.2339 → +0.2334`, `spotlight` `+0.9283 → +0.9282`, `sunlight` `+0.9729 → +0.9763`,
`tiles` `+0.0221 → +0.0221` — **and that reading was wrong**, because it was taken while the massif
still wore its per-fragment surface body. That body clipped far more of the frame than the ceiling
did, so the ceiling was measuring a difference the frame had already thrown away.

Re-measured after the body came off ([investigate/landscape-agreement-drop.md](landscape-agreement-drop.md)),
on `main` at `c8c7ec9`, same Cycles frames, whole frame:

| scene | `rgba8unorm` | `rgba16float` | change |
|---|---|---|---|
| **landscape** | +0.6029 | **+0.7812** | **+0.1783** |
| lanterns | +0.8352 | +0.8571 | +0.0219 |
| foldyard | +0.1050 | +0.1168 | +0.0118 |
| sunlight | +0.9729 | +0.9763 | +0.0034 |
| spotlight | +0.9283 | +0.9282 | −0.0001 |
| tiles | +0.0221 | +0.0221 | 0 |

so the ceiling was worth **+0.178 on `landscape`** — more than any single change since the camera
frame — and nothing anywhere. The gain is in kind what the table above says it is: the rasterised
massif goes from one flat cream blob to reading its block faces, because those faces were the pixels
whose span went from 24.3 to 127.9.

**a general lesson worth keeping.** Two defects sat on the same pixels, and the larger one hid the
smaller. Measuring the smaller one while the larger was in place produced a clean, plausible, wrong
answer. The order of these measurements was wrong, not the measurements.

one line beyond the format: the one texel black map a scene naming **no** environment light is
bound carried the shaded image's format with four bytes of pixels behind it, and every such scene
failed to upload with `Number of bytes per row is less than the number of bytes in a complete
row`. That map is an image and not a radiance, so it now has a format constant of its own.

## Sources

- `out/probe-clipping.py` — counts the pre-grade clipped pixels, separates them from sky by the
  `depth` channel, and compares the output spread of clipped against unclipped regions.
- `out/rad3/g-grade.txt`, `out/rad3/g-depth.txt` — the rasteriser's own sidecars for those channels.
- `crates/gallery/src/landscape/mod.rs:924` — the grade's five stages, in order.
- `crates/scene/src/standard/pass_schemas.rs:312` — `COLOR_FORMAT`.
- `crates/scene/src/standard/pass_schemas.rs:709` — the `exposure` socket and its stated ceiling.
