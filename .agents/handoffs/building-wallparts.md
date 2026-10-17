# Parts on walls

Started 2026-10-27. Branch `feature/building-wallparts` (worktree `.worktrees/building-wallparts`, from `main`
at `107fcc9`, which closed the instancing slice). The design is
[`agentic-building-generation.md`](../proposals/agentic-building-generation.md). **This file and the proposal
are the whole record.**

## The gap this slice closes

Every part the crate could place stood in a **bay of a wall of a mass**, and `yard` has a courtyard wall with
a 2.4 m gate gap and **no gate in it**, because a leaf is not a thing a bay holds: it hangs from the post at
the end of one run and closes the opening beyond it. The vocabulary to say that was missing, and it is the
last target a building's parts need.

## What is here

`building::parts::Target` — **what a placement stands its template on**:

```rust
pub enum Target {
    Bay { face: FaceRef, row: usize, column: usize },
    Wall { wall: WallId },
}
```

`Placement` keeps **one** position field, `at: Pin`, measured in **the target's own frame**: across a bay (from
its start) and up from its sill, or along a run (from its start) and up from the ground under the pin. So the
two targets differ in the domain they are measured against and in nothing else. `Placement::new` keeps its
signature (a bay), and `Placement::on_wall(template, wall, at)` is the new constructor — which is why no
existing scene, probe or verb changed shape.

The rest follows from the one decision:

- **`skeleton::Placed`** now carries `bay: Option<Bay>` and `wall: Option<WallId>`, and `placed` resolves
  whichever the target names.
- **`expand::part_frame`** has one branch per target. A wall run **is its own frame**: the run's direction is
  `along`, its normal is `inward`, and the base a pin's `up` is measured from is the ground **interpolated
  along the run**, because a run's own bottom follows the ground between its two nodes.
- **`validate`** measures a bay placement against its rectangle and a wall placement against the run's line —
  and **only the pin**, not the part's box: *a gate leaf is hung at a wall's end and must reach past it.*
  Three new violations carry the three new faults: `PlacementWallMissing`, `PlacementOffItsWall` and
  `PlacementMountOnAWall`.
- **A joint on a wall is refused**, twice: `place_part` answers `None` (there is no bay for an anchor to name),
  and a document that holds one anyway reports `PlacementMountOnAWall` rather than resolving the anchor
  against the run's ends by guesswork.

## Acceptance, measured

`examples/wall_part_probe.rs` — an 8 m run, a gate gap from `x = 2.0` to `x = 4.4`, and a 2.4 m leaf hung at
the run's end:

| measurement | value |
| --- | --- |
| the frame | origin **`(2.0000, 0.0000, -5.0000)`** — the run's end, on its centre line; `along (1, 0)`, `inward (0, 1)` — the run's own |
| the leaf | spans **`x 2.0000 .. 4.4000`**, so it closes **`2.4000 m`** of the gap; **watertight**, 28 faces |
| the document | **0 violations** |
| a pin `0.5 m` beyond the run | `PlacementOffItsWall { placement: 0, wall: WallId(0), past: 0.5 }` |
| a wall the document does not hold | `place_part` answers **`None`** |
| a joint on a wall | `place_part` answers `None`; a document holding one reports `PlacementMountOnAWall` |
| seating on a `0.25 x` slope | the leaf's foot moves to **`y 0.5000`** — the ground under the pin, not the ground at the run's start |

**That last row is a defect the probe found.** The first implementation read the base at the run's `from`
node, so the gate hung **1.5 m under its own doorway** on the slope: the run's bottom follows the ground, and
a part's base has to follow it the same way. The frame now interpolates along the run.

The crate's eighteen probes are green and the **`shed` frame is byte-identical** (`0AF51AEC…`), which is the
evidence that the change disturbed no bay-mounted part.

## Status log

- **2026-10-27** — worktree and branch from `107fcc9`; `Target`, `on_wall`, the three resolutions, the three
  violations, the probe, and the slope fix it found.
- **Next**: the **yard's gate leaf** — the scene has the gap and now the crate has the shape; drawing it there
  is the slice's visible payoff.
