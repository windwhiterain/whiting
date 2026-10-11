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

**Why, and it is the per-mesh statistic above.** Before these changes the tracer was reading
**no** normals at all and shading from each triangle's geometric normal. The reversal changes
the geometric normal, and enabling `N` changes the shading normal, and the two together flip
the surface for meshes whose authored normals are the negation of their winding — which, in this
gallery, is **half of them and not consistently within a scene**. `sunlight` carries a mesh at
−1.0000 and another at +1.0000; the reversal fixes the first and inverts the second.

So a rule applied at the mapper cannot be right for meshes that do not share a convention. The
disagreement is in the documents, and the documents are where it has to be settled.

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

## The survey: 620 meshes, and the split is not two groups

The correction above says the next attempt is a survey. It has been run: every gallery scene
was exported and the per-mesh statistic taken for every mesh in it
(`out/probe-winding-survey.py`, over the thirteen documents in `out/sv-*.cycles.xml`).

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

**620 meshes. It is not a two-group split, and the largest bucket is the one that is neither.**
382 meshes have a mean dot near zero — `stilllife` carries five identical 2592-triangle meshes
at **+0.0277**, `tidewater` four identical 7-triangle meshes at **−0.1039**. A mean near zero on
a mesh with a definite extent is not a small disagreement; it is a mesh whose triangles do not
share an orientation at all, so no single reversal can describe it.

Two things follow, and the second is the one that matters.

**First: this was guessed wrong twice, and the guesses were both cheap.** Once from
`landscape`, which was −0.99 on all three of its meshes and looked like a fact about the
gallery. Once from `sunlight`, which was −1 on one mesh and +1 on another and looked like a
two-group split. The gallery has **620** meshes, not six.

**Second: there is no stated convention to be wrong about.** `SPEC` says nothing about winding,
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

The exporter-side rule is dead, and the survey says why in a way the six-scene table did not: it
is not that the rule is wrong for most meshes, it is that **there is no rule to apply to most of
them**. A document-layer project has to (a) state the convention in `SPEC` and (b) bring the
builders into line, and neither half is a change any one of us should start without deciding
which way the convention goes — because the answer changes every scene's appearance on one
backend, and 620 meshes means 620 chances to be wrong.

The statistic is written and re-runnable, which is what makes the project checkable rather than
a matter of taste: `out/probe-winding-survey.py`, per mesh, per scene, with the triangle count
and extent so the builder behind each row can be named.

## Sources

- `out/probe-mesh-normals.py` — per-mesh normal statistics, including the face-normal dot.
- `out/probe-normal-variants.py` — the three documents, by raw text surgery.
- `out/probe-interpolation.py` — adds `interpolation` to `<state>`.
- `out/probe-weld.py` — the weld, keyed on position and normal.
- `out/probe-striation.py`, `out/probe-diff.py`, `out/probe-scanline.py` — the measurements.
- `C:\resource\cycles\src\app\cycles_xml.cpp:39, 396-479, 715-767` — the loader.
- `C:\resource\cycles\src\scene\attribute.cpp:453-475` — `standard_name`, and the `"N"`.
- `C:\resource\cycles\src\scene\mesh.cpp:661` — custom normals are used when present.

An XML round trip was tried for the variants and rejected: it drops comments and re-serialises
every float, and the renders that came back carried a colour shift that would have made every
contrast measurement worthless. The scripts edit the attribute as text and leave the rest of
the file byte for byte identical.
