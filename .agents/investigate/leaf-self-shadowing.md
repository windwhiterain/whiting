# Does a leaf of a vegetation scene cast a shadow on another leaf, and if not, where is that decided?

Measured **2026-10-19**, by reading only. Branch `main` at `f82ee2c` ("Cut a hole in a plate");
nothing in the repository was modified except this note. The Yunnan-pine context was read from the
worktree `C:\resource\whiting\.worktrees\swchina` (branch `feature/southwest-china-trees`, tip
`5f5db28`), which was not touched either.

Paths are absolute under `C:\resource\whiting`. Line numbers are from the revisions named above.

---

## Answer

**Mechanically yes, observably no — and the place it is lost is the bias, not a bucket.**

Nothing excludes foliage. The leaf rows sit in the `shadowing` bucket, the leaf mesh is rasterised
into the shadow page, and the lighting stage runs exactly the same `shadow_term` on a leaf as on a
wall. What cannot happen is that the test *fires*: the receiver is first displaced **one page texel
along its own normal** and then given a further depth bias, and at these scales that displacement is
**about 17 cm** — two orders of magnitude more than a blade's thickness. A caster one millimetre in
front of the shaded blade is inside the push, so the test returns "lit" and no caster anywhere near
the blade can fail it.

There is a second, independent cause that alone produces the measured `interior/rim = 0.99`: the
sky/environment term is added **once, with no shadow lookup at all**. Every leaf in the crown gets
the same unshadowed sky, so halving `sky_radiance` scales interior and rim together — which is
precisely what was observed, and is a statement about the ambient term rather than about the key.

| # | where | what it decides |
| --- | --- | --- |
| **1** | `crates/gallery/src/shaders.rs:1482-1484` (push) and `:1505-1506` (bias) | the one-texel normal push and the `0.0006 + 0.003 * slope` depth bias of the pool path — **leaf-on-leaf shadowing is lost here** |
| 2 | `crates/gallery/src/shaders.rs:2308-2310` | the sky term, added with no shadow, which keeps interior and rim equal by construction |

---

## 1. What the foliage lighting stage samples, and what it biases

### The shader named `plant_lighting`

`plant_lighting` is not a name written into `shaders.rs`; it is assembled. `stage_set("plant", …)`
(`crates/gallery/src/vegetation_picture.rs:1053`) calls
`stage_set` → `mesh_shaders_over` (`crates/gallery/src/building_identity.rs:197-220`), which builds
`<prefix>_<stage>` for each of the four stages (`crates/gallery/src/shaders.rs:2838-2899`), and the
lighting one is then rebuilt by `lighting_shader_declaring` under the name `plant_lighting`
(`crates/gallery/src/building_identity.rs:210-218`). Its body is `lighting_with_normal`, which is
the **same** template every gallery surface is shaded by.

### The sample

`fragment_main` accumulates clustered lights, then global ones, then the directional, then the
ambient (`crates/gallery/src/shaders.rs:2236-2310`). Both light loops call `record_contribution`
(`:2245`, `:2267`), and **one** function serves both (`crates/gallery/src/shaders.rs:2017-2064`):

- `record_contribution` reads the light's record and calls `shadow_term(light.shadow_projection,
  light.page, light.virtual_page, direction, world_position, normal)` — `crates/gallery/src/shaders.rs:2037-2044`;
- `shadow_term` chooses between two shapes by the **address**, not by the kind
  (`crates/gallery/src/shaders.rs:1783-1807`): `address.x < 0` → `shadowed` (the pool page), else
  `shadowed_pages` (virtual pages).

**A `Point` light never reaches the pages path.** `LightRead::virtual_volume` returns `None` for
anything but a `Spot` (`crates/scene/src/standard/emit.rs:557-565`, and its own comment at
`:544-550`: *"Only a spot light comes through here … a point light's grid is a cube of six faces …
neither is a grid this compiler can build yet"*). The vegetation scenes declare `light_with_radius`,
which is a `Point` variant (`crates/gallery/src/kit.rs:151-165`), so the key and fill light are
both shaded through **`shadowed`** at `crates/gallery/src/shaders.rs:1430-1524`.

### The bias and the offset — the two numbers

`shadowed`, in order:

1. **`size`** — `vec2<f32>(textureDimensions(shadow_pool))` (`:1444`). The pool page's own size.
2. **normal push** — the receiver is projected again at
   `world_position + normal * (texel_world * PUSH_TEXELS)` (`:1483-1484`), where
   `texel_world = 2.0 / (max(size.x, size.y) * max(per_unit, 1e-6))` (`:1482`) and
   **`PUSH_TEXELS = 1.0`** (`:1552`, documented at `:1546-1552`: *"a point read a whole texel off
   its own surface is not in the neighbourhood that recorded it"*).
3. **depth bias** — `let slope = 1.0 - abs(dot(normal, to_light)); let z = ndc.z - (0.0006 + 0.003 * slope);`
   (`:1505-1506`), in the page's **normalised [0,1] depth**, not in world units.
4. **kernel** — 3×3 `textureLoad` (`:1508-1523`), taps offset by whole texels after the conversion.

`shadowed_pages` carries the same pair with the bias scaled by the level:
`let bias = (0.0006 + 0.003 * slope) * f32(1u << level);` (`crates/gallery/src/shaders.rs:1742`).

**There is no hardware bias.** The rasteriser builds its depth state with
`bias: wgpu::DepthBiasState::default()` (`crates/device-rasterize/src/wgpu_renderer/raster.rs:253`),
so every bit of the bias is the shader comparison above.

### Can the test ever fail for a caster a millimetre in front? **No — and here is the arithmetic.**

`shadow_extent` is the only scene input that sets the page's size, and for a point light it is
turned into a cube of that half width centred on the scene's bounds centre
(`LightRead::shadow_volume` → `light_projection`, `crates/scene/src/standard/emit.rs:527-539` and
`:4086-4097`), then into `projection_of` (`crates/scene/src/standard/math.rs:237-247`), which passes
`2.0 * largest_half_width * FRAMING_PADDING` to `orthographic` as its **half width**
(`crates/scene/src/standard/math.rs:326-331`; `FRAMING_PADDING = 1.1` at
`crates/scene/src/standard/emit.rs:131`). The pool page is **1024 × 1024** unless the document says
otherwise (`read_light_pool`, `crates/scene/src/lighting/buffers.rs:606-623`) — and neither the
gallery nor the pine scene's `frame` block declares `shadow_page_width`
(`.worktrees/swchina/crates/gallery/src/vegetation_picture.rs:1292-1298`).

For the **Yunnan pine**, `key_extent = 40.0`
(`.worktrees/swchina/crates/gallery/src/southwest.rs:465`):

| quantity | value |
| --- | --- |
| page half width | `2 × 40 × 1.1` = **88 world units**, i.e. 176 m across |
| texels across | 1024 |
| **one texel** | **≈ 0.172 m** (the shader's `texel_world = 2/(1024 · per_unit)` with `per_unit = \|across_world.xy\|/88`, so `0.172 / \|across.xy\|`, i.e. ≥ 0.172 m) |
| page depth range | `far − near = 0.75 · (distance(key→centre) + 80)` ≈ **80 m** |
| push (`PUSH_TEXELS = 1`) | **≈ 0.17 m** along the normal |
| bias at `slope = 0` | `0.0006 × 80 m` = **48 mm** |
| bias at `slope = 1` | `0.0036 × 80 m` = **288 mm** |
| **dead zone along the light ray** | **≈ 0.17 m → 0.29 m** |

The pine's spray card is a **0.45 m** blade of thickness **0.0006 m** (half axis, so 1.2 mm total) —
`.worktrees/swchina/crates/gallery/src/southwest.rs:106-116`, and the argument's meaning at
`crates/vegetation/src/leaf.rs:64-68`. Roughly 20 000 of them on a 16 m tree is a mean
nearest-neighbour spacing of about a centimetre. **The test's dead zone is 17–29 times the spacing
and 140–240 times the card's thickness.** The caster cannot fail the test.

The same arithmetic on `main`'s bamboo, where `KEY_EXTENT = 3.0`
(`crates/gallery/src/vegetation_picture.rs:115`): page half width `2 × 3 × 1.1` = 6.6 m → 13.2 m
across → **12.9 mm/texel**; depth range ≈ 8 m → bias **4.8–29 mm**. Dead zone ≈ 5–29 mm, still an
order of magnitude above a blade.

---

## 2. How the shadow map for a point light is built

### Which instances are drawn into it

- The scene puts **every** instance in three buckets (`crates/gallery/src/vegetation_picture.rs:1139-1141`):
  `(Stage::Shadowing, placed_rows("cast"))`, `(Stage::Depthing, …)`, `(Stage::Lighting, …)`.
- `placed_rows` emits **one row per leaf instance** for every stage, and for `cast` the material is
  the shared `leaf-cast`, whose shader is `plant_shadowing`
  (`crates/gallery/src/vegetation_picture.rs:948-969` and `:845-849`). **The `plant_shadowing`
  bucket therefore does include the instanced leaf rows**, together with the culm, the knots
  (`:877-883`) and the floor.
- The emitter takes **every** instance whose stage is `Shadowing` and groups them by
  `(shader label, mesh label, images)` — the material is *not* in the key
  (`shadow_groups`, `crates/scene/src/standard/emit.rs:4488-4517`; the same rule is stated for the
  ordinary grouping at `crates/gallery/src/kit.rs:453-460`).
- **One shadowing pass per (light that was given a pool page, group)**
  (`crates/scene/src/standard/emit.rs:1003-1033`; the header at `:998-1002`). A light the pool
  turned away is drawn into nothing and is lit unshadowed by design.
  For the pine that is two point lights (`key`, `fill`, both carrying `shadow_extent`) × three
  meshes — so the leaf mesh is rasterised **twice**, once per lamp, into its page.
- The shadowing stage's fragment entry writes white and depth comes from the pipeline
  (`crates/gallery/src/shaders.rs:646-649`); the vertex stage places points through the light's own
  projection (`:634-644`).

### Resolution and extent

- **Resolution**: 1024 × 1024 per page by default (`crates/scene/src/lighting/buffers.rs:606-623`),
  `depth32float`, in a `texture_depth_2d_array` pool of 64 pages by default (`:624-632`).
- **Extent**: a **point** light's page is a box of half width `shadow_extent` centred on the scene
  bounds' centre and reaching `distance + 2 × half_width` deep
  (`crates/scene/src/standard/emit.rs:527-539`, `:4086-4097`, `crates/scene/src/standard/math.rs:237-247`).

### How `shadow_extent` and `radius`/`reach` map onto it

- **`shadow_extent` is the whole of it.** It is the only field `shadow_volume` reads
  (`crates/scene/src/standard/emit.rs:528`), and it fixes both the page's width — hence the texel
  size, hence the push — and the page's depth range, hence the bias in world units.
  The schema's own sentence: `crates/scene/src/standard/scene_schema.rs:961-963`.
- **`radius`/`reach` has nothing to do with the shadow map.** `reach()` reads only `radius`
  (`crates/scene/src/standard/emit.rs:400-406`) and feeds `membership()`
  (`:430-444`), which is cluster culling and the promotion of a light that reaches everywhere to a
  **global** light. A promoted light is still shaded by the same `shadow_term` — the shader says so
  at `crates/gallery/src/shaders.rs:2259-2265`. The pine's `key_reach = 1200.0`
  (`.worktrees/swchina/crates/gallery/src/southwest.rs:464`) therefore buys *no* shadow resolution
  at all; only `key_extent = 40.0` does, and it buys 172 mm texels.

**The mechanism behind the measurement:** `key_extent = 40.0` was chosen so the *floor* is inside
the page (`.worktrees/swchina/crates/gallery/src/southwest.rs:462-465`), and a page fitted to a
16 m tree plus its ground is a page whose texel is 17 cm. The ground gets a correct, feathery shadow
because its casters are metres away; the crown gets none because its casters are centimetres away.

---

## 3. Is there a two-sided / double-sided / alpha-tested special case?

**No, and there does not need to be one — the leaf is already a closed solid with correct
outward normals.**

- **No alpha test, no `discard`.** Grepping every `crates/**/*.rs` for `discard` finds only prose
  (`crates/graphics/src/pipeline.rs:54` — *"Every fragment is discarded"* — about an unwritten
  attachment) and unrelated words. There is no cut-out texture anywhere in the shadow path.
- **No back-face culling**, and that is deliberate rather than accidental: the pipeline is
  `PrimitiveState { topology, ..Default::default() }`
  (`crates/device-rasterize/src/wgpu_renderer/raster.rs:244-247`), whose `cull_mode` defaults to
  `None`, so both faces of every card are rasterised into the page and the nearest one wins.
- **The normal, not the cull, was the two-sidedness problem, and it is closed.** A leaf template is
  a **thin closed solid**: `Section::flat(1.0, thickness)`, capped
  (`crates/vegetation/src/leaf.rs:63-89`), so the back of a card carries its own outward normal and
  needs no `@builtin(front_facing)` flip.

### The decision records

`rg`-equivalent grep over `.agents/proposals/`, `.agents/handoffs/` and `.agents/SPEC*` for
*shadow bias, leaf shadow, two-sided, double-sided, foliage shadow* finds **no `wontfix:D<n>` line at
all** anywhere under `.agents` — so nothing here is a contract that forbids the behaviour.

Three records do bear on it, and **all three require the behaviour rather than excusing it**:

1. **`.agents/proposals/agentic-vegetation-generation.md:1542-1547` — open question 3, "Whether
   leaves cast shadow":**

   > **yes**, and `shadow_density` is the dial, not a bucket exclusion. D7's "outside the
   > `shadowing` bucket" becomes the *default setting* for a distant grove rather than a
   > prohibition, **because a close crown's self-shadowing is most of what makes a canopy read as
   > deep**, and a design that cannot express it has thrown away the thing it was built for.

   This is the decisive line. **Leaf-on-leaf shadowing is sanctioned and expected; its absence is a
   defect, not a sanctioned exception.**

2. **`.agents/proposals/agentic-vegetation-generation.md:726-752` — D18:** *"a leaf template is a
   thin closed solid, so two-sidedness is closure rather than a shading trick."* It quotes the same
   `PrimitiveState` line as its evidence that *"culling was never the problem; the normal is"* — and
   the leaf mesh the vegetation crate builds satisfies it. Confirmed above.

3. **`.agents/handoffs/shadow-texel-push.md`** — the closest existing work. It fixed a **world-unit**
   push of `1.6` in `shadowed_pages` (a *hundred and thirty texels*) and replaced it with one texel
   measured from the light's own projection. Two sentences matter here:

   - `:93-99` — *"**The pool path's 1.6 is untouched.** `shadowed` carries the same constant and the
     same latent fault … It is left alone because `stilllife` — thirty five point lights, all on the
     pool — is the control that says this change did not move the pool, and moving the pool's
     constant moves that control. It is a one-line change of the same shape when the owner wants
     it, and it wants its own measurement rather than this one's."*
     *(The gallery's `shadowed` has since been given the texel-relative push — `shaders.rs:1482-1484`
     — so that particular number is no longer there; the **remaining** one-texel push is what this
     note is about, and the handoff's "wants its own measurement" still stands.)*
   - `:15-26` — the ratio that names the defect: *"a push far larger than the kernel it protects. A
     caster shorter than the push is not between the lamp and the point being read, so it casts
     nothing — not softly, not in the wrong place, not at all."* **This is exactly the pine crown.**
     The handoff is the precedent for the fix and the reason the numbers must be stated, not guessed.

The other three named handoffs are relevant background and change nothing here:
`virtual-shadow-maps.md` (the pages path, `Spot`-only — the pine is a `Point` and cannot use it),
`shadow-level-walk.md` (the level walk inside `shadowed_pages`), `shadow-probes.md` (the probe
harness, `crates/device-rasterize/examples/shadow_probe.rs`, which is the measurement rig a fix
should use).

---

## 4. The minimal change that would make foliage shadow foliage

### The mechanism, and why the magnitude is the hard part

The leaf material is already a per-leaf-group material whose custom data rides the instance row
(`crates/gallery/src/vegetation_picture.rs:861-869`), and *"the instance row is the model matrix, an
optional normal matrix, and **the material's custom data**"*
(`.agents/proposals/agentic-vegetation-generation.md:455-457`). So a **per-material shadow-bias
scale** is expressible without any new pass, bucket or mesh:

- one extra optional `f32` in a **plant-only** custom schema, read as a varying and multiplied into
  both the push (`shaders.rs:1483-1484`) and the bias (`:1505-1506`);
- the leaf materials name it at a small value; every other material leaves it at the default of 1.

**Cost.** One extra float in the plant instance rows (the gallery's own invariant is that an
**absent** optional field costs nothing — `crates/gallery/src/kit.rs:368-370`, from
`gallery_custom_schema_with_image`), one varying, one multiply per shadow tap. No new pass, no new
group, no new shader, no runtime texture.

**Risk to the pinned building pass counts (`crates/scene/tests/clustered_lights.rs`): none, provided
the field goes into a plant-only schema.** That test walks `Gallery::names()`
(`crates/gallery/src/lib.rs:179-207`), which contains **no vegetation scene** — `bamboo`, `grove`,
`sculpted`, `yunnan_pine` are built by `Gallery::build` (`:215-228`) and pinned by nothing. Pass
counts come from grouping by `(stage, shader, mesh, images)` (`emit.rs:4488-4517`), and a
custom-data field is not in that key, so no group splits. The only pinned numbers a change could
move are the **row sizes** at `clustered_lights.rs:79-89` (`shadow/key/shadowing/triangle_instance_data = 76`,
`lighting/lighting/triangle_instance_data = 152`), and those are the `example_standard_scene`'s,
built from `gallery_custom_schema()`. **If the field were added to the shared
`kit::gallery_custom_schema()` instead, every scene's row could grow and those two assertions are
at risk** — `stage_set` is what every building scene calls, and it hard-codes the shared schema at
`crates/gallery/src/building_identity.rs:215`. So the change needs one small step: let `stage_set`
take a schema (or add `stage_set_with_schema`, beside the existing
`lighting_shader_with_schema` / `placement_shaders_with_schema` pair) and have
`plant_scene` pass a plant-only one. The precedent for a per-scene schema variant is
`gallery_custom_schema_with_image` (`crates/gallery/src/kit.rs:371-379`).

### The honest caveat: a bias-only change cannot do it at `shadow_extent = 40`

A multiplier of `1` on the bias still leaves `PUSH_TEXELS = 1.0`, i.e. **17 cm**. Multiplying the
bias alone would therefore change nothing observable. Both halves must scale together, and both must
come down by roughly two orders of magnitude — which means **acne on the leaves themselves**, since a
0.45 m card is only 2.6 texels across on a 172 mm page. That is a resolution problem, not a bias
problem, and no bias number fixes it.

**So the minimal change that actually produces leaf-on-leaf shadow is two numbers in one scene:**

1. **`key_extent: 40.0 → ~8`** (`.worktrees/swchina/crates/gallery/src/southwest.rs:465`) — page
   half width `2 × 8 × 1.1` = 17.6 m, 35.2 m across → **34 mm/texel**, push 34–50 mm, depth range
   `0.75·(27+16)` ≈ 32 m → bias 19–115 mm. Still 2–10× the card spacing, so this alone is not
   enough either — but it is the half that makes the other half affordable.
2. **the per-material bias-and-push scale above, set to ≈0.05 on the leaf materials** — push ≈ 2 mm,
   bias ≈ 1–6 mm. That is under the 1.2 mm card thickness's worth of separation *plus* a margin, and
   it is where leaf-on-leaf shadow begins to appear.

**Cost of (1).** `shadow_extent` is the light's own declaration of how much of the scene it shadows
(`crates/scene/src/standard/emit.rs:527-528`; the rule stated in prose at
`crates/scene/src/standard/emit.rs:4727-4733`). Narrowing it from 40 to 8 takes the ground outside
±8 m of the scene centre out of the key's shadow, which for a whole-tree plate on a large floor is
visible. The pine's `key_extent: 40.0` was written for exactly that reason
(`.worktrees/swchina/crates/gallery/src/southwest.rs:462-465`), so this is a composition decision,
not a free fix. Raising `shadow_page_width` instead is the other half of the same trade and is
cheaper compositionally — 2048² is 1 GiB for a 64-page pool and 4× the rasterisation cost — and
`texture_side_ceiling()` (`crates/scene/src/limits.rs:36`, `crates/scene/src/lighting/buffers.rs:604`)
is the ceiling to check first.

**Recommended order**, cheapest and least risky first:

1. add the plant-only `shadow_bias_scale` and a probe scene, leaving every number as it is — this
   makes the mechanism exist and measures the acne, and it is the step that can be done on `main`
   with no risk to any pinned number;
2. measure the pine at `key_extent` 8 and 12 to find where the crown's shadow survives and the
   ground's does not;
3. only then decide the composition change.

The measurement rig already exists and is named in the handoffs:
`crates/device-rasterize/examples/shadow_probe.rs`.

---

## Sources

| what | where |
| --- | --- |
| lighting template, `fragment_main` | `crates/gallery/src/shaders.rs:2160-2318` |
| pool shadow test, push, bias, kernel | `crates/gallery/src/shaders.rs:1430-1524`; `PUSH_TEXELS` `:1552` |
| pages shadow test, walk, per-level bias | `crates/gallery/src/shaders.rs:1630-1771`; bias `:1742` |
| shape selection | `crates/gallery/src/shaders.rs:1783-1807` |
| the one light body | `crates/gallery/src/shaders.rs:2017-2064` |
| the unshadowed sky term | `crates/gallery/src/shaders.rs:2308-2310` |
| shadowing stage (writes depth only) | `crates/gallery/src/shaders.rs:628-652` |
| no rasteriser depth bias | `crates/device-rasterize/src/wgpu_renderer/raster.rs:244-254` |
| point light's shadow volume | `crates/scene/src/standard/emit.rs:527-539`, `:4086-4097` |
| `virtual_volume` is Spot-only | `crates/scene/src/standard/emit.rs:557-565` |
| `radius` is culling only | `crates/scene/src/standard/emit.rs:400-406`, `:430-444` |
| shadowing pass per (light, group) | `crates/scene/src/standard/emit.rs:997-1033`, `:4488-4517` |
| pool page size, default 1024² | `crates/scene/src/lighting/buffers.rs:598-641` |
| projection geometry | `crates/scene/src/standard/math.rs:237-247`, `:318-347`, `:110-124` |
| leaf is a closed thin solid | `crates/vegetation/src/leaf.rs:63-89` |
| leaves in the shadowing bucket | `crates/gallery/src/vegetation_picture.rs:845-849`, `:948-969`, `:1139-1141` |
| pine light, extent 40 | `.worktrees/swchina/crates/gallery/src/southwest.rs:456-482` |
| pine spray card | `.worktrees/swchina/crates/gallery/src/southwest.rs:88-117` |
| D7 / open question 3 (leaves **do** cast) | `.agents/proposals/agentic-vegetation-generation.md:1542-1547` |
| D18 (two-sidedness is closure) | `.agents/proposals/agentic-vegetation-generation.md:726-752` |
| the push precedent | `.agents/handoffs/shadow-texel-push.md:15-26`, `:91-99` |
| pinned pass counts and row sizes | `crates/scene/tests/clustered_lights.rs:79-89`, `:248-252`; `crates/gallery/src/lib.rs:179-207` |
