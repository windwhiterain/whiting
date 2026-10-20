# 鼓楼 — the drum tower, and the three things the crate cannot say

A **Dong drum tower** (侗寨鼓楼) is the other shape southwest China builds: not a ring of ranges round a court
and not a row along a street, but a **timber tower whose eaves are stacked**, one skirt of four lean-tos per
storey, each storey stepping back from the one below, and a hip closing the top.

It was attempted as a scene (`crates/gallery/src/drum_tower.rs`) and abandoned inside one round, because the
crate refuses it in three places. The scene is not merged; the numbers below are what the validator and the
expander said, measured on a tower of seven storeys, body 6.0 × 4.6 m, storey 2.4 m, eave reach 1.5 m.

## 1. A stacked tower is "a pile of boxes" to the plan-overlap check

Seven concentric storeys — each smaller than the one below, sitting on its top — are the type's own form.
`validate` answers **`UnattachedOverlap` for every pair**:

- adjacent pairs only (storey *i* attached to storey *i−1* through `Mass::attachment`): 15 faults remained,
  because storey 1's plan also overlaps storeys 3, 4, 5 and 6;
- with no attachments: 21 faults, every pair.

The rule as written is right for wings — two plans that share area with nothing said about how they meet *are*
a pile of boxes — but it has no way to say "this one stands on that one". A tower needs a third relation
beside *apart* and *attached at a station*: **stacked on**, which is neither an overlap to be explained away
nor a joint to be mitigated.

## 2. A hip over a square plan is refused as non-planar

`derive::hip_roof` handles the degenerate case — `hip_over` reads that a crest shorter than epsilon is an apex
and falls back to four `end_plane`s — but the result does not pass `validate`:

| crown | faces reported | departure |
| --- | --- | --- |
| square body 6 × 6, rise 2.6, overhang 1.1, carved 举架 profile | 4 | 0.0025 – 0.0038 |
| square body 6 × 6, rise 2.6, straight slope | 4 | 0.0133 – 0.0199 |
| rectangular body 6 × 4.6, rise 2.6, straight slope | 4 | 0.0098 – 0.0155 |
| rectangular body 6 × 4.6, rise 1.8, straight slope | 4 | 0.0102 – 0.0202 |

The departures are ten to twenty times `PLANARITY_TOLERANCE`, so they are not rounding: the pyramid fallback's
four planes and the hip's two `long_plane`s are each built against an assumed relationship between the crest
line and the eave edge, and a hip that is not a long thin rectangle with a deep inset breaks it.

## 3. A lean-to whose crest is exactly on the wall top is refused as non-planar

The seven skirts are four `derive::lean_to` calls each, a rectangle between the eave line (`low`) and the wall
line (`high`):

```text
plan: front-left, front-right, back-right, back-left   (front = the eave, back = the wall)
low  = storey top − 0.72,  high = storey top
```

Twenty-eight of them expand to **196 faces with departures 0.0102 – 0.0202**. The four points of a skirt *are*
coplanar by construction — two parallels at two heights — so the fault is in how `lean_to` reads the eave loop
and lays its single plane over it, not in the authored rectangle.

## What the type needs, in the order the crate would have to learn it

1. **A `stacked on` relation between masses**, so a storey can stand on the one below without claiming a
   station on its wall — and `validate`'s overlap check can accept it.
2. **A pyramid derivation** (`攒尖顶`), because a drum tower's crown is not a hip and a hip over a square has no
   ridge to hold.
3. **`lean_to` read against a sloped or level crest line that is not the eave offset's own line**, because the
   skirt is the type's whole silhouette and it is currently refused.

Until then the drum tower is a proposal and not a scene, and the honest summary is that this repository builds
**courtyard houses, street rows and stilted houses** — three types of the region — well, and has not yet
learned the fourth.

## Follow-up, measured with a probe: the real blocker is the covering

`crates/building/examples/lean_to_probe.rs` was written to find out which of the three refusals above is the
one that matters, by building the tower's own pieces in isolation. It answers, and the answer is none of them:

| document | violations |
| --- | --- |
| one lean-to on a rectangular wall (the skirt, on its own) | **0** |
| one mass with two skirts, the second one a storey below the wall top | **0** |
| one mass, four skirts a level, at 1, 2 and 7 levels | **0** |
| a hip over plans from 6 × 6 to 6 × 2, rises 0.9 to 2.6, overhangs 0.8 to 1.1 | **0** |
| **the whole tower: one mass, four skirts at seven levels, a hip crown** | **0** |
| **the same tower, every roof wearing a `Covering`** | **284 `FaceNotPlanar`, departures 0.0102 – 0.0202** |

So the crate's roofs are fine and **the tiles are not**: a `Covering` on a lean-to (and on a hip) lifts its
faces out of plane by one to two centimetres, which is ten to twenty times `PLANARITY_TOLERANCE`. That is why
the earlier attempts read as three separate refusals — the scene carried coverings, and the covering is what
the validator was naming.

Two consequences:

1. **A drum tower can be built today without tiles.** The same tower with board roofs is a clean document, and
   that is what the scene does: seven skirts and a crown, no `Covering`, so the eaves read as boarded soffits
   rather than as tile. It is a real tower and a provisional one.
2. **The covering's fault is a defect of its own and wants its own round**: the roll geometry is laid against
   the skin but its side faces are built in a plane that does not follow the roof's slope, and a lean-to is
   where that shows. `crates/building/src/covering.rs` owns it.
