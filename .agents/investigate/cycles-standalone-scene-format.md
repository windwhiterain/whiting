# Cycles Standalone: what its XML scene format actually accepts

Recorded 2026-10-07. Question answered: can our `StandardScene` be emitted into the
Cycles Standalone XML format, and what does the emitter have to know?

**Provenance.** Every claim below was read from source, not from documentation,
because there is no published specification. Files read at
`github.com/blender/cycles`, branch `main`, commit
`a456b761034dda42c32eef9f4aae0fa5a5c9f604` (2026-09-08):

- `src/app/cycles_standalone.cpp`
- `src/app/cycles_xml.cpp`
- `src/scene/film.h`, `src/scene/integrator.h`, `src/scene/shader_nodes.h`
- `examples/scene_monkey.xml`, `examples/scene_cube_surface.xml`

The format has no version field and no schema. It is whatever the loader in
`cycles_xml.cpp` happens to accept on the day you run it. Any adoption must pin
the commit, and the pin must be recorded next to the emitter.

## The format is a reflection over the node registry, not a fixed document

This is the single most important structural fact, and it is the opposite of what
"undocumented format" suggests.

`xml_read_node()` does not parse a schema. It walks the XML element's attributes
and assigns each one to the matching registered socket on the target node:

```cpp
snode = graph->create_node(node_type);
xml_read_node(graph_reader, snode, node);
```

So the set of legal attributes for `<film>`, `<integrator>`, `<camera>`, `<object>`,
`<light>` and every shader node is exactly the set of `NODE_SOCKET_API` members on
the corresponding C++ class. The effective schema is `src/scene/film.h`,
`src/scene/integrator.h` and `src/scene/shader_nodes.h`, which are readable,
stable, and far better specified than any prose document would be.

Two consequences worth stating plainly:

- An unknown attribute is a logged error, not a crash. The loader keeps going.
  A typo therefore produces a plausible-looking render that is quietly wrong,
  which is the same failure mode as any silent config mistake and the reason the
  emitter needs a round-trip test rather than a smoke test.
- There is no negotiation and no versioning. When upstream renames a socket, our
  emitter breaks silently at the next pinned-commit bump. This is the real cost of
  the format, and it is a maintenance cost, not a design cost.

## Element dispatch

The root element is `<cycles>`. `xml_read_scene()` dispatches on each child:

| Element | Effect |
| --- | --- |
| `film` | Sets attributes on `scene->film` |
| `integrator` | Sets attributes on `scene->integrator` |
| `camera` | `width`/`height` plus socket attributes; takes the enclosing transform |
| `shader` | Creates a `Shader`, then reads the child elements as a node graph |
| `background` | World. Sets `scene->background` attributes, then the graph drives `default_background` |
| `mesh` | Geometry. Requires the enclosing `state` for its material |
| `light` | Requires `light_type` |
| `transform` | Pushes a transform scope, recurses |
| `state` | Pushes shader/object/interpolation scope, recurses |
| `object` | Creates an empty mesh object, recurses so children attach to it |
| `include` | Loads another file relative to the current one |

`transform` and `state` are the scoping primitives. Both copy the reader state,
mutate one field, and recurse into their children, so they nest arbitrarily. This
is a lexical scope, and it maps directly onto our `Instance`.

`<object>` exists so several `<mesh>` elements can share one transform as a single
object. We do not need it: each `Instance` gets its own `<transform>` scope.

## Mesh encoding

```
<mesh P="x y z  x y z  x y z" verts="0 1 2" nverts="3" />
```

- `P` — flat `float3` array, space separated.
- `verts` — flat index array, consumed `nverts[i]` at a time.
- `nverts` — per-polygon vertex count.

Polygons are **fan triangulated** by the loader, so any polygon is legal and
degenerates to a plain triangle when `nverts` is all threes. `UV` is accepted under
either the short name or the standard name. Vertex normals, UV tangents and tangent
signs are read under `Attribute::standard_name(...)`.

`interpolation` is not a mesh attribute. It is scoped state, set by `<state
interpolation="smooth|flat">`, and applied to every mesh inside.

`subdivision="catmull-clark|linear"` switches the mesh into the subdivision path,
which takes a different code branch. We do not want it.

## Materials and lights

`light_type` accepts `point`, `sun`, `background`, `area` and `spot`. Lights are
created invisible to camera rays
(`PATH_RAY_VISIBILITY_ALL & ~PATH_RAY_VISIBILITY_CAMERA`) and take the enclosing
transform.

**`light_type="point"` is a sphere light, not a delta light.** An earlier revision of this
file said Cycles' point light has no radius and that our `Point` therefore had to become a
sphere light by conversion. That is wrong. `PointLight` declares

```
SOCKET_FLOAT(radius, "Radius", 0.0f);
SOCKET_BOOLEAN(is_sphere, "Is Sphere", true);
```

and `PointLight::copy_to_kernel` sets `is_sphere = is_sphere && radius != 0.0f`. A point
light with a radius is a genuine sphere source: omnidirectional, and soft-shadowed.
Measured on 2026-10-08: two point lights of equal `strength`, one at `radius="0"` and one
at `radius="1.2"`, cast shadows whose penumbra visibly differs. Our `Point` maps onto
Cycles' `point` directly. See the table below.

### A light that is not given an emitting shader emits nothing

This is the one fact about the format that is not visible anywhere in the format, and the
failure is silent. `Light::has_contribution` (`src/scene/light.cpp`) ends with

```
const Shader *effective_shader = (get_shader()) ? get_shader() : scene->default_light;
return !is_zero(effective_shader->emission_estimate);
```

`get_shader()` returns the light's first used shader, which `xml_read_light` fills from
`state.shader` — the shader of the enclosing `<state>`. A `<light>` written bare sits in
whatever state the document is in, which at the top level is `scene->default_surface`, a
diffuse closure whose emission estimate is zero. The light is created, is in the scene, is
not culled, and contributes nothing.

Every punctual light therefore needs its own `<emission>` shader and a `<state>` naming it:

```xml
<shader name="key_emission">
  <emission name="key_source" color="1, 0.9, 0.8" strength="120" />
  <connect from="key_source emission" to="output surface" />
</shader>
<state shader="key_emission">
  <transform translate="2, 2, -2">
    <light name="key" light_type="point" strength="1, 0.9, 0.8" />
  </transform>
</state>
```

Measured: the same light with and without the wrapper goes from a black frame to a lit one,
with no other change and no error in the log either way.

### The sockets are not named what they look like

There is no `energy` and no `color` on any light. A light is `strength`, and `strength` is a
**colour**, not a number. An attribute the node does not declare is ignored without a word,
so a light written with `energy` and `color` renders happily at the default strength of
white. Per kind, on top of the base `Light`:

| kind    | its own sockets                          |
|---------|------------------------------------------|
| point   | `radius`, `is_sphere`                    |
| spot    | `radius`, `is_sphere`, `angle`, `smooth` |
| sun     | `angle`                                  |
| area    | `sizeu`, `sizev`, `ellipse`, `spread`    |

The base `Light` declares `light_type`, `strength`, `cast_shadow`, `use_mis`,
`use_caustics`, `max_bounces`, `is_enabled` and `normalize`. A spot's `angle` is the **whole
cone** — the kernel takes `cosf(angle * 0.5)` — where our `outer_angle` is a half-angle in
radians.

Our `radius` is deliberately **not** mapped. It means how far a light reaches for cluster
culling, not how big its source is, and a reach of sixty units would become a light a
hundred and twenty units across. Cycles' zero radius gives a delta light and hard shadows,
which is the honest rendering of a document that says nothing about source size.

A shader graph is named nodes plus explicit connections:

```
<shader name="mat">
  <image_texture name="tex" filename="albedo.exr" />
  <principled_bsdf name="bsdf" base_color="0.8, 0.8, 0.8" />
  <connect from="tex color" to="bsdf base_color" />
  <connect from="bsdf bsdf" to="output surface" />
</shader>
```

`output` is pre-bound to the graph's output node, so connections may target it
directly. Socket names in `from`/`to` are matched case-insensitively.

`image_texture` and `environment_texture` resolve `filename` against the directory
of the scene file (`state.base`), and `<include>` rebases that directory per file.
Our emitter controls the layout, so we should emit absolute paths or paths relative
to a directory we control, and never depend on the include mechanism.

## Mapping `StandardScene` to the format

| Ours | Cycles XML | Status |
| --- | --- | --- |
| `Camera` | `<camera width height/>` inside `<transform>` | Direct |
| `Light::Directional` | `<light light_type="sun"/>` | Direct, see below |
| `Light::Spot` | `<light light_type="spot"/>` | Direct |
| `Light::Point` | `<light light_type="point"/>` | Mechanically direct, **physically wrong** |
| `Light::Environment` | `<background>` with `environment_texture` | Direct |
| `Mesh` + `Topology::Triangles` | `<mesh P verts nverts/>`, all `nverts` = 3 | Direct |
| `Topology::Segments` | none | **No representation** |
| `Instance` | `<transform>` scope | Direct |
| `MeshMaterial` + `Shader` | `<shader name>` + `<state shader>` | Direct, via `principled_bsdf` |
| `Texture` | `image_texture` inside the graph | Direct |

Three of these need a decision, not just code.

**Point lights stay physically wrong.** Cycles' point light is a true delta light,
so its shadow is infinitely sharp and it produces no soft falloff. Our `Point`
light is a clustered sphere light with a radius. Emitting it as a Cycles point
light yields an image that looks correct everywhere except in the penumbra, and
penumbrae are exactly what a lighting comparison is measuring. The fix is
`light_type="area"` sized to the same solid angle, which is a scene-space decision
the emitter must make from a radius the format does not carry.

**Segments have no representation.** `xml_read_scene()` has no curve element and
`xml_read_mesh()` only builds triangles. If anything in our content is currently a
`Topology::Segments`, it has to be converted to geometry on our side before export.
Nothing in the current content profile needs this; the gap is recorded so that it
is not rediscovered later.

**A mesh carries exactly one material.** `xml_read_mesh()` hardcodes
`const int shader = 0` and pushes only `state.shader` into the mesh's used shader
list. There is no per-face material index in this format. An object that needs
several materials must be split into several `Instance`s, each with its own
material and its own copy of the geometry. Our `Instance` already binds one
material, so this matches our model, but the emitter must reject rather than
silently flatten a multi-material mesh.

## Settings that matter for comparison

`Film` exposes `exposure` as a plain `float` socket. **Cycles exposure is a linear
multiplier, not a stop count. Measured 2026-10-07, not inferred.**

A scene with no geometry, no lights and a constant world at
`strength="1.0" color="0.5, 0.5, 0.5"` was rendered three times through
`<film exposure="…">` and the resulting PNG bytes were read back:

| `exposure` | Predicted linear value | If a stop count | Actual 8-bit sRGB |
| --- | --- | --- | --- |
| `0.5` | 0.25 | 149 (half a stop) | **64** |
| `1.0` | 0.5 | 187 | **128** |
| `2.0` | 1.0 | 255 | **255** |

The `0.5` row decides it. Half a stop would be a factor of `0.707`, encoding to
149. The measured 64 is exactly `0.25` linear. It is a multiplier.

Note that `1.0` producing 128 rather than 187 is itself the sRGB transfer
function, not an error: it confirms the encode path is intact and that the value
underneath was linear 0.5.

**This settles the Cycles side. The open question is now ours:** whether
`crates/scene/src/standard/exposure.rs` is in stops or a multiplier. If ours is
in stops, the emitter must apply `2^stops` before writing the attribute. That
file has still not been read, and it is now the single highest-value unverified
item in the integration.

`Integrator` exposes everything a controlled comparison needs, and all of it is
reachable as XML attributes: `seed`, `use_denoise`, `denoise_start_sample`,
`denoiser_type`, `use_light_tree`, `use_adaptive_sampling`, `adaptive_threshold`,
`adaptive_min_samples`, `max_bounce` and the per-channel bounce limits. A
`<integrator>` element is accepted at the top level of the scene and was used
without complaint in the tests above.

For a reference comparison: set `seed` explicitly, set `use_denoise` false, fix
`--samples`, and keep the sample counts identical on both sides. Denoising is a
nonlinear filter over neighbouring pixels and will dominate an RMSE comparison if
it is left on one side and off the other.

## What the build and these tests actually established

A Cycles Standalone binary was built on this machine and run, so the statements
above are now partly observed rather than read. See
[cycles-standalone-windows-build.md](cycles-standalone-windows-build.md) for the
build. The rendered results:

- **`principled_bsdf` works, and `metallic` genuinely varies the shading.** Two
  quads sharing one node type, differing only in parameters, rendered as a matte
  red (`metallic=0, roughness=0.35`) and a dark specular gold
  (`metallic=1, roughness=0.15`). This is the material path we want, and it
  removes the earlier suggestion that a hand-assembled Lambert-plus-GGGX subset
  was needed.
- **Mixed polygon sizes triangulate.** A scene containing two `nverts="4"`
  quads and one `nverts="3"` triangle rendered all three correctly, confirming
  the fan triangulation of the loader.
- **A sun light with no rotation illuminates along its local +Z.** Observed as
  two front-facing quads lit evenly, so the expected `-Z` emission convention
  holds in the default orientation and needs no correction at the origin.
- **OpenColorIO is compiled in but disabled at runtime** unless `$OCIO` is set.
  The binary prints "Color management disabled" and then warns
  `Colorspace '' not found, using scene linear instead`. This is good for us:
  output is scene-linear by default, which is what a reference comparison needs.
  Set `$OCIO` only if a display transform is ever wanted.

## A correction: a misspelled `light_type` does report an error

This corrects a claim made earlier in this file. Reading the loader, the
`light_type` dispatch in `cycles_xml.cpp` ends in an unguarded `else` containing
`assert(light_type == "spot")` followed by `create_node<SpotLight>()`. Since
Release builds define `NDEBUG`, the assert compiles out, so the source suggests a
misspelled value would silently become a spot light with no diagnostic.

**It does not.** Rendering a scene with `light_type="pointt"` produced, on the
first line of output:

```
ERROR: Unknown value "pointt" for attribute "light_type"
```

The enum is validated on the generic reflection path before the dispatch chain is
reached, so the reflection layer catches what the dispatch chain would not.

The failure mode is still worth guarding against, but for a different reason than
originally stated: **the error is reported and the render continues**, exit code
zero, image written. The process will not fail on our behalf. An emitter must
still validate its own output, and a scheduler must treat `ERROR` in the child's
stderr as a failure signal rather than relying on the exit code alone.

## What is still unverified

- **The official `examples/scene_caustics.xml` renders black on this build, and the cause
  is not known.** It has a point light wrapped in `<state>` around an `<emission>` shader,
  which is the arrangement confirmed to work, so the light is not the explanation. Its
  camera carries `full_width`/`full_height`, which `xml_read_camera` does not read — it
  reads `width`/`height` — so its resolution attributes are silently ignored; correcting
  those did not make it render. Removing its mirrored `scale="1 1 -1"` camera scale did not
  either. Unresolved: the examples in the repository should not be used as an A/B reference
  for anything but element nesting, and this one should be reported upstream.
- **`crates/scene/src/standard/exposure.rs` has not been read.** The Cycles side
  is now measured and is a multiplier; ours is unknown. See above.
- **`Attribute::standard_name` literals.** The loader calls it at runtime for
  vertex normals, UV tangents and tangent signs. We use the short `UV` alias,
  which is confirmed, so this only matters if we need authored normals.
- **Texture path resolution.** `image_texture` and `environment_texture` resolve
  `filename` against the scene file's directory. Confirmed by reading the loader,
  not by loading a texture.
- **Multi-material rejection.** The single-material constraint was read from
  source (`const int shader = 0`) but not tested by feeding it a violating scene.
- **Spot cones have not been rendered.** The half-angle doubling and the `smooth` band are
  read from source (`cosf(angle * 0.5)`) but never exercised by a frame.
- **Our light brightness now has the unit Cycles expects.** It did not: ours was a
  brightness multiplier and Cycles' `strength` is radiant flux. A `Point` or `Spot` now
  states a `power`, a `Directional` an `irradiance` and an `Environment` a `radiance`, and
  the exporter passes each straight through. See
  `.agents/proposals/physical-light-units.md`.
- **The pinned commit was never bumped**, so none of the drift discussed above
  has been observed. The maintenance cost remains theoretical.

## What the emitter established by rendering

Measured on 2026-10-08 with the emitter in `.worktrees/cycles-emitter`, CPU, Cycles 5.3.0
at the pinned commit.

- `<transform>` **and** `<state>` are both containers. `xml_read_scene` copies the read
  state, applies the attributes to the copy, and recurses into the children with it, for
  each of them alike. Geometry is a child of the state, which is a child of the transform.
  Every wrong nesting renders successfully, logs nothing, and writes an empty frame, so
  "no errors in the log" is not evidence that a scene is right. This shape was wrong four
  times during the work before the source was read.
- An `<instance>`'s `<transform>` scope works end to end. A translation, a rotation and a
  scale on one instance moved, turned and resized its mesh as written, alongside a second
  instance with an identity transform written without the scope at all.
- `principled_bsdf` is present and its parameters bite. Two quads differing only in
  `metallic` and `roughness` came out as a matte surface and a specular one.
- A light emits only when wrapped in a `<state>` naming an emitting shader. See above.
- `atof` tolerates trailing commas, so `translate="0, 0, -6"` parses the same as the
  space-separated form the examples use. `xml_read_float3` splits on whitespace and calls
  `atof` per token.

## What this changes

Rejecting USD was the right call, and the format turns out to be a better fit than
the "no published specification" framing suggested. The schema is the node
registry, the registry is readable source, and the element set is about a dozen
tags. The remaining work is a round-trip emitter plus three decisions, not a
reverse-engineering project.

The honest cost is maintenance: an unpinned, schemaless format that reports a
renamed socket by logging and rendering a wrong image anyway.
