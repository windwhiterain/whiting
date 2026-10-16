# What contracts the workspace places on a new agent-authored `building` crate

**The question.** What contracts does the whiting workspace impose on a new agent-authored
`building` crate, and what existing machinery can such a crate reuse?

**Measured 2026-10-16** against `C:\resource\whiting` at its checked-out state. Read-only
reconnaissance; no build, no git, no edits. Every claim below names a file and a line.

---

## 1. Corrections to the brief (these came first, and they matter)

1. **The render harness is not where either place says.**
   `crates/device/examples/render.rs` **does not exist** — that directory holds only
   `bytes_format_probe.rs` and `facet_value_probe.rs`. `crates/device/src/render.rs` also is not
   the harness: it is the library `Renderer` trait module (`Renderer` at :129, `RenderedImage` :19,
   `Frame` :74, `RenderedFrame` :102, `NoopRenderer` :265). **The harness is
   `crates/device-rasterize/examples/render.rs`** (92 KB). The proposal repeats the wrong path at
   `proposals/agent-scene-iteration-loop.md:6`, so the error is in the repo, not only in the brief.

2. **Scale notes are gone.** `scale_notes` matches nothing in any `.rs` in the workspace.
   `handoffs/physical-light-units.md:97` records `crates/scene/tests/scale_notes.rs` as deleted and
   `proposals/physical-light-units.md:50-54` removes the function with its constants. Only
   `camera_notes` survives (`scene/src/standard/emit.rs:4868`, tests at `scene/tests/camera_notes.rs`).
   The harness must be described as *manifest + camera notes*, not *manifest + camera + scale*.

3. **The foldyard investigate note is not demand-side evidence.** The brief says an investigate
   note "says foldyard composes poorly". It does — but *composes* there means **agrees between the
   raster and Cycles backends**, not "its architecture is badly assembled".
   `investigate/foldyard-composes-poorly-and-these-are-its-reasons.md` (measured 2026-10-14) gives
   two causes, both post/lighting, neither geometric: atmosphere painted in the post stage and so
   invisible to a document (:57-83), and three ridge layers painted into pixels geometry never
   reached (:85-90). It cannot be cited as evidence that hand-built architecture needs a crate.
   The real duplication evidence is `gallery/src/foldyard.rs:822`.

4. **`SPEC.md` covers five crates and not the workspace.** It has no `patch`, no `terrain`, no
   `gallery` — a grep for those words in `SPEC.md` returns exactly one hit, the word "building"
   inside "building undo-redo system" at :32. So there is no template for an infrastructure crate,
   and a new authoring crate's SPEC entry is a human decision with no precedent.

5. **`Topology` has no `Quads`.** `SPEC.md:104-106` and `scene/src/standard/scene_schema.rs:1041-1051`
   give exactly `Triangles` and `Segments`; `emit.rs:721-723` reads only those two. A building crate
   whose output is quad faces cannot reach a frame until this changes.

6. **`mountain` has no dependencies at all** — `crates/mountain/Cargo.toml:11` is a bare
   `[dependencies]`, commented "Nothing." So the delta/History machinery is **not** a pattern
   mountain establishes; the question in the brief is genuinely open.

---

## 2. Sources actually read

| Path | What was taken from it |
| --- | --- |
| `.agents/SPEC.md` | Definition/Delta/History (:9-32), device (:60-71), mountain (:73-75), StandardScene shape (:77-128), `Topology` (:104-106) |
| `.agents/SPEC/mountain.md` | R1..R6 (:15-62), out-of-scope list (:64-68) |
| `crates/mountain/src/lib.rs` | crate shape, the one-idea doc (:13-25), `Massif` (:76-106), `footprint_area` (:117-129) |
| `crates/mountain/src/skeleton.rs` | `End` (:74-79), `Station` correspondence (:153-190), footprint stored open (:262-268), `locate` binary search (:611-633) |
| `crates/mountain/src/ops.rs` | the edit vocabulary; **no op refuses an invalid edit** (:10-16); `attach_ridge` splits the parent first (:48-52); `bind` (:389-403) |
| `crates/mountain/src/expand.rs` | belt enumeration order (:236-253), deterministic build (:96-108), winding by plan area (:407-460) |
| `crates/mountain/src/facet.rs` | planarity/watertightness invariants (:3-16), `BlockFace` (:46-94), `CornerNormals::of_grouped` (:505-533), self-checks (:232-358) |
| `crates/mountain/src/validate.rs` | `Violation` enum (:37-156), staged checking (:160-202), coverage as the check that matters (:10-16) |
| `crates/mountain/src/derive.rs` | `Starter` (R3 defaults, :35-101), fan-only derivation and why spur ridges are not generated (:19-23) |
| `crates/mountain/examples/census.rs` | the probe-as-report pattern (:16-79) |
| `crates/patch/src/lib.rs` | two models side by side (:1-15), exported surface (:32-44) |
| `crates/patch/src/tessellate.rs` | shared edges sampled once (:1-24) |
| `crates/patch/Cargo.toml` | patch owns no mesh type; the conversion lives above it (:13-24) |
| `crates/terrain/src/lib.rs` | **there is no geometry output** (:26-30); rasterize + erode only |
| `crates/scene/src/standard/scene_schema.rs` | `mesh_schema` :1036, `instance_schema` :1136, `transform_schema` :1186, `default_point_schema` :1358 |
| `crates/scene/src/standard/compiler.rs` | `StandardScene` and `register_standard_scene_schema` :118-131 |
| `crates/scene/src/standard/emit.rs` | `camera_notes` :4868-4909, `OWNED_FRAME` :4840, packed point path :2221-2300 |
| `crates/scene/src/diff.rs`, `crates/scene/src/lib.rs:300-369` | `document_delta`; `CompileOutput.graphics_delta` / `instance_delta` |
| `crates/device/src/render.rs` | the `Renderer` abstraction (see correction 1) |
| `crates/device-rasterize/examples/render.rs` | the harness: usage/flags :13-23, `RenderRecord` :1137, `RenderManifest` :1156, `report_against` :1269 and its four-row table :1253-1259 |
| `crates/gallery/src/built.rs`, `geometry.rs`, `kit.rs`, `foldyard.rs`, `terraces.rs`, `identity.rs`, `patchmesh.rs`, `lib.rs` | see §4 |
| `crates/definition/src/{definition,delta,history}.rs` | `Definition` :19, `PrimitiveDelta` :28, `Delta` :92, `History` :28 |
| `.agents/proposals/agent-scene-iteration-loop.md` | items landed/rejected :362-421; manifest contract :221-233 |
| `.agents/proposals/{hierarchical-nurbs-multipatch,arbitrary-topology-smoothness,two-soft-edges-one-face,block-face-front-end}.md`, `.agents/SPEC.align/hierarchical-nurbs-multipatch.md`, `.agents/handoffs/hierarchical-nurbs-multipatch.md` | see §6 |
| `.agents/investigate/foldyard-composes-poorly-and-these-are-its-reasons.md` | see correction 3 |

---

## 3. Per-layer contract table

| Layer | Requires of a new authoring crate | Offers it in return |
| --- | --- | --- |
| **definition** (`Definition`/`Delta`/`History`) | Nothing today. A geometry authoring crate need not touch it: `mountain` has zero deps. If building were a *document* it would need a registered `Schema` (`registry.rs`), a `Definition` per instance, and `Delta`s whose ops are `Replace`/`Append`/`Remove` only — `Append`/`Remove` are **contiguous** by construction (`delta.rs:22-25`, :47-55), so a non-contiguous edit is inexpressible. | `Registry::apply` returns the inverse and records it (`registry.rs:106-128`); `History` replays it (`history.rs:40-76`). `scene::diff::document_delta` (`diff.rs:25`) already turns one scene `Value` into the delta producing the next, and treats a packed mesh as one `memcmp` or one whole replace (`diff.rs:59-67`). |
| **mountain / patch / terrain** | `mountain`: publish name `whiting-mountain`, in-workspace `mountain` (`SPEC.md:3`, workspace `Cargo.toml:24-33`); R5 "pure rust crate, no serialization, no frontend" (SPEC/mountain.md:53-57); R6 "no simulation or beautification" (:59-62); output is a triangle mesh that reaches the scene only through a gallery adapter. `patch`: consumes **whatever mesh the caller has** — owns none (`patch/Cargo.toml:9-11`). `terrain`: produces numbers, never geometry (`terrain/src/lib.rs:26-30`). | `Massif` as the authoring-shape template: `Skeleton` is the document, `Tessellation` is not part of it (`lib.rs:68-74`); `End::On(Station)` makes a junction a **shared vertex**, never coincident coordinates (`skeleton.rs:56-72`); derived positions are never stored. `FacetMesh` invariants: every face exactly planar, watertight, coverage checked by signed `plan_area` (`facet.rs:3-16`, :232). `CornerNormals::of_grouped` gives hard/soft creases by grouping (`facet.rs:505`). `patch::subdivide` (Doo–Sabin) + `gallery::patchmesh::from_facet` already convert a `FacetMesh` to a `PatchMesh` (`patchmesh.rs:68-83`), refusing non-manifold meshes by name. |
| **scene** (`StandardScene`) | A scene document must carry `name` (`compiler.rs:83`). Meshes: `label`, `topology` ∈ {`Triangles`,`Segments`}, `point_schema`, `points` (`scene_schema.rs:1036-1065`). Instances: `label`, `mesh` (by label), `transform` = translation/rotation(x,y,z radians)/scale, `material` (`scene_schema.rs:1136-1199`), and **every instance must be listed under each pass bucket that draws it** (`kit.rs:479`). Points are verified against the mesh's own `point_schema`; the minimum is `position` alone (`scene_schema.rs:1358`). A shader is validated against pass+instance+point schemas, so a custom point field needs a matching shader attribute list (`gallery/attributes.rs:41`). | `Topology` triangle-only is the hard gate. `transform_schema` gives non-uniform scale per axis for free — `terraces.rs:563-578` places a whole roof by scale/translate/rotate. `default_point_schema` is the floor; `gallery::kit::shaded_point_schema` (`kit.rs:597`) is the working one. `CompileOutput` already emits an incremental delta per document (`scene/src/lib.rs:300-326`). |
| **device + gallery** | The harness takes a **scene name from `Gallery::build`**, not a mesh (`render.rs:93`, `gallery/lib.rs:113-120`) — a building crate's output must reach a scene to be seen. Compile notes are the only channel a compile has for "I could not do this exactly" (`scene/src/lib.rs:317-325`). Camera notes fire on eye-inside-box, box-filling-frame, target nearer than near plane (`emit.rs:4842-4909`); a building that encloses the camera produces the first. | The harness's flags: `<scene> <out> [times] [mode]`, `--sheet`, `--sheet-full`, `--video`, `--debug <channel>` (`render.rs:13-23`). The manifest hashes build/documents/pixels and prints one of four verdicts, including "the edit did not reach the documents this camera reads" (`render.rs:1245-1269`). `gallery::built`, `gallery::geometry`, `gallery::kit` are a ready-made generator layer returning `definition::PackedList` (`built.rs:11-16`, `kit.rs:268-291`). |

---

## 4. Reusable machinery inventory

- **Mesh / point types.** `mountain::FacetMesh {points, faces, blocks}` with `plan_area`, `bounds`,
  `shortest_edge`, `boundary_edges`, `boundary_loops`, `euler_characteristic`, `is_manifold`,
  `locate(face, uv) -> BeltPoint` (`facet.rs:179-407`). `mountain::CornerNormals::{of, of_grouped, at}`.
  `patch::{PatchMesh, HermiteSurface, Point3, RectNet, QuadMesh, EdgeKey, FaceId, VertexId,
  Continuity, Tessellation, FaceGrid}` (`patch/src/lib.rs:32-44`). `terrain::HeightField` (numbers only).
- **Transform types.** `scene::standard::transform_schema()` (`scene_schema.rs:1186`); `kit::instance`
  (`kit.rs:415`); `kit::instances(buckets)` (`kit.rs:479`). No transform type exists in `mountain` or `patch`.
- **Point-buffer writing.** `definition::{PackedBuilder, PackedList, PackedLayout, FieldMap}`;
  `graphics::layout::packed_layout(&schema)` — the layout *is* the schema's field order
  (`identity.rs:391-392`); `gallery::geometry::Shaded` for append-style composition (`geometry.rs:50-84`).
- **Validate patterns.** `Massif::validate() -> Vec<Violation>` returning *all* of them, ordered so
  two runs diff (`validate.rs:160-202`); checks staged so downstream checks are skipped when the
  document cannot be sampled (:182-184); ops never refuse an invalid edit (`ops.rs:10-16`); build
  failure is a *list* (`expand.rs:36-92`), not a first error. Geometry self-checks live on the mesh
  (`facet.rs:232-358`). The reporting vehicle is an `examples/` probe, not a test file
  (`mountain/examples/census.rs`, `patch/examples/*_probe.rs`).
- **Render-harness flags.** `--sheet`, `--sheet-full`, `--video`, `--debug <channel>`; times are
  positional and a range is rendered by patching `time` (`render.rs:7-23`). Manifest verdict table at
  `render.rs:1253-1259`.
- **Delta machinery.** `definition::{Delta, PrimitiveDelta, History}`, `Registry::apply`,
  `scene::diff::{document_delta, list_delta, struct_delta}`, `CompileOutput::{graphics_delta,
  instance_delta}`. Unused by any geometry crate today.
- **Provenance/identity.** `gallery::identity::{Identity, faceted, across_the_flank, across_the_belt}`
  gives every authored element an id per pixel and a name table, numbering taken from
  `mountain::expand::belt_references` so the two cannot drift (`identity.rs:33-35`, :384). A building
  crate has no equivalent.

---

## 5. How buildings are hand-built today, and the pain

**There is already an architectural form library — inside the gallery crate.**
`gallery/src/built.rs:1-7` says so: *"Built forms: the solid things a scene is put together from.*
`edge_normal` :64 · `wall_run` :98 (five faces, open at the bottom) · `push_wall_run` :124 ·
`WallGap` :181 · `wall_ring` :237 · `roof_ridge` :347 · `sweep` :417. Primitives sit next door in
`geometry.rs`: `cuboid` :620, `prism` :1010 (gable roof), `heightfield` :267, `plane_facing` :324,
`lathe` :438.

**Two incompatible strategies coexist.**

- *Baked mesh.* `foldyard.rs`: `ring_corners` :776, `enclosure` :872 (wall + coping + both gate piers
  as **one** mesh), `roof_edge` :922, lintel by `built::sweep` :969, field wall by `built::wall_run`
  :978, shed by `built::wall_ring` :1020.
- *Instanced unit mesh.* `terraces.rs`: two unit meshes — `wall` = `cuboid(1,1,1)` :361, `roof` =
  `prism(1,1,1)` :367 — and `place_house` :533-579 places nine of them purely by instance transform.

**The pain this shows, in the repo's own words.**

1. `foldyard.rs:822 push_ring` is *literally* `wall_ring`'s walk rewritten to append, and says so at
   :811-816. One behaviour, two implementations.
2. **`PackedList` cannot be read back and rejoined** (`built.rs:116-121`, `foldyard.rs:813-816`).
   Every composition must therefore walk the ring itself, which is what created (1). This is the
   hardest structural constraint on building composition.
3. Roofs are **height functions and therefore single-sided**: every normal points up, so a camera
   below the eave sees through them, and the soffit must be a separate hand-written band
   (`built.rs:339-346`, `foldyard.rs:906-921`). A building crate must answer this once.
4. **A wrong outward normal is invisible**: *"A run whose outer face points the wrong way is not a
   broken wall but a wall that is not drawn"* (`built.rs:86-88`). There is no `validate()` here at all.
5. No provenance: an agent looking at a pixel cannot say which wall it is — `identity.rs` exists for
   mountain belts only.

---

## 6. What a building crate inherits if hierarchical NURBS lands

**Status.** Approved and *in flight*, not landed: branch `feature/nurbs-multipatch`, worktree
`.worktrees/nurbs-multipatch`, HEAD `8f6aa66`, seven probes green
(`handoffs/hierarchical-nurbs-multipatch.md:1-6`). The authority is
`.agents/SPEC.align/hierarchical-nurbs-multipatch.md`, which is explicitly **not** merged into
`SPEC.md` (:3-5).

**Inherited:** a mesh-of-patches model with one rectangular Bernstein net per **quad** face and a
hard/soft tag per edge. Two measured facts make this the natural representation for architecture:

- **A planar face is reproduced exactly.** Level 0 is a *degree-1 bilinear net through the quad's four
  corners* (`SPEC.align:86-88`), and Doo–Sabin is planarity-preserving to `1.047e-15`
  (`SPEC.align:63-68`). A flat wall is a bilinear patch, exactly.
- **A hard edge costs nothing**: *"A hard edge carries no invariant at all. `A_1` and `B_1` are simply
  independent"* (`proposals/hierarchical-nurbs-multipatch.md:136`). Wall-on-wall, eave-on-wall, and
  every building corner is a hard edge — the case the model handles for free.

**Not inherited:** soft edges are **not** continuous. A lone seam already folds `8.632e-2` rad at its
middle and `5.1e-1`–`6.7e-1` at its ends with nothing to contest
(`proposals/arbitrary-topology-smoothness.md:17-22`); a *regular* 4×4 grid with every interior edge
soft reads median `2.128e-1` rad (`:23-25`); a T-junction is **not representable at all** (`:26-30`).
Two soft edges on one face remain over-determined (`two-soft-edges-one-face.md:1-4`, :91-96).

**Facet mesh or patch mesh as the base?** They are complementary, not competing, and the evidence
points both ways:

- *Facet mesh is the better **authoring** base.* Its planarity is a guarantee of the expansion
  (`facet.rs:5-9`), its correspondence makes watertightness structural rather than searched for
  (`expand.rs:104-107`), and `patch` has no mesh type of its own — the conversion already exists at
  `gallery/patchmesh.rs:68`.
- *Patch mesh is the better **representation** for architecture* for the two exactness reasons above.
- *But* `block-face-front-end.md` records the price of going per-block: 182 patches replace 7146, yet
  **124 of 182 blocks have boundary runs of unequal length** (`:90-93`), named as the blocker (`:147-149`),
  and a winding defect leaves 46 one-way edges whose shading reverses across them (`:97-143`).

---

## 7. Open questions for the chief designer — **not decided here**

1. **Package position.** `whiting-building` in workspace members + `[workspace.dependencies]`
   (mirroring `mountain`), with **zero** dependencies like `mountain`, or depending on `patch`?
2. **Output representation.** A `FacetMesh`; or a `FacetMesh` → `subdivide` → `PatchMesh`; or plain
   triangles straight into a `PackedList`? (Blocked partly on whether `Topology::Quads` is added.)
3. **Baked or instanced.** One mesh per building (`foldyard` style) or a unit mesh plus instance
   transforms (`terraces` style)? The scene layer supports both and nothing in the framework prefers.
4. **Creases.** Are all building edges hard — the free case? Does the crate need soft edges at all,
   given they are not continuous today?
5. **`validate()`.** Which checks does a building own: wall coverage, closed volumes, planarity,
   outward normal, eave closure? (None of these exist in `built.rs` today.)
6. **Terrain interaction.** Does a building seat itself on a `terrain::HeightField`? If so, does that
   make `terrain` a dependency, and does it brush SPEC/mountain.md R6 ("erosion … out of scope")?
7. **Code edit or document delta.** The skeleton is code. Is a building expected to be a
   `Definition` under a registered schema so an agent patches it with `Delta`s? **No geometry crate
   does this today** — this is greenfield, not a pattern to copy.
8. **Identity.** Does a building mesh need per-element ids the way `identity.rs` numbers belts, so a
   pixel resolves to "that wall"? If yes, the id must come from the same enumeration order the
   expansion used.
9. **SPEC.** SPEC.md has no `patch`/`terrain`/`gallery` entry and `Topology` has no `Quads`. Who
   writes the normative `building` section (SPEC is human-editable only), and does a building crate
   require a `Topology::Quads` first?
10. **Migration.** Is `gallery/src/built.rs` moved into the new crate, or duplicated? `foldyard.rs:822`
    would be the first consumer either way.
