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

**0.0053 is eight bit rounding.**

## Where the gamma is applied is not the writer, and that is settled by an emitter

This file first said the writer, on the strength of `src/app/oiio_output_driver.cpp:61-69` applying
`pow(image, 1/2.2)` to a png. **That is wrong too**, and the second attempt at locating it had a
control that did not control anything: a document naming an undefined `shader`, so the "emitter"
was the default surface shader and sampled the environment like everything else. Both regions then
moved by the same factor, which proves nothing.

With a real `<emission>` in the document, and sampled inside the quad's actual extent (it is 0.5
units across at depth 2 where the half frame is 1.0276, so it covers **24.3%** of the frame, and the
first attempt sampled a region twice that and measured mostly world):

| region | unnamed | named | ratio |
|---|---|---|---|
| **the emissive quad** | **1.00000** | **1.00000** | **1.00000** |
| the world around it | 0.35671 | 0.60567 | 1.69792 |

**The emitter does not move and the world does.** The gamma is applied by the texture node, and
`oiio_output_driver.cpp` never sees it.

## Which also settles the file's encoding, which nothing here had settled before

The quad is an `<emission>` of strength 1.0 and white. **It reads exactly 1.00000 in the file.**
So the eight bit png holds linear radiance with no gamma and no view transform, which is what
`[OpenColorIO Info]: Color management disabled` says and what every previous note in this directory
had assumed without a measurement.

**The texture node applies the transfer function in the wrong direction.** `srgb_to_linear` is
predicted to error by 0.5129 and `pow(x, 0.4545)` by 0.0053, and the node returns the **encode**.
A map whose bytes are sRGB encoded, read through this socket, comes out brighter than its own bytes
at every value below one.

## Whether any name decodes: no, and that is the whole answer

Eight names, one generated ramp, one camera frame covering the whole unit interval
(`out/probe-colorspace-sweep.py`):

| name | what the frame follows | largest error | loader |
|---|---|---|---|
| *(unnamed)* | identity | 0.00000 | accepted |
| `scene_linear` | identity | 0.00000 | accepted |
| `__builtin_srgb` | `pow(x, 0.4545)` | 0.00528 | accepted |
| `srgb` | identity | 0.00000 | **rejected** |
| `linear` | identity | 0.00000 | **rejected** |
| `Non-Color` | identity | 0.00000 | **rejected** |
| `data` | identity | 0.00000 | accepted |
| `raw` | identity | 0.00000 | accepted |

**No name decodes.** The four the loader accepts give identity three times and the encode once; the
three it rejects fall back to scene linear, which is identity here.

A sweep rather than one guess because a single name failing would not have established that the
build cannot do it, and the answer changes where a fix lives. This establishes it:

- **A name that decodes** would make the defect an emitter defect, fixed by one attribute.
- **No name decodes** — what is measured here — puts it in the build or in how this build is
  configured, and **no attribute the emitter writes will change it.**

The three rejected names are worth recording for a second reason: `srgb`, `linear` and `Non-Color`
are the spellings a person reaches for, `Non-Color` and `linear` being Blender's own, and each one
is refused with `Color space '<name>' could not be found`. A rejected name renders exactly as if it
had not been written, so a document that looks like it names a colour space may name nothing at all.

## So the emitter change was wrong, and so was the reason it was withdrawn for

Both halves of `e75a18e` are gone: it does not fix the colour space defect, and it was not fixing it
at the writer either. It makes the environment brighter, which moved `landscape`'s numbers towards a
rasterised frame whose sky is itself gamma encoded. That is the same kind of coincidence this
investigation has already had to withdraw twice.

`323322c` reverts it. The emitter writes no colour space and the traced file is linear radiance, and
every measurement has a single stated baseline.

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

**Why the texture node applies the wrong direction.** `src/util/colorspace.cpp:121-123` routes
`u_colorspace_srgb` through `to_scene_linear`, which by its name converts *to* linear and would make
the map darker. This build makes it brighter. The route between the socket and the returned value is
not traced.

**Whether the build could be made to decode.** The sweep covers the names the loader's own
`ColorSpaceManager` enumerates plus the three a person would reach for. It does not cover changing
`$OCIO`, installing a configuration, or a different build.

**It does not cover `$OCIO`, and that route is closed too.** The log says
`[OpenColorIO Info]: Color management disabled. (Specify the $OCIO environment variable to enable.)`,
which names the environment variable as the way in, and the variable is empty. **There is no
configuration to point it at**: `C:\resource\cycles\install` holds `license` and `source` and
nothing else, and a search of the whole of `C:\resource\cycles` finds no `*.ocio` file and no
`colormanagement` directory. OpenColorIO is compiled into this build with nothing to configure.

So the three routes an emitter has — name a colour space, spell it right, or turn colour management
on — are all measured and none of them decodes the map. **A fix cannot live in the attribute.** It
would have to live in the pixels the exporter writes beside the document, and that is a different
piece of work with its own cost: an eight bit file cannot hold a linear value faithfully, because
decoding darkens every texel and the quantisation lands where it hurts most.

**Whether either path decodes the map correctly** is still unestablished. What is established is
that the file is linear radiance, that the unnamed case uses the map's bytes as they are, that
naming a colour space makes the map brighter rather than darker, that no name tried decodes, and
that there is no configuration in this build that would.