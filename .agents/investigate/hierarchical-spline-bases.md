# Hierarchical and adaptive spline bases

Recorded 2026-10-07.

## Question

Which spline representations support genuinely local (single-knot-span) refinement and
coarsening, what does the literature actually verify about them, and what has reached
production?

## Answer in one line

There is a well-established academic line (HB → THB → T-splines → PHT/LR) that supports
local refinement and coarsening, but **none of it has become a native production CAD
kernel feature**; ordinary tensor-product NURBS remains the only representation every
commercial kernel implements natively.

## Verified families

### Hierarchical B-splines (HB) — Forsey & Bartels 1988

- SIGGRAPH 1988, <https://doi.org/10.1145/378456.378512>; extended in "Surface fitting with
  hierarchical splines", TOG 1995, <https://doi.org/10.1145/221659.221665>.
- Mechanism: nested levels with locally controlled subdivisions ("overlays").
- The 1988 abstract itself states the limitation that drives the whole field: refinement is
  local for curves but **not** for tensor-product surfaces, because control vertices far from
  the region of interest change.
- Known defect: the hierarchical basis **lacks partition of unity**. The repair (OUS /
  optimal hierarchical basis) is standard folklore but **I could not locate a primary source**;
  do not cite it without checking.

### THB-splines — Giannelli, Jüttler & Speleers 2012

- "THB-splines: The truncated basis for hierarchical splines", CAGD 29(7):485–498,
  <https://doi.org/10.1016/j.cagd.2012.03.025>. Full review: CMAME 299:337–365 (2016),
  <https://doi.org/10.1016/j.cma.2015.11.002>.
- Truncates the hierarchical tensor-product basis at one level per element, then renormalises,
  which restores partition of unity and reduces overlap between levels.
- The coarse level is ordinary NURBS, so THB is a NURBS superset — this matters for delivery.
- **Open in my evidence**: the cost of building the coarse-level element matrices without a
  global pass. Do not assume it is O(1).
- Industrial precedent exists but as a paper, not a product: Kiss et al. 2014,
  "Adaptive CAD model (re-)construction with THB-splines" (MTU Aero Engines),
  <https://doi.org/10.1016/j.gmod.2014.03.017>.

### Coarsening / unrefinement — the half of "sparse editing" that is easy to forget

- Heydarov, Buffa & Jüttler, "An unrefinement algorithm for planar THB-spline
  parameterizations", CAGD 99:102157 (2022-10-06),
  <https://doi.org/10.1016/j.cagd.2022.102157>.
- Produces coarser geometry from a THB representation using local linear projectors over a
  cell-based hierarchy. **A hierarchy you cannot collapse is not sparse — it only grows.**
  Any design that adds levels must plan the removal path.
- Related: classical knot removal is the exact algebraic inverse of knot insertion, but is
  numerically fragile in floating point. I did **not** find a primary source quantifying the
  tolerance, so treat the stability caveat as unverified.

### T-splines / T-NURCCs — Sederberg, Zheng, Bakenov & Nasri 2003

- "T-splines and T-NURCCs", TOG 22(3):477–484, <https://doi.org/10.1145/882262.882295>.
- The verified abstract confirms: T-junctions let control points avoid traversing the entire
  grid, and **insertions do not alter control points away from the location**. This is the
  genuine local-refinement property classical NURBS lacks.
- Naming correction: the paper is about **T-NURCC** (Non-Uniform Rational Catmull-Clark
  Surfaces with T-junctions), a superset of *both* T-splines and NURBS — not "T-NURBS".
- Li, Zheng, Sederberg, Hughes & Scott 2012, CAGD 29(1):63–76,
  <https://doi.org/10.1016/j.cagd.2011.08.005> is a **linear-independence** result. The
  common "minimal support" framing is a reading of downstream literature, not the paper title.
- Simplification: Sederberg et al. 2004, <https://doi.org/10.1145/1015706.1015715>.

### PHT-splines — Deng, Chen, Li, Hu, Tong, Yang & Feng 2008

- "Polynomial splines over hierarchical T-meshes", Graphical Models 70(4):76–86,
  <https://doi.org/10.1016/j.gmod.2008.03.001>; earlier Visual Computer 2007,
  <https://doi.org/10.1007/s00371-007-0170-3>.
- Handles arbitrary topology, which no other entry in this table does.
- Generally **not representable as standard NURBS**.

### LR B*-splines — Dokken, Lyche & Pettersen 2013

- "Polynomial splines over locally refined box-partitions", CAGD 30(3):331–356,
  <https://doi.org/10.1016/j.cagd.2012.12.005>. Terrain application: Skytt et al. 2015,
  <https://doi.org/10.1016/j.cag.2015.03.006>, <https://arxiv.org/abs/2010.01980>.
- Knot-line / box-partition based, structurally dual to T-splines (the "dual" framing is
  standard but **unverified** from a primary abstract).

### 2023–2026 frontier

Almost entirely **isogeometric-analysis** driven, not CAD-delivery driven:

- Hong & Li, "Greedy Local Refinement for Analysis-Suitable T-splines with Linear Complexity",
  J. Comput. Math. 44(2), **published online 2026-04-15**,
  <https://doi.org/10.4208/jcm.2502-m2024-0179>.
- Goates, Shepherd & Thomas, "Combinatorial maps for hierarchical splines",
  <https://arxiv.org/abs/2608.29545> (2026-08-30). Relevant because typical hierarchical
  formulations store only a list of elements and do not maintain adjacency; adding a
  combinatorial map gave up to an order-of-magnitude reduction in downstream computation.
  This is the practical engineering state of the art for hierarchy bookkeeping.
- Maier, Morgenstern & Takacs, CAGD 96:102117 (2022), <https://doi.org/10.1016/j.cagd.2022.102117>.
- Bracco et al., "Adaptive fitting with THB-splines: error analysis and industrial
  applications", CAGD 62:239 (2018), <https://doi.org/10.1016/j.cagd.2018.03.026>.
- Mokriš & Jüttler 2014 truncated-decoupled bases, <https://doi.org/10.1016/j.cagd.2014.05.004>.

## Comparison table

| Representation | Local refine | Local coarsen | Partition of unity | Arbitrary topology | Standard-NURBS-compatible | Source |
| --- | --- | --- | --- | --- | --- | --- |
| Classical NURBS/B-spline | no (control-point cascade on surfaces) | yes but numerically fragile | yes | no (tensor product) | is NURBS | 10.1145/378456.378512 |
| HB-splines | yes (overlays) | not verified | **no** | no | coarse level yes | 10.1145/54852.378512 |
| THB-splines | yes | yes (2022 algorithm) | yes (renormalised) | no | coarse level is NURBS | 10.1016/j.cagd.2012.03.025 |
| T-splines / T-NURCCs | yes (no CP cascade) | yes (2004 simplification) | yes | T-junctions only | superset of NURBS | 10.1145/882262.882295 |
| PHT-splines | yes | not verified | yes | **yes** | generally no | 10.1016/j.gmod.2008.03.001 |
| LR B*-splines | yes | not verified | yes | box-partition based | knot-line based | 10.1016/j.cagd.2012.12.005 |

## What reaches production

Judgement, separated from the evidence above:

- **Ordinary tensor-product NURBS** is the only representation implemented natively by every
  commercial kernel.
- **T-splines** are plausibly production, but via a commercial plugin (T-splines, Inc.), not as
  a native kernel feature. Sederberg co-authored a production hull-design framing at SNAME
  2010, <https://doi.org/10.5957/cpbs-2010-014>. Current vendor availability **not confirmed** —
  vendor pages were unreachable during this survey.
- **THB-splines** are research plus one industrial paper (Kiss et al. 2014). No shipped product.
- **PHT / LR** are analysis-side; no CAD delivery path.

## Negative results

Searched and found nothing; do not repeat these searches expecting a different answer.

1. **"Volumetric NURBS Curve", ACM TOG 2022 — does not exist.** Disproved three independent
   ways on 2026-10-07: arXiv API `all:"volumetric NURBS"` → `totalResults = 0`; Crossref
   `query.bibliographic` → no such paper; OpenAlex full-text search → no such paper; and a
   Crossref enumeration of *all* ACM TOG papers with "NURBS" in the title (15 results,
   1994–2026) contains no such paper. Any citation of it is a fabrication.
2. **Boehm 1992, "Local knots, local control points" (CAD vol. 24)** is absent from Crossref's
   indexed record for that journal/period and from OpenAlex. The title is very likely correct
   but is **unverified** here.
3. **MRAF (Sederberg & Zheng)** — could not verify what the acronym expands to, nor its cost.
4. **All the familiar big-O exponents** (O(p²) for a curve, O(p_u·p_v·(p_u+p_v)) for a surface)
   are **not verified from any primary source** in this survey. Piegl & Tiller's appendix
   numbers are not indexed by Crossref/OpenAlex. Do not present them as confirmed.
5. **"NURBS-O"** as a published family — not found.
6. **PHT-to-NURBS conversion** — not found (though see the note below; work does exist under
   a different framing).
7. **GPU evaluation of hierarchical bases** — nothing found.
8. **"PolyPiecewise Smooth Surfaces" / Floener–Yuksel–Schaefer poly-Bezier** — nothing found.
   I had carried "NURBS-Over" as a citation; it is unverifiable and should be treated as
   fabricated until a primary record is produced.

## A correction that slipped past

Later survey work *did* find PHT-to-NURBS conversion under a different title: "Optimized
PHT-to-NURBS conversion: A weighted rectangle partition approach", Wu, Deng & Yang, CAD 189
(2025-12), <https://doi.org/10.1016/j.cad.2025.103915>, reporting up to 78.35 % control-point
reduction on 3D subjects. So the conversion is real; my earlier framing was wrong. See
[nurbs-izing-arbitrary-polygons.md](nurbs-izing-arbitrary-polygons.md).

## Sources

All fetched 2026-10-07. See the DOI links inline above. Tooling note: Semantic Scholar was
HTTP 429 throughout, DBLP was behind an Anubis bot wall, and `opencascade.org` / `libfive.com`
timed out. Crossref and OpenAlex were the only reliable bibliographic surfaces, and Crossref's
fuzzy `query.bibliographic` repeatedly returned plausible-but-irrelevant work — those results
were discarded rather than trusted.