# Sparse and compact geometry representations

Recorded 2026-10-07.

## Question

Where do NURBS-based and NURBS-adjacent representations win and lose at representing geometry
sparsely and editing it locally, and what is the 2023–2026 state of the art including
learned representations?

## Answer in one line

NURBS wins on *hand editability and exactness* and loses on *compactness and automatic
refinement*; the only verified technique that compresses dense geometry into a NURBS-like
editable form is the trivariate V-rep line, and the learned-NURBS line is about
**representation learning for B-rep tasks, not about editing**.

## The landscape

### Trivariate volumetric NURBS ("V-rep") — the strongest verified candidate

- **Elber, "A Review of a B-spline based Volumetric Representation: Design, Analysis and
  Fabrication of Porous and/or Heterogeneous Geometries", Computer-Aided Design 163:103587
  (2023-06-15), <https://doi.org/10.1016/j.cad.2023.103587>**. Single-author review, 44
  references. This is the closest thing to a review of "sparse editable geometry" and it
  should be read before designing anything in this space.
  - Core idea: extend the *boundary* representation to a **volumetric** representation — trimmed
    trivariate B-spline/NURBS functions, with B-rep primitives, constructors and Boolean
    operations all extended to trivariates.
  - Why this is the sparsity answer: a control net over a trimmed trivariate domain is small,
    hand-editable, and carries interior heterogeneous fields that a B-rep structurally cannot.
  - Target domain is design + additive manufacturing (porous, graded, anisotropic), not
    rendering and not arbitrary polygon input.
- Ancestors: Park, "Volumetric NURBS Representation of Multidimensional and Heterogeneous
  Objects", Korean J. Comput. Des. Eng. 10(5), two parts (2005) — OpenAlex W798358926
  (Concepts and Formation) and W776766392 (Modeling and Applications).
- Martin & Cohen, "Representation and Extraction of Volumetric Attributes Using Trivariate
  Splines: A Mathematical Framework", Solid Modeling 2001,
  <https://www2.cs.utah.edu/gdc/projects/trivar/>. The property that matters for sparsity is
  stated on that page: "The representation is independent of the granularity of the physical
  geometry, allowing for a decoupling of the resolution of the carried data from that of the
  volume."

### Compact parametric abstraction learned end-to-end

- **SweepNet**, "Unsupervised Learning Shape Abstraction via Neural Sweepers", ECCV 2024,
  <https://arxiv.org/abs/2407.06305>. Parameterizes a sweep surface with superellipse profiles
  and B-spline axis curves — the abstract states the representation needs "as few as 14 float
  numbers" and that this "facilitates intuitive and interactive editing while preserving shape
  details". A differentiable neural sweeper predicts it without supervision. This is a genuinely
  interesting point on the sparse/editable spectrum: extremely compact, but only valid for
  sweep-shaped geometry.
- **Flow-TAG**, <https://arxiv.org/abs/2609.30955> (2026-09-25). A generative flow model with a
  1D U-Net that maps a curve's geometry to the **optimal parametrization** for a cubic B-spline,
  reporting 55 % lower RMSE and 52 % lower Hausdorff distance than prior data-driven methods.
  Directly targets the parameterization problem that blocks naive NURBS fitting.

### Learning NURBS parameters directly

- **NeuroNURBS**, "NeuroNURBS: Learning Efficient Surface Representations for 3D Solids",
  <https://arxiv.org/abs/2411.10848>. Leiden University and BMW. Encodes a NURBS surface's
  control points, weights and two knot vectors directly into a transformer VAE, replacing the
  usual UV-grid sampling. Reported: 86.7 % less GPU memory for the surface autoencoder, 79.9 %
  less memory to store solids, 92.9 % faster surface→NURBS construction; plugged into BrepGen it
  moves FID 30.04 → 27.24 and resolves the "undulating surface" artifact of UV-grid fitting.
  Extraction uses OCCT's `BRepBuilderAPI_NurbsConvert()` via pythonOCC.
  - Read this correctly: it is a **representation-learning paper for B-rep tasks** (generation,
    segmentation). It is not a sparse-editing method, and it introduces a training dependency and
    a repair step that the paper does not quantify.
- **NURBS-Diff**, "A differentiable programming module for NURBS", CAD 146 (2022),
  <https://doi.org/10.1016/j.cad.2022.103199>. Gradient-based NURBS fitting to point clouds.
- Differentially-optimized multi-patch CAD: arXiv <https://arxiv.org/abs/2606.07198> (2026-06),
  an MLP deformation field acting on the control points of multi-patch NURBS surfaces, with
  differentiable volume/area/centroid integrals via Gauss-Legendre quadrature and analytical
  B-spline derivatives. Relevant to constrained optimization, not to sparse editing.

### Classical alternatives still worth knowing

- **Variational Shape Approximation (VSA)** — Cohen-Steiner, Alliez & Desbrun, TOG 23(3)
  (2004), <https://doi.org/10.1145/1015706.1015817>. Resamples a triangle mesh into a small set
  of locally-planar oriented planes. Local editing is the entire point (each plane is
  independent, no coupling), and the count is far below the face count. The trade-off is real:
  the output is piecewise flat and carries **no curvature continuity**. Extension to point-set
  surfaces: Skrodzki, Zimmermann & Polthier, CAGD (2020),
  <https://doi.org/10.1016/j.cagd.2020.101875>.
  - Correction: I initially attributed VSA to "Oger et al., Pacific Graphics 2009". That
    attribution is **wrong**; 2004 Cohen-Steiner et al. is the canonical paper.
- **PN (point-normal) surfaces** — Yang, "Point-normal subdivision curves and surfaces", CAGD
  104:102207 (2023-05-09), <https://doi.org/10.1016/j.cagd.2023.102207>.
- **ManifoldPlus** — Huang, Zhou & Guibas, <https://arxiv.org/abs/2005.11621> (2020-05-23).
  Converts a triangle soup to a watertight manifold, then optimises with a per-step convex
  Gauss-Seidel scheme. Relevant as a *repair* stage feeding a NURBS-ization pipeline; it is not
  itself a sparse parametric representation.
  - Correction: this is a **2020 arXiv preprint**, not SIGGRAPH Asia 2022.
  - No successor "ManifoldPlus+" was found.

## Cost model

Operationally, the four distinct cost regimes (see
[nurbs-izing-arbitrary-polygons.md](nurbs-izing-arbitrary-polygons.md) for the OCCT-verified
primitive list):

| Operation | Cost | Conditions |
| --- | --- | --- |
| Move a single control point | O(1) | tensor-product patch, `MovePoint` |
| Local knot insert / remove | O(p·q) changed poles, O(n) array bookkeeping | `InsertUKnot` / `RemoveUKnot` |
| Coarsening a hierarchy level | not O(1) | THB unrefinement is a local-projector algorithm, <https://doi.org/10.1016/j.cagd.2022.102157> |
| Degree elevation / multiplicity change | O(n) global | `IncreaseDegree`, `IncreaseUMultiplicity` |

The familiar big-O exponents for Boehm insertion (O(p²) curve, etc.) are **not verified from
any primary source** in this survey. See the negative results in
[hierarchical-spline-bases.md](hierarchical-spline-bases.md).

## Comparison table

| Representation | Sparsity | Local edit | Needs training | Hand-editable | Source |
| --- | --- | --- | --- | --- | --- |
| Trivariate V-rep (trimmed NURBS solid) | compact control net over trimmed volume | local CP move; propagates over one Bézier element | no | yes | 10.1016/j.cad.2023.103587 |
| Tensor-product NURBS patch | compact per patch | O(1) CP move; refinement cascades | no | yes | 10.1145/378456.378512 |
| THB-spline hierarchy | compact, multi-level | local refine **and** local coarsen | no | yes | 10.1016/j.cagd.2012.03.025 |
| SweepNet sweep surface | ~14 floats | direct parameter edit | yes (unsupervised) | yes, parametrically | arXiv 2407.06305 |
| Variational Shape Approximation | planes ≪ faces | independent planes | no | yes (planes only) | 10.1145/1015706.1015817 |
| NeuroNURBS latent | smaller than UV-grid | inherited from NURBS params | yes (VAE) | no | arXiv 2411.10848 |
| ManifoldPlus | dense output | n/a, not parametric | no | no | arXiv 2005.11621 |

## Reviews found

1. **Elber 2023, CAD 163:103587** — closest to an on-topic review. Abstract was not
   retrievable (ScienceDirect bot wall); scope inferred from the verified 44-item reference
   list. **Partially verified** — read the paper itself before relying on it.
2. Farshian et al., "Deep-Learning-Based 3-D Surface Reconstruction — A Survey", Proc. IEEE
   (2023), <https://doi.org/10.1109/jproc.2023.3321433>.
3. Huang et al., "Surface Reconstruction From Point Clouds: A Survey and a Benchmark", IEEE
   TPAMI (2024), <https://doi.org/10.1109/tpami.2024.3429209>.
4. Berger et al., CGF (2016), <https://doi.org/10.1111/cgf.12802>.

None of these is a review of *sparse editing* specifically.

## Negative results

These were carried as candidate citations in the original brief and **failed verification**.
Do not resurrect them without a primary record.

1. **"Volumetric NURBS Curve", ACM TOG 2022 — does not exist.** Disproven four independent ways
   (parent agent plus three separate search passes): arXiv full-text 0 hits; Crossref no hit;
   OpenAlex no hit; and a Crossref enumeration of every ACM TOG paper with "NURBS" in the title
   (1994–2026, 15 results) contains no such paper.
2. **"NURBS-Over" / Floener–Reutenauer–Ouaknine–Stamminger** — no record in OpenAlex, Crossref,
   or web search. Treat as fabricated.
3. **"PolyPiecewise Smooth Surfaces" / Floener–Yuksel–Schaefer poly-Bezier** — no record found.
4. **"Neural NURBS" (CVPR 2022), "EB-Spline" (ECCV 2024), "NeuNURBs", "NUCOMP" (CVPR 2022),
   "BRepNet"** — not located under those exact names. **Caveat:** absence under a title search
   is not absence of the work; the search tooling (title.search) is weak against these.
   NeuroNURBS *does* exist and was found only by keyword search — see above.
5. **ManifoldPlus successor** — none found.
6. **GPU evaluation of sparse parametric bases** — nothing found.
7. **Sparse voxel / octree / SDF state of the art for compact editable geometry** — not
   surveyed here; this is a genuine gap, not a negative finding.

## Sources

All fetched 2026-10-07. Crossref and OpenAlex were the reliable bibliographic surfaces;
Semantic Scholar was HTTP 429 throughout and DBLP was behind an Anubis bot wall.