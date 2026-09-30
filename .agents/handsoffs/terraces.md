# Handsoff: the `terraces` gallery scene

Branch: `k3`, worktree `.worktrees/k3`.

## Goal

A realistic 3D landscape for the gallery: **Chinese mountain countryside fields** —
flooded rice terraces mirroring a morning sky, a village among China firs, valley mist,
and ridge lines fading into haze. Judged on technical difficulty and aesthetics; framework
changes are judged on whether they would be accepted upstream.

## Framework changes (each its own commit)

1. `gallery::shaders::post` gains a `helpers` argument, inserted at module level beside
   `ray_through`, mirroring `lighting(surface, helpers)`. Call sites (`dunes`,
   `monoliths`) pass `""`. Needed because a sky with ridges, clouds and height fog is not
   expressible inside a single fragment body — WGSL has no nested functions.
2. `gallery::geometry` gains `cuboid` and `prism` generators (village walls and gable
   roofs), following the same invariants as `cone`/`sphere`.
3. `Camera::projection` (perspective path) fits the near/far planes to the corners' view
   depths instead of `distance ± half_diagonal`. The diagonal overstates the depth span by
   an order of magnitude for a wide flat scene, and `depth32float` precision near one is
   what z-fighting is made of. Framing is unchanged; only the z mapping moves.
4. `SHADOW_MAP_WIDTH/HEIGHT` 1024 → 2048. Terrace risers are two to three units; a 0.37
   unit texel cannot hold their shadows. The align note's "penumbra wider than it should
   be" is this.

## Scene design

- One terrain mesh (heightfield, extent ±190, ~220 divisions). Height = valley
  cross-section + back mountain mass, quantized into contour-following steps where a
  terrace mask says so (smooth riser profile, C0, so the heightfield's finite-difference
  normals stay true). Valley floor gets a coarse quantization: the patchwork fields.
- Surface hook (terrain lighting shader): paddy flats (`normal.y` high, inside the
  terrace band, below the treeline) are water — Fresnel reflection of a procedural sky,
  ripple-perturbed, wind lanes; risers are earth and straw; steeper is forest, then rock.
  Valley floor is a patchwork of crop colours keyed by cell hash.
- A second lighting shader for props (village, firs) with plain PBR and the same haze.
- Village: ~9 houses (cuboid walls, prism roofs) seated on the terrain height function,
  deterministic via `hash01`. Firs: two stacked cones per tree, one mesh pair, instanced.
- Post: procedural sky (gradient + sun disk/glow + stratus), two far ridge silhouette
  layers from 1D noise over ray azimuth, and analytic exponential height fog using the
  world position recovered from `depth` and the projection matrix
  (`view_distance = m[3][2] / (depth + m[2][2])`). Fog scatters toward the sun.
- The sun direction is one Rust const formatted into both the light and the WGSL, so the
  water's reflected sun and the post stage's sun cannot drift apart.

## Status

- [x] Worktree and plan
- [ ] Smoke render (dunes) on this machine
- [ ] Framework changes 1–4
- [ ] Scene builds and renders
- [ ] Aesthetic tuning pass
- [ ] Final frames + contact sheet, docs synced, commits, handsoff deleted
