# Offline Ray Tracing Renderers in the Rust Ecosystem

Surveyed 2026-10-07. Question answered: if we need an offline path tracer, what
can Rust give us today?

## Conclusion

There is no production-grade offline renderer in Rust. Every serious candidate is
an individual project, and the three most complete ones have all but stopped
developing. The only Rust project that is both actively maintained and
technically substantial, `pbrt-r4`, is roughly two months old with almost no
outside testing.

This is not a problem, provided the integration boundary is chosen correctly. It
implies the boundary must be **a scene file plus a subprocess**, never a library
call. See [offline-renderer-backend.md](offline-renderer-backend.md) for why no
embeddable option exists and what we should use instead.

## Tier 1 — Genuine offline renderers

| Project | Device | Licence | Stars | Last push |
| --- | --- | --- | --- | --- |
| [ototoi/pbrt-r4](https://github.com/ototoi/pbrt-r4) | CPU | Apache-2.0 | 14 | 2026-10-07 |
| [wahn/rs_pbrt](https://github.com/wahn/rs_pbrt) | CPU | Apache-2.0 / MIT | 831 | 2024-01-29 |
| [ekzhang/rpt](https://github.com/ekzhang/rpt) | CPU | Apache-2.0 / MIT | 431 | 2022-04-15 |
| [oceanusxiv/pathtracer-rs](https://github.com/oceanusxiv/pathtracer-rs) | CPU plus preview | not stated | — | 2023-04-02 |

### pbrt-r4

The most ambitious Rust renderer that exists. A port of pbrt-v4 published on
crates.io as `pbrt-r4` (v4.3.3 at time of writing), roughly 117 thousand lines of
source.

Mirrors the pbrt-v4 class structure on every major axis: spectral rendering via
four-wavelength `SampledSpectrum` packets, a `Light` trait matching `SampleLi`,
`PDF_Li`, `SampleLe` and `Phi`, and integrators translated line by line:
Path, SimplePath, VolPath, SimpleVolPath, LightPath, SPPM, BDPT, MLT,
RandomWalk, AO and Function. Also ships a self-authored quad-Bounding Volume
Hierarchy accelerator with hand-written SSE and NEON aggregation, `rayon` in
place of pbrt's hand-rolled thread pool, a `nom` grammar in place of the
recursive-descent parser, the pbrt-v4 command line utilities (`imgtool`,
`plytool`, `nanovdb2pbrt`, `cyhair2pbrt`, `pspec`), a `--upgrade` path that
converts pbrt-v3 scenes to v4, and tev display-server support.

Risks, stated plainly:

- Repository created 2026-08-09. Fourteen stars. The crates.io release page
  reported roughly twenty downloads in total.
- The project states it is typically slower than the C++ reference and does not
  guarantee pixel-level agreement, noise, render time or memory parity.
- No GPU or wavefront path. No PTex. Medium vertices in BDPT and MLT, and full
  volumetric BSSRDF parity, are incomplete.

Usable for reading and experimentation. Not usable as a source of truth.

### rs_pbrt

The counterpart to the third-edition PBRT book, maintained by Jan Walter, with
published documentation. 1697 commits, released on crates.io as `rs_pbrt`
(v0.9.12). Integrators: AO, direct lighting, Whitted, path, BDPT, MLT, SPPM and
volumetric path. Runs the book's official test scenes including the Ganesha
statue, the subsurface-scattering dragon and the ecosystem.

No commits since 2024-01-29. No spectral rendering. Dependency versions are
stale enough that recent toolchains may reject them.

### rpt

Built for a MIT course, but the most pleasant API in the set. One hundred percent
safe Rust, a declarative scene builder, microfacet bidirectional reflectance
distribution functions with multiple importance sampling, kd-trees, OBJ, MTL
and STL loading, high dynamic range environment maps, depth of field, iterative
rendering with variance estimation and firefly suppression, and linear scaling
across up to 96 cores.

Published as a library (`rpt = "0.2"`), which makes it the only candidate here we
could genuinely depend on rather than vendor. No commits since 2022-04-15.
Material model is narrow.

One section of its README is worth reading for unrelated reasons: it explains how
the author translated the book's C++ idioms into Rust — composition replacing
class inheritance, enums replacing tagged pointers, and a generic-parameter trick
for working around the absence of operator overloading. That applies directly to
porting other C++ rendering code.

### pathtracer-rs

Built on the PBRT book using `nalgebra` for mathematics, `gltf` for scene input
and `wgpu` for a preview front end. The only candidate in this tier that accepts
glTF, with partial support for `KHR_lights_punctual`, `KHR_materials_ior` and
`KHR_materials_transmission`. Has a genuine `--headless` mode, plus Mitsuba scene
files and tev remote display.

No commits since 2023-04-02, and its `wgpu` dependency is old enough that a
modern toolchain will probably refuse it.

## Tier 2 — Offline path tracers on the graphics processor

| Project | Notes | Licence | Stars | Last push |
| --- | --- | --- | --- | --- |
| [alanjian85/prisma](https://github.com/alanjian85/prisma) | Most complete of the set | MIT | 28 | 2024-12-23 |
| [pema99/rust-path-tracer](https://github.com/pema99/rust-path-tracer) | Most interesting architecturally | MIT | 21 | — |

**prisma** renders glTF scenes offline through `wgpu`: microfacet models, a
Bounding Volume Hierarchy with surface area heuristic, importance sampling.
Small community, no forks, untouched since late 2024.

**pema99/rust-path-tracer** transpiles Rust to SPIR-V with `rust-gpu` and executes
it through `wgpu`, so the GPU kernel is also Rust. Kernel code, shared structures
and host code live in separate directories, and the interface can run the GPU
kernel on the CPU from a dropdown for debugging. Surface area heuristic binned
Bounding Volume Hierarchy, low-discrepancy sequences, next event estimation,
`assimp` scene loading, optional OpenImageDenoise behind a feature flag. The
author calls it a toy tracer and it ships with an interactive window rather than
a batch entry point.

## Tier 3 — Real-time first, offline as an add-on

**[acccoco/TruvisRenderer](https://github.com/acccoco/TruvisRenderer)** — Rust,
Vulkan 1.3 and Slang, 33 stars, last push 2026-09-25, **no licence file
present**, which is a problem for anything we intend to build on. It has a real
offline path tracing mode with its own accumulation state, but by its own
documentation that path does not reuse the real-time machinery. Locked to
Windows on 64-bit and NVIDIA RTX hardware, with a heavy build chain spanning
Tauri, React, CMake, VCPKG and `just`. Best-engineered project in the survey and
still the wrong shape for us.

## Spectral rendering

**[gillett-hernandez/rust-pathtracer](https://github.com/gillett-hernandez/rust-pathtracer)**
— wavelength-aware path tracer, 24 stars, last push 2024-07-09, **GPL-3.0**.
Worth knowing that it exists, because spectral rendering is otherwise absent from
this landscape. The copyleft matters if we ever build on it.

## Look-alikes to avoid

**[panxinmiao/myth](https://github.com/panxinmiao/myth)** has 220 stars, a
hardware rendering engine topic tag and a name that invites confusion. It is a
real-time rasteriser: clustered forward lighting, screen-space global
illumination, screen-space reflections, ambient occlusion, 3D Gaussian
Splatting, a render graph architecture. It contains no offline path tracing at
all.

Also real-time interactive path tracers built from the Ray Tracing in One Weekend
book, and therefore not offline renderers:
[tchauffi/rust-rasterizer](https://github.com/tchauffi/rust-rasterizer),
[0dpe/path-tracer](https://github.com/0dpe/path-tracer),
[Mrmayman/rustracer](https://github.com/Mrmayman/rustracer),
[gam0022/hanamaru-renderer](https://github.com/gam0022/hanamaru-renderer).
Several of these surface prominently in search and are easy to mistake for the
thing we need.

## Reusable components

Worth adopting independently of any renderer choice:

- **[svenstaro/bvh](https://github.com/svenstaro/bvh)** — 270 stars. Surface area
  heuristic Bounding Volume Hierarchy with `rayon`-parallel construction.
  Supports partial tree updates, so a small scene edit does not force a full
  rebuild. Exposes a flattened traversal suitable for shaders. Hand-written SIMD
  for ray-AABB intersection, gated behind a nightly `simd` feature. Good fit if
  we ever want our own path tracer as a debug tool.
- `exr`, `image`, `ply-rs`, `gltf` — image and mesh interchange, all mature.
- `nanovdb-rs` — pure Rust NanoVDB reader, used by pbrt-r4.
- `optica` — radiative transfer and optical depth primitives, version 0.3,
  supports `no_std`. Not a renderer; a building block.

## Verification status

Star counts, creation dates and last-push dates were read directly from the
GitHub REST API on 2026-10-07. Feature descriptions were taken from project
READMEs and from crates.io release notes.

Search results surfaced numerous mirror sites serving altered copies of GitHub
pages (`github.es`, `ghub.com`, `togithub.com`, `gitgub.com` and similar). Their
figures were discarded and the API was used instead. A few secondary articles
aggregating renderer comparisons appeared; none were used for the table above.

We have not compiled, run or benchmarked any project in this document.