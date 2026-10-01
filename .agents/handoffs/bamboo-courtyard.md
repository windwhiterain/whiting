# A bamboo courtyard: a Chinese mountain village, in six frames

Worktree: `./worktrees/dsf`, branch `feature/dsf-bamboo-courtyard`, on top of `main` at
`232a861`.

The deliverable is **six rendered images** of one place — a bamboo courtyard in a Chinese
mountain village — judged on how they look. Nothing in the repository changes meaning: the
framework is used as it stands, and where it is changed it is changed additively.

```text
cargo run --release -p whiting-device --features wgpu --example render -- courtyard-dawn  out/final/courtyard-1-dawn-mist     0
cargo run --release -p whiting-device --features wgpu --example render -- courtyard-cloud out/final/courtyard-2-clouded-noon  0
cargo run --release -p whiting-device --features wgpu --example render -- courtyard-rain  out/final/courtyard-3-rain        0
cargo run --release -p whiting-device --features wgpu --example render -- courtyard-dusk  out/final/courtyard-4-dusk        0
cargo run --release -p whiting-device --features wgpu --example render -- courtyard-night out/final/courtyard-5-night-lamp  0
cargo run --release -p whiting-device --features wgpu --example render -- courtyard-snow  out/final/courtyard-6-snow        0
```

`--census` prints what a scene holds instead of drawing it — every list's count, every pass
bucket, the lights, the camera and the post stage's own constants. It is a diagnostic and not
part of the framework.

## What was built

One place, six moments. `crates/gallery/src/courtyard.rs` builds the place — a terraced
hillside, a rammed-earth farmhouse with a curved tile roof, a bamboo grove, a paved yard, a
stone pond, a clay-tiled wall with a gate, and the air around all of it — and each moment is
that place under a different sky, sun and lens. The six are separate gallery scenes because
the harness renders one `time` per document, and a moment is a fact about the light.

| moment | the frame |
| --- | --- |
| `courtyard-dawn` | mist in the low ground, the sun behind the ridge, the grove a silhouette |
| `courtyard-cloud` | a bright broken sky, the yard in the sun, the wall's shaded side cold |
| `courtyard-rain` | no sun at all, wet stone, and a fog that is one flat wash |
| `courtyard-dusk` | the sun off the yard and the cloud lit from beneath |
| `courtyard-night` | a moon over the roof, one lamp lit on the bank |
| `courtyard-snow` | the place under a low white sky, at a long lens |

## Geometry added

Five generators in `crates/gallery/src/geometry.rs`, because a bamboo grove, a grass bank and
a courtyard floor are not expressible from a sphere, a cone and a cuboid:

| generator | what it is |
| --- | --- |
| `culm(nodes, radius, height)` | one bamboo stem, with the ringed joint at every node |
| `blade(length, width, curve)` | one leaf or grass blade, a one-sided ribbon |
| `double_blade(length, width, curve)` | the same, two-sided, for a leaf seen from behind |
| `leaf_spray(blades, length, width, droop)` | the ring of leaves a twig carries |
| `tuft(blades, height, spread)` | a tuft of grass rising from one root |
| `masked_heightfield(...)` | a heightfield that carries a **surface mask** in a normal's free component |

## What the framework had to gain

Three changes, all additive, all in `crates/gallery`:

- **`attributes::gallery_vertex_inputs_with`** — a location is a position in the merge of
  `instance_schema` and `custom_schema`, so a scene whose materials carry a field of their own
  moves every field that sorts after it. The gallery's list was written from the gallery's own
  schema and could not see a scene's.
- **`shaders::lighting_shader_with_schema`, `emission_shader_with_schema`,
  `placement_shaders_with_schema`** — the four stages over a custom schema the scene supplies.
  One shader per material would be one pass per material, and this scene has a grove.
- **`Gallery::census` and `Gallery::bounds`, and `--census` in the harness** — the count of
  every list and bucket, and the box every placement occupies. A scene author places things by
  arithmetic and then has to know where they ended up; the alternative to this is rendering a
  frame and guessing from the picture.

## What was measured, and what it cost

Rendered on an RTX 3060 through `wgpu`, 1536 × 1024.

| moment | passes | compile | render |
| --- | --- | --- | --- |
| `courtyard-dawn` | 87 | 257 ms | 386 ms |
| `courtyard-cloud` | 72 | 213 ms | 324 ms |
| `courtyard-rain` | 72 | 213 ms | 345 ms |
| `courtyard-dusk` | 87 | 211 ms | 344 ms |
| `courtyard-night` | 87 | 227 ms | 365 ms |
| `courtyard-snow` | 72 | 203 ms | 329 ms |

The place is 21 meshes, 2525 materials, 1152 instances in each of the depth and shading
buckets and 220 in the shadow map. The grove is two meshes and roughly seven hundred
placements: 205 culms, 636 leaf sprays and 410 single leaves.

## State

- [x] geometry generators
- [x] the place: terrain, water, house, wall, gate, paving, props
- [x] the grove: clumps, culms, sprays, leaves
- [x] the air: sky, ridge backdrop, height fog, grade — one per moment
- [x] six moments registered, rendered and measured
- [x] `cargo test --workspace` green; the gallery pass-count table carries all sixteen scenes

## What is not finished, stated rather than left to be found

- **Every frame is high key.** The exposure was set against a first version that blew out, and
  it is now a little under what the place wants: the sunlit plaster and the night wall both sit
  near the top of the range. A pass over `Tone::exposure` per moment against a histogram would
  be the first thing to do next.
- **The bamboo leaves are geometry, not billboards.** They read as flat ribbons at four units
  and as a mass at twenty. The fix is a leaf with a normal that varies across it — a fold, or a
  twist — and the mesh is one function.
- **There is no shadow of the grove's own leaves.** A leaf is single-sided and does not cast,
  so the yard's dappling is the culms' shadows alone.
- **The snow is a value and not a substance.** It lies on whatever faces up rather than
  accumulating in the hollows, so a ridge of the roof and a flat of it catch the same white.

