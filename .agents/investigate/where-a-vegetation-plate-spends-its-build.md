# Where a vegetation plate spends its build

**Question answered:** an `alpine_oak` plate's document build is reported at ~47 s, which is
absurd for one tree — so **which phase is it, and is 47 s the number an author actually pays?**

**Measured:** 2026-10-19, `main` at `812599a`, on a 6-core `i5-12400F` with Windows Defender
resident. Both build profiles are reported, because the two differ by a factor of five and the
harness runs the debug one by default.

**Sources:** the probe `crates/gallery/examples/vegetation_cost_probe.rs` (new, prints and
asserts nothing), `cargo run -p whiting-device-rasterize --example render -- <scene> <stem>
--census` for the document's own counts, and temporary per-phase census prints inside
`vegetation::species::grow`, `vegetation::envelope::trim` and `gallery::southwest_oak::alpine_oak`
— all three reverted after the readings, so nothing in the tree carries them.

## The two clocks, and the first correction

| build | `alpine_oak_picture()` |
| --- | --- |
| debug (`target/debug`) | **48.5 s** |
| release | **8.5 s** |

**The 40-plus seconds is a debug-build number.** The ratio is 5.7. Every figure below is the
release one unless it says otherwise, because 8.5 s is what an author pays for a `--release`
render and 48.5 s is what they pay for the default one.

**Every millisecond in this note is wall clock on one 6-core `i5-12400F` with Windows Defender
resident and other sessions possibly compiling**, so an absolute constant may carry a co-tenant
tax the readings cannot separate. The **shares** are the reading; the constants are not, and
two runs of the same arm disagreed by up to 3 % (`expand` 2 377 and 2 388 ms).

## The decomposition

`alpine_oak_picture()` builds the species, expands it, and then assembles the document. The
four phases below are additive: the picture re-derives the species and the expansion itself,
so `species + expand + assembly` is the whole build.

| phase | release ms | share | what it produced |
| --- | --- | --- | --- |
| `species` — `alpine_oak(34)` | **4 139** | 48 % | 10 273 live nodes |
| — of which `grow` (the rule's skeleton) | 804 | 9 % | 10 273 nodes, 3 generations |
| — of which `trim` (the crown envelope) | **3 357** | **39 %** | 98 length nudges, 525 pressed |
| — of which templates, fields, decorator | 1 | — | 2 blade templates, 5 fields, 1 ornament |
| `expand` — the plant's geometry | 2 377 | 28 % | 10 273 solids, **1 972 848 wood vertices**, 10 632 leaf groups, 54 576 blades |
| — of which `wood` alone | 1 389 | 16 % | |
| — of which `leaves` alone | 1 403 | 16 % | |
| document assembly (the rest of `picture`) | ~2 030 | 24 % | 15 585 materials, 59 521 placements × 3 stages, 9 shaders |
| **picture** | **8 547** | | 12 top-level fields |

Two things are worth reading off this table before the one below it.

- **The tree's skeleton is not the cost.** `grow` is 804 ms of 8 547, and within it the shape
  laws (`limb`, `secondary`, `shoot`) are 770 ms — 10 273 knots-to-curve calls at 75 µs each.
  That is the procedural generation proper and it is not what is slow.
- **`trim` is the largest single term in the whole plate**, at 39 %, and it produces no
  geometry at all: 98 length nudges and a list of 525 nodes it could not save.

## Inside `trim`, which is the finding

`envelope::trim` runs two passes over every live non-root node, with one `Derived` table per
pass and the trunk's bracketing scan hoisted out of the loop.

| | release ms | what it is |
| --- | --- | --- |
| pass 1 | **2 277** | of 10 272 candidates |
| — `spine_in` derivation, all 10 272 nodes | 999 | every node's placed spine, once |
| — the envelope tip test, 10 272 calls | 850 | `envelope.outside_of(trunk, tip)` |
| — `placement_in` for the 1 519 outside | 0 | already in the table |
| — bisection, 1 519 nodes × 32 probes | 304 | ~6 µs per `outside_of` |
| `apply` — the 98 nudges | **0.1** | an edit table, applied once |
| pass 2 — every node's tip, re-derived | **1 051** | 525 pressed |

So the shape of the cost is: **every node's placed spine is derived twice** (999 + 1 051 ms),
and the envelope is asked for **~58 900 point-to-trunk queries** (850 + 304 ms). Those three
terms are 3.2 of the 3.36 s. The edit itself — the thing the pass exists to produce — is
one tenth of a millisecond.

The first tip test costs 83 µs per call and the bisection probes cost 6 µs, which says the
first `outside_of` on a point is dearer than the thirty-second on the point beside it; this
note does not attribute that difference, and it is the one number in the table that a reader
should treat as unexplained rather than as a property of the query.

## What this corrects

[vegetation-scene-build-cost](vegetation-scene-build-cost.md) attributes the same plate's cost
to two things that are no longer in the code it cites:

- **"A plant field is charged once per leaf instance, per ancestor"** and **"`Plant::surface`
  is called once per leaf placement and re-solves the whole ancestor chain"**. `expand`,
  `wood`, `leaf`, `decorator`, `thin` and `trim` all thread one `plant::Derived` table now
  (`expand.rs:25`, `leaf.rs:36`, `envelope.rs:386-390`), and `Plant::surface` is used only by
  the edit ops (`ops.rs:480`). The note's `plant.rs:263` and `:240` now name `walk()` and
  `get_mut()`.
- **Its `Field::light` rows measure a field this species does not carry.** `alpine_oak_rule`
  declares `fields: Vec::new()`, with a comment recording the same measurement the older note
  does — 3.3 ms per leaf instance — as the reason the tropism was removed rather than fixed.

Its **numbers** are not contradicted: the same plate measured 47.3 s per build in this probe's
debug run, against its 200 s "whole-tree plate as delivered" on a denser build of the same
species in a feature worktree. What is superseded is the mechanism, not the clock.

## How the plate compares with the rest of the gallery

`--census` calls `Gallery::build` twice, so these are the wall clock halved. **The two columns
are the two profiles of the same tree**, and the release column is the `target/release`
harness that was already in the tree rather than a rebuild.

| scene | debug | release | ratio |
| --- | --- | --- | --- |
| `tiles` | 0.27 s | — | |
| `monoliths` | 0.32 s | — | |
| `bamboo` | 0.33 s | — | |
| `dunes` | 0.54 s | — | |
| `yikeyin` | 0.67 s | — | |
| `terraces` | 5.0 s | **1.6 s** | 3.1 |
| `yunnan_pine` | 5.8 s | — | |
| `landscape` | 8.3 s | **1.0 s** | 8.2 |
| `alpine_oak` | 47.3 s | **9.6 s** | 4.9 |
| `site_montane` | 83.3 s | **18.6 s** | 4.5 |

Two readings, and the first is the one an author can act on today:

- **The harness defaults to the debug profile, and that is most of the wait.** `alpine_oak` is
  47.3 s in `target/debug` and 9.6 s in `target/release`, which is a 4.9× difference for
  changing which binary is run. The ratio is not one number across the gallery — 3.1 on
  `terraces`, 8.2 on `landscape` — so it is not a constant anyone can divide out.
- **The vegetation plates are the slow half of the gallery by one and a half orders of
  magnitude**, and the two tables above say the slow half is slow in `trim` and in the sheer
  size of what it then expands (10 273 solids, 1.97 M vertices) rather than in the tree's
  growth.

## What is not answered

- **Whether the trim can be one pass.** Both passes read a spine for every node; the second
  exists only to report what the linear length model could not save (525 pressed). Whether the
  press can be predicted from the first pass's own numbers — the node's origin and its
  direction — is a question about `Envelope`, not about this measurement.
- **Why the first `outside_of` call costs 83 µs and the probes cost 6 µs.** Every reading in
  this note is a wall clock sum; nothing here sampled a stack, so a hot sub-call inside
  `Nearest::closest` would not have been seen.
- **Where the ~2.0 s of document assembly goes.** It was measured as a remainder rather than
  as a phase: 15 585 materials and 178 563 placements are built there, and none of them was
  timed separately.
- **Whether a field on this species is still 3.3 ms per leaf instance.** Nothing here tested a
  field; the only field claim in this note is the one `alpine_oak_rule` already records.
