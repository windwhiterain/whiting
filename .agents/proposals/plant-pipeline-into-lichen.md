# The plant pipeline, as six cached lichen stages

Status: **measured**. The generated dylib loads under the language's program
gate, the program compiles with no diagnostics, all six stage ops run, and five
edits are timed one to a run. Measured 2026-02-13 on Windows, rustc 1.99.0.

Crates: `crates/lichen-stages` (`whiting-lichen-stages`), `crates/lichen-link`
(`whiting-lichen-link`, the generated dylib), `crates/lichen` (`whiting-lichen`,
the program and the probe). The probe's raw output is committed beside the code
at `crates/lichen/measurements.txt`.

Companion: `docs/notes/whiting-scene-document.md`, and on the lichen side
`docs/notes/native-link-library.md` §11, §15 and §17.

## The question

[Where a vegetation plate spends its build](../investigate/where-a-vegetation-plate-spends-its-build.md)
records an `alpine_oak` document build at 8.5 s in release and calls it "the
whole build". A plate has six natural stages — grow, dress, expand wood, expand
leaves, draw, assemble — and an author turning one dial of one stage should not
pay for the other five. **Can those six stages be cache cells, with every dial a
value in the program's own source?**

Yes, and the stages do not have to be written twice. The answer is a lichen
program whose six `@cache` bindings call six `extern "C"` entries in a generated
`dylib`, reached through the language's `link` directive.

## Why the arrow points this way

**lichen is the runtime; this repository is the application.** A lichen-vm
checkout must build, test and run its CI with no `Whiting` beside it, so no
lichen-vm manifest may name a crate here. The dependency is stated once, in this
workspace's `[workspace.dependencies]`, as a path into `../lichen-vm/crates/…`,
and it points **out** of this repository only.

That direction is what the vertical needs rather than merely permits: a `dylib`
must be built by the same rustc from the same lichen sources as the toolchain
that loads it, and the manifest's build and program tags refuse anything else.
The dependency here is the only thing that can make those tags true, and putting
it on this side is what lets a lichen-vm `dev` stay clean. The cost is that the
vertical is **not** part of lichen's CI: `crates/lichen` and its two siblings
build here, against a sibling checkout, and a lichen-vm change that breaks the
`link` ABI is caught when someone next runs the probe, not on a pull request.

## The shape

```
--- oak = link "../../../target/release/whiting_lichen_link.dll" ---
seed = 34
envelope = 0.0
stations = 8
around = 24
spacing = 0.45
density = 0.0
key_x = 0.0
key_y = 0.0
key_z = 0.0
@cache tree_id = tree seed
@cache dressed_id = dressed tree_id envelope
@cache wood_id = wood dressed_id stations around spacing
@cache leaves_id = leaves dressed_id density
@cache drawn_id = drawn wood_id leaves_id 0
@cache scene_id = document drawn_id key_x key_y key_z
publish scene_id
```

Every dial above is a lichen value, and every one of them is read by the stage
that names it. The stages are not native ops compiled into the language: they
are one `extern "C"` entry per family in a generated `dylib`, reached through the
`link` mechanism's `External` operator.

| stage | reads | produces |
| --- | --- | --- |
| `tree` | `seed` | the grown skeleton |
| `dressed` | `tree`, `envelope` | the fluted bole under the crown's guard |
| `wood` | `dressed`, `stations`, `around`, `spacing` | one closed solid per node |
| `leaves` | `dressed`, `density` | one group per leafed node |
| `drawn` | `wood`, `leaves`, `index` | the expansion, the ids, the decorators |
| `document` | `drawn`, key-light position | the scene document |

### One constraint decides it

The language's value trait is `Debug + Copy + PartialEq`, so a lichen value
cannot hold an `Arc`, a `Box`, or any handle at all. A `Plant` is megabytes of
curves and a `Drawn` holds a whole expansion, so **neither can be a lichen
value**. The boundary carries a `usize` and a process-global slab inside the
dylib carries the object.

That makes **interning a correctness requirement, not an optimisation**. A
stage's id is the only name the next stage is given, so the next stage's key is a
function of it; an id that moved between two compiles would move every downstream
key, every `@cache` cell would miss, and nothing downstream of the first
recompute would ever be reused. The key is `(stage tag, parent ids, parameter
bits)` and `slab::intern` answers the same id for the same key for the slab's
whole life.

The measurements confirm this rather than assuming it: the five fingerprints
below are stable across two independently constructed slab generations, and each
session's document id lands on the exact next free slot after its edit (6, 8, 8,
10 and 11, for edits that add 1, 3, 3, 5 and 6 entries to a six-entry slab).

## The five edits, one per run

`cargo run --release -p whiting-lichen --example incremental_probe -- <edit>`.
Each run opens one buffer session, compiles the unedited program, applies one
edit, compiles again, then **empties the slab** and compiles the same source
through a brand-new store and session. The fresh figure is therefore a cold
build through the same machinery, and the two document ids are deliberately
different numbers, because the slab was emptied between them. What must match is
the document's structural fingerprint.

| edit | what moves | baseline | session | fresh (cold) | speed-up | reused / frozen / dropped | equivalent |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `lighting` | `key_x = 0.0 → 7.0` | 12.068 s | **1.972 s** | 12.005 s | **6.09×** | 5 / 1 / 1 | yes |
| `detail` | `stations = 8 → 9` | 12.081 s | **3.880 s** | 12.227 s | **3.15×** | 3 / 3 / 3 | yes |
| `leaves` | `density = 0.0 → 18.0` | 12.069 s | **4.441 s** | 12.908 s | **2.91×** | 3 / 3 / 3 | yes |
| `envelope` | `envelope = 0.0 → 0.7` | 12.026 s | **10.963 s** | 11.749 s | **1.07×** | 1 / 5 / 5 | yes |
| `seed` | `seed = 34 → 35` | 12.075 s | **9.825 s** | 9.836 s | **1.00×** | 0 / 6 / 6 | yes |

| edit | session fingerprint | cold fingerprint |
| --- | --- | --- |
| `lighting` | `cf82f9f2fcee03c2` | `cf82f9f2fcee03c2` |
| `detail` | `17b47bde3b28fa6b` | `17b47bde3b28fa6b` |
| `leaves` | `bde4d5f12a67a844` | `bde4d5f12a67a844` |
| `envelope` | `2d52c39a234b8a81` | `2d52c39a234b8a81` |
| `seed` | `5beedf1789fd4241` | `5beedf1789fd4241` |

The fingerprints are reproducible across a rebuild: an earlier run of the same
five edits, before the generated crate was made `rustfmt`-stable, produced the
same five values in both columns and the same six session ids. The document a
plant scene produces is a function of its dials and of nothing else.

### What these say against the earlier cost note

[Where a vegetation plate spends its build](../investigate/where-a-vegetation-plate-spends-its-build.md)
reports `alpine_oak_picture()` at **8.5 s** in release on an `i5-12400F`, and a
prediction made from it expected this vertical at "8.5 s → ~2.0 s" for a
presentation edit, "about 2.5×" for a geometry dial, and 1.0× for the rest.

The document half matches exactly: this vertical's `document` stage alone is
**1.97 s** on the lighting row, against that note's ~2 030 ms for assembly. The
cold figure does not: **12.0 s**, not 8.5 s, and the whole difference is in the
geometry half. Two causes are on the table and they are not separated here.

- **The machines differ.** Every constant in that note is wall clock on a
  6-core `i5-12400F` with Defender resident; these are wall clock on whatever
  ran this. That note itself warns its absolute constants may carry a co-tenant
  tax its readings cannot separate.
- **The stage split duplicates work the single path did once.** `oak_fruit`
  scatters seats over the whole plant, and it is called in both `drawn` and
  `document`; `alpine_oak_species` is built four times across `drawn`, `document`
  and the two `oak_fruit` calls. `alpine_oak_picture()` pays for one of each.
  The duplicate scatter is the obvious suspect for the 3.5 s, and it has not
  been separately measured — the fix is to seat the planting once and carry the
  seats in the slab, which would not change the document and so would not move
  a fingerprint.

The prediction is therefore **right about what is incremental and wrong about
what it costs**, and its ratios survive the correction: 6.09× where it expected
about 4.3×, 3.15× and 2.91× where it expected about 2.5×, and 1.07× and 1.00×
where it expected 1.0×.

## What the design gives up

- **No persistence across processes.** The slab is a `static` inside the loaded
  dylib. A second process starts empty and re-runs every stage. That is sound —
  interning is content-addressed, so a cold process reaches the same answers,
  which is what the five matching fingerprints show — but nothing is carried
  between runs. A cell cache would change this and is not wired here.
- **The slab only grows.** An edit that lowers `stations` leaves the old wood
  interned under its own key forever, because a key some other cell may still
  name cannot be dropped. `slab::reset()` exists so a probe can measure a cold
  build, and nothing else should call it: it invalidates every id already handed
  out.
- **A stage is atomic to the cache.** `wood` at `stations: 9` is one key, so the
  cell that depends on it is rebuilt whole. There is no partial reuse inside a
  stage and there cannot be, because the stage's output is a `Vec<Wood>` the
  next stage names as a whole. That is the detail row's 3.15× and the leaves
  row's 2.91×: each re-runs one stage and everything downstream, and nothing else.
- **Two processes are two worlds.** An id means nothing outside the process that
  interned it, so a host may compare ids only within one process and must
  compare fingerprints across two.

## How a program value is read back

A `SessionReport` carries its build behind an `Arc` the session's own cache also
holds, and the module has no `&self` value reader, so a program's value cannot be
read out of a report at all. The program therefore ends with an explicit
`publish scene_id`, and the dylib's `whiting_publish` entry records the argument
it was handed. The tap reads **what the program actually evaluated**, on every
compile including a fully reused one — it is a call and not a "last interned"
slot, so it cannot go stale.

## What was reachable without changing anything here

Everything `drawn` and `document` need is already `pub`, and **nothing was
missing**. `gallery::vegetation_picture::{Drawn, draw, plant_scene_drawn, View,
View::PHOTOGRAPH, Planting, PLANTING_STRIDE}`,
`gallery::vegetation_mesh::{LEAF_BASE_ID, node_ids_from}`,
`vegetation::decorator::placed`, and `gallery::southwest_oak::{alpine_oak_tree,
alpine_oak_dress, alpine_oak_species, crown_envelope, oak_lighting, oak_palette,
oak_fruit, OAK_DETAIL, OAK_FRAME, ALPINE_OAK, SHOOTS}` — so `drawn` reproduces
`draw`'s four fields exactly, and no accessor had to be opened to get there.

Two dials needed a reachable spelling rather than the constant this repository
holds:

- **`envelope`**: `Envelope::radius` is a `pub Scalar`, so the dial is
  `crown_envelope().radius.offset(metres)`. `Scalar::offset` keeps the law's own
  knots and slopes, so `0.0` is exactly `crown_envelope()` — the committed
  composition is the baseline, and the dial is the one number added to it.
- **`density`**: `LeafField::density` is a `pub Scalar` and `Scalar` has no
  scale, so the dial is an **offset** on each field's own density law, keeping
  every knot and every slope. `0.0` is this repository's own leafing.

Both are offsets because `Scalar`'s public surface is `constant`, `new`,
`with_slopes`, `at`, `slope`, `offset` and `plus` — there is no public knot
reader, so a scaled law would have been an approximation and a duplicate of the
numbers here at the same time.

## Two findings from wiring it

- **A multi-argument `$` native op is curried in the wrapper, not
  parenthesised.** `dressed = (tree : Int, envelope : Float) => …` does not
  parse: the language writes a parameter's type in the parameter position and one
  parameter at a time, so the wrapper is
  `dressed = tree: Int => envelope: Float => { … }`. The `$` call itself is
  ordinary — `$whiting_dressed(tree, envelope)` — and the entry receives the whole
  argument array either way.
- **The language's lexer reads no negative float literal.** `key_x = -6.0` is a
  parse error (`found '-'` at the statement start), so the envelope and lighting
  dials are measured as positive offsets. Worth recording on its own: a negative
  number is not a refusal an author can see coming.

## Building and running

```
cargo run -p whiting-lichen-stages --example generate_link   # writes crates/lichen-link
cargo build --release -p whiting-lichen-link                 # target/release/whiting_lichen_link.dll
cargo run --release -p whiting-lichen --example incremental_probe -- lighting
```

The generator writes `crates/lichen-link/{Cargo.toml,src/lib.rs}` from
`lichen_stages::library::source()`. Both are **committed**, so the dylib builds
without running the generator first; the generator is what to re-run after a
change to the wrapper, the bridge or an entry body.

`render` emits `#[rustfmt::skip]` on every entry, so regenerating and then
running `cargo fmt --all` is a fixed point — verified by generating, formatting
and generating again, and comparing. Without it the committed generated file and
the generator's output would disagree on formatting alone.
