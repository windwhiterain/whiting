# What a vegetation scene's CPU build costs, and where it goes

**Question answered:** why does `Gallery::build` on a whole-tree scene cost minutes when the same
scene's GPU compile is seconds — and which numbers in a species module move that cost?

**Measured:** 2026-10-19, in the worktree `.worktrees/sw-oak2`, branch `feature/sw-oak2`, on
川滇高山栎 *Quercus aquifolioides* (`crates/gallery/src/southwest_oak.rs`).

**Sources:** `cargo run -p whiting-device-rasterize --example render -- <scene> <stem> --census`,
which returns before any GPU work and so times the document build alone;
`crates/vegetation/src/plant.rs`, `crates/vegetation/src/field.rs`,
`crates/shape/src/curve.rs`, `crates/gallery/src/vegetation_picture.rs`.

## The measurements

`--census` calls `Gallery::build` **twice** (once for the report, once through `Gallery::bounds`),
so divide the wall clock by two.

| what changed | rows | census wall clock | per build |
| --- | --- | --- | --- |
| bough plate, no field, no `trim` | ~4 800 | 5.8 s | 2.9 s |
| bough plate, **one** `Field::light` added | ~4 800 | 30.7 s | 15.4 s |
| tree plate, `trim` replaced by a 40 m envelope | 8 482 | 37.7 s | 19 s |
| tree plate, the real `trim`, no field | 8 482 | 87.4 s | 44 s |
| tree plate, the real `trim`, `Field::light` on | 8 482 | 236 s | 118 s |
| whole-tree plate as delivered | ~50 000 | — | ~200 s |

## What the rows mean

1. **A plant field is charged once per leaf instance, per ancestor.**
   `Plant::shape` folds every field over the node's spine on *every read*, and `Plant::surface`
   is called once per leaf placement and re-solves the whole ancestor chain each time
   (`plant.rs:263`, `plant.rs:240`). Adding one `Field::light` to a scene that was otherwise
   untouched cost **12.5 s over 3 600 leaf instances = 3.5 ms each**. A crown that needs three
   hundred thousand leaf instances cannot carry a field at all.
2. **`trim` makes the same read expensive even without a field.** `trim` writes a length nudge
   onto each node it shortens, so `Node::spine_now` can no longer hand back the authored curve
   and rebuilds a `Curve` instead (`curve.rs:296`, `rebuild_samples` over `segments × 8`).
   Measured: the same scene builds in **19 s** through a 40 m envelope and **44 s** through the
   species' real one. **A crown's shape is bought with this dial**: an envelope loose enough to
   leave the rule's own dome alone is five times cheaper to build.
3. **`Field::shape` is the same cost again** and stacks: 44 s → 118 s when the field is put back.
4. **One `mesh_material` per leaf group** (`vegetation_picture.rs:1186`), so the material count is
   `fields × nodes`, not rows — at this species's density that is ~16 000 materials for ~50 000 rows.
5. **The render harness adds a one-time GPU cost of about 130 s** that `--census` never pays. It
   is not part of the build and it does not recur between renders in the same session; a *second*
   render of a 13 575-row scene took 18.6 s total against the first's 291 s.

## What this leaves an author

- **Budget in leaf instances, not in rows.** A sprig template buys nine blades for one row, and
  that is the only lever that moves crown area without moving the build cost.
- **A field is a per-instance tax on the whole crown.** Use the rule's own shape laws instead.
- **`trim` is a per-instance tax too**, twice over once it has written a nudge. Draw the envelope
  to sit outside the architecture (which this module already says) and the trim takes a few per
  cent.
- **Report both clocks.** The harness prints `compiled` and `rendered`; neither includes the
  build, and the build is the larger term for a whole tree.

## What is not answered

Whether the re-solve is avoidable in the crate — caching the shaped spine per node would remove
terms 1, 2 and 3 together — is a `shape`/`vegetation` change and not this note's business to
decide. No measurement here says the cache would be correct under `Change::Spine` bakes or under a
field that reads the node's own nudge.
