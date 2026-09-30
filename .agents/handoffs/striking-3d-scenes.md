# Handoff: striking 3D scenes

Branch `feature/showcase-scenes`, worktree `.worktrees/showcase`. Started from `main` at `2cf5121`.

## Where this is

The question was to build visually striking 3D scenes with this system, and to improve the system or
propose improvements where it gets in the way. The analysis is in
[proposals/striking-3d-scenes.md](../proposals/striking-3d-scenes.md). The plan it proposes has **not**
been approved yet, and nothing in tiers 1 to 5 is implemented.

## What is done

**The authoring loop works end to end on a real adapter.** `crates/device/examples/render.rs` builds
a named scene, compiles it with `StandardScene`, renders it with `WgpuRenderer` and writes a png:

```text
cargo run -p whiting-device --features wgpu --example render -- studio target/baseline.png
```

It renders the shipped example scene at 1024x1024 in about a second on an RTX 3060. The output is
two flat triangles on black, which is the evidence for the proposal: see its section on the camera,
the projection and the shadows.

**A scene gallery is started.** `crates/scene/src/gallery/`:

- `kit.rs` — one builder per field of the scene schema, so a scene reads as a scene.
- `geometry.rs` — `MeshBuilder`, the vertex generators, and the vector helpers.
- `shaders.rs` — the WGSL library, with a Cook-Torrance lighting stage.
- `mod.rs` — `Gallery::build`, currently dispatching only `studio`.

**`crates/standard/mod.rs` re-exports the scene schema's field-name constants.** They were private
to the module; a builder outside the crate needs them.

## What is known-incomplete in the gallery

Stated rather than left to be discovered:

- `shaders.rs` writes a Cook-Torrance lighting stage that is **not yet exercised by any scene** —
  `Gallery::build` still returns the shipped `studio` scene, which uses the example's own shaders.
  The instance row locations in that WGSL (`emissive` at 3, `metallic` at 4, the model matrix at 5
  to 8, `roughness` at 9, `tint` at 10) are written against a row schema of
  `{emissive, metallic, model, roughness, tint}` and **must be checked against
  `vertex_layout` when a scene first uses them** — unlike the uniform, that layout is not verified by
  the device.
- `geometry.rs` has unused helpers (`normalized`, `dot`, `cross`, the two point schemas) and the
  crate warns about them.
- The lighting stage in `shaders.rs` has no shadow term, for the reason given in the proposal.

## Invariants to keep

- A gallery scene is an ordinary standard scene document. It is not a new kind of scene, and it goes
  through the same schema and the same compiler.
- A shader carries its own source, so a scene stays self-contained and renders wherever it is sent.
- The uniform struct of a shader is written in the order its interface schema sorts its fields into.
  The device checks this against the parsed shader and will say so.
