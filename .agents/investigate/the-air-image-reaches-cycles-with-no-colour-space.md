# The environment map reaches Cycles with no colour space, and it is read as scene linear

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

## `__builtin_srgb` is the correct name, not a constant that happened to help

Two independent reasons, and the second is the one that matters.

**It is Cycles' own sRGB space.** `colorspace.cpp:28` binds `u_colorspace_srgb` to the string
`__builtin_srgb`, and `colorspace.cpp:121-123` routes it through `to_scene_linear`, which is the sRGB
electro-optical transfer function. Naming it asks for the decode the file format already implies.

**It does not depend on the OpenColorIO configuration the machine happens to have.** The obvious
spelling, `colorspace="sRGB"`, is **rejected** by this build — `Color space 'sRGB' could not be
found` — and renders byte-for-byte the shipped frame, because the fallback is the same scene linear.
A name that only works on a machine with a full OCIO config installed is the wrong thing for an
exporter to write; a builtin name is the same everywhere.

The rasterised side is the control, not the other way round. Its sky sits within three per cent of
the map's own mean, which is what a decode on read and an encode on write round-trips to. That is the
behaviour `__builtin_srgb` reproduces on the traced side, and it is why the level lands where it does.

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
read as agreement.

**The warning is not only about the environment map.** It appears in the render of the document that
has no environment texture at all, so a second, unidentified colour space lookup is also unnamed.

**Only `landscape` was measured.** `dunes` and `terraces` carry skies too and neither was re-rendered
under the change.

**The comparison above is a whole-frame mean over display-referred PNG values.** It is not the
bounce-free masked-median figure the census uses, and the two must not be quoted as one number.

The exporter change is written up in `proposals/` rather than applied here.