# An id offset per instance: the arithmetic first

Started 2026-10-27. Branch `feature/building-offset` (worktree `.worktrees/building-offset`, from `main` at
`368a012`, which closed the identify slice). The design is
[`agentic-building-generation.md`](../proposals/agentic-building-generation.md). **This file and the proposal
are the whole record.**

## The tension this slice resolves

The identify slice recorded it: **the id channel and instancing do not compose**. An `authored_id` is a
**per-vertex** value, so one instanced mesh drawn five times carries **one** set of ids — the workshop's five
shutters would all answer the same element, or, packed from the template's own mesh, a different table's
numbers. Two designs were named: a per-instance offset the shader adds, or two channels.

**The offset design is chosen**, and this slice's first step is to measure the arithmetic it rests on rather
than assume it:

```text
baked_id(placement, surface) = template_local_id(surface) + base(placement) - 1
```

That equation is only a mapping if the baked table lays each placement's surfaces down as a **contiguous run, in
the order the template's own table uses**.

## Measured

`examples/instance_identity_probe.rs`, six placements of one shutter:

| measurement | value |
| --- | --- |
| the two tables | baked `108` rows, template `14` — six placements of one surface are six rows |
| each placement's run | `25..=38`, `39..=52`, `53..=66`, `67..=80`, `81..=94`, `95..=108` — **all contiguous, all 14 rows** |
| the equation, surface by surface | **true over 84 surfaces** |
| the offsets a shader would need | `24, 38, 52, 66, 80, 94` (`base − 1` each) |

Contiguity is not luck: `Identity::of` builds its rows from a `BTreeSet<Element>`, and `Element::Part { placement,
surface }` orders by `placement` first — so a placement's rows are adjacent **by construction**, and the
per-placement order is the same `PartSurface` order the template's own table uses.

## What is not in this slice

**The shader half**, and this slice's reconnaissance found where the offset can actually live — which is not
where I first assumed.

### The finding: an instance carries no custom values

The scene format's instance is `{label, mesh, transform, material}`, and the custom values a shader reads
(`emissive`, `metallic`, `roughness`, …) belong to the **material**: `kit::mesh_material(label, shader,
custom_data)`. So every instance that names one material shares its values, and a per-instance number is
**not expressible** in today's format.

That leaves two routes:

1. **Extend the scene format** so an instance may override its material's custom values. That is the `scene`
   crate's slice and the compiler's, not the building crate's.
2. **One material per placement**, each carrying its own offset. A material already carries custom data, and
   five materials may all name the **same** shader — so five shutters become fifteen materials (three stages
   each) whose `authored_offset` is `base − 1`, and the shader sums a **point** field (`authored_id`) with a
   **material** field (`authored_offset`). No format change at all.

**Route 2 is the plan**: it needs three edits and no new subsystem —

- `kit`: an `authored_offset` float in `gallery_custom_schema`, written `0.0` by `mesh_material` (so every
  existing scene is unchanged), and a variant a building scene uses to set it;
- `shaders::Authored`: a `Sum { point, custom }` variant whose vertex assignment is `<read the point field> +
  <custom field>`; the varying already carries one `f32`, so the fragment side does not change;
- `building_identity`: `building_extras()` uses the Sum, and an `offset` variant sets the material field.

The acceptance is the one the tension was recorded for: **the workshop's five shutters answer five different
ids**, each naming its own placement, and a pixel of the ground still answers zero.

## Status log

- **2026-10-27** — worktree and branch from `368a012`; the probe and the measurement above.
- **2026-10-27** — reconnaissance for the shader half: the instance carries no custom values, so the offset
  lives in a **material** field, one material per placement, and the plan above is exact.
