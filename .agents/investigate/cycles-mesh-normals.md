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

## Finding 2: the normals we do write face the opposite way from the winding

Renaming the array to `N` makes it take effect immediately — and the massif turns **black**,
which is the signature of a shading normal pointing into the surface.

The array and the geometry disagree, and they do so in every mesh:

| mesh | triangles | mean dot of exported normal with the face normal | corners facing away |
|---|---|---|---|
| 0 (massif) | 752 | **−0.9897** | 99.6% |
| 1 (far range) | 752 | **−0.9897** | 99.6% |
| 2 (plain) | 18432 | **−0.9991** | 99.9% |

Hand-checked on triangle 0 of mesh 0, from the exported positions and the exported normal:

```text
u = v1 − v0 = (43.333, −10.0, −33.333)
v = v2 − v0 = (16.25,  −34.5, −38.75)
u × v        = (−762.5, 1137.5, −1332.5)  →  normalised (−0.3990, 0.5953, −0.6974)
exported N[0] = ( 0.399062, −0.595325, 0.697379)
```

They are the same three numbers with opposite signs.

**This is not the axis conversion's fault.** The conversion is one proper rotation
(`(x, y, z) → (x, −z, y)`, determinant +1) applied to positions and normals alike, and a
rotation preserves dot products — so a dot of −1 in Cycles' frame was already −1 in ours.

**It is not the rasteriser's either.** The rasteriser renders correctly from these same
normals, which it can only do if its front-face convention is the opposite of the document's
winding: its geometric normal is then the negation of the document's winding normal and agrees
with the authored one. Both conventions are internally consistent; they simply disagree with
each other, and Cycles follows the one the document does not.

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
