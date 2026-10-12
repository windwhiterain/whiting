# NURBS libraries usable from Rust, and what they cost

Recorded 2026-10-07. Every version, download count and date below was read from the crates.io
API or a source file on that date; these numbers decay, so re-read them before relying on them.

## Question

Which concrete libraries can implement hierarchical NURBS sparse editing and NURBS-ization of
arbitrary polygons, and what do they cost in licence, language, maturity and completeness?

## Answer in one line

**No Rust crate exposes a multi-patch NURBS model with per-edge continuity, and none exposes
genuinely local hierarchical refinement.** The tensor-product algebra itself *is* available —
`curvo` (MIT) and `axiolid-nurbs` (MPL-2.0) both carry classical knot insert/remove and
surface degree elevation. What no crate provides is the layer above them: multi-patch
assembly, per-edge continuity constraints, or OCCT's edge regularity concept.
See "Multi-patch continuity is the actual gap" below, added 2026-10-07.

## Pure-Rust NURBS

### `curvo` — the one to start from

- <https://crates.io/crates/curvo>, <https://github.com/mattatz/curvo>. Author: Masatatsu
  Nakamura.
- **MIT** licence (crates.io metadata for v0.3.2 says `MIT`; the README badge claims
  "MIT/Apache" — the crates.io metadata is the authoritative field).
- v0.3.2, 98 versions, first published 2024-04-18, **updated 2026-10-07** (the day of this
  survey). 374,018 total downloads, 78,151 recent.
- 18,752 lines of Rust across 143 files.
- Depends on `nalgebra` and uses it heavily. Feature flags: `approx`, `rayon`, `serde`, `log`,
  `bevy`. The `bevy` feature exists with interactive viewers, i.e. this is an actively used
  interactive modelling library, not an abandoned one.
- Verified capability from the README: construction from control points / knots / weights;
  interpolation through given points; periodic curves; surface **extrude, loft, sweep,
  revolve**; closest point on curves *and* surfaces; curve-curve and curve-surface intersection;
  divide by arc length; **adaptive tessellation** (`AdaptiveTessellationOptions` with
  normal tolerance); trimmed surfaces; offset and fillet curves; isocurves; surface split;
  curvature and discontinuity queries; booleans between NURBS curves (sponsor-developed).
- **Knot algebra is present and verified on docs.rs.** `KnotVector<T>` exposes `add`,
  `multiplicity`, `find_knot_span_index` (plus a `_with_hint` and a `_linear` variant),
  `greville_abscissae`, `basis_functions`, `derivative_basis_functions`, `clamp`, `normalize`,
  `remap`, `is_clamped`, `domain`, `floor`, `first`, `last`, `len`, `multiplicity`. The curve
  and surface types build on that with `try_add_knot` / `try_remove_knot` / `try_refine_knot` /
  `try_reduce_knots` / `try_elevate_degree`.
  - I initially concluded from the README alone that `curvo` had no knot operations. **That was
    wrong** — the README simply does not list them. Read docs.rs, not the README.
- **Verified gaps:** the knot operations above are the *classical* (Boehm / NURBS-Book A5.x)
  whole-knot-vector kind. There is no local-support function, no T-mesh, no THB basis, and no
  least-squares or mesh fitting — `curvo` interpolates a point list, which is a different thing
  from fitting. Dependencies confirm the absence of a fitting path: `argmin` is present for
  optimisation but there is no spline-approximation crate, and the optional `approx` feature is
  a numeric-tolerance dependency, not a NURBS fitter. Docs coverage is 50.47 %.

### Multi-patch continuity is the actual gap

Added 2026-10-07, after the target changed from "fit a mesh to NURBS" to "turn each face of a
low-poly mesh into a NURBS patch, with a hard/soft tag per shared edge". The requirement
collapsed from a fitting problem to a multi-patch B-rep problem, and that reframes the answer.

**What the tensor-product layer already gives us** (verified on docs.rs, 2026-10-07):

- `axiolid_surface::BSplineSurface` (v0.3.2, MPL-2.0) exposes its whole value as **public
  mutable fields**: `control_points: Vec<Vec<DVec3>>` (rectangular net, row-major in `u` then
  `v`), `u_knots` / `u_multiplicities`, `v_knots` / `v_multiplicities`, `weights: Option<…>`,
  `u_closed`, `v_closed`, `knot_spec`, `self_intersect`; methods `domain()` and
  `jet(u, v) -> Option<SurfaceJet>`. Moving one control point is therefore a plain field write,
  with no API-level obstruction.
  Source: <https://docs.rs/axiolid-surface/latest/axiolid_surface/struct.BSplineSurface.html>
- `axiolid_nurbs` (v0.3.5) supplies the operations a multi-patch model needs and that a plain
  control-net type cannot: `iso_curve_at_u` / `iso_curve_at_v` (extract a patch boundary as a
  curve — this is how a shared edge is obtained), `invert_surface_certified` (point → `(u, v)`),
  `analyze_surface` (first/second partials, for verifying continuity), `insert_surface_knot_u/v`
  and `remove_surface_knot_u/v` (shape-preserving), `elevate_surface_degree_u/v`,
  `reduce_surface_degree_u/v`, `reverse_surface_u/v`.

**What no crate in the table above has**: multi-patch assembly, a shared-edge topology, and a
per-edge continuity constraint. For reference, OCCT *does* have the last of these natively, and
it is worth knowing the shape of the API even if we do not bind it:

- `GeomAbs_Shape` enumerates `C0, G1, C1, G2, C2, C3, CN`.
- `BRep_Builder::Continuity(E, F1, F2, C)` sets it; `BRep_Tool::Continuity(E, F1, F2)` reads it
  and **falls back to `GeomAbs_C0` when no regularity is stored** — so "no tag" means "hard
  edge", which matches the intended default.
  Source: <https://dev.opencascade.org/doc/refman/html/class_b_rep___builder.html>
- Known limitation, from the OCCT issue archive: `BRepLib::EncodeRegularity()` "can set only C0
  and G1 continuity in general case" and "does not set C2 or higher continuity".
  Source: <https://occt3d.com/dev/tickets/0027383>

This reframes the recommendation that follows: the work is **not** NURBS fitting and **not** a
research-grade hierarchical basis. It is a multi-patch boundary representation with a per-edge
continuity invariant.

### Other pure-Rust NURBS crates (crates.io search `q=nurbs`, 106 total matches)

| Crate | Version | Updated | Downloads | Licence | Notes |
| --- | --- | --- | --- | --- | --- |
| `curvo` | 0.3.2 | 2026-10-07 | 374,018 | MIT | see above |
| `axiolid-nurbs` | 0.3.5 | 2026-10-02 | 1,518 | **MPL-2.0** | <https://github.com/axiolid/kernel>. Deepest NURBS API found in Rust: `insert_knot2/3`, `insert_surface_knot_u/v`, `remove_knot2/3`, `remove_surface_knot_u/v`, `elevate_degree2/3`, `reduce_degree2/3`, `split2/3`, `bezier_segments2/3`, periodic variants, exact + certified intersection/projection, **and fitting** (`fit_curve3`, `fit_curve3_to_tolerance`, `fit_surface_grid`, `SurfaceFitOptions`). Self-described "early kernel", not a drop-in CAD replacement. |
| `brepkit-math` | 4.1.37 | 2026-10-07 | 10,223 | **AGPL-3.0-only** | <https://github.com/andymai/brepkit>. **Relicensed** from `MIT OR Apache-2.0` at 3.0.0 on 2026-08-07 — see the licence section, this is the single most consequential finding here. Exposes `nurbs::knot_ops::{curve_knot_insert, curve_knot_refine, curve_knot_remove, curve_split, surface_knot_insert_u/v}`, `nurbs::decompose::{curve_degree_elevate, curve_to_bezier_segments, surface_to_bezier_patches}`, and `nurbs::fitting::{approximate, approximate_lspia, interpolate}` / `nurbs::surface_fitting::{approximate_surface_lspia, interpolate_surface}`. |
| `brepkit-geometry` | 4.1.37 | 2026-10-07 | 8,462 | **AGPL-3.0-only** | same relicensing; sampling, extrema, conversion. Consumes the math layer. |
| `splinify` + `dierckx-sys` | 0.2.2 / 0.1.1 | 2026-03-10 / 2026-02-17 | — | MIT OR Apache-2.0 | <https://github.com/harbik/splinify>. Rust wrappers around Dierckx **FITPACK Fortran** — curve/surface fitting, interpolation and smoothing. `dierckx-sys` has only `cc` + `libc`, so it compiles vendored Fortran at build time. |
| `monstertruck` / `monstertruck-geometry` | 0.4.1 | 2026-09-19 | 431 | Apache-2.0 | <https://github.com/virtualritz/monstertruck>. Has a `t_spline` module with `Tnurcc` / `Tmesh` / `TmeshControlPoint` / `TmeshConnection` — **data types only, no refinement algorithms.** Its own doc text: "T-meshes and T-NURCCs are *surfaces*. It does not make sense to have a T-mesh curve or T-NURCC curve". Cites Sederberg 1998 and 2003, so the intent is real but the algorithms are unpublished. |
| `vcad-kernel-nurbs` | 0.10.0 | 2026-09-09 | 304 | unverified | evaluation for the vcad kernel, <https://github.com/ecto/vcad> |
| `neco-nurbs` | 0.1.0 | 2026-04-03 | 328 | unverified | 2D rational NURBS curves and regions |
| `tpt-eng-nurbs` | 0.1.0 | 2026-08-15 | 40 | unverified | splines + tessellation, very new and tiny |
| `splinosaurus` | 0.2.2 | **2024-02-05** | 5,076 | unverified | the *only* crate matching `q=tspline`; "flexible spline and surface library", ~2 years stale |
| `capstan` | 0.0.3 | **2020-10-20** | 4,890 | unverified | "NURBS library with a CAD focus" — dormant ~6 years |
| `splines` | 5.0.0 | 2025-04-26 | 1,154,823 | BSD-3-Clause | **Not a B-spline library.** Per-segment interpolation (Linear/Cubic/Hermite/CatmullRom); there is no knot-vector object at all. Popularity is misleading here. |

### Correction: `curves` is not a NURBS library

`GET https://crates.io/api/v1/crates/curves` returns a **3-line placeholder** published
2024-01-04 by user `bgraceful`, licence "non-standard", no lib target, 1,478 downloads. It is
not the curve_similarity NURBS crate. Do not add `curves` to a Cargo.toml expecting splines.

## Rust CAD kernels (crates.io search `q=opencascade`, 17 total matches)

A much richer Rust CAD ecosystem exists in 2026 than is commonly known:

| Crate | Version | Updated | Downloads | What it is |
| --- | --- | --- | --- | --- |
| `occt-sys` | 7.8.1 | 2026-08-22 | 155,685 | **static** build of the OCCT C++ kernel, <https://github.com/bschwind/opencascade-rs> |
| `opencascade` / `opencascade-sys` | 0.3.0 | 2026-08-24 | 5,507 / 7,025 | high-level bindings over the same kernel |
| `monstertruck` | 0.4.1 | 2026-09-19 | 431 | "A Rust CAD kernel – B-rep modeling, NURBS, meshing, and GPU rendering", <https://github.com/virtualritz/monstertruck> — **closest in domain to this repository** |
| `cadrum` | 0.8.20 | 2026-09-13 | 6,134 | statically-linked headless OpenCASCADE, runs natively and in WASM, <https://github.com/lzpel/cadrum> |
| `BREP_kernel` | 0.8.0 | 2026-10-07 | 300 | B-rep kernel, <https://github.com/mmiscool/next.BREP.io_RUST_BREP_KERNEL> |
| `occt-wasm` | 4.0.0 | 2026-09-21 | 354 | OpenCASCADE via WebAssembly, **no C++ toolchain required** |
| `brepkit-math` / `-geometry` / `-io` | 4.1.37 | 2026-10-07 | ~10k each | `brepkit-math` is "Vector math, transforms, NURBS, and geometric predicates"; `brepkit-io` reads/writes STEP, IGES, STL, 3MF, OBJ, PLY, glTF |
| `cadcore` / `cadcore-ops` | — | 2026-06-06 | 727 | "analytic B-Rep construction without Boolean ops" |
| `oxideav-step` | 0.0.1 | 2026-10-06 | 95 | pure-Rust STEP AP242/AP214/AP203 B-rep reader |

`chijin` is **deprecated** in its own description in favour of `cadrum`.

### Correction: the `occt` crate does not exist

`GET https://crates.io/api/v1/crates/occt` returns `{"errors":[{"detail":"crate `occt` does not
exist"}]}`. The real names are `occt-sys` (static kernel) and `opencascade` / `opencascade-sys`
(bindings).

## What does not exist in Rust

- **No crate exposes genuinely local hierarchical NURBS refinement** — THB-splines, T-splines,
  PHT-splines, LR B*-splines. This is the headline negative result and it survives every check.
  - `monstertruck-geometry` has the T-spline *types* but no refinement functions.
  - A crates.io search for `tspline` returns exactly one crate, `splinosaurus`, which is ~2
    years stale with unverified licence and no NURBS-local-support evidence.
- **No crate does arbitrary-topology mesh → NURBS decomposition + parameterization + fitting.**
  Fitting *primitives* do exist in pure Rust — `brepkit-math`
  `nurbs::surface_fitting::approximate_surface_lspia` (AGPL), `axiolid-nurbs`
  `fit_surface_grid` (MPL-2.0), FITPACK via `splinify` — but all of them consume a regular grid
  or point list. The decomposition and parameterization stage in front of them is missing
  everywhere, which matches the C++ finding in
  [nurbs-izing-arbitrary-polygons.md](nurbs-izing-arbitrary-polygons.md).
- **No Rust bindings for CGAL** that are worth using (and see the CGAL retirement note in
  [nurbs-izing-arbitrary-polygons.md](nurbs-izing-arbitrary-polygons.md)).
- The `opensubdiv` and `opensubdiv-sys` crates are dead placeholders (scott-wilson,
  self-described as "placeholder", last touched 2020). The live ones are `opensubdiv-rs` (a
  pure-Rust port, created 2026-08-09, 221 downloads) and `opensubdiv-petite` — both give
  subdivision surfaces, not NURBS.
- Bare `manifold` is an unrelated 183-line 2D/3D primitive crate; do not conflate with
  `manifold3d`.

## Licence risk — read this before choosing a kernel

### brepkit — AGPL-3.0-only, and it changed recently. Verified directly.

I fetched `raw.githubusercontent.com/andymai/brepkit/main/LICENSE` on 2026-10-07: it is the
full **GNU Affero General Public License Version 3**. The whole `brepkit-*` family went from
`MIT OR Apache-2.0` to **AGPL-3.0-only at 3.0.0 on 2026-08-07**.

This matters more than anything else in this survey, because `brepkit-math` is otherwise the
most capable pure-Rust NURBS option found (knot ops, Bézier decomposition, and least-squares
surface fitting). AGPL §13 reaches **network interaction**, and linking AGPL code into anything
you distribute — even just the geometry crate — is a strong contamination risk. A permissively
licensed predecessor exists, but it is two months old and the API has moved since.

### OCCT — LGPL-2.1 + a specific exception

Verified by reading `OCCT_LGPL_EXCEPTION.txt` from the OCCT `master` tree on 2026-10-07:

> Open CASCADE exception (version 1.0) to GNU LGPL version 2.1.
> The object code (i.e. not a source) form of a "work that uses the Library" can incorporate
> material from a header file that is part of the Library. […] you may distribute such object
> code incorporating material from header files […] **under terms of your choice**, provided
> that you give prominent notice in supporting documentation to this code that it makes use of
> or is based on facilities provided by the Open CASCADE Technology software.

Consequences:

- The library is **LGPL-2.1** with this exception (distributions label it
  `LGPL-2.1-only WITH OCCT-exception-1.0`).
- The exception covers **header-file material in object code**. It does not cover the compiled
  `.a`/`.lib` and it does not remove the LGPL's own obligations for the library itself.
- **`occt-sys` is a static build of OCCT**, which is precisely the configuration LGPL-6
  restricts. OCCT **7.9.0 explicitly added "a warning message regarding LGPL 2.1 licensing
  limitations for static linking"** to its build system
  (<https://github.com/Open-Cascade-SAS/OCCT/releases/V7_9_0>).
- `cadrum` is also statically-linked OpenCASCADE. Its binding layer is MIT, but that does not
  relicense the OCCT body underneath.
- A commercial Open CASCADE licence is the clean exit. Otherwise, **dynamic linking is the
  low-risk path** and a static `occt-sys` in a distributed product means satisfying the LGPL
  relinking obligation (ship OCCT source or object files, or offer them).
- Also note the two Rust routes expose **different OCCT majors**: `occt-sys` pins 7.8.1 while
  `cadrum` pins 8.0.0.

### Others

- **CGAL** — GPL/LGPL decided per package, `NOASSERTION` on GitHub. Kernels and the support
  library are LGPL v3+; most geometric algorithms are GPL. The exact licence of
  `Shape_approximation` was **not confirmed** — check
  <https://doc.cgal.org/latest/Manual/packages.html> before relying on it. Commercial licensing
  via GeometryFactory.
- **`axiolid-nurbs`** — MPL-2.0, weak file-level copyleft. The most manageable of the
  capable options: modified MPL files stay MPL, larger works need not.
- **OpenSubdiv** — **TOST 1.0** (Tomorrow Open Source Technology Licence), an Apache-2.0
  variant differing only in §6 (trademarks). Permissive, but **not** BSD as commonly assumed.
- **geogram** — BSD 3-Clause (Inria). Retain the notice; reproduce it in binary distributions.
- **Permissive and clean**: `curvo` MIT, `splinify` MIT OR Apache-2.0, `manifold3d`
  Apache-2.0 OR MIT, `monstertruck*` Apache-2.0, `geomdl` MIT, QuadriFlow MIT.

### Build-time (not licence) risk on Windows

- `dierckx-sys` compiles **vendored Fortran** at build time; on Windows that means an
  MSVC-compatible Fortran toolchain. This is the sharpest edge among the candidates.
- `occt-sys` averages ~13m46s and `opencascade` ~8m34s per docs.rs build, and **both failed**
  to build at 0.3.0 — the last successful `opencascade` docs are 0.2.0 (2023-08-16), so its
  published API surface is stale.
- `occt-wasm` and `cadrum`'s WASM target avoid the toolchain entirely but pay a WASM boundary
  cost on every call, which defeats fine-grained per-control-point editing.

## C++ reference implementations

| Library | Licence | NURBS local edit | Mesh → NURBS | Windows story |
| --- | --- | --- | --- | --- |
| **OpenCASCADE** | LGPL-2.1 + OCCT exception | Yes — `InsertUKnot`, `RemoveUKnot`, `MovePoint` (verified in source) | Partial — `GeomAPI_PointsToBSplineSurface` needs a rectangular point grid; `GeomConvert_ApproxSurface` approximates existing analytic surfaces | Official MSVC and MinGW builds; 7.9.0 added vcpkg manifest mode and MinGW defaults |
| QuadriFlow | **MIT** (verified) | no | Quad-dominant remeshing only, **manifold input required** | C++11, builds standalone |
| CGAL 6.2 | GPL/LGPL per package | no | `Surface_mesh_approximation` (VSA) yes; `Nurbs_fitting` appears **retired** | header-heavy, needs Boost ≥ 1.74, VS 2017+ |
| libigl | MPL-2.0 (`LICENSE.MPL2`; GitHub's detector separately flags GPL-3.0 — treat as mixed) | no | no NURBS (triangulation / polygon utilities) | header-only |
| OpenSubdiv | **TOST 1.0** (not BSD) | n/a (subdivision) | n/a | README documents `-G "Visual Studio 16 2019" -A x64 -D NO_PTEX=1` |
| geogram | BSD-3-Clause (Inria) | no | `mesh_repair` yes | CMake; MSVC support exists |

OCCT 7.9.0 (<https://github.com/Open-Cascade-SAS/OCCT/releases/V7_9_0>) is a minor release with
250+ changes; **nothing in its notes concerns local B-spline editing**. Its modelling work is
BOP, offset, fill and boolean robustness. Do not expect a new refinement API.

A useful Python reference for the exact operation set sought — `geomdl` / NURBS-Python 5.4.0
(MIT, 2025-05-23) — explicitly lists knot insertion, knot removal, knot-vector refinement, degree
elevation, degree reduction, **and curve/surface fitting by both interpolation and least-squares
approximation**. It is the cleanest executable specification of the classical op set.

## Recommendation, clearly labelled as judgement

Not a specification — this is a survey note, and any decision belongs in a proposal.

1. **Do not build a hierarchical spline kernel before it is needed.** Every hierarchical family
   is research-grade except as a commercial plugin. The honest cost/benefit depends entirely on
   whether "sparse editing" means *editing a few control points* or *refining without cascade*.
   The first is O(1) and already available; only the second needs THB/T-splines.
2. **If per-control-point editing is the goal, `curvo` is sufficient and cheap.** It has
   classical knot algebra (`try_add_knot`, `try_remove_knot`, `try_refine_knot`,
   `try_elevate_degree`) plus adaptive tessellation, which is what a renderer needs downstream.
   MIT, pure Rust, no toolchain — for per-control-point work at interactive rates this is the
   only genuinely cheap option.
3. **If least-squares surface fitting is the goal, `axiolid-nurbs` (MPL-2.0) is the pragmatic
   pick** and `brepkit-math` is the capable one — but brepkit is now AGPL-3.0-only, which is
   close to disqualifying for anything distributed. `splinify`/FITPACK is permissive but drags in
   a Fortran toolchain.
4. **If true local refinement is the goal, implement THB-splines directly** (the coarse level
   stays ordinary NURBS, so it remains exportable) rather than binding OCCT, which gives you
   classical NURBS refinement — which cascades — plus an LGPL static-linking problem. Remember
   the coarsening side exists and is not optional: a hierarchy you cannot collapse only grows.
5. **For NURBS-izing arbitrary polygons, do not start from the mesh.** QuadriFlow requires a
   manifold, and every fitter requires a parameterized patch. The `mountain` crate already
   produces a *structured* triangle mesh whose every face traces back to an authored skeleton
   (`.agents/SPEC/mountain.md` R4.1) — that correspondence is exactly the patch decomposition and
   parameterization that the general pipeline lacks. That is an asset, not a problem.

## Sources

- crates.io API: `/api/v1/crates/curvo`, `/api/v1/crates/{axiolid-nurbs,brepkit-math,occt-sys,occt}`,
  searches `?q=nurbs`, `?q=opencascade`. Note the parameter is `q`, not `search` — `search` is
  silently ignored and returns alphabetical filler.
- docs.rs: `curvo/prelude/struct.KnotVector.html` (confirmed the knot API directly),
  `axiolid-nurbs/all.html`, `brepkit-math/all.html`, `monstertruck-geometry/t_spline/index.html`.
- `raw.githubusercontent.com/mattatz/curvo/main/README.md`
- `raw.githubusercontent.com/andymai/brepkit/main/LICENSE` — full AGPL-3 text, verified directly.
- `raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/OCCT_LGPL_EXCEPTION.txt`
- `raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/src/ModelingData/TKG3d/Geom/Geom_BSplineSurface.hxx`
- <https://github.com/Open-Cascade-SAS/OCCT/releases/V7_9_0>
- <https://axiolid.github.io/kernel/capabilities>, <https://www.cgal.org/license.html>

**Failed during this survey:** `raw.githubusercontent.com/virtualritz/monstertruck/main/README.md`
returned 404 (branch name may differ); `semanticscholar.org` API was HTTP 429; `dblp.org` is
behind an Anubis bot wall; docs.rs could not build `opencascade` 0.3.0.