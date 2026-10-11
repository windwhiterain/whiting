# Why Cycles shades our meshes from the wrong hemisphere

measured 2026-10-11 on `main` at `c681658`. landscape, 1024x1024, 32 samples on CPU, Cycles
5.3.0 at `C:\resource\cycles`. every number below is the renderer's own output or its own
source; the scripts are in `out/`.

## The question

`landscape`'s massif reads with fine striations on the Cycles side that the rasteriser does
not draw, and Cycles' frame sits at a different brightness. The rasteriser interpolates the
document's per-corner normals. Does Cycles?

## Short answer

No — and for two independent reasons, both of them ours.

## Finding 1: the normal array is written under a name the loader never looks for

The loader reads the array by the name the kernel uses on the wire:

```text
cycles_xml.cpp:470
  if (xml_read_float3_array(VN, node, Attribute::standard_name(ATTR_STD_VERTEX_NORMAL))) {
```

and that name is short:

```text
scene/attribute.cpp:460-462
  case ATTR_STD_VERTEX_NORMAL:
  case ATTR_STD_CORNER_NORMAL:
    return "N";
```

`Attribute::standard_name` returns **`"N"`**. Our emitter writes `standard_vertex_normal` —
the *enum* name, not the wire name — so the read misses, no attribute is added, and Cycles
computes its own. `crates/scene/src/cycles_export/attribute_map.rs` even documents deriving
it from `Attribute::standard_name`; the derivation was done by eye from the enum instead.

UVs escape this by luck: the loader accepts two spellings.

```text
cycles_xml.cpp:482-483
  if (xml_read_float_array(UV, node, "UV") ||
      xml_read_float_array(UV, node, Attribute::standard_name(ATTR_STD_UV)))
```

Normals have no such alias. So the attribute that is supposed to be the one thing carrying a
smooth surface was never read, and **nothing said so** — a missing attribute is not an error.

### Measured

Three variants of the exported document, each rendered by the same binary, each differing in
one attribute. Byte-identical output means the attribute had no effect at all.

| document | render MD5 |
|---|---|
| as emitted (`standard_vertex_normal` present) | `227D9C4A…` |
| attribute removed entirely | `227D9C4A…` |
| attribute replaced with per-triangle face normals | `227D9C4A…` |

## Finding 2: the normals we do write face the opposite way from the winding — **on some meshes only**

Renaming the array to `N` makes it take effect immediately — and the massif turns **black**,
which is the signature of a shading normal pointing into the surface.

The array and the geometry disagree, and **which way they disagree is per mesh, and not
even consistent within one scene:**

| scene | mesh 0 | mesh 1 |
|---|---|---|
| landscape | −0.9897 | −0.9897 |
| landscape | −0.9991 (the plain) | |
| sunlight | **−1.0000** | **+1.0000** |
| lanterns | **−1.0000** | **+1.0000** |

So the gallery's mesh builders do not share one winding convention. Some emit normals that
agree with their winding and some emit normals that are its exact negation. That is the
finding that decides the fix, and it arrived after the fix was written — see
[the correction below](#the-correction-an-exporter-side-fix-measures-worse).

**This is not the axis conversion's fault.** The conversion is one proper rotation
(`(x, y, z) → (x, −z, y)`, determinant +1) applied to positions and normals alike, and a
rotation preserves dot products — so a dot of −1 in Cycles' frame was already −1 in ours.

**It is not the rasteriser's either.** The rasteriser renders correctly from these same
normals, which it can only do if its front-face convention is the opposite of the document's
winding: its geometric normal is then the negation of the document's winding normal and agrees
with the authored one. Each backend is internally consistent; they disagree with each other, and
neither agrees with every mesh.

## Finding 3: a mesh cannot be welded, so "just give it connectivity" is not available

The obvious repair — emit shared vertices and let Cycles average the normals itself — is not
expressible. The loader consumes `verts` as per-triangle vertex indices and never writes the
loop-to-vertex map, which `resize_mesh` leaves as the identity:

```text
cycles_xml.cpp:435-436
  mesh->resize_mesh(P.size(), num_triangles);
  std::copy_n(P.data(), P.size(), mesh->get_position_for_write());

cycles_xml.cpp:438
  int *triangles = mesh->get_triangles().data();
```

Measured: welding mesh 0 from 2256 vertices to 887 (keyed on position **and** normal, so a
crease is not welded shut) keeps every array length self-consistent — `P` 2661 components,
`verts` 2256 loop indices, `nverts` 752, `UV` 4512 — and **Cycles dies with an access
violation** (`0xC0000005`) inside `update_mesh()`. Removing the UV array does not help. The
non-indexed case works only because vertex index and loop index coincide there.

## What is *not* the cause

Three things that were worth ruling out, each by measurement rather than by argument.

- **It is not path-tracing noise.** 32 samples and 256 samples produce the same image; 105
  differing pixels out of 1,048,576.
- **It is not the absence of ambient occlusion.** True of the rasteriser and relevant to the
  brightness gap, but the striation survives at every test above.
- **It is not an interpolator limitation.** Path tracers interpolate normals exactly as a
  rasteriser does — barycentrically, within one triangle, using that triangle's own three
  vertices. That step needs no connectivity. *Averaging* a normal across an edge does need a
  shared vertex, and is a separate offline operation; but the normals we already carry are
  already averaged, and only finding 1 stands between them and the tracer.

## The fix, measured

Both defects are emitter-side and both are small. Text surgery on the exported document:

| variant | whole frame |
|---|---|
| as emitted today | +0.7812 |
| array renamed to `N`, mesh left flat | +0.7812 (byte-identical) |
| array renamed to `N` **and** `interpolation="smooth"` | −0.6122 (massif black) |
| as above **and** every normal negated | **+0.7941** |

so the emitter needs three changes, and only in this order does the result hold together:

1. **write the array as `N`.** One string. `attribute_map.rs` should derive it the way the
   loader does rather than by eye.
2. **write `interpolation="smooth"` on the `<state>` that carries a mesh with normals.**
   Otherwise the kernel reads `N` and ignores it, because `get_smooth()` is false by default
   (`cycles_xml.cpp:39`) and is only read on `<state>` (`:757-763`, inside `xml_read_state`).
3. **resolve the normal/winding opposition** — by reversing the emitted winding, or by
   negating on export. Which of the two is right is a decision about which convention the
   document means, not a fact this file settles; it is in
   [proposals/cycles-mesh-normal-orientation.md](../proposals/cycles-mesh-normal-orientation.md).

Change 1 alone is worth doing regardless: as emitted, the emitter has been writing an
attribute the format never reads, and reporting no error for doing it.

## The correction: an exporter-side fix measures worse

The fix above was written and then measured, on every scene, and **it is not a net win.** The
three changes were implemented in Rust — `N` for the array, `interpolation="smooth"` on the
state, and the winding reversed in `write_mesh` — and the whole gallery re-rendered against the
same raster frames:

| scene | as emitted | with all three changes |
|---|---|---|
| landscape | +0.7812 | **+0.7941** |
| tiles | +0.9740 | +0.9740 |
| spotlight | +0.9282 | **+0.8735** |
| lanterns | +0.8571 | **+0.7338** |
| sunlight | +0.9763 | **+0.8105** |
| foldyard | +0.1168 | **−0.2411** |

The Rust result reproduces the text surgery exactly (+0.7941), so the implementation is
faithful; the *rule* is wrong. One scene improves by 0.013 and four get worse, one of them
catastrophically.

**Why, and it is the per-mesh statistic below.** Before these changes the tracer was reading
**no** normals at all and shading from each triangle's geometric normal. The reversal changes
the geometric normal, and enabling `N` changes the shading normal, and the two together flip
the surface for meshes whose authored normals are the negation of their winding. `sunlight`
carries a mesh at −1.0000 and another at +1.0000, so a rule applied to the whole document fixes
the first and inverts the second.

So a rule applied at the mapper cannot be right for a document that holds both. The disagreement
is in the documents, and the documents are where it has to be settled.

**the size of that disagreement was stated wrong here and is corrected below.** This section
originally put it at *half the meshes, inconsistently within a scene*, on the strength of the
per-mesh mean. The distribution puts it at **447 of 620, and consistently within every scene but
four**: nine scenes are unanimous one way or the other, and the four that carry both are the four
carrying non-manifold meshes. The rule failed on the gallery in **both** accounts — it was not
merely a minority to be reasoned about, it was a majority to be obeyed.

**This branch's code change is therefore not to be merged.** It is kept uncommitted in
`.worktrees/emitter-normals` as the reference implementation, because the three edits are
correct as *code* and become correct as a *rule* the moment the convention is settled.

## What is left standing

- **Finding 1 stands and is independent of all of this.** The array must be written as `N`. It
  is a name that does not match the format, and a name that does not match the format is a
  defect whatever the shading turns out to be. It cannot land on its own, because the loader
  then reads a normal array that disagrees with its mesh.
- **Findings 2 and 3 stand**, and finding 2 now says something sharper than it did: not "the
  normals are inverted" but **"the gallery's mesh builders do not share a winding
  convention."** That is the thing to go and fix.
- **The striation explanation stands.** The tracer was reading no normals and shading from
  geometric face normals, which is what the facet striation is.

## The survey: 620 meshes — and the statistic was the wrong one

The correction above says the next attempt is a survey. It has been run: every gallery scene was
exported and the per-mesh statistic taken for every mesh in it (`out/probe-winding-survey.py`,
over the thirteen documents in `out/sv-*.cycles.xml`). It reported, on **one number per mesh**:

| scene | same | opposed | mixed |
|---|---|---|---|
| landscape | 0 | **3** | 0 |
| tiles | 0 | **1** | 0 |
| dunes | 0 | **1** | 0 |
| monoliths | 0 | **1** | 40 |
| lanterns | **72** | 1 | 0 |
| spotlight | **3** | 1 | 0 |
| sunlight | **4** | 1 | 0 |
| nurbs | **1** | 0 | 0 |
| studio | 0 | 0 | 2 |
| stilllife | 5 | 1 | 6 |
| foldyard | 8 | 4 | 8 |
| terraces | 18 | 111 | **116** |
| tidewater | 0 | 2 | **210** |
| **all** | **111** | **127** | **382** |

**that table is wrong, and the error is in the statistic and not in the reading of it.** A mean
per mesh has three causes that need three different remedies, and the mean cannot tell them
apart: a mesh can average near zero because half its triangles agree and half are reversed,
because every triangle sits near ninety degrees to its own normal, or because a smooth corner
normal leans away from any one face and pulls the mean down without anything being wrong.

So the survey was re-run reporting the **distribution** — how many triangles agree, how many
disagree, how many sit within ten degrees of perpendicular — together with a second measurement
that never looks at a normal at all (`out/probe-orientation.py`, output in `out/orientation.txt`):

| scene | same | opposed | split | no verdict | edges same-direction | branched |
|---|---|---|---|---|---|---|
| landscape | 0 | **3** | 0 | 0 | 0 | 0 |
| tiles | 0 | **1** | 0 | 0 | 0 | 0 |
| dunes | 0 | **1** | 0 | 0 | 0 | 0 |
| lanterns | **72** | 1 | 0 | 0 | 0 | 0 |
| spotlight | **3** | 1 | 0 | 0 | 0 | 0 |
| sunlight | **4** | 1 | 0 | 0 | 0 | 0 |
| nurbs | **1** | 0 | 0 | 0 | 0 | 0 |
| monoliths | 0 | 1 | 40 | 0 | 0 | 9440 |
| stilllife | 5 | 1 | 6 | 0 | 0 | 5592 |
| terraces | 18 | **221** | 6 | 0 | 0 | 288 |
| tidewater | 0 | **212** | 0 | 0 | 0 | 0 |
| foldyard | 7 | 4 | 9 | 0 | 156 | 208 |
| studio | 0 | 0 | 0 | 2 | 0 | 0 |
| **all** | **110** | **447** | **61** | **2** | 156 | 15536 |

**557 of the 620 are unanimous**, and that is the fact the mean could not express: a mesh is
filed as *opposed* only when **not one** of its triangles agrees, and as *same* only when **not
one** of them disagrees. `tiles` is 4 608 triangles with **0 agreeing and 4 608 disagreeing**,
median **−1.000**. `tidewater` mesh 210 is 7 triangles with **0 agreeing and 7 disagreeing**,
median **−0.104** — the mesh this section previously cited as its clearest example of a split is
unanimous, merely shallow, because a seven-triangle sheet tilted off its own normal disagrees
*weakly*. **319 of the 382 "mixed" meshes were never mixed at all**; they were unanimous meshes
whose mean had been pulled towards zero by angle rather than by sign.

That is the third wrong guess about this gallery, and the same failure each time: **one scalar
per mesh, then a story told about it.** The first read `landscape`'s three meshes at −0.99 as a
fact about the gallery. The second read `sunlight`'s two meshes as a two-group split. The third
is this one.

**the second measurement answers a question no normal can.** Welding the corners and counting
how each interior edge is traversed says whether a mesh's winding agrees with *itself*, whatever
any normal claims: a coherent surface traverses every interior edge once in each direction. Nine
of the thirteen scenes have **zero** edges traversed twice the same way. The four that do —
`monoliths` 9 440, `stilllife` 5 592, `foldyard` 156, `terraces` 288 — are the four carrying the
61 split meshes. That the branching causes the split is **not** established: a fan-capped prism
puts more than two triangles on its own boundary edges and is not wrong for it.

### two meshes carry a normal that is not the surface's normal at all

`studio` is the only scene the distribution cannot classify, and both its meshes fail the same
way. It writes one triangle:

```text
P="-1 -0 0 1 -0 0 0 -0 1"   standard_vertex_normal="0 -0 1 0 -0 1 0 -0 1"
```

every position has `z = 0`, so the triangle lies in the plane `z = 0` and its geometric normal
is `±Y`; the authored normal is `+Z`, which lies **inside** that plane. No winding makes those
agree, and no rule applies to a mesh whose normal is perpendicular to its own surface.
`crates/scene/src/standard/example.rs:249-263` builds it from `[(-1.0, 0.0), (1.0, 0.0),
(0.0, 1.0)]` as `(x, y, 0.0)` — the third component is never varied, so the triangle is built in
the wrong plane. Its own comment says *"the triangle lies flat on the ground plane and faces the
light side of the room"*, and in a y-up engine the ground plane is `y = 0`: the positions were
meant to be `(x, 0.0, z)`. Under the normal-is-outward convention these two meshes are
**malformed documents**, not mis-wound ones.

### nothing in the rasterised path reads a winding at all

searched, and the answer is negative: no `cull_mode`, no `FrontFace` and no `CullMode` appears
in any `.rs` under `crates/`, and no `front_facing`, `faceforward` or `face_normal` in any
`.wgsl`. the rasteriser shades from the authored normal and draws both sides of every triangle,
so **the winding is read by Cycles alone.** re-winding the gallery is therefore invisible to the
rasterised frames by construction, and no raster render needs re-grading to prove it.

### what this changes about the fix

The survey's conclusion — *there is no rule to apply to most of them* — does not survive. One
rule applies to **557 of the 620**, and it is the same rule for every one of them: where the
winding disagrees with the normal, reverse it, which is what "the authored normal is outward"
means. The 61 split meshes are confined to five scenes and are a bounded follow-up; the 2 in
`studio` are not a winding question at all.

**the second finding stands and is the root of all of it.** `SPEC` says nothing about winding,
front faces or culling anywhere — grepping it for `winding`, `counter clockwise`, `front face`,
`back face` and `cull` returns only light culling. So this is not a case of the code departing
from a written rule; there is no written rule, which is the actual defect, and it is why 620
meshes came to disagree.

## The source-level half of it, from `geometry.rs`

The two helpers in `crates/gallery/src/geometry.rs` can be checked by reading them, and both
disagree with their own comments.

`heightfield` gives every point the normal `[-slope_x, 1, -slope_z]`, so a flat field is
**(0, 1, 0), up**. Its first triangle is `p00 → p10 → p01` with `u = (s, 0, 0)` and
`v = (0, 0, s)`, so **u × v = (0, −s², 0), down**. The two are exact opposites, which is the
measured −1.0000. Its comment says *"wound counter clockwise seen from above, so the surface
faces up and the winding the rasterizer wants is the winding that makes it visible"* — the last
clause is the honest one and it is the whole story: **the winding was chosen for our own
rasteriser**, and nothing in it obliges another backend to agree.

`plane_facing` claims *"the winding follows the normal rather than being fixed"*. It does not:
with `a = (−e, h, −e)`, `b = (e, h, −e)`, `c = (−e, h, e)` and `facing_up`, the first triangle is
`(a, b, c)`, `u = (2e, 0, 0)`, `v = (0, 0, 2e)`, and **u × v = (0, −4e², 0)** — down, while the
normal is up. So it is in the opposed group too, **against its own documentation**.

## What this means for the fix

The exporter-side rule is dead, and the survey's first version gave the wrong reason for it. It
is not that the rule fails for most meshes; it is that **557 meshes all need the same rule and
61 need none that a single rule can state.** So the project has to (a) state the convention in
`SPEC` and (b) bring the documents into line, and the decision of *which way the convention
goes* has been taken: **the authored normal is the outward direction and the winding obeys it.**
Geometry is the asset; the winding is the notation that records it.

That settles the direction and not the mechanism. Where the correction is applied is still open,
and the two ways of applying it do not cost the same:

- **at the builders** — each generator emits its triangles in the order that agrees with its own
  normals. Honest, and it keeps the document meaning what it says. It is also an edit per
  generator, in a file several sessions are working on at once.
- **once in the geometry pipeline** — one place orders each triangle's corners so that
  `cross(b − a, c − a)` agrees with the normals, which makes the convention true by
  construction and covers the 61 split meshes per triangle rather than per mesh. The cost is
  that the winding stops being authored and becomes derived, which is in tension with the rule
  this `SPEC` already states — *a reader may apply documented defaults but may not compute a
  value the document did not carry*.

Either way the `N` array and the `interpolation` flag land with it, because renaming the array
on its own drops the massif to **−0.6122**.

The statistic is written and re-runnable, which is what makes the project checkable rather than
a matter of taste: `out/probe-orientation.py` for the distribution and the topology-only
coherence test, and `out/probe-winding-survey.py` for the mean it replaced.

## The builders, named

**decided: at the builders.** the pipeline-derived alternative was rejected because it would
have made the winding a computed value, and because it would have silently repaired
`studio`'s malformed mesh rather than showing it — a generator that cannot be caught cannot be
corrected. the mesh count is misleading about the size of the job: the 447 opposed meshes come
from **twelve distinct shapes**, of which `cone(9, …)` accounts for 220 in `terraces` and
`cone(7, …)` for 210 in `tidewater`, and every large mesh in the gallery is a `heightfield(n)`
of exactly `2n²` triangles. nine generators carry all 620:

| builder | verdict | why |
|---|---|---|
| `geometry.rs:356` `sphere` | split | `dP/dθ × dP/dφ = cos φ · n`, so the ring's angular sense reverses at the equator and half the mesh faced inward |
| `geometry.rs:267` `heightfield` | opposed | `p00 → p10 → p01` crosses to `(0, −s², 0)` against a normal of `(0, 1, 0)` |
| `geometry.rs:913` `masked_heightfield` | opposed | the same construction with the mask in the normal's third component |
| `geometry.rs:323` `plane_facing` | opposed | opposed in **both** facings, which is what makes its own claim false rather than half true |
| `geometry.rs:485` `cone` | opposed | `cross(b − a, apex − a)` is the exact negation of the `cross(slope, b − a)` the normal is taken from |
| `geometry.rs:855` `double_blade` | opposed | the second ribbon flipped its normal and not its winding |
| `built.rs:417` `sweep` | split | the sides agreed and **both** end caps were opposed |
| `foldyard.rs:575` `cut_heightfield` | opposed | a second copy of `heightfield` that wound the other way |
| `foldyard.rs:889` `roof_edge` | split | the wall band agreed on 80 triangles and the two soffit triangles were opposed |
| `identity.rs:384` `faceted` | opposed | the massif crate hands over negated normals — see below |

`quad`, `cuboid`, `prism`, `lathe`, `culm`, `blade`, `leaf_spray`, `tuft` and `nurbs` all agree,
and two of the last three agree **exactly**: `lathe`'s `u × v` is `r · δ` times the normal it
writes, and `blade`'s is proportional to `(−lean, 1, 0)`, which is what it writes. `tuft`'s
normal is the ideal upright blade's rather than its surface's own, so it agrees in sign without
being parallel — worth knowing, and not a winding defect.

**the massif is the one place the engine's convention is deliberately the other way.**
`mountain::math::face_normal` is `normal(c − a, b − a)` with the arguments swapped, and its own
documentation explains why at length: positive plan area in `(x, z)` crosses to `−y` in a y-up
right-handed frame, so a height field's upward normal is the negation of the right-handed one.
That reasoning is sound and it is load-bearing — `signed_area` and every belt walk read it. so
the correction is applied where the massif becomes a **document**, in
`gallery::identity::faceted`, which writes `[a, c, b]` instead of `[a, b, c]`. each corner still
carries its own normal, its own `uv` and its own position.

## Why this cannot move a rasterised frame

every correction above changes **the order three corners are written in** and nothing else —
positions, normals, texture coordinates and instance transforms are all untouched. `studio` is
the single exception, because its fix changes the plane the triangle is built in.

and nothing reads that order on this side: no `cull_mode`, no `FrontFace` and no `CullMode` in
any `.rs` under `crates/`, no `front_facing`, `faceforward` or `face_normal` in any `.wgsl`. the
rasteriser shades from the authored normal and draws both sides of every triangle. so the
rasterised frames are a control rather than a measurement: the existing ones stand, and any
movement in a Cycles frame is attributable to the winding alone.

## What the fix measured

all thirteen documents re-exported and the same probe run over both sets:

| | same | opposed | split | no verdict |
|---|---|---|---|---|
| before | 110 | 447 | 61 | 2 |
| after | **620** | **0** | **0** | **0** |

`studio` is among the 620 and comes out agreeing, its malformed mesh now a well-formed one.

one thing to know before reading a per-row report of a sphere: the document is y-up while the
export is z-up, so `sphere`'s single hemisphere at `y >= 0` arrives as `z >= 0`, and bucketing
the triangles by index against the wrong axis puts the pole band in the wrong place. the exporter
applies `(x, y, z) -> (x, -z, y)` to positions and normals alike, so a **dot product is
invariant** and every verdict above was never in doubt — only the row arithmetic was.

### and the measurement itself needed correcting first

the first run after the fix reported **52 meshes still split**, every one of them a `sphere`,
and every one of them disagreeing on exactly one triangle per column. that is not a winding
defect and the numbers say so: the pole band's triangles sit at an area of **3.5e-19 to 9.5e-18**
against a mesh whose smallest meaningful face is **1e-3** — sixteen orders of magnitude, with
nothing in between. the band is the row whose ring is `phi = pi / 2`, where `cos(phi)` is zero
and all four corners of the cell coincide, so the second triangle of that cell has two identical
corners and its cross product is rounding rather than a direction.

so the probe now judges degeneracy against **the mesh's own size** — a triangle is collapsed
when its area is below `1e-12` of the mesh's squared bounding-box diagonal — and reports the
count rather than hiding it: **2 408 collapsed triangles** over the gallery, 1 600 of them in
`monoliths` and 680 in `stilllife`. that is the same class of error as the mean that started
this, one level down: a verdict computed from a quantity too small to carry it.

## and `sphere` is not a sphere

found on the way, measured, and **not fixed here** because fixing it moves every rasterised
frame in the gallery.

`geometry.rs:356` sweeps `phi` from 0 to `pi` and takes the vertex as
`(sin t cos p, sin p, cos t cos p)`. `sin p` is never negative over that range, so **no point of
the mesh is below the equator**: measured on `sphere(10, 1.0)`, every one of its 600 corners has
`y >= 0`. and `cos p` changes sign at `pi / 2`, so the second half of the sweep retraces the
first with `t` shifted by a half turn: the mesh is **one hemisphere drawn twice**.

`sphere(10, 1.0)` emits 600 corners at **51 distinct positions**, and 60 of those corners lie on
the equator at only **10** distinct points — the rim is emitted six times over. every sphere in
the gallery is therefore half a sphere, doubled, with a degenerate pole band between the halves,
and the two coincident halves are necessarily wound in opposite senses once each agrees with
its own radial normal. that is where the **800 same-direction edges** in the coherence column
come from, and it is a fact about `sphere`'s shape rather than a defect in the correction.

this is the same root as the winding defect and one layer deeper: `sphere` was written to a
shape nobody wrote down, which is the state this whole file is about.

**it has since been fixed**, and the topology columns are what it bought. with `phi` the polar
angle the doubled surface is gone, and with it every edge the survey counted as traversed twice
the same way — `monoliths` **800 → 0**, `stilllife` **204 → 0**, `terraces` **60 → 0** — while the
branched count falls with it, 9440 → **840**, 5592 → **486** and 288 → **66**. all 620 meshes
still agree with their own normals, and the winding is now coherent with itself in every scene
that has a sphere in it, which it was not before. the proposal, with the measurement that led to
it, is [proposals/sphere-is-a-hemisphere-twice.md](../proposals/sphere-is-a-hemisphere-twice.md).

## and the picture did not move, which is the finding

all thirteen frames re-rendered on Cycles and compared against the raster frames re-rendered
from the same branch, at 1024x1024 and 32 samples on CPU:

| scene | correlation before | after | |
|---|---|---|---|
| tiles | +0.9740 | **+0.9740** | unchanged |
| sunlight | +0.9763 | **+0.9763** | unchanged |
| spotlight | +0.9282 | **+0.9282** | unchanged |
| lanterns | +0.8571 | **+0.8571** | unchanged |
| foldyard | +0.1168 | **+0.1168** | unchanged |
| landscape | +0.7812 | +0.7826 | +0.0014 |

**447 meshes had their winding reversed and not one scene moved.** every rendered frame did
change — the eleven that carry a raster comparison all hash differently — and the change is
noise:

| scene | mean luma before | after | pixels changed by more than 2 of 255 | mean absolute difference |
|---|---|---|---|---|
| tiles | 87.69 | 87.69 | 0.07% | 0.01 |
| dunes | 69.92 | 69.94 | 16.91% | 1.52 |
| landscape | 134.01 | 134.03 | 24.07% | 1.73 |
| sunlight | 57.82 | 57.83 | 38.29% | 2.18 |

the reason is that **the sign of the winding is not what the tracer shades from.** the normal
array is still written under a name the loader never reads, so `Ng` comes from the winding —
and a path tracer flips its shading normal towards the ray it is shading, so a surface lit from
the front is lit from the front whichever way its corners were written. a floor seen from above
is a floor.

so the defect was real and is now fixed, and it was **never the cause of the two backends
disagreeing**. that is worth more than the agreement number would have been: it removes the
last standing explanation for the striation, and it means the remaining work is the thing this
change unblocks — writing the array as `N`, which can no longer black out the massif because
every mesh in the gallery now agrees with its own normals.

two scenes have no raster frame to compare against, and did not have one before this either,
measured on `aeb4ced` itself: **`studio`** renders its frame in `rgba16float` and the harness
reads a frame as an 8 bit rgba image, and **`nurbs`** fails to compile with *"redefinition of
`fresnel`"* in `lighting/lighting/sheet`. neither is a winding matter and neither is fixed here.

the remaining scenes, compared for the first time here because no baseline existed for them:
`dunes` **−0.6584**, `terraces` **−0.8264**, `stilllife` +0.4887, `tidewater` +0.3435,
`monoliths` **−0.1594**.

## Sources

- `out/probe-mesh-normals.py` — per-mesh normal statistics, including the face-normal dot.
- `out/probe-orientation.py` — the distribution that replaced the mean, and the orientability
  test that never reads a normal. Its output is `out/orientation.txt`.
- `out/probe-normal-variants.py` — the three documents, by raw text surgery.
- `out/probe-interpolation.py` — adds `interpolation` to `<state>`.
- `out/probe-weld.py` — the weld, keyed on position and normal.
- `out/probe-striation.py`, `out/probe-diff.py`, `out/probe-scanline.py` — the measurements.
- `C:\resource\cycles\src\app\cycles_xml.cpp:39, 396-479, 715-767` — the loader.
- `C:\resource\cycles\src\scene\attribute.cpp:453-475` — `standard_name`, and the `"N"`.
- `C:\resource\cycles\src\scene\mesh.cpp:661` — custom normals are used when present.

Two things went wrong in `probe-orientation.py` before it reported anything, and both reported
nothing rather than something wrong — the failure mode to watch for in a probe:

- the first coherence test stored the **smaller endpoint** of each directed edge, so both
  traversals of an interior edge compared equal and every interior edge was reported as a
  flip. The direction has to be kept as its own key for the comparison to mean anything.
- the scene accumulator was one element shorter than the columns being summed into it, which is
  an `IndexError` rather than a silent wrong number, and was the better of the two failures.

A negative result is recorded too, because it is the reason the rasterised frames do not have to
be re-rendered to check any of this: `grep` finds no `cull_mode`, `FrontFace` or `CullMode` in
any `.rs` under `crates/`, and no `front_facing`, `faceforward` or `face_normal` in any `.wgsl`.
That is a search result and not a proof that no consumer exists — it is worth re-running if the
rasterised path ever grows a back-face test.

An XML round trip was tried for the variants and rejected: it drops comments and re-serialises
every float, and the renders that came back carried a colour shift that would have made every
contrast measurement worthless. The scripts edit the attribute as text and leave the rest of
the file byte for byte identical.
