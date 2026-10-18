# Arc 1 of the Chinese vernacular level: the ground is a query

Status: **steps 1 and 2 landed and green; the scene renders and is not yet in the suite**, 2026-10-18.
Design record: [chinese-vernacular-architecture.md](../proposals/chinese-vernacular-architecture.md),
§D4 and §"The road" (Arc 1). Research:
[chongqing-stilt-houses.md](../investigate/chongqing-stilt-houses.md),
[guizhou-dwellings.md](../investigate/guizhou-dwellings.md).

## What this arc is for

The 吊脚楼 is the owner's first-named type and the one the site model refuses. `site::seat` gives every
mass **one level — the highest ground under its outline plus an offset** — and its own doc says why:
"cut and fill to a level floor, and let the downhill side show its slab" (`site.rs`). That is the right
answer for masonry and the wrong one for a timber house on a river bank, where the floor is set by the
**uphill** grade and the downhill side stands on posts with air under it.

## Step 1, done: `site::posts` and its measurement

`site::posts(skeleton, ground, spacing, section, embed)` appends one `Wall` run per post — a square
post is a run of `section` metres with a `section`-metre thickness, a level top and a bottom that
follows the ground at its ends — under a mass that `seat` has already levelled. It moves nothing.
`cargo run -p whiting-building --example stilt_probe`, a 10 × 6 m body on a 35 % bank, posts every
1.5 m of 0.2 m section:

| number | value |
| --- | --- |
| the floor's level / terrace fall | **1.7500 m** / **3.5000 m** |
| posts placed / refused | **17 / 0**, and 17 walls in the document |
| the building's underside | **1.4500 m** = the mass's lowest node less one 0.30 m slab |
| the ground under the posts | **−1.7850 .. 1.2850 m** ⇒ **drop 3.2350 m, relief 3.0700 m** |
| the post lengths | **0.2550 .. 3.2550 m** (0.02 of each is the embed into the plate) |
| a post's foot against the analytic ground | **0.00e0 m** |
| the floor's spread after posts | **0.00e0 m** |
| posts uphill of where the ground reaches the underside | **0** (last post at x = 3.4714 of an outline ending at 5.0000) |
| a flat site / a half-answerable ground | **0 posts** / **11 placed, 11 refused** |
| the mesh | **268 faces / 172 points** against 64 / 36; **0 violations**; one post = 12 faces, euler 2, **0.128802 m³** |

The embed exists because of a rendering measurement, not a geometric one: a post whose top is exactly
the plate's bottom puts two faces at one depth, and the scene's first render combed along the plate's
edge. A post is tenoned into the floor; `0.02 m` is that, and it is reported rather than hidden.

## Step 2, done: the scene

`crates/gallery/src/stilt.rs`, buildable as `Gallery::build("stilt")`. It draws the body on its posts
over a bank that is **cut flat where the lane runs** (`x ≤ 4` level, falling at 35 % below it, and a
ridge past `x = 25` so the horizon is not a plane), with the ground drawn whole rather than cut away
under the plate — the one thing `ground::mesh` does by default and the one thing a house standing *over*
its ground cannot have.

It draws **no** 举架 profile, **no** crowned wall top, **no** timber frame, **no** gallery and **no**
tiles, because those are Arcs 2–5 and a scene that faked them would be a picture of a document that
does not exist.

Measured from the `authored_id` image of the frame it produces:

| number | value |
| --- | --- |
| the frame | **1152 × 768** (1.5:1), 884,736 pixels |
| the silhouette | **x 209..=902, y 128..=717** ⇒ **693 × 589 px, aspect 1.177** |
| margins | left 209, right 250, top 128, bottom 51 — the silhouette sits 38 px below the middle |
| the picture's hash on this machine | `64685171182193EC` (sha256 of the file, first 16 upper) — **not pinned** |

Two findings the picture produced, both recorded rather than papered over:

1. **A plate's rim is coplanar with the wall's inner face.** `derive::starter` seats the plate on the
   mass's inner loop on purpose, and the wall's *inner* face stands on that same loop, so a camera
   below the floor sees two coincident planes fighting. The scene draws the plate's two caps and lets
   the wall's inner face close the band; the crate-level alternative — a plate inset by a millimetre —
   would move every pinned hash in the gallery and belongs to a later arc that can re-pin them.
2. **The lens had to widen from `0.52` to `0.56` of `FRAC_PI_3`.** At the narrower lens the id image
   carried building pixels at row 767: the nearest posts' feet are five metres below the eye and
   sixteen away, which is 22° against a 15.6° half-angle.

## What is left before this arc closes

The scene is **buildable and unlisted**, which is the state 15 other scenes in this crate are in
deliberately. Joining the suite is the remaining work, and it is all gates:

- `Gallery::names` — and the `23` / `38` counts its own doc quotes in three places.
- `crates/scene/tests/clustered_lights.rs`: a pass-count row, **measured** (the scene renders in 16
  passes today, which is the pipeline's count and not the test's).
- `audit/full-audit.ps1` **[6/8]**: a **tenth** pinned hash.
- `audit/full-audit.ps1` **[7/8]**: `scene_identity_probe`'s row count, which moves when a scene joins.
- `audit/full-audit.ps1` **[5/8]**: the gallery example count, only if the scene brings an example.

## Gates

- `cargo run -p whiting-building --example stilt_probe` prints every number it claims and asserts the
  ones that must not drift.
- `cargo run -p whiting-device-rasterize --example render -- stilt <stem>` draws the picture.
- `audit/full-audit.ps1` green on `main` after the merge.

## What would make this arc wrong

If a post's top face and the mass's underside produce a non-manifold edge in the combined mesh — they
do not, because the two solids are separate by the two-level rule and the embed keeps their faces
apart — the posts would have to be inset in plan as well, and the reason belongs here rather than in a
comment. If the scene's own render stops matching its hash without a change to the scene, the first
thing to suspect is the light radius: a light radius below about 100 m turned the whole wall black in
one experiment and was reverted, which means this scene's lighting is calibrated to a setting nobody
has explained.
