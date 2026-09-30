# Handoff: striking 3D scenes

Branch `feature/showcase-scenes`, worktree `.worktrees/showcase`. Started from `main` at `2cf5121`.

## Where this is

All four tiers of [proposals/striking-3d-scenes.md](../proposals/striking-3d-scenes.md) are built and
two scenes render on a real adapter. The analysis of the system is recorded in
[SPEC.align](../SPEC.align.md) under *What the standard pipeline gained, and what it got wrong*.

**A frame was 99% compile**, and profiling it rather than reasoning about it found that the cost was
not the diff at all. Three changes took a frame from 3.7 s to 0.37 s, and made the cost *flat* over a
sequence rather than growing. The measurements, what was fixed and what is still open are in
[proposals/compile-cost.md](../proposals/compile-cost.md) — **read that before optimising anything
here.** What is left is the structural one: reuse the nodes a compile read last time, by identity,
rather than rebuilding the document every time.

## How to run it

```text
cargo run --release -p whiting-device --features wgpu --example render -- <scene> <out> [times] [mode]

  render dunes out/dunes                    one frame, at the time the scene carries
  render dunes out/dunes 0 5 0.25           twenty-one frames, a quarter second apart
  render dunes out/dunes 0 5 0.25 --sheet   the same range as one contact sheet
  render dunes out/dunes 0 5 0.25 --video   the same range as an mp4, encoded with ffmpeg
```

`--release` matters: the compile is the bottleneck and a debug build is three times it. The harness
reports the compile and the render apart, which is the measurement the compile-cost proposal rests
on. `ffmpeg` must be on the path for `--video`; the frames are written either way.

Scenes: `dunes`, `monoliths`, `studio` (the shipped example).

## What is done

- **Tier 1** — perspective projection, a `camera` field on the scene, `light_projection` and
  `light_shadow_texel` on the lighting uniform, and a nine-tap shadow with a normal offset.
- **Tier 2** — `time` on every pass uniform, read from the scene; the harness patches it across a
  range and encodes the result.
- **Tier 3** — `blend`, `depth` and `samples` as fields of a rasterize pass, defaulting to exactly
  what the backend always did.
- **Tier 4** — a post stage driven by a `ScreenShader`, carrying the camera basis so it can draw a
  sky, bloom or a grade.
- **Tier 5** — `crates/scene/src/gallery/`, holding the WGSL library, the geometry and the two scenes.
- **Shader and parse caches in the renderer.** A module is keyed by its text, so two passes naming
  the same shader share one and a frame that changes no shader pays for no compilation. The
  uniform layout check parses through the same cache, which the code had flagged as a known cost.
- **Three compile fixes**, all sound and all in the framework rather than in a scene:
  points are verified per mesh against each *distinct* shader point schema rather than once per
  instance that draws the mesh; `pack_points` sizes its buffer from the schema's declared widths
  rather than growing to it; and `find_reference` stops at a schema that constrains nothing,
  because a reference is declared and an unconstrained slot has no declaration to mark.

## What is known-incomplete

Stated rather than left to be discovered:

- **`dunes` is tuned, not finished.** The shadow map is 1024² over a 380-unit volume, and the
  penumbra is wider than it should be. The horizon has a bright seam where the field's far edge
  meets the sky, and there is faint shadow acne at the far edge. The dune field is also
  divided into 96 cells, which is coarse — see the note on `DIVISIONS` for why.
- **`monoliths` is a sketch.** The composition is a row of small stones on a plain; the camera
  is framed so the ground, which reaches much further than the monoliths, decides the fit, so
  the crop is a blunt instrument. The wet-ground reflection is a Fresnel stand-in, not a
  reflection pass, and it is why the ground is drawn as geometry rather than a clear colour.
- **The instance layout is not checked by the device.** Only the uniform is, against the parsed
  shader. A shader that reads its instance row at the wrong locations gets no error, and the
  gallery's shaders write the locations out in `shaders.rs` for exactly that reason. This is
  the same gap the uniform check was built to close, one level down.
- **No textures.** A mesh carries texcoords and a material carries a `tint`, but nothing samples
  an image, because a binding carries a texture and no sampler.

## Invariants to keep

- A gallery scene is an ordinary standard scene document: the same `Value`, the same schema, the
  same compiler. It is not a second kind of scene.
- A shader carries its own source, so a scene stays self-contained and renders wherever it is
  sent.
- The uniform struct of a shader is written in the order its interface schema sorts its fields
  into, and the device checks it against the parsed shader and will say so. It caught two real
  mistakes here: a stage declaring a uniform its pass does not supply, and a struct written in
  the wrong order.
- A surface body is written into the body of the lighting stage's fragment function, so it
  cannot redeclare a name the template or another surface already uses. A scene's helper
  functions go in the `helpers` argument, because `WGSL` has no nested functions.
