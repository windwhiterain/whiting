# The level a caster is recorded at, and the walk that finds it

Two defects in the virtual shadow map, both about a caster's declared density failing to decide
the precision of its own shadow. The texel-push defect in the pages path was fixed in `cde9baa`;
this handoff covers what is left.

## The settled rule

`vshadow.rs` restricts a caster to pages at its own level and every coarser one
(`for level in own..levels`). That half of the invariant is **kept**. The other half — *a receiver
that finds nothing at a finer level keeps going down the walk* — was not implemented, and the two
halves are only true together.

A page records depth, so a page at level `k` can only hold what `k` can express. A caster that
asked for level 2 cannot be recorded into a level-0 page; the honest placement is its own level and
coarser, and the receiver has to go looking for it there.

## The three defects

1. **The walk stops at the first page that exists.** `shaders.rs::shadowed_pages` returns
   `lit / 9.0` at the first level whose mask bit is set, whether or not that page recorded
   anything at this point. A level-0 page opened by a fine neighbour is *empty* where a coarse
   caster's shadow falls, because the coarse caster is not drawn into it, so the walk returns lit
   and the coarse caster's shadow is lost. `vshadow.rs:41-44` forbids exactly this ("a scene that
   renders and quietly records its shadows at a fraction of the resolution it asked for").

   The fix: a level that records nothing — all nine taps at the clear value — does not answer.
   The walk continues. It is sound because the atlas layer is cleared to 1.0 by the first pass
   that writes it and loaded thereafter (`device/src/wgpu_renderer/raster.rs:10-11`, keyed per
   layer at `:15-18`; `DEPTH_CLEAR` at `scene/src/standard/emit.rs:4996`), and a page's geometry is
   clipped to its own rectangle by viewport and scissor together (`raster.rs:19-22`).

2. **`caster_level` decides by ratio rather than by resolution.** It answers with
   `2^k <= max_density / density`, which measures against the density the light was *asked* for.
   The grid is sized by rounding `plane_width * max_density` up to a whole power-of-two number of
   pages (`grid_side_for`), so the texel the grid actually has is **finer** than `1 / max_density`.
   The ratio test is therefore conservative: a caster can be left at a level finer than the one
   its own declaration pays for. The constraint to evaluate is the real one,
   `plane_width * 2^k / (pages_per_side * PAGE_SIZE) <= 1 / density`, which is exact and can put
   such a caster **one level coarser** — never coarser than it asked for.

3. **The pool path pushes a surface 1.6 world units off itself** (`shaders.rs::shadowed`), which
   on the old pool is 126 to 166 texels. `cde9baa` replaced that constant in the pages path with
   one texel of the recording measured from the light's own projection; the pool path has the same
   bug and the same fix.

## What is stale in the tree

- `.agents/proposals/virtual-shadow-maps.md:83` says a caster's pages are allocated at its own
  level "**and at every finer level down to 0**". That is the opposite of the code, and it was
  never implemented. It is the rule this handoff settles against.
- `crates/device/examples/shadow_probe.rs:245-250` explains the level result by claiming a page is
  drawn with *every* caster that covers it. `vshadow.rs:1030` is the opposite: a caster never
  reaches a page finer than its own.

## Files this work owns

- `crates/scene/src/vshadow.rs` — `caster_level`'s signature and constraint, the module invariant's
  second half, and the call in `allocate`.
- `crates/gallery/src/shaders.rs` — `shadowed_pages`'s walk and its documentation;
  `shadowed`'s push.
- `crates/device/examples/shadow_probe.rs` — the probe that can see defect 1, and the stale
  comment.
- `.agents/proposals/virtual-shadow-maps.md`, `.agents/SPEC.align.md` — the settled rule.
- `crates/scene/tests/virtual_shadow_maps.rs` — only the `caster_level` call, whose arguments
  change with the signature. No new test.

## What must not change

No scene in the tree declares `shadow_density`, so every caster in `spotlight`, `stilllife` and
`monoliths` sits at the scene default, which is the light's maximum density and therefore level
zero under both the old and the new constraint. Their frames are expected to be byte identical, and
`scene/tests/clustered_lights.rs`'s pass counts are expected to hold. The density ladder is the
only place the numbers are supposed to move, and it moves because the probe asks for it.
