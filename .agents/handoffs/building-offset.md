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

**The shader half**: an `Extras` output whose value is `point field + instance field`, the instance field a
scene sets to the placement's own offset, and a scene (the workshop) whose shutters then answer five different
ids. This slice establishes that the numbers the shader would add are the right ones; the next one adds them.

## Status log

- **2026-10-27** — worktree and branch from `368a012`; the probe and the measurement above.
