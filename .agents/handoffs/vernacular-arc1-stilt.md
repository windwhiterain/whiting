# Arc 1 of the Chinese vernacular level: the ground is a query

Status: in progress, 2026-10-18. Design record:
[chinese-vernacular-architecture.md](../proposals/chinese-vernacular-architecture.md), §D4 and §"The road"
(Arc 1). Research: [chongqing-stilt-houses.md](../investigate/chongqing-stilt-houses.md),
[guizhou-dwellings.md](../investigate/guizhou-dwellings.md).

## What this arc is for

The 吊脚楼 is the owner's first-named type and the one the site model refuses. `site::seat` gives every
mass **one level — the highest ground under its outline plus an offset** — and its own doc says why:
"cut and fill to a level floor, and let the downhill side show its slab" (`site.rs:24`). That is the
right answer for masonry and the wrong one for a timber house on a river bank, where the floor is set by
the **uphill** grade and the downhill side stands on posts with air under it.

Arc 1 makes the *site* expressible, and nothing else. The building is a plain three-bay body on the
crate's existing hip roof; the Chinese detail (profile, crown, frame, tiles) arrives in Arcs 2–5.

## The decision this arc makes, and the one it defers

- **Posts are short `Wall` runs in this arc.** A square post is a run of `section` metres with a
  `section`-metre thickness, a level top and a bottom that follows the ground at its two ends — which is
  exactly what `Wall` already is (`wall.rs:69`), and `validate`'s three wall refusals all pass it
  (`WallTooShort` is only the zero-length run, `validate.rs:737`).
- **`Member` (D3) is Arc 4.** When it lands, the posts become members and this arc's call site changes
  type, not shape. Nothing in this arc may assume a `Member` exists.

## What to build

1. `site::posts(skeleton, ground, spacing, section) -> StiltReport` — after `seat`, walk each mass's
   closed outline at `spacing`, sample the ground under every post position, and append a `Wall` under
   the mass where the ground is **below** the building's underside (`mass.lowest() - mass.slab`).
   Report the count, the refusals and the deepest drop.
2. `examples/stilt_probe.rs` — an analytic slope, a three-bay body, the numbers printed as a table.

## Invariants

- **The floor stays level.** `posts` moves nothing; it only appends solids below an underside that
  `seat` already levelled. A post may not raise, lower or rotate anything.
- **A post is placed where the ground is below the floor and nowhere else.** The uphill side of a 吊脚楼
  has no posts because the wall meets the ground there; a post whose ground is at or above the
  underside is a post buried in the hill.
- **A ground that does not answer is refused, not guessed.** A non-finite sample skips that post and
  counts it, the way `seat` skips a mass rather than seating it on half a ground (`site.rs:110`).
- **The post's length is the query.** Its top is the building's underside and its bottom is the sampled
  ground, so the range of post lengths *is* the site's relief under the house. That range is the number
  the arc is measured by.
- **Two solids may touch without authorization; they may not overlap.** A post's top is the mass's
  underside exactly, which is a joint by coincidence of numbers — acceptable here because the crate has
  no wall-to-mass authorization at all today (the violation list has no such variant), and **recorded**
  rather than solved: when `Member` arrives with its `host` (D3), the member's authorization is
  stricter than the wall's and the posts should move to it.

## Gates

- `cargo run -p whiting-building --example stilt_probe` prints every number it claims.
- `audit/full-audit.ps1` goes green: the example count for `building` moves **29 → 30** (line 215) and
  nothing else in the audit changes, because this arc adds no scene.
- `cargo clippy` 0, `cargo doc` 0, `cargo fmt --all -- --check` clean.

## Numbers to print, and why each one

| number | what it settles |
| --- | --- |
| posts placed / refused | that the walk covers the outline and that an unknown ground is refused |
| the deepest drop and the shallowest | the house's own proportion: how far it hangs |
| the largest error between a post's bottom and the analytic ground | that "the post's length is the terrain query" is exact, not approximate |
| the floor's spread (max − min node elevation) | that seating is still levelling the floor |
| mesh faces, points, and the validation count | that 30-odd extra solids keep the mesh closed and green |

## What it measures, 2026-10-18

`cargo run -p whiting-building --example stilt_probe`, a 10 × 6 m body on a 35 % bank, posts every
1.5 m of 0.2 m section:

| number | value |
| --- | --- |
| the floor's level | **1.7500 m** = the highest ground under the outline; the terrace fall under it is **3.5000 m** |
| posts placed / refused | **17 / 0** — and 17 walls in the document, so nothing else was mistaken for a post |
| the underside the posts reach | **1.4500 m** = the mass's lowest node less one 0.30 m slab |
| the ground under the posts | **−1.7850 .. 1.2850 m** ⇒ **drop 3.2350 m, relief 3.0700 m** |
| the post lengths | **0.2350 .. 3.2350 m** — the range *is* the site's relief |
| the largest gap between a post's foot and the analytic ground under it | **0.00e0 m** (exact: both sample the same point of a linear ground) |
| the floor's spread after posts | **0.00e0 m** |
| posts uphill of where the ground reaches the underside | **0** — the last post stands at x = 3.4714 of an outline ending at 5.0000 |
| a flat site | **0 posts, 0 masses** — a house on its ground is not a stilt house |
| a half-answerable ground (NaN downhill of x = 0) | **11 placed, 11 refused** |
| the mesh | **268 faces / 172 points** against **64 / 36** without posts; **0 violations** |
| one post | 12 faces, euler 2, manifold, **0.128 m³** = 0.2 × 0.2 × 3.2 |

## What would make this arc wrong

If a post's top face and the mass's underside plane produce a non-manifold edge in the *combined* mesh
— they should not, because the two solids are separate by the two-level rule — then the post must be
inset by a tolerance and the reason written down here rather than in a comment.
