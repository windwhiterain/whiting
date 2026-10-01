# Handoff: instances bucketed by pass, and the emission stage

Branch: `migrate/pass-instances`, worktree `.worktrees/pass-instances`.

## Why

SPEC.md was rewritten. Two earlier readings were abandoned: a `MeshMaterial` naming four
stage shaders, and a `meshes` map keyed by pass. The spec that settled is:

```text
meshes: Vec<Mesh>
instances: HashMap<Pass, Vec<Instance>>
  transform: Transform
  mesh: Mesh
  material: MeshMaterial
mesh_materials: Vec<MeshMaterial>
  shader: MeshShader
  custom_data
```

So an instance names **one** material, whose one shader has one stage, and the set of passes
an instance is drawn in is the **buckets it is listed under**. The duplication that the old
document carried implicitly, by letting a material's shader decide the stage, is now explicit
and grouped: one bucket per pass.

## Decisions taken on review

- **The bucket key is the stage suffix** — `shadowing`, `depthing`, `lighting`, `emission`.
  `post` is not a key: a post process draws a fullscreen triangle rather than a scene
  instance, so it has no instances to bucket.
- **The material's shader stage must equal the bucket's pass.** This is a new check the
  bucket makes possible: `instances.lighting[x].material` naming a `DepthShader` used to be
  unrepresentable and is now a compile error naming both.
- **`post_processes` names entries in `screen_materials` by label** rather than inlining
  them, for the same reason `meshes` and `mesh_materials` are declared once and referred to
  by label. The spec writes the type inline; this is the one place the referencing design is
  ours to choose.
- **`emissive` stays in `gallery_custom_schema()`.** The spec's `MeshMaterial` carries one
  `custom_data` and `Struct` is closed, so every stage of a material must accept the same
  dict. Moving emission into a schema of its own would need a second dict on the material.
  The emission shader reads `emissive`; the lighting template stops adding it.

## Steps

Each step ends with all four gallery scenes rendering, and the numbers are unchanged from
`main` unless the step is meant to change them.

1. **The document shape.** `mesh_materials` as a list, `Instance.material` as a label,
   `instances` as a map of pass to list, and the emitter reading buckets and checking the
   stage against the bucket. The gallery and the example build the map. **Pixel-identical.**
2. **The renames.** `ShadowSchema`, `DepthSchema`, `LightSchema`, and the `LightShader` tag.
   **Pixel-identical.**
3. **The emission stage.** `Stage::Emission`, its tag and suffix, an `emission()` template, a
   kit builder, and the emitter's pass loop. `color = color + in.emissive` leaves the
   lighting template. **Pixel-identical**, because every gallery scene passes `[0.0; 3]` and
   names no emission shader.
4. **A gallery scene that emits**, because the feature is unverifiable until something does.
5. **`cameras: Vec<Camera>`,** rendering every camera.
6. **`screen_materials` and `post_processes`.**
7. **Align `SPEC.align.md`** — the per-stage-material decision in it is stale and has to go —
   and delete this handoff.

## Invariants to keep

- A gallery scene stays an ordinary standard scene document: the same `Value`, the same
  schema, the same compiler.
- `crates/device/src/wgpu_renderer/shader_interface.rs` compares the bindings and vertex
  locations a shader declares against what the pass built, so a step that changes a shader's
  attribute list is reported as an error rather than as a wrong frame.
