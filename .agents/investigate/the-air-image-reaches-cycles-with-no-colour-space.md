# The environment map reaches Cycles with no colour space, and it is read as scene linear

> **The premise of this file is wrong, and the correction is at the bottom.** The two backends read
> the map **the same way** — neither decodes it — so there is no colour space disagreement between
> them to fix. `crates/gallery/src/kit.rs:69` declares every gallery image as `rgba8unorm`, and
> `wgpu::TextureFormat::Rgba8Unorm` returns the bytes as they are, exactly as Cycles reads them as
> scene linear. **The scene's world radiance agrees across the backends, and the census shows the
> pictures agreeing to one part in a thousand for that reason.** What remains true below is only
> that the map's bytes are not decoded by either side, which is a question about how the gallery
> authors its skies and not about the exporter or the renderer.

Measured 2026-10-13, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`, source at
`C:\resource\cycles` @ `a456b761034dda42c32eef9f4aae0fa5a5c9f604`. Probes: `out/probe-landscape-sky.py`
(where the gap is), `out/probe-world-sky.py` (is the texture read at all), `out/probe-world-colorspace.py`
(the measurement). Documents: `out/wc-named.cycles.xml` and `out/wc-double.cycles.xml`, which are
`out/rz-landscape.cycles.xml` with one attribute changed each. Frames hashed on the way in:
`out/rz-landscape.png` `2683f1a2abda4f1c` (traced) and `out/rr-landscape.png` `f7c00ad9cc65d009`
(rasterised).

## The question

`landscape` is out by more than twenty per cent on level, the largest gap in the gallery, and
`a-low-correlation-measures-the-bounce-and-the-samples.md` closed every control it could think of
without explaining it. A row profile puts the whole gap in the upper half of the frame and leaves the
near ground alone, so the sky is the place to look.

## The answer

**The exporter writes `<environment_texture>` with no `colorspace`, and this build answers for an
unnamed space by loading the image as scene linear.** The map is an eight-bit sRGB PNG, so its
encoded bytes were being used directly as linear radiance.

The evidence is three rows of a table and one warning.

| rows | shipped | `colorspace="__builtin_srgb"` | strength doubled | rasterised |
|---|---|---|---|---|
| 0–128 | 0.2470 | **0.3817** | 0.4940 | 0.4407 |
| 128–256 | 0.2612 | 0.3921 | 0.5224 | 0.4782 |
| 256–384 | 0.2797 | 0.4051 | 0.5594 | 0.4838 |
| 384–512 | 0.3036 | 0.4210 | 0.6071 | 0.5152 |
| 512–640 | 0.3628 | 0.4633 | 0.6845 | 0.5592 |
| 640–768 | 0.7225 | 0.7616 | 0.8618 | 0.6518 |
| 768–896 | 0.7899 | 0.8156 | 0.8690 | 0.6588 |
| 896–1024 | 0.6864 | 0.7260 | 0.7861 | 0.6102 |
| **whole frame** | 0.4566 | **0.5458** | 0.6730 | 0.5497 |

`out/air.png` itself reads 0.4555, so the rasterised sky is within three per cent of the map it is
showing and the shipped traced sky is forty six per cent below it. Naming the colour space takes the
whole frame from **0.831** of the rasterised one to **0.993**.

The warning, on every render of the shipped document:

```
WARNING: Colorspace  can't be converted to scene_linear: ColorSpaceTransform: empty source color space name.
WARNING: Colorspace  not found, using scene linear instead
```

The name in both lines is empty, and it is empty because the socket's default is empty:
`shader_nodes.cpp:542` declares `SOCKET_STRING(colorspace, "Colorspace", u_colorspace_auto)` and
`colorspace.cpp:24` declares `ustring u_colorspace_auto;` — a default-constructed `ustring` is the
empty string. `colorspace.cpp:391-401` then takes the "not found" branch and caches scene linear for
it.

**These two lines are not evidence about the environment map, and counting them proves nothing.**
They appear, unchanged and the same number of times, in the render of a document that carries **no
environment texture at all** (`out/wx-c-unconnected.log`), and naming the colour space does not
reduce them: `out/wc-named.log` still has exactly the pair. `colorspace.cpp:400` caches the miss under
the empty name, so however many nodes ask, the first one warns and the rest are silent. A warning
whose count is independent of the thing being fixed cannot be the check that the thing was fixed, and
the first version of this file claimed it was.

The image is the evidence, not the log.

## `__builtin_srgb` is the correct name, not a constant that happened to help

**FALSIFIED on 2026-10-13, later the same day. The rest of this section is kept because it is the
reasoning that led to the change and the measurement that killed it is in
[what-the-colour-space-attribute-actually-does](what-the-colour-space-attribute-actually-does.md).**

`colorspace.cpp:28` binds `u_colorspace_srgb` to the string `__builtin_srgb`, and `colorspace.cpp:121-123`
routes it through `to_scene_linear`, which is the sRGB electro-optical transfer function. **This
build does not do that.** Rendering one world-only document twice, changing only that attribute:

| candidate for what the attribute does to the frame | largest error against the measured curve |
|---|---|
| `srgb_to_linear` | 0.5129 |
| **`pow(x, 0.4545)`** | **0.0053** |
| identity | 0.2258 |

0.0053 is eight bit rounding. **The attribute applies a 1/2.2 encode to the file being written, not
an sRGB decode to the texture being read.** Two documents that differ only in this attribute share
the same texture, so nothing about the texture can explain the difference between them.

The change `e75a18e` shipped on the strength of this section. The frame moved, and it moved towards
the rasterised frame, and **both of those are still true** — but the rasterised frame is also gamma
encoded, so an output gamma agreeing with it is a coincidence of the same kind as the one this file
already records twice. The stated mechanism was wrong and the change is under review.

The two spelling arguments below are unaffected and still hold: `sRGB` is rejected outright by this
build, and a builtin name does not depend on the machine's OpenColorIO configuration.

The rasterised side was the control. Its sky sits within three per cent of the map's own mean, which
is what a decode on read and an encode on write round-trips to. **That reasoning is now known to be
wrong on its own terms**: the rasterised side round-trips and the traced side after this change does
not, because the change applies an encode to the whole frame rather than a decode to the map.

## Three explanations that were wrong, and how each died

**The air image did not exist when `b0-landscape` was rendered.** The timestamps suggested it:
`out/air.png` is stamped 21:28:52 and `out/b0-landscape.png` 21:25:13. It is wrong, because a
re-render made after the image was certainly present came back **byte identical** to the earlier one —
`out/lz-landscape.png` and `out/rz-landscape.png` are both sha256 `2683f1a2abda4f1c`. The stamp only
records the most recent write. A timestamp is not a witness.

**The loader looked for the map somewhere other than beside the document.**
`cycles_xml.cpp:877` sets `state.base = path_dirname(filepath)` and `cycles_xml.cpp:345-348` joins the
file name onto it, so `out/air.png` is found. The path was never the fault.

**The map was never read, and the sky was a flat colour.** This one is half true and was worth
separating, because it predicts a completely different fix. `out/probe-world-sky.py` renders a
document with no geometry at all, so the whole frame is world: with the `world_texture color` connect
in place the frame has structure, with that one line removed the frame is perfectly flat (mean and
median both 0.163641). The map **is** being read, and reading it makes the sky *darker* than the
background node's own colour.

## Not done

**The fix does not close the scene.** After naming the colour space the sky is still 0.82 to 0.87 of
the rasterised frame and the near ground is 1.17 to 1.24 *hot*, in opposite directions. The
whole-frame mean of 0.993 is an average over two errors that do not cancel cleanly and should not be
read as agreement. On the median over shared pixels the ground moved the wrong way too, 1.13 to 1.21
and 1.29 to 1.36, so the change helped the sky and did nothing for the ground.

**Only `landscape` was measured.** `dunes` and `terraces` carry skies too and neither was re-rendered
under the change.

**The comparison above is a whole-frame mean over display-referred PNG values.** It is not the
bounce-free masked-median figure the census uses, and the two must not be quoted as one number.

The exporter change is written up in `proposals/` rather than applied here.