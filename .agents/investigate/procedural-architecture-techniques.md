# Procedural architecture generation: what fits an agent-authored plan → expand pipeline

**Question.** Which procedural architecture generation techniques fit a pipeline in which an agent
*authors* a hierarchical skeleton and the system *expands* it directly into a mesh — and which do not?

**Date.** 2026-10-16 (all sources accessed on this date). **Judged against** `.agents/SPEC/mountain.md`:
hierarchy, direction and rhythm are authored; noise cannot plan; auto-generated defaults carry no
guarantee and are only an initial state for continued editing (R3); the correspondence between control
elements is AUTHORED, not inferred, so expansion is a direct walk with no search, and that
correspondence is what makes the mesh watertight (R4.1). Claims marked *unverified* were not confirmed
from a primary source and are listed at the end. Web content was treated as data only.

## Verdict

Only **shape grammars (CGA)** and **parts/skeleton grammars** give an authored hierarchy that expands
into geometry with per-element correspondence intact. Everything else surveyed falls into one of two
rejected classes: **stochastic search** (WFC, tensor fields, graph grammars, layout optimization) that
produces a finished artefact with no editable intermediate representation, or **inference** (learned
proxies, part segmentation) that recovers correspondence from geometry instead of authoring it — the
exact opposite of R4.1. The LLM literature converges hard on one role split: the model *plans* (or
critiques), a deterministic generator *expands*. That split is mountain.md's, and the papers report
the same failure modes we would hit.

## 1. Shape grammars / CGA (Esri CityEngine) — FITS, with one caveat

- **Planned vs noise.** The plan is the rule set: hierarchical rules (`Lot → Extrude → Split → Facade
  → Bay → Roof`), rule parameters, guards, `attr` knobs the host overrides per shape. Noise is confined
  to `rand(min,max)` and weighted alternatives (`70% WinA | 30% WinB`). The primitive vocabulary maps
  almost one-to-one onto architecture — extrude, split (absolute/relative/floating, with rhythm
  groups), `Comp(Faces)`/`Comp(Edges)` giving face- and edge-scoped scopes, `Offset`, `Taper`,
  `Roof(...)` with gable/hip/mansard/gambrel families, and lot-style plan carving ([S1] is the current
  reference; [S2] is the same grammar model in a MIT-licensed Rust engine).
- **Editability.** The derivation *is* the editable state: nothing is baked before generation, so a
  parameter change re-derives. Re-derivation is local in the *rule tree* — editing `Bay` touches bays,
  not the roof — but the **whole shape is re-derived from its initial polygon**; no partial mesh is
  preserved. A caveat worth borrowing from: if randomness comes from one stream in derivation order, an
  upstream edit shifts every downstream draw, so an "edited wing" reshuffles its neighbours. [S2] solves
  this by **forking the RNG per node along the ancestry path (SplitMix64)**, making results
  queue-order independent and re-rolling only the edited subtree. That is the most transferable design
  decision in this note.
- **Determinism / watertightness.** Deterministic given a seed: PRT takes a per-shape `seed` attribute
  ([S3], `shape_attributes = {'shapeName': 'myShape', 'seed': 555}`). *Unverified:* how PRT hashes seed +
  shape path, and whether a parameter change leaves untouched siblings bit-identical. CGA output is
  **not** watertight — it is a bag of polygon faces and mesh instances, watertight only if downstream
  code stitches it; [S2] explicitly builds oriented bounding boxes with "no mesh booleans". CGA
  therefore delivers watertightness only if *our* expander owns stitching, which is what R4.1 wants:
  two faces meet because they were built from the same authored rib.
- **Rust feasibility.** Cheap and small: an interpreter plus ~20 op functions, no geometry kernel.
  `symbios-shape` (MIT) proves a CGA engine in Rust is a few-thousand-line crate ([S2]); `csgrs` 0.20.1
  (MIT, 2026-09-01) supplies optional booleans ([S4]). CityEngine itself is **not** reusable — PRT is
  non-commercial-only ([S3]) — so copy the *language design*, not the runtime.
- **Fit with R4.1.** Strong, with one gap to design around: CGA correspondence is *implicit in the
  derivation tree* (a facade scope knows its parent face because it descends from it). That is
  "derived from structure", close to but not the same as "authored". To fully match R4.1, a skeleton
  node must be able to *name* its partner explicitly.

## 2. Floor-plan generation — DOES NOT FIT as a representation; usable only as an R3 default generator

- **WFC.** Planned: nothing. The plan is a tileset plus adjacency constraints; the output is a flat
  collapsed grid. Propagation is local to the collapsed cell's neighbourhood, and the algorithm does
  support constraining regions a human already placed ([S5] "WFC autocompleting a level started by a
  human") — the one hook an agent could use. Decisive objection: contradiction is possible and the
  algorithm then returns **nothing**; satisfiability is NP-hard, so completion is not guaranteed and a
  local edit can invalidate the whole solve ([S5]). Rust ports exist (`collapse`, `wfc`,
  `simple_wfc` 0.1.0-alpha.7, MIT OR Apache-2.0, ~1 100 lines, immature — [S6]).
- **Tensor fields.** Planned: the *rule* is authored — space subdivision, termination, merging and
  setback parameters are explicit and tunable, which fits the philosophy. But split boundaries inside a
  region are chosen from a sampled field: the region hierarchy is authored, the layout inside it is
  sampled. Good as an R3 initial-state generator, bad as the representation the agent edits.
- **Graph grammars.** Planned: a lot, in principle — non-terminals and rewrite rules are an editable
  hierarchy. Against them: rewriting is search (match a rule, test applicability), which R4.1 excludes,
  and the result is a graph, not a solid, so watertightness is unsolved. Rust: `petgraph` available,
  rewriting engine is yours to write.
- **"Make It Home"-style optimization.** Pure stochastic search over a scored candidate pool, with no
  authored structure at all. Rejected under R3: it produces a finished arrangement that carries no
  guarantee and no edit handle.

## 3. Typological / pattern-language / city–building grammars — FITS as a hierarchy vocabulary only

- Alexander's pattern language supplies the *levels* (region → settlement → street → building → bay →
  opening) and the idea that each pattern is applied to a context. Its weakness for us: the patterns
  form a **lattice with cross-references, not a tree** ([S7]; "A City Is Not a Tree" attacks exactly that
  decomposition habit). Lattices are why a pattern language is a good catalogue of intent and a bad
  representation.
- Parish & Müller's city modelling ([S8]) is the systematic version: a semantic hierarchy (zone → block
  → lot → building → opening) where each level is a rule over the level above. It is CGA across four
  scales, with the same editability and the same global-re-derivation caveat as family 1.
- Style-grammar work on historic/Gothic architecture repeats the pattern: hierarchical rules
  (nave → bay → bay-window → tracery) with parameters carrying proportions.
- **Take:** the *level vocabulary* and naming discipline — mass, storey, bay, opening, component — as
  the node kinds of our skeleton. Nothing else.

## 4. Structural and parts-based — FITS; the best fit for authored correspondence

- **Planned vs noise.** A truss is a near-perfect plan: span, bearing, slope, spacing and material
  determine chord and web members by rule. Kit/parts systems go further — the part vocabulary carries
  interfaces (studs, rebates, snap pins) and **a part names the interface it mates with**. That is
  authored correspondence in its purest form: joinery is data, not a search. Real write-ups show the
  pattern scaling (a parametric timber roof of ~49 800 slat elements generated from a grammar, [S9]).
- **Editability.** Parts are independently editable and independently re-expandable — the property we
  want most. Two failure modes: a parts soup is *not* watertight unless a join is enforced, and
  snapping systems commonly infer the mate from proximity rather than from an authored reference.
- **Rust feasibility.** Truss synthesis is closed-form arithmetic plus 2D member solids: trivial. If
  robust solids are needed, `manifold` is the library to reach for (family 6).
- **Fit with R4.1.** Highest of any family, provided every joint is an *explicit* reference (part A names
  part B's socket) and incomplete references are reported by `validate()` rather than repaired — R4.1
  verbatim.

## 5. LLM / agentic (2023–2026) — FITS as the planner; the literature rejects it as the expander

The consensus role split is unambiguous and matches mountain.md exactly.

- **LayoutGPT** ([S10]): the LLM is a *visual planner* emitting a layout in a style-sheet language; a
  downstream deterministic model does the generation. The LLM never touches geometry.
- **CityCraft** ([S11]): a diffusion model generates a 2D layout, an LLM makes land-use plans from it,
  then Blender plus an asset library do the geometry. It states why the LLM does not emit numbers:
  "direct numerical outputs from LLMs can sometimes result in physically implausible layouts, such as
  overlapping assets" — so the LLM emits **constraints** and a solver does the arrangement.
- **Holodeck 2.0** ([S12]): a VLM identifies and parses the objects, a 3D generative model makes each
  asset, and spatial constraints are applied iteratively. Assets, not rules.
- **BuildingBlock** (SIGGRAPH 2025, [S13]) is the closest published analogue of what we want: an LLM
  extends a layout into **rule-based hierarchical designs**, and PCG then builds them
  "local-customizable".
- **ProxyBuild** (2026-09, [S14]) is the most instructive negative result. Its abstract says it plainly:
  "procedural modeling can generate editable buildings with hierarchical structures, but rule authoring is
  laborious, and **even with the aid of large language models, it remains challenging to effectively solve
  procedural rules under geometric constraints**." Its fix is a *learned* face-edge bigraph encoder that
  **infers** the semantic role of faces and edges of a semantic-free shell, then anchors proxies under
  hard constraints — an inferred correspondence, the thing R4.1 rules out — and it still reports
  over-smoothing, component collisions and structural corruption. Our authored route should fix those
  classes by construction instead.
- **Rust feasibility.** Not a crate concern: the model is separate; it emits Rust construction code
  against our API (R5) or edits the skeleton.
- **Take:** LLM = plan + critique + parameter choice; deterministic expander = geometry; a
  solver/validator is where a hallucinated reference surfaces as a diagnostic.

## 6. CSG / solid modeling vs direct mesh — direct mesh primary, CSG as an opt-in consolidation

- `manifold` ([S15]) is the reference to judge against: its stated primary goal is "**guaranteed
  manifold output without caveats or edge cases**", it has a robust mesh Boolean ("first of its kind"),
  and — decisive for us — it "has **IDs that make it easy to keep track of materials and what surfaces
  belong to what input objects or faces**". A boolean therefore does *not* destroy traceability provided
  each authored element enters as a tagged input. Rust: `manifold-rs` 0.7.0 (MIT OR Apache-2.0,
  updated 2026-02-24, a wrapper around the C++ kernel) and the native-port track `manifold-rust`.
- `csgrs` 0.20.1 (MIT, BSP-tree booleans) is lighter ([S4]); BSP CSG is exactly the kind of kernel whose
  robustness on degenerate building geometry is unproven, so it is the wrong default if watertightness
  is a contract.
- **Direct mesh wins on provenance:** emitting faces straight from the skeleton preserves exact per-face
  attribution, with no boolean in the path. CSG wins on guaranteed solidity and on clean openings (a
  window is a difference).
- **Shape of the answer:** direct mesh as the primary path; `manifold` only to consolidate parts that
  must behave as one solid, carrying the per-input ID channel so the trace to the authored element
  survives.

## Comparison table

Legend: **P** planned/authored · **n** noise/sampled · **L** re-expands locally · **G** globally ·
**D** deterministic · **W** watertight by construction.

| Family | (a) planned vs noise | (b) editability | (c) determinism / watertightness | (d) Rust feasibility | (e) fit with authored correspondence (R4.1) |
|---|---|---|---|---|---|
| 1. CGA / shape grammar | P: rules, params, guards, attrs · n: `rand()`, weighted alternatives | L in the rule tree, G for the shape (re-derived from the initial polygon) | D given a per-shape seed; **not** watertight (face bag, no booleans) | Rewrite is small; MIT reference implementation exists; Esri runtime non-commercial | **High.** Correspondence is structural (parent→child scopes); needs an explicit partner reference to fully match R4.1 |
| 2a. WFC | P: tileset + adjacency only · n: collapse order, sampling | L for propagation, but a contradiction kills the whole solve | D with a seed; **may return nothing** (NP-hard) | `wfc`, `collapse`, `simple_wfc` (MIT OR Apache-2.0, immature) | **Low.** No editable hierarchy; fitting cells is constraint satisfaction |
| 2b. Tensor fields | P: subdivision/termination/merge/setback rules · n: field-driven interior layout | L per region | D given a seed; watertightness not addressed | `ndarray` + custom rules; easy rewrite | **Medium, as R3 defaults only.** Rules authored; layout inside a region sampled |
| 2c. Graph grammars | P: non-terminals + rewrite rules · n: rule selection | L per rewrite | D; solidity unresolved | `petgraph` available; rewriting engine is a rewrite | **Medium.** Editable hierarchy, but applicability is a search — excluded by R4.1 |
| 2d. Layout optimization | P: nothing · n: everything (scored search) | n/a | D per seed; no edit handle | trivial, wrong tool | **None.** Rejected under R3 |
| 3. Pattern language / city grammar | P: level vocabulary, proportions · n: which pattern applies where | L per level | D; solidity not addressed | concepts only; no code to port | **Medium.** Right vocabulary (mass→floor→bay→component), wrong structure — a lattice, not a tree |
| 4. Structural / parts / kit | P: spans, slopes, spacing, interfaces · n: almost none | **L**, per part | D; watertight only if the joint is enforced | closed-form member solids; trivial | **Highest.** A part names the interface it mates with — authored correspondence, exactly |
| 5. LLM / agentic | P: the LLM's job (layout, rules, style, constraints) · n: the generative model that makes assets | L once rules exist | D on the generator side; LLM output is unverifiable, often implausible | N/A (separate concern); LLM writes Rust against our API | **High as planner, low as expander.** Consensus split = plan/critique vs deterministic expansion |
| 6. CSG vs direct mesh | CSG: P = authored solids, correspondence inferred through the boolean · Direct mesh: P = skeleton, one face per authored element | CSG G; direct mesh L | CSG: `manifold` guarantees manifold output; direct mesh: watertight only if the expander stitches it | `manifold-rs` 0.7.0 (MIT OR Apache-2.0); `csgrs` 0.20.1 (MIT) | **Direct mesh.** `manifold` retains per-input IDs, so CSG can preserve trace if IDs are used |

## Recommendation

Build on three complementary families, in this order.

**1. Parts / skeleton grammar with authored joints — the backbone.** The only family that matches
R4.1 without qualification, and the only one where watertightness and editability fall out of the same
mechanism. Borrow: the *interface vocabulary* (a part declares named sockets and says which socket of
which part it mates with); the *level vocabulary* from family 3 to fix node kinds (mass → storey → bay →
opening → component); and family 1's *immediate mode* — every part is an ordinary editable node,
re-expandable alone. Require that an unfinished joint is reported by `validate()` and never repaired by
the expander.

**2. CGA as the expansion vocabulary.** The only mature published architecture of "authored hierarchy →
expanded geometry", with an op set that maps onto building almost one-to-one. Borrow the vocabulary and
the *derivation tree as the editable state*, not the Esri runtime (non-commercial). Two decisions to
copy deliberately: **fork the RNG per node along the ancestry path** so an edit re-rolls only its own
subtree, and **express randomness as an explicit rule alternative** (`70% WinA | 30% WinB`) rather than
parameter jitter, so a derivable default is visibly a default (R3) and a deliberate choice is visibly
authored.

**3. Direct-mesh expansion, with `manifold` as an opt-in consolidation.** Every face is emitted by the
expander from the skeleton, so "every face traces back to the skeleton" holds by construction with no
boolean in the critical path. When parts must behave as one watertight solid (a shell with openings),
run `manifold`, passing each authored element as a tagged input so the per-face ID channel survives the
Boolean. The guarantee is bought without paying provenance.

**Explicitly not used as the representation:** WFC, layout optimization, learned proxy prediction. WFC
can return nothing; optimization produces an artefact with no edit handle (R3 forbids relying on it);
learned proxies *infer* the correspondence R4.1 requires to be authored — ProxyBuild's own results
quantify the cost. If a stochastic method ever appears, it may only produce **default skeleton nodes**,
guarantee-free and immediately editable, never the mesh.

**Agent loop.** The literature converges, so we should too: the LLM authors and edits skeleton nodes
and rule parameters and critiques renders; it never emits geometry, and never emits numbers where a
reference is expected. Dangling references surface as `validate()` diagnostics — which is also the
cheapest way to stop an LLM corrupting a good state.

## Sources

All accessed **2026-10-16**.

- **[S1]** Esri, *CityEngine Help* (CGA reference: rules, split/extrude/comp/roof, `rand`, shape attributes). https://downloads.esri.com/resources/cityengine/2024.1/help/cityengine-help-2024.1.pdf
- **[S2]** TheJanusStream, `symbios-shape` — pure-Rust CGA derivation engine, MIT: op set, 15 roof types, per-shape forked RNG ("editing the rules of one wing re-rolls that wing alone"), "OBB scopes … no mesh booleans". https://github.com/TheJanusStream/symbios-shape · https://raw.githubusercontent.com/TheJanusStream/symbios-shape/main/README.md
- **[S3]** Esri, `pyprt` — Python bindings to the CityEngine Procedural Runtime; per-shape `seed` attribute; licensing (non-commercial without a CityEngine licence; example code Apache-2.0). https://github.com/Esri/pyprt · https://raw.githubusercontent.com/Esri/pyprt/master/README.md
- **[S4]** `csgrs` 0.20.1 — BSP-tree CSG in Rust, MIT, docs.rs, 2026-09-01. https://docs.rs/csgrs/latest/csgrs/
- **[S5]** mxgmn, *WaveFunctionCollapse* README: observation/propagation cycle, NP-hard satisfiability, "finishes without returning anything" on contradiction, human-seeded constraints, Rust ports, "modifying in blocks" for local edits. https://raw.githubusercontent.com/mxgmn/WaveFunctionCollapse/master/README.md
- **[S6]** crates.io API, `simple_wfc` 0.1.0-alpha.7 (MIT OR Apache-2.0, ~1 100 lines, 2025-11-12). https://crates.io/api/v1/crates/simple_wfc
- **[S7]** C. Alexander, *A City Is Not a Tree* (1965), discussed in "The Tree in the System". https://ced.berkeley.edu/wp-content/uploads/2022/11/The-Tree-in-the-System.pdf · abstract: https://www.acsa-arch.org/wp-content/uploads/2025/08/2025-Summer-Conference-Abstract-Book-Virtual-25Aug2025.pdf
- **[S8]** Parish & Müller, *Procedural Modeling of Cities*, SIGGRAPH 2001 — semantic hierarchy zone → block → lot → building. https://dl.acm.org/doi/abs/10.1145/383259.383292 · mirror: https://github.com/Jaycbee/procedural-cities/blob/master/paper.pdf
- **[S9]** Grammaziokohler, parametric timber roof (~49 858 slat elements generated by grammar). https://www.gramaziokohler.com/data/publikationen/1204.pdf
- **[S10]** Feng et al., *LayoutGPT*, NeurIPS 2023. https://arxiv.org/abs/2305.15393
- **[S11]** Deng et al., *CityCraft* (arXiv 2406.04983) — DiT layout → LLM land-use planner → asset library + Blender. https://ar5iv.labs.arxiv.org/html/2406.04983
- **[S12]** Bian et al., *HOLODECK 2.0* (v3, 2026-07-28). https://arxiv.org/abs/2508.05899
- **[S13]** Huang et al., *BuildingBlock*, SIGGRAPH 2025 — LLM extends layouts into rule-based hierarchical designs; PCG builds them. https://arxiv.org/abs/2505.04051
- **[S14]** Tang et al., *ProxyBuild* (2026-09-20) — the "LLMs cannot solve procedural rules under geometric constraints" result; inferred face/edge semantics; collision and corruption failures. https://arxiv.org/abs/2609.23386
- **[S15]** elalish, *Manifold* — guaranteed manifold output, robust Boolean, per-input surface IDs. https://github.com/elalish/manifold · https://raw.githubusercontent.com/elalish/manifold/master/README.md · Rust wrapper: https://crates.io/api/v1/crates/manifold-rs (`manifold-rs` 0.7.0, MIT OR Apache-2.0, 2026-02-24) · native port: https://github.com/larsbrubaker/manifold-rust
- **[S16]** Bahmani et al., *CC3D* — recorded only to note that arXiv 2303.12074 is CC3D, **not** LayoutGPT; a mistaken identifier corrected during this survey. https://arxiv.org/abs/2303.12074

### Not established this session (do not treat as fact)

- The exact PRT seeding/re-derivation semantics beyond "a per-shape `seed` attribute exists".
- Primary sources for tensor-field and graph-grammar building layouts, and for style-grammar work on Gothic/historic architecture.
- That a CGA parameter edit leaves sibling subtrees bit-identical — true for [S2]'s forked-RNG design; **not** verified for Esri PRT.
