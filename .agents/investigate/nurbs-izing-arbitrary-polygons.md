# NURBS-izing an arbitrary polygon mesh

Recorded 2026-10-07.

## Question

What techniques exist to turn an arbitrary polygon mesh into a NURBS representation that is
still editable, and what do industrial kernels actually support for editing it afterwards?

## Answer in one line

Six pipeline families exist, but only two are actually editable; the dominant obstacle is not
NURBS fitting, it is that **no open tool accepts non-manifold / non-watertight input** and no
open tool does patch decomposition plus parameterization.

## The six pipeline families

| # | Pipeline | Produces | Topological assumption | Editable? |
| --- | --- | --- | --- | --- |
| 1 | Per-face NURBS (`MeshToNURB`) | one surface per polygon | none | no |
| 2 | SubD cage → NURBS | packed / unpacked quad patches | quad cage with extraordinary points | partly |
| 3 | Reverse engineering / Scan-to-CAD | analytic primitives + freeform regions | clean watertight scans | yes, human-in-loop |
| 4 | Quad retopology → patch fit | quad-dominant cage → patches | **manifold input required** | yes |
| 5 | Spline fitting to points/mesh | 1..k B-spline surfaces per region | region must be grid-parameterizable | yes |
| 6 | Volumetric / trivariate NURBS | compact NURBS-encoded field | any | yes, as a field |

**1. Naive per-face conversion** is a format change, not a modeling change: O(F) independent
surfaces, no continuity, nothing to grab. Rhino's `MeshToNURB`/`ToNURBS` is the canonical
instance. Rhino's own docs (`docs.mcneel.com`) returned HTTP 403 on every attempt during this
survey, so the specific Packed/Unpacked and G0/G1/G2 extraordinary-vertex option semantics are
**unverified** — do not quote them without re-checking.

**2. SubD → NURBS** has a real mathematical obstruction: a quad cage with extraordinary points
(valence ≠ 4) cannot be compiled to a NURBS of any degree, because a tensor-product B-spline
patch has exactly four control points per corner. The "packed" workaround *removes* the
extraordinary points to make the cage a regular grid, at the cost of a geometric error you
choose — which is why G0/G1/G2 trade-offs exist.

**3. Reverse engineering** is the commercial state of the art, and it is *not* kernel-level
mesh→NURBS. Geomagic/PolyWorks-class tools run **feature recognition → analytic primitives**
first (planes, cylinders, cones, spheres), then fall back to region-growing plus NURBS fitting
only for residual freeform regions. **Negative result:** no vendor SIGGRAPH/SIGRADI technical
paper describing this algorithm was found. Note also that Sorkine-Hornung's differentiable CAD
work is *forward* CAD optimization, a different problem, not mesh→NURBS.

**4. Quad retopology → patch fit** is the best-documented open line:

- **QuadriFlow**, Huang, Zhou, Niessner, Shewchuk & Guibas, Computer-Aided Geometric Design
  (Eurographics SGP 2018), <https://doi.org/10.1111/cgf.13498>. Source
  <https://github.com/hjwdzh/QuadriFlow>, **MIT** (verified by reading `LICENSE.txt`), last
  push 2019-12-07.
- **The blocking constraint, verified from the README:** "We take a manifold triangle mesh
  `input.obj` and generate a manifold quad mesh `output.obj`." Non-manifold / non-watertight
  input is out of scope, and the authors ship a *separate* tool (`hjwdzh/Manifold`) to make
  the input watertight first.
- Feature alignment and sharp preservation are **off by default** (`-sharp` flag); output
  watertightness needs `-sat` with a bundled SAT solver.
- PHT-to-NURBS conversion for the resulting hierarchy is real and recent: "Optimized
  PHT-to-NURBS conversion: A weighted rectangle partition approach", Wu, Deng & Yang,
  CAD 189 (2025-12), <https://doi.org/10.1016/j.cad.2025.103915>, reporting up to 48.94 %
  control-point reduction in 2D and 78.35 % in 3D, with a lossless conversion algorithm for
  open and closed surfaces.
- **Unverified:** PatchLayUp (Tarini et al. SIGGRAPH 2014) and Instant Field-Aligned Meshes
  (`github.com/Instant-Meshes/InstantMeshes` returned 404). Do not cite either as current.

**5. Spline fitting** is the workhorse and the easiest step; segmentation and parameterization
in front of it are the hard steps. From the actual OCCT header
`GeomAPI_PointsToBSplineSurface.hxx`:

- Three algorithms: least-squares approximation; least-squares plus minimization of local shape
  characteristics (first/second/third partial derivatives); interpolation through the points.
- Defaults: degree range [3, 8], C2 continuity, `Tol3D1e-3`. Parametrization options are
  `Approx_ChordLength` (the default), `Approx_Centripetal`, or `Approx_IsoParametric`.
- **The structural limitation is visible in the signature**: the input is
  `NCollection_Array2<gp_Pnt>`, a *rectangular grid*. A caller must first cut the mesh into
  patches **and** parameterize each patch into a regular grid.
- `GeomConvert_ApproxSurface` is a different thing: it approximates an *existing analytic*
  surface to a tolerance with hard degree/segment caps, and exposes `HasResult()` plus
  `MaxError()` — an honest admission that the result may **not** be within tolerance.
- `BRep_NurbsConvertor_approximation.hxx` was **not located** in the current OCCT tree; the
  old `src/OpenCascade/` layout was split into `ModelingData` / `ModelingAlgorithms` / `TKBRep`.

**6. Volumetric / trivariate NURBS.** See [sparse-nurbs-representations.md](sparse-nurbs-representations.md).

## What industrial kernels support for editing afterwards

Verified by reading `Geom_BSplineSurface.hxx` from the OCCT `master` tree on 2026-10-07. This
is the concrete API surface, so it is worth stating precisely which operations are local.

**The sparse-editing primitive is `MovePoint`:**

```
MovePoint(U, V, P, UIndex1, UIndex2, VIndex1, VIndex2,
          &UFirstIndex, &ULastIndex, &VFirstIndex, &VLastIndex)
```

Header comment: "Move a point with parameter U and V to P… indicates the poles which can be
moved… if Problem in BSplineBasis calculation, no change for the curve". This solves for only
the control points in the bounded index box `[UIndex1,UIndex2] × [VIndex1,VIndex2]` — a
genuinely local edit, and the one that matches "only touch what the user touched".

**Refinement and coarsening are local in geometry but global in bookkeeping:**

- `InsertUKnot(U, M, ParametricTolerance, Add=true)` / `InsertVKnot` / `InsertUKnots` /
  `InsertVKnots` — insert knots, surface unchanged geometrically. The poles table is
  renumbered, so the array is O(n) even though only p·q control points actually change.
- `RemoveUKnot(Index, M, Tolerance) -> bool` — the inverse. The header is explicit about the
  guarantee: "Two different algorithms are used systematically to compute the new poles… the
  distance between the pole calculated using the first algorithm and the same pole calculated
  using the second algorithm, is checked. If this distance is less than Tolerance it ensures
  that the surface is not modified by more than Tolerance. Under these conditions, the
  function returns true; otherwise, it returns false." So **knot removal is tolerance-controlled
  and may fail**, and the tolerance is a smoothing knob, not a safety bound.
- `IncreaseDegree`, `IncreaseUMultiplicity`, `IncrementUMultiplicity`, `Segment`,
  `CheckAndSegment`, `SetPole`, `SetPoleRow/Col` — all global renumbering operations.
- `MaxDegree()` is 25.
- `GeomAPI_PointsToBSplineSurface::Init` / `Interpolate` rebuild a surface from scratch; there
  is no incremental or local update path in that class at all.

The honest summary: **local geometric editing is O(1) control points; local refinement is
O(p·q) changed control points but O(n) array bookkeeping; coarsening is not guaranteed to
succeed.** Any design that claims O(1) knot insertion on a tensor-product patch is wrong about
the bookkeeping.

## Open-source implementations

| Project | Language | License | Converts to | Maturity | Source |
| --- | --- | --- | --- | --- | --- |
| QuadriFlow | C++ | MIT (verified) | quad-dominant quad mesh | last push 2019-12-07 | <https://github.com/hjwdzh/QuadriFlow> |
| OCCT | C++ | LGPL-2.1 + OCCT exception (verified in source header) | Geom/B-rep | pushed 2026-10-07 | <https://github.com/Open-Cascade-SAS/OCCT> |
| verb | JS | MIT | NURBS ↔ tessellated mesh | push 2025-04-02 | <https://github.com/pboyer/verb> |
| tinynurbs | C++ | BSD-3 | curve/surface primitives | push 2024-06-13 | <https://github.com/pradeep-pyro/tinynurbs> |
| LNLib | C++ | LGPL-2.1 | NURBS algorithms incl. volumes | push 2026-08-26 | <https://github.com/BIMCoderLiang/LNLib> |
| NURBS-Python (geomdl) | Python | MIT | fitting / surfaces | push 2025-05-24 | <https://github.com/orbingol/NURBS-Python> |
| breptile | Python | MIT | STL → STEP B-rep | 9★, push 2026-08-24 | <https://github.com/David-Feldt/breptile> |

**CGAL's `Shape_approximation` / `Nurbs_fitting` appears to have been retired.** Verified three
ways: CGAL's `copyright` manifest on `main` enumerates every package and contains no
`Shape_approximation` entry (it does list `Surface_mesh_approximation`, a different mesh-only
package); there is no `Shape_approximation` directory on `main`, `6.1.x-branch` or
`6.2.x-branch`; and `Nurbs_fitting_3.h` 404s on every branch. Current CGAL is 6.2 (June 2026).
Any plan that assumes a live open-source CGAL NURBS-fitting API must re-verify this first.

## Gaps nobody has closed

1. **Non-manifold / non-watertight input to a quad layout.** The best open tool requires a
   manifold and ships a separate watertighting step. Nobody has unified these.
2. **Patch decomposition plus parameterization.** `GeomAPI_PointsToBSplineSurface` will only
   accept a rectangular point grid, so segmentation and parameterization must happen first. No
   production-grade open solution was found.
3. **Multi-patch G1 continuity as an automatic post-process.** Nothing found.
4. **Error-controlled fitting from arbitrary topology.** The one verified OCCT approximator
   openly reports when it missed tolerance.
5. **Instant-meshing claims.** No 2024–2026 learned quad-layout method was found in the
   literature; the arXiv `ti:"Quadrangulation"` query returned only pure-mathematics
   combinatorics. Treat "learned quad meshing" as unproven for this use case.

## Sources

All fetched 2026-10-07 unless noted. QuadriFlow README and `LICENSE.txt` via
`raw.githubusercontent.com`; OCCT headers via `raw.githubusercontent.com/Open-Cascade-SAS/OCCT/master/src/...`;
CGAL package list via `raw.githubusercontent.com/CGAL/cgal/main/copyright`; arXiv via the
export API.

**Blocked during this survey** (negative results worth keeping): `docs.mcneel.com` (403),
`doc.cffcs.de/CGAL` (network failure), `volumetric-nurbs.github.io` (404),
`github.com/Instant-Meshes/InstantMeshes` (404), `BRep_NurbsConvertor_approximation.hxx` (404).