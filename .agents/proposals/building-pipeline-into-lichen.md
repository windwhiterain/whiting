# The building pipeline, as four cached lichen stages

Status: **measured**. The generated dylib loads under the language's program gate, the hall program
compiles with no diagnostics, all four stage ops run, and four edits are timed one to a run — and
the unedited program publishes `gallery::hall::hall()`'s own document, fingerprint for fingerprint.
Measured 2026-10-20 on Windows, rustc 1.99.0, on an `i5-12400F`.

Crates: `crates/lichen-stages` (`whiting-lichen-stages`, the four stages beside the oak's six),
`crates/lichen-link` (`whiting-lichen-link`, the generated dylib), `crates/lichen` (`whiting-lichen`,
the program and the probe). The probe's raw output is committed beside the code at
`crates/lichen/hall_measurements.txt`, and the stage timings at
`cargo run --release -p whiting-gallery --example building_cost_probe`.

Companion: [the plant pipeline, as six cached lichen stages](plant-pipeline-into-lichen.md), which
is the shape this one follows and whose measurements are the ones to compare against.

## The question

Can a **building** plate be stage-cached the way the alpine oak's was — a plate whose generation is
four cached cells, whose dials are values in the program's own source, and whose design law stays
where it is?

Yes, and at four seams rather than six. What the numbers then say is the second half of the
answer, and it is not the half the oak's vertical had: **this plate is 14 ms of work, not 12 s**,
so a cache here buys milliseconds. What it buys instead is that the dials stop being Rust.

## The correction that comes first: the census was never the build

The 250–580 ms I attributed to these builds was **`--census` wall clock**. The build is **~14 ms**.
Measured on this machine, release:

| what | ms |
| --- | --- |
| `render --census hall`, warm process, three runs | 565, 548, 533 |
| the same, first run in a fresh process | 1 280 |
| `render` with no arguments — process start, which exits **before** the rasteriser | 80, 82, 77 |
| the census's own work: `Gallery::census` **plus** `Gallery::bounds`, two whole documents | 28.7 |

The code says why. `render.rs:467` builds the rasteriser — `rasterizer().await`, which is
`WgpuRenderer::new()` at `device-rasterize/src/lib.rs:48`, an instance, an adapter and a device —
and `render.rs:494` is where `--census` returns. So of the **548 ms median**, about **440 ms is the
device** and 80 ms is loading a 10 MB binary, both by difference, and the document work is the
28.7 ms the stage probe measures directly. (The brief's figure of ~240 ms for that overhead is
about half of what this machine reads; the direction of the correction is the same and the number is
what it is.)

**This corrects a sentence in an earlier note.**
[`investigate/vegetation-scene-build-cost.md`](../investigate/vegetation-scene-build-cost.md) says
`--census` "returns before any GPU work and so times the document build alone". It returns before
any *drawing*; the device is already built. Its rows survive — divide by two for `Gallery::build`
running twice — and its mechanism sentence does not.

And what the build really costs, timed straight through Whiting rather than through the harness, and
**before** the stages were cut:

| phase | ms |
| --- | --- |
| plan (`hall::document_at()`, skeleton plus the scene's `ops`) | 0.1 |
| build (`Building::build()` → `BuildingMesh`, 228 points, 496 faces, 3 solids) | 0.1 |
| points (`building_mesh::points_of` per solid, 1 488 elements) | 0.7 |
| **document (`hall::hall()` → `definition::Value`)** | **13.3** |

## The four stages

`hall_framed` had the seams in it. They are cut here, in `crates/gallery/src/hall.rs`, and the
three scene entry points reach the same body through them.

| stage | reads | produces | lichen op |
| --- | --- | --- | --- |
| `plan(width, depth, storeys)` | the three dials | the skeleton and the scene's `ops` | `hall_plan` |
| `built(plan)` | the plan id | the expanded solids | `hall_built` |
| `parts(built)` | the built id | the identity table and the four packed point lists | `hall_parts` |
| `document(parts, lens, aim)` | the parts id, two dials | the scene document | `hall_document` |

```
--- hall = link "../../../target/release/whiting_lichen_link.dll" ---
width = 10.0
depth = 16.0
storeys = 1
lens = 1.0
aim_x = 0.0
aim_y = 0.0
aim_z = 0.0
@cache plan_id = hall_plan width depth storeys
@cache built_id = hall_built plan_id
@cache parts_id = hall_parts built_id
@cache scene_id = hall_document parts_id lens aim_x aim_y aim_z
publish scene_id
```

**The presentation dials are the lens and the aim**, and they are a **scale** and an **offset**
rather than the scene's own `f64` constants. A program holds its dials as `f32`, so a dial spelled
as `HALL_FIELD_OF_VIEW` would come back a different number and the baseline program would draw a
picture this scene never drew. `1.0` and a zero aim are `gallery::hall::hall_view`'s own view
**exactly** — a multiplication by one and an addition of zero are the two operations that leave an
`f64` alone — which is the same contract the oak's `envelope` and `key_position` dials keep.

**The ground is in `parts`, not in `document`.** It is cut from the plan and not from the picture,
so leaving it in the document stage would have made every camera edit re-run 117 690 elements of
heightfield it had no reason to touch.

### What each stage costs, measured

`cargo run --release -p whiting-gallery --example building_cost_probe`, one hall at 10 × 16:

| stage | ms | what is in it |
| --- | --- | --- |
| `plan` | 0.2 | the skeleton, three bays a face and one window in each |
| `built` | 0.1 | 228 points, 496 faces, 3 solids |
| `parts` | **11.1** | `Identity::of`, three packed lists (1 488 elements), and the ground (117 690) |
| re-read | 0.7 | what a reused `parts` cell costs the document stage: the clone |
| `document` | 3.5 | 12 top-level fields, the four meshes, the shaders, the instances |
| **the whole plate** | **14.8** | |

The plate is not evenly divided, and the stage that dominates is not a building: **`parts` is 75 %
of the build and 11.1 of its 14.8 ms is the ground heightfield** the scene has always generated.
The building's own geometry — the plan, the expansion, the identity table and the packed lists —
is 0.2 + 0.1 + a fraction of 11.1.

**The cut costs 0.6 ms.** Before it the whole plate was 14.2 ms; after it, 14.8. The difference is
the four `PackedList` clones on the way from `parts` into the document, which is the price of the
seam and is paid on every document build — reused parts or not.

### The scenes did not move

`hall()`, `hall_at()`, and `hall_large()` are byte-identical to what they were. That was measured,
not argued: `crates/gallery/src/hall.rs` as `a11582e^` held it was compiled beside the new one in
one throwaway example — the old file with its `crate::` paths pointed at the package — and the two
documents compared equal for `hall()`, `hall_at(10, 16, 1)`, `hall_at(18, 7, 2)`, `hall_large()` and
`document_at(12, 12, 1)`. Both files are deleted; **a copy of `hall.rs` is not kept in the tree**, and
the check is reproducible from `git show a11582e^:crates/gallery/src/hall.rs`.

The probe also prints the composition itself, on every run:

```
hall()           : fp 1b88435f8c900ab1
baseline is hall : yes
```

which is the four stages composing `gallery::hall::hall()` **through the dylib**, fingerprinted on
the far side of the boundary and compared here.

## The four edits, one per run

`cargo run --release -p whiting-lichen --example hall_probe -- <edit>`. Each run opens one buffer
session, compiles the unedited program, applies one edit, compiles again, then **empties the slab**
and compiles the same source through a brand-new store and session. The fresh figure is therefore a
cold build through the same machinery. Milliseconds, three runs each, the readings identical every
time:

| edit | what moves | baseline | session | fresh (cold) | speed-up | reused / frozen / dropped | equivalent |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `width` | `width = 10.0 → 12.0` | 18 ms | **17.5 ms** | 17 ms | **1.00×** | 0 / 4 / 4 | yes |
| `lens` | `lens = 1.0 → 1.08` | 18 ms | **4.9 ms** | 17 ms | **3.4×** | 3 / 1 / 1 | yes |
| `whitespace` | two spaces after `depth = 16.0` | 18 ms | **0.14 ms** | 17 ms | 121× | 0 / 0 / 0 | yes |
| `same` | the source again | 18 ms | **0.18 ms** | 18 ms | 100× | 0 / 0 / 0 | yes |

| edit | session fingerprint | cold fingerprint | session document id |
| --- | --- | --- | --- |
| `width` | `5e15c4a7140d7771` | `5e15c4a7140d7771` | 7 |
| `lens` | `e4b0d0f50ed419f2` | `e4b0d0f50ed419f2` | 4 |
| `whitespace` | `1b88435f8c900ab1` | `1b88435f8c900ab1` | 3 |
| `same` | `1b88435f8c900ab1` | `1b88435f8c900ab1` | 3 |

What each row is worth saying:

- **`width` is the honest ceiling.** Nothing is upstream of a plan dial, so the cache buys nothing
  and says so: 0 reused, 4 frozen, 4 dropped, and a session that costs the same as a cold compile.
  A width edit re-derives the plate — 14.8 ms of generation — and that is the number to quote for
  "what an edit that reaches the plan actually costs".
- **`lens` is the row the design is for.** Three of four cells are reused: plan, build and parts are
  all skipped, and what is left is the document stage (3.5 ms) plus the session's own work
  (4.9 ms together). The interning is what makes the reuse possible — the lens edit adds **one** entry
  to the slab (document id 4, after the baseline's four), because the parts id the document names
  did not move.
- **`whitespace` and `same` move no cell at all** — not "all cells reused", but zero cell events,
  because nothing was invalidated. The document id is the baseline's own id, 3.
- **The whitespace edit adds trailing spaces rather than a blank line**, deliberately: the session
  keys its cells on spans, so a blank line would move every span below it and would have measured
  the session rather than the edit. The oak's program has the same property and does not depend on
  it.

## The loop, honestly

The generation is **14.8 ms**; the loop an author actually lives in was **6.6 s** —
`cargo build --release` after touching `crates/gallery/src/hall.rs` reads `Finished ... in 6.57s`
on this machine, and every dial a plate is tuned by now lives in a file that rebuild in **nothing**.

That is the whole of what this refactor buys, and it is not a speed-up: **a tuning loop that used to
pay 6.6 s of Rust rebuild now pays 4.9 ms of lichen compile**, and the 13.3 ms document stops being
re-emitted at all when the mesh did not move. In absolute terms the cache saves 12 ms per
presentation edit on a 17 ms compile. Anyone reaching for a "12 s → 2 s" story from the oak's
measurements will not find it here, and should not tell it.

**The plate is 14 ms because the hall is small.** A scene whose `parts` stage carried a forest
would move these numbers the way the oak's moved theirs, and the same four seams would then be
worth the same ratios at two orders of magnitude more work. The seam is what transfers; the
milliseconds are this plate's.

## The design law stays rust

`gallery::hall::document_at` is the scene: it decides three bays a face, calls `ops::split_face`
and `ops::add_opening`, and holds the `HALL` relations that set the walls, the sill and the head.
**None of that is in the lichen program.** The program names four calls and eight dial values; it
does not know what a bay is, that a window is 0.52 of one, or that the rhythm is uniform round the
building. `parts`'s identity table is numbered from one by `building::identity::Identity`, and the
`authored_id` a pixel carries is still the gallery's own.

**Only the pipeline is in the language**, and what the language gained is a place to put a dial.

## What is not cached, and why it is not

- **The ground is the biggest single stage and it is not presentation-dependent**, so it moves with
  the plan. A camera edit reuses it; a plan edit re-runs it, and a plate whose ground were authored
  would buy back most of the 11.1 ms.
- **A stage is atomic to the cache.** There is no partial reuse inside `parts`; a plan edit re-derives
  the identity table and all four lists.
- **The slab only grows and never empties itself**, so a lowering edit leaves the old parts interned
  under their own keys. `slab::reset` exists for the probe's cold compile and nothing else should
  call it.
- **Nothing carries between processes.** The slab is a `static` in the loaded dylib. Interning is
  content-addressed, so a cold process reaches the same answers — which is what the four matching
  fingerprints are — but nothing is carried between runs.

## What a merge would need

1. **A regenerated dylib.** `lichen_language::link::build_tag()` is the crate version plus
   `git describe` of `crates/lichen-language/build.rs`, so the committed `crates/lichen-link/src/lib.rs`
   is stale the moment `lichen-language` is rebuilt, and the gate refuses it **loudly**
   (`the library was built for "…"`) rather than drawing the wrong picture. The durable fix —
   generating the source in the dylib crate's own `build.rs`, so tag and entries cannot be out of
   step — is still not written, and it is what a merge should settle.
2. **A statement that this vertical is not in lichen's CI.** `crates/lichen` and its two siblings
   build here against `../lichen-vm`, so a change to the `link` ABI is caught when someone next runs
   the probe rather than on a pull request. The plant note prices this and the price has not moved.
3. **`gallery` gains four public items** — `HallParts`, `parts`, `hall_view` and
   `hall_document_over_parts` — and with them an invariant worth keeping: **nothing in `HallParts` is
   a function of a dial of presentation.** A scene that puts a lens-dependent list in there would
   make every camera edit re-run the whole parts stage, and the 3.4× row would quietly become 1.0×.
4. **The ground's move into `parts` is a decision, not a refactor detail.** It is right because the
   ground is cut from the plan; a future scene whose ground *is* a dial has to say so loudly.
5. **Both measurement files** (`crates/lichen/measurements.txt`, `crates/lichen/hall_measurements.txt`)
   are raw probe output and are kept on purpose — they are the note's evidence — but a merge should
   say whether artefacts belong in the tree.
6. **No merge has been made.** The branch is `feature/building-cache`, off `main`, in the worktree
   `C:\resource\whiting-building-cache`.
