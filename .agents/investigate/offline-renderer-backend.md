# Selecting an Offline Renderer Backend

Surveyed 2026-10-07. Question answered: we need production final output, not a
reference image. The rasterised path exists only for fast iteration during
content authoring and for debugging the pipeline. Which mature open source
renderer should we drive, and how?

## The constraint that shapes everything

No mature offline renderer is embeddable. There is no stable library interface to
link against and no C or Rust ABI. Every candidate is a program that reads a
scene description and writes an image.

Therefore the integration boundary is fixed: **the scene compiler emits a file,
and a subprocess runs the renderer.** There is no third option, and any proposal
that assumes otherwise should be rejected without further analysis.

This makes the target's scene format the single most important selection
criterion. It matters more than image quality, because a documented format with a
one-line command interface beats a slightly better renderer behind an
undocumented one.

## Cycles without Blender

The direct answer to "is there a way to use Cycles other than through Blender":
**yes, Cycles Standalone exists.**

The official `blender/cycles` repository states that Cycles "can be built as a
standalone application or a Hydra render delegate", and gives this usage:

```
./cycles scene_monkey.xml
./cycles --samples 100 --output ./image.png scene_monkey.xml
./cycles --shadingsys osl scene_osl_stripes.xml
```

The command line lives in `src/app/cycles_standalone.cpp`. Relevant options:
`--device` (CPU, CUDA, OPTIX, HIP, OneAPI), `--samples`, `--background`,
`--output`, and `--shadingsys` selecting either SVM or OSL. Output is written
through OpenImageIO.

The full chain is therefore: our compiler emits a scene file, and one subprocess
call produces the image. No bpy, no Blender, no `.blend`. Cycles is Apache-2.0,
so this path also sidesteps the question of driving the GPL-licensed Blender
application through its Python interface.

Two costs, both revised on 2026-10-07 after reading the source. See
[cycles-standalone-scene-format.md](cycles-standalone-scene-format.md) and
[cycles-standalone-windows-build.md](cycles-standalone-windows-build.md).

1. **No official prebuilt binaries — but the build is not the obstacle this
   document first claimed.** It must be built with
   `-DWITH_CYCLES_STANDALONE`. On Windows, `make.bat update` fetches a
   precompiled dependency set by Git submodule, including OpenImageIO, OSL,
   Embree and OpenImageDenoise. Nothing is assembled by hand. The earlier claim
   of a "day-scale obstacle" came from a third-party wiki describing the retired
   Subversion dependency layout, and is **retracted**.

2. **The scene format has no published specification — but it is readable.** The
   authority is the loader source `src/app/cycles_xml.cpp`, and the format is a
   reflection over the node registry rather than a fixed document: legal
   attributes are exactly the `NODE_SOCKET_API` members declared in
   `src/scene/film.h`, `src/scene/integrator.h` and `src/scene/shader_nodes.h`.
   The residual cost is maintenance, not reverse engineering. There is no version
   field, an unknown attribute is a logged error rather than a failure, and the
   format drifts with upstream commits, so any adoption must pin an exact commit.

A third-party code index records that the standalone application reads "a custom
XML scene format or USD files". **Confirmed from source on 2026-10-07:** the USD
path is real, under `WITH_USD`, via `HD_CYCLES_NS::HdCyclesFileReader::read`, and
USD ships in the precompiled dependency set. It is still rejected, for reasons
that are about us rather than about USD: it duplicates what `StandardScene`
already models, each renderer carries its own specialisms regardless, writing one
emitter per renderer is a small bounded job, and a USD toolchain would have to be
driven from Rust to yield any benefit. The format we are adopting instead is
recorded in [cycles-standalone-scene-format.md](cycles-standalone-scene-format.md).

## Open source candidates

| Project | Licence | Stars | Last push | Invocation |
| --- | --- | --- | --- | --- |
| [appleseed](https://github.com/appleseedhq/appleseed) | MIT | 2327 | 2026-06-11 | `appleseed.cli --threads 8 --output out.exr scene.appleseed` |
| [LuxCore](https://github.com/LuxCoreRender/LuxCore) | Apache-2.0 | 1335 | 2026-10-04 | `luxcoreconsole -D renderengine.type BIDIRCPU -D film.width 1920 -o out.exr scene.cfg` |
| [Mitsuba 3](https://github.com/mitsuba-renderer/mitsuba3) | MIT | — | v3.8.0, 2026-02-23 | `mitsuba -m cuda_ad_rgb scene.xml -o out.exr` |
| [Cycles](https://github.com/blender/cycles) standalone | Apache-2.0 | — | — | `cycles --device OPTIX --samples 512 --background -o out.exr scene.xml` |

### appleseed

Positions itself as "a modern open source rendering engine for animation and
visual effects", not as a research tool, which is the correct framing for a
production backend.

Its native scene format is XML with a published schema. It offers a Python API,
Open Shading Language support, **built-in Open Image Denoise**, stochastic
progressive photon mapping, adaptive sampling, object instancing, displacement
mapping, and the Flamenco farm manager. Official prebuilt binaries exist for
Windows.

Weaknesses: the graphics processor backends are far less mature than Cycles', and
the ecosystem is narrow.

### LuxCoreRender

The most actively developed project in this set, and the cleanest fit to our
scene content.

Bidirectional path tracing plus PhotonGI plus Metropolis light transport, so
caustics and specular-diffuse-specular paths converge far better than a
unidirectional tracer manages. Network rendering through LuxCoreNet. Official
binaries for Windows. Descended from LuxRender, which had architectural
visualisation as its origin, and it shows.

Its `-D key=value` override mechanism is particularly good for us: the compiler
emits the scene, while sample count, resolution and device are injected as
command line arguments by the scheduling layer. Scene and parameters stay
decoupled without a second settings file.

Weaknesses: the `.cfg` and `.rsm` formats carry a dialect of their own, the
material system is LuxCore's own with Open Shading Language alongside it, and the
graphics processor paths trail Cycles.

### Mitsuba 3

The easiest of the four to call, and the most physically rigorous, with spectral
rendering, MIS, bidirectional path tracing, MLT, volumes, an `openpbr` plugin and
an `aov` plugin. Prebuilt Python wheels by default ship four variants
(`scalar_spectral`, `scalar_rgb`, `llvm_ad_rgb`, `cuda_ad_rgb`); the scalar
variants need no LLVM.

But it is positioned as a research renderer. It has no light tree and no built-in
denoising system. A Blender Guru comparison across three scenes found it
competitive or faster than Cycles on standard interior work (18:50 against 19:44
on the interior scene) and rated it the fastest with good out-of-the-box results,
while noting it lacks hair support.

Keep it as the fallback for when physical correctness outranks production
convenience, not as the primary backend.

## Commercial options

If budget allows, Arnold (`.ass` text format plus the `kick` command line) and
V-Ray (`vray` command line plus `.vrscene` XML) are the easiest of all: both have
documented formats, one-line invocations, and genuinely cinematic output quality.
Redshift, Indigo Renderer and Corona also qualify. None of these are open source,
and the quality ceiling of the open source options sits below theirs.

## Fit to our scene content

Our content is procedural mountains, bamboo courtyards and photographic still
lifes: architecture visualisation and product still life. No hair, no character
rigging, no cloth simulation.

That profile happens to remove exactly the areas where Cycles and Mitsuba are
weakest, while favouring LuxCoreRender and appleseed, whose material systems are
tuned for this kind of content. This is worth weighing, but it should not
override the format question — quality differences between these renderers on
architectural content are modest, whereas format documentation is binary.

## Proposed integration architecture

Keep the target format out of the core scene representation.

```
crates/scene/src/standard/     existing StandardScene, the renderer-independent core
crates/scene/src/export/      new: one trait, several backends, sharing the core
```

Define a single emission trait over `StandardScene`, with one implementation per
renderer. Adding a backend then costs a new file and touches nothing else, and
when the core representation evolves, each backend absorbs the change
independently. Start with exactly one backend; the abstraction should be
justified by the second one, not pre-emptively.

The existing `device` crate already provides headless rendering over HTTP with
incremental application of `Delta`, which is the right place to orchestrate
subprocesses, retry failures and collect results.

## Semantic gaps to close before writing any emitter

Format mapping is mechanical. These are not, and each one will otherwise produce
a plausible-looking image that is quietly wrong.

1. **Light types do not correspond.** Our `Light` enum offers Directional,
   Point, Spot and Environment. Offline renderers want area lights and delta
   lights. Directional and Spot map naturally to delta lights. **Point must be
   converted to a sphere light** to obtain physically correct soft shadows;
   rendered as a true point light it will not match our clustered lighting.
2. **Material subset.** Freeze the smallest comparable subset first — Lambertian,
   GGX metal and dielectric. Emitting full OpenPBR or Principled parameters before
   the pipeline can be compared will only produce noise in the comparison.
3. **Environment prefiltering.** `Environment` is image-based lighting on our
   side and an emitter or environment map on theirs. The roughness-driven
   blur must match `proposals/environment-light-limits.md`, or the energy will not
   balance.
4. **Exposure units.** Confirm whether `crates/scene/src/standard/exposure.rs`
   works in stops or multipliers. Mismatched exposure definitions are among the
   most common reasons two renderers disagree, and the image looks plausible
   either way.
5. **Linear output, no tone mapping.** Both sides must produce linear, un-tone-mapped
   EXR, with tone mapping performed in whiting afterwards. If either side
   applies a display transform, no comparison is meaningful.
6. **Geometry.** Our `Topology::Triangles` maps directly. The `mountain` crate's
   facet mesh will need exporting to PLY or OBJ.
7. **Monte Carlo noise.** Comparison is a root mean squared error or relative
   error image at a fixed seed and sample count, not an absolute pixel
   difference.

## Recommendation

1. **Adopt Cycles Standalone with its XML format.** Confirmed on 2026-10-07: the
   format is a readable reflection over the node registry, the command line is a
   single subprocess invocation, OpenImageDenoise is in the precompiled dependency
   set, and the build on this machine is a submodule fetch plus a CMake configure.
2. **Build it before writing the emitter.** A binary makes the remaining unknowns
   cheap: the node type name strings, the sun orientation convention, and whether
   `exposure` is stops or a multiplier on our side. See
   [cycles-standalone-windows-build.md](cycles-standalone-windows-build.md).
3. **Hold appleseed** as the fallback, being the only other candidate with both a
   documented format and official Windows binaries.
4. Hold **LuxCoreRender** as the third choice, favoured if bidirectional path
   tracing for caustics and indirect lighting proves necessary.
5. Hold **Mitsuba 3** as the physical-correctness escape hatch.

USD is **rejected**, superseding the earlier recommendation in this document. It
was the right question to ask and the wrong answer to adopt: it duplicates what
`StandardScene` already models, and a per-renderer emitter is a small bounded job.

In every case, place the renderer behind the emission trait in
`crates/scene/src/export/` rather than inside `StandardScene`.

## Verification status

The Cycles standalone command line and its XML examples were read from the
official `blender/cycles` README and cross-referenced against a code index
description of `src/app/cycles_standalone.cpp`. The Cycles USD claim, marked unverified when this document was first written, was
**confirmed from source on 2026-10-07** and then rejected on our own terms rather
than on technical grounds. The Cycles XML loader, film, integrator and shader node
headers were read directly at commit `a456b761`. See
[cycles-standalone-scene-format.md](cycles-standalone-scene-format.md) for what
that reading established, and
[cycles-standalone-windows-build.md](cycles-standalone-windows-build.md) for the
retracted build-cost claim.

appleseed and LuxCore star counts, licences and last-push dates were read from
the GitHub REST API on 2026-10-07. Mitsuba 3 version and date come from its Zenodo
record. Renderer comparison timings are from a secondary article and are
illustrative, not measurements.

We have not built, run or benchmarked any renderer in this document. Nothing here
has been validated against an actual render.