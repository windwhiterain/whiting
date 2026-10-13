# What the colour space attribute actually does, and it is not what it was shipped for

Measured 2026-10-13, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`, at `29cdc94` and
`e75a18e` in the tree. Probes: `out/probe-colorspace-decode.py`, `out/probe-colorspace-curve.py`.
Documents: `out/cd-{named,linear,unnamed}.cycles.xml`, `out/cc-{named,linear}.cycles.xml`. Neither
probe reads any gallery scene, and neither reads a file it did not write.

## The question

`e75a18e` writes `colorspace="__builtin_srgb"` on the environment texture, on the grounds that
`src/util/cycles_xml.cpp` reaches `ShaderNodeBackground` and `src/util/colorspace.cpp:28` binds that
string to `u_colorspace_srgb`, and `colorspace.cpp:121-123` routes it through `to_scene_linear`,
which is the sRGB electro-optical transfer function. That would decode an eight bit sRGB png instead
of using its bytes as linear radiance.

The frame moved, and it moved towards the rasterised frame. **The question is whether it moved for
that reason.** The source can be read; what it does on this build has to be measured.

## The measurement, and it is unambiguous

One world-only document, rendered twice, changing **only** the `colorspace` attribute. Both frames
read the same texture, so nothing about the texture can explain the difference between them, and no
camera-to-texel mapping is needed to compare two frames pixel by pixel.

The gallery's own map is a narrow gradient — the first attempt gave two populated bins and no curve
could be read off them — so the map here is generated: a 256 by 128 equirectangular ramp, black at
the top, white at the bottom, with a camera at the origin and `fov = 3.0` so that one frame covers
the whole unit interval.

| scene linear reads | `__builtin_srgb` reads | ratio | `srgb_to_linear` | `pow(x, 0.4545)` |
|---|---|---|---|---|
| 0.5269 | 0.7526 | 1.4285 | 0.2398 | 0.7473 |
| 0.6211 | 0.8101 | 1.3043 | 0.3438 | 0.8054 |
| 0.7222 | 0.8662 | 1.1995 | 0.4802 | 0.8625 |
| 0.8215 | 0.9171 | 1.1164 | 0.6410 | 0.9145 |

| candidate | largest error against the measured curve |
|---|---|
| `srgb_to_linear` | 0.5129 |
| **`pow(x, 0.4545)`** | **0.0053** |
| identity | 0.2258 |

**0.0053 is eight bit rounding.** The attribute applies a 1/2.2 encode to the file being written.

## So the attribute is not doing what the source says, and the change is wrong as stated

**The texture is not decoded.** An unnamed space and an explicit `scene_linear` render
**byte-identically** (`max difference 0.000000` over the whole frame), which confirms the fallback
the emitter was written to avoid, and both share one texture, so any decode would have to be the
same in both and cannot be what differs.

**What differs is the output.** `src/app/oiio_output_driver.cpp:61-69` applies `pow(image, 1/2.2)` to
a png when `detect_known_colorspace(u_colorspace_auto, "", format, is_float = true)` answers
`u_colorspace_srgb`. Whatever route gets there, the measured effect is a 1/2.2 encode on the frame,
and `src/util/colorspace.cpp:121-123` is not describing what this build does with the attribute.

**The agreement with the rasterised frame was a coincidence.** The rasterised frame is gamma encoded
too, so a gamma on the traced frame moves the two together without either of them reading the map
correctly. This is the second time this work has found a number that improved for the wrong reason,
and the first time it survived a full session's worth of agreement before being caught; the thing
that caught it was generating a ramp instead of trusting a gallery map.

## What is still true

**`colorspace="sRGB"` is rejected by this build** — `Color space 'sRGB' could not be found` — and
renders byte-identically to the shipped frame.

**A builtin name does not depend on the machine.** A configuration name would resolve differently on
a box with a full OpenColorIO configuration installed.

**The environment map is read, and reading it is not free.** `out/probe-world-sky.py` shows a
document with no texture renders perfectly flat while the shipped one has structure.

**`landscape` and `dunes` still do not disagree about the picture.** Those conclusions rest on shape
correlation and on the tone curve, both of which are unaffected by a gamma.

## What follows

**`e75a18e` is under review and its stated reason is withdrawn.** Three options, and they are not
the same change:

1. **Revert.** The emitter goes back to writing no colour space, and the traced file stays raw linear
   radiance — which is what the whole of
   [every-traced-frame-loses-its-highlights-to-the-file](every-traced-frame-loses-its-highlights-to-the-file.md)
   was measured against.
2. **Keep it as a display transform and say so.** A gamma-encoded eight bit png is a better file than
   a raw linear one — highlights still clip, but mid-tones land where a viewer expects them. It then
   has to be named for what it is, and every comparison against it has to grade it, which
   [what-is-left-of-landscapes-level-gap](what-is-left-of-landscapes-level-gap.md) already does by
   hand.
3. **Replace it with a real view transform** on `<film>`, so the choice is made where Cycles means it
   rather than through a texture attribute that happens to reach the writer.

The honest answer to "does the environment map reach Cycles with no colour space" is **yes, and
naming it does not fix it on this build.** The defect is real; this was not its fix.

## Not measured

**Why the attribute reaches the writer.** The route from a texture's `colorspace` string to
`oiio_output_driver.cpp`'s gamma decision is not traced, and `check_invalidate_caches` at
`colorspace.cpp:363` is a plausible place a cached lookup could make an unrelated node's request
decide the file's encoding. It is named here as a lead and not as a conclusion.

**Whether the two frames' textures were decoded identically.** The frames establish that the
difference is not a texture decode; they do not establish that either of them decodes the map
correctly, which is the separate question
[what-is-left-of-landscapes-level-gap](what-is-left-of-landscapes-level-gap.md) could not reach
either.