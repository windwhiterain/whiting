# Cycles Backend

`cycles.exe` as the offline `Backend`. emits a Cycles Standalone XML document from a `Scene`
document, runs one subprocess, decodes the image it wrote.

## Boundary

Cycles cannot be linked. it is a program that reads a file. so the whole integration is an
emitter producing one XML file and a caller running one command, and nothing else.

the emitter is a pure function of its input and spawns nothing, so a scene document can be turned
into a target-format document with no renderer present. the scheduling — locating the binary,
building the command line, deciding whether what came back is a render or a failure — is a
separate crate that knows about the format and the emitter does not.

## Terms

### Pinned Format

the document format the emitter was written against, which is decided by the loader's source at
the commit the binary was built from:

```text
blender/cycles @ a456b761034dda42c32eef9f4aae0fa5a5c9f604  (5.3.0)
```

bumping it means re-reading `src/app/cycles_xml.cpp`, because the loader changes with it. there
is no version field and no schema in the format itself; the pin is ours.

### `Resolution` comes from the command line

the loader reads `width` and `height` off `<camera>` and applies them to the **camera's** view
plane. the **buffer** comes from `--width` and `--height`. measured: a document whose camera says
`1920, 1080` writes a `1024, 512` png.

there is nowhere in the document to put it. the `Film` node declares `exposure`, `filter_type`,
`mist_*`, `display_pass` and the cryptomatte sockets, and no resolution socket at all. so the
size is read out of the document by the emitter — through the same reader that wrote it, so the
two cannot drift — and passed to the process.

### a light travels along its own `-Z`

the kernel reads a light's basis and negates it:

```text
const float3 dir = -transform_get_column(&object->get_tfm(), 2);
```

an identity transform's third column is `+Z`, so a light with no rotation travels along `-Z`. a
rotation carrying `+Z` onto a scene's stated `direction` aims the light **exactly backwards**,
and nothing says so: the scene loads, the light is built, no ray reaches a surface, the render
returns the world's own background with no error and a full sample count.

this is **not** the camera's convention. the camera's axis was established separately, by moving
it and watching what came into frame, and the two differ in the sign of the pitch.

### a light emits nothing without a shader

`Light::has_contribution` returns false when the shader it resolves has a zero emission estimate,
and a `<light>` written bare resolves against `scene->default_surface` — a diffuse closure that
emits nothing. so the light is created, is in the scene, is not culled, and contributes nothing.

every punctual light therefore carries an `<emission>` shader of its own inside a `<state>` that
names it. measured: the same light with and without the wrapper went from black to lit with no
other change.

### `strength` is a colour and carries everything

there is no `energy` and no `color` socket on any light. a light is `strength`, and `strength` is
a **colour**, not a number. the kernel multiplies it into the shader's own emission, so the
emission shader is white at unit strength and `strength` is the whole of the light in one
multiplication.

an attribute the node does not declare is ignored without a word, so a light written with the
wrong socket name renders perfectly happily at the default.

per-kind sockets, which are ignored entirely when absent:

| kind    | its own sockets                          |
|---------|-----------------------------------------|
| point   | `radius`, `is_sphere`                    |
| spot    | `radius`, `is_sphere`, `angle`, `smooth` |
| sun     | `angle`                                  |
| area    | `sizeu`, `sizev`, `ellipse`, `spread`    |

### exit status zero is not success

Cycles reports a fatal-looking scene error, renders the rest, writes the image and exits **zero**.
measured twice: a `light_type` of `"pointt"` and a missing `<include>` each produced
`ERROR:` on stderr, a complete png, and exit status 0.

so the runner treats `ERROR` on stderr as failure. the exit status is still checked, because a
crash or a missing binary produces neither an image nor a diagnostic worth reading. a `WARNING`
is not matched: this build prints two colour-management warnings on renders that are perfectly
good.

### no view transform

with no OCIO configuration this build writes **scene-linear radiance straight into the file**.
measured: a world of constant `color = 0.1, strength = 1.0` came back as 26 of 255, which is
`0.1 x 255` and not the 89 an sRGB encode gives. there is no `--view_transform` flag in this
build either, so there is no curve at all, and linear 1.0 is already the top of an 8-bit file.

this is why a scene exposed for the rasterised post stage's Reinhard curve is overexposed here by
exactly the top of that curve, and why the two backends are not expected to produce the same
image.

### `exposure` is the document's own field, and it is a multiplier on both sides

`scene_schema::EXPOSURE` is documented as a linear multiplier on the radiance the post stage
reads, before that stage's own curve. `<film exposure>` is a linear multiplier on the radiance
Cycles writes, which was measured: a constant world at `strength = 1.0, color = 0.5` came back as
64, 128 and 255 under exposures of `0.5`, `1.0` and `2.0` — `0.25`, `0.5` and `1.0` linear, where
a stop count would have given 149 for the first.

they are the same number, so nothing is converted and **the document wins**. `EmitOptions::exposure`
is the default for a scene that names no exposure, which is what it was before the document was
read at all.

no gallery scene declares one, so this is a rule with no scene behind it yet.

### a light's brightness is read through `brightness_field`, and there is no second read

`brightness_field` maps each light kind to the name the **schema** declares for that kind's
quantity — a point or spot light a `power`, a directional light an `irradiance`, an
environment light a `radiance`. Every kind is read through it, and a missing value is a
`Malformed` error rather than a default, because a light whose brightness cannot be
read is a scene whose lighting is not the scene.

**there is no other field, and that is not a simplification.** the `Environment` arm once read
a second field, `strength`, which the schema does not declare for that kind —
`light_variant(shared_fields(), [radiance(), image()])` and nothing else. the read found
nothing and defaulted to 1.0, so **every environment light in the gallery rendered at unit
radiance whatever its scene declared**, and the rendered image said nothing, because a sky at
the wrong brightness is still a sky.

**the environment light's `radiance` is the world's radiance, and that is one quantity rather
than two jobs.** it is worth saying because a scene's comment can describe only half of it and
then be read as describing the whole: `foldyard` justified its `0.26` as *"a quarter of the
sun's strength; the sun is what makes the shape"*, which is a true reason about the **fill** and
says nothing about the sky — and the same `0.26` is also the sky. both backends read it that way
and neither reads it two ways:

| | what reads the number | what it does with it |
|---|---|---|
| rasteriser | `environment_radiance` (`shaders.rs:2081`) | the ambient every shadowed surface is lit by |
| rasteriser | `environment_sky` (`shaders.rs:495`) | the background the frame's empty pixels are drawn from |
| exporter | `kind: "background"`, `strength` = `radiance` (`cycles_export/light.rs:222`) | **one** world, and Cycles has nowhere else to put it (`cycles_export/scene.rs:124`) |

both rasteriser entries are the map scaled by the same `uniforms.environment.rgb`, which is the
same statement the exporter makes in the other direction. **so there is nothing to split**: a
scene that wants a quarter of a sun as fill has also chosen a quarter of a sun as sky, and the
number is called `radiance` on purpose. whether a particular scene wants *that* sky is a question
about the picture and is not a question this chapter has an opinion about.

measured on `dunes`, whose declared 0.55 came out as `strength="1, 1, 1"`:

| radiance | traced sky mean | gap against the rasteriser's 155.73 |
|---|---|---|
| 0.55 | 79.58 | -76.15 |
| 1.00 (what it actually rendered at) | 146.09 | -9.65 |
| **1.06** | **153.21** | **-2.52** |
| 1.30 | 189.88 | +34.15 |

the response is close to linear — about 145 of luma per unit — which is
what a linear radiance written straight into an 8-bit file with no view transform should give.
the investigation, and the three conditions that let it go unnoticed, are in
`.agents/investigate/environment-radiance-never-arrived.md`.

`Light::strength` was removed rather than corrected. it had one reader, that reader was the
wrong one, and keeping a second field answering a question `brightness` already answers is how
the wrong answer survived in the first place.

## Materials

### a material's parameters all travel in `custom_data`, so all of them are read from there

a `mesh_material` carries `label`, `shader` and `custom_data`, and **`custom_data` is the only
place any parameter is written** — the gallery's `surface()` row is `tint`, `metallic`,
`roughness`, `emissive` and `reflect` (`crates/gallery/src/kit.rs:641`). the material itself has
no field for any of them.

so reading `roughness` or `metallic` off the material finds nothing on any material that has ever
been written, and the emitter silently fell back to `0.5` and `0.0` while writing both into
`principled_bsdf`. that is not a small error: a sand declared `roughness = 0.92` came back as a
half-rough surface, and a floor declared `1.0` came back glossier than any surface in the scene.

`tint`, `metallic` and `roughness` each have a socket to land in and are folded into it.
`emissive` is folded in too — measured, see
[investigate/cycles-emitter-dropped-material-fields.md](../../investigate/cycles-emitter-dropped-material-fields.md).

`reflect` **has** a socket that sounds like it, and writing it was measured and **reverted**:
ours is how much sky a surface shows at a grazing angle and `Specular IOR Level` is the
reflectance at normal incidence. Writing it drops `landscape`'s correlation against the
rasterised frame from `+0.6223` to `+0.5733`, and reverting restores both numbers to the digit.
so `reflect` is reported rather than dropped, which is the point: reported is visible, and written
would read as honoured.

a material's `image` is reported by name with its reason, which is narrower than "no socket":
`principled_bsdf` **has** a `Base Color`, and the loader splits a connect's endpoints on
whitespace (`src/app/cycles_xml.cpp:219`), so a socket named with a space cannot be connected at
all. a field that silently failed to apply is the same invisible-wrong-image failure this backend
is arranged to prevent.

`ior` and `transmission_weight` are always written and never read, because the document has no
field for either. they are stated as the dielectric defaults rather than left to the node's own,
so a reader of the document does not have to know them to know what was meant.

### a surface body must not re-apply the albedo the shading already carries

`base_color` **is** honoured, which was verified rather than assumed: a `principled_bsdf` carrying
`base_color="0.85, 0.30, 0.08"` renders a saturated orange-red, with a dielectric highlight that
goes yellow-white near the light. the exporter writes `material.tint` there
(`cycles_export/material.rs:206`), and the template's own diffuse is
`in.tint * (1 - metallic)`. **so a body spliced at `{surface}` receives a colour that already has
the material's albedo in it**, and multiplying by `in.tint` there applies it twice.

`foldyard` did exactly that, and it was the largest disagreement in the gallery. its ground tint is
`[0.232, 0.238, 0.152]`, so the two backends should have differed by exactly the tint — and the
ground band, measured as the ratio of medians of linear radiance, read **0.274**, which is the
tint to within eighteen percent.

| | before | after |
|---|---|---|
| ground | 0.274 | **0.982** |
| whole frame | 0.342 | **1.042** |
| lower 62% | 0.384 | 0.963 |
| pearson, whole frame | +0.6047 | **+0.8278** |

the number was predicted from the scene before anything was changed, and the fix landed where the
prediction said, which is the only reason this is written as a finding rather than as a repair.

**a survey of every body in the gallery found this shape once.** `terraces` has the same kind of
body and never had the bug, because its material row is white and a white multiplied twice is the
white it was. `stilllife` names `in.tint` too and is correct: its term is **added**
(`color + fresnel * room_environment(...) * in.tint`), and a reflection carrying the colour of what
reflects it is the right physics rather than a second coat of paint.

**the rule this leaves is that a body's multiplier is about one.** that is what makes a body
portable at all: a second backend is handed the material row and nothing else, so anything a body
does with a quantity the row already states is a quantity only one backend has.

## Camera

### the frame is the scene's, and the camera node is not the buffer

`<camera width height>` is the camera's **view plane**. the buffer is `--width`/`--height` —
see [`Resolution` comes from the command line](#resolution-comes-from-the-command-line) — and both
are written from the same `Frame`, so the view plane and the buffer cannot drift apart.

a camera in this workspace's schema carries neither. the size is the **scene's** `frame`, a field
of the scene and not of a camera; the angle is `field_of_view`, or `focal_length` against
`sensor_width`. reading either off the camera found nothing on any camera that has ever been
written, so every scene fell through to a hardcoded ninety degrees at `1920 x 1080`.

### `fov` is the full angle along the frame's longer side, in radians

the node carries **one** angle and applies it along the frame's longer side, where the sensor fit
is automatic. our `field_of_view` is **vertical**: the schema calls it "the angle seen through,
vertically", and a lens is a width, so `vertical_field_of_view` divides the horizontal tangent by
the aspect to reach it.

so the two are the same angle on different axes, and which one the node wants is the frame's
aspect and nothing else:

- **wider than tall** — the longer side is the horizontal, which is the side a sensor is named
  on, so the angle widens by the aspect;
- **taller than wide** — the longer side is the vertical, which is already what we measure, so
  it passes through unchanged.

for `aspect >= 1` the two conversions are exact inverses, so a wide frame round-trips and the
emitted `fov` is the lens's own horizontal angle. the unit is **radians**: a prediction of
`pi / 2` matched the field of view measured off a render, and `90` describes no field of view at
all.

the precedence between an angle and a lens — angle, then lens, then the convention — belongs to
`standard::camera::vertical_angle` and is not restated in the exporter. a second reader would be
a second answer to the same question, and the two would drift exactly where the drift is
invisible: a scene that names a lens.

measured over all twelve gallery scenes. before, every one was `1920 x 1080` at `fov = 1.5708`.
after, each carries its own:

| scene          | frame       | `fov`     | the lens or angle it came from                     |
|----------------|-------------|-----------|----------------------------------------------------|
| `foldyard`     | `1536x1024` | `0.95002` | `35 mm` on `36 mm`, first of three cameras          |
| `tidewater`    | `1536x1024` | `0.95002` | `35 mm` on `36 mm`, first of two cameras           |
| `stilllife`    | `1536x1024` | `0.41736` | `85 mm` on `36 mm`                                 |
| the other nine | `1024x1024` | authored  | the schema's default frame; `field_of_view` as named |

the three wide frames were rendered at a shape they did not declare and now are; `1024` square is
`frame`'s own default and is what the rasterised path drew them at already, so those nine are
unchanged in size and changed in angle only.

### the eye is written down, not fitted at read time

**A scene document carries decisions, not the arithmetic that made them.** A camera names where
it stands; nothing downstream moves it.

this used not to hold. `Camera::projection` used to take a camera that named no depth planes and
**fit** it — pull the eye back along the eye-to-target ray until the whole scene's bounds fit the
lens, then multiply that distance by the camera's `padding`. Fitting never turns the view; it only
changes how far back the eye sits, and on a large scene that is enough to change the picture
completely. The emitter reads the document and therefore read an eye the rasterised path was not
using.

the fitting is gone, along with `padding`, and the four gallery scenes that relied on it now write
the eye they were being fitted to. measured, on `terraces`: the eye moved from 444.1 units off the
target to 260.9, **41% of the distance**, which is the difference between the whole valley and a
close crop of it.

**the rasterised output is unchanged.** all three renderable scenes — `dunes`, `monoliths`,
`terraces` — come back **pixel-identical** to what the reader used to compute, at nine decimal
places of the eye. three is fewer than three decimals would manage: rounding the eye to 0.001
units flips a z-buffer pixel on a silhouette, so the written number carries the precision the
picture needs and not one digit more.

what is left is the honest consequence: a scene that wants its camera framed against its own
geometry works that out **in the scene that knows the geometry** and writes the answer down, which
is what `kit::camera` and the four scenes that use it now do. a reader may still choose between
documented spellings and apply documented defaults — an angle over a lens, `exposure` over the
caller's number — but it may not compute a value the document did not carry.

### the camera frame is left-handed, and the reflection is not optional

Cycles draws the camera's own `+X` on the **right** of the frame. `rastertocamera` is
`screentocamera * rastertoscreen` (`src/scene/camera.cpp:265`), and both factors are positive
diagonals built from the view plane (`src/scene/camera.cpp:236-249`) — there is no sign flip
anywhere on that path.

a camera whose `+X` is its own right-hand side is **left-handed**, because its `+Z` is the
direction it looks along. for orthonormal columns `a, b, c` of a rotation, `b × c = det · a`, so
`a = cross(c, b) = cross(forward, up)` — the right-hand side — forces `det = -1`. a camera
transform that is a rotation cannot therefore be right, and no rotation can stand in for the
reflection. measured, each of these attempted on `landscape`:

| attempt | what happened |
|---|---|
| half a turn about the view axis | right **and** up flip; `up · world-up = -0.9981` |
| half a turn about local `Y` | up is kept, the camera turns away and the markers leave the frame |
| half a turn about local `Z` | markers to the left and upside down |
| conjugating instead of left-multiplying | markers at `0.129` and `0.649`, worse |

what the two backends disagreed about was **handedness**, and handedness is exactly what the
direction checks cannot see: forward, up, the sun's travel and every vertex read correct while
the picture came out mirrored. measured on `landscape`, two markers set on the rasterised
camera's right and left:

| marker | rasterised | Cycles, before |
|---|---|---|
| camera's right | `0.794` right half | `0.240` **left half** |
| camera's left | `0.190` left half | `0.818` **right half** |

so the reflection rides in the transform's `scale`, which the loader applies after `rotate`
(`src/app/cycles_xml.cpp:694-710`) and which therefore renames the local `X` while leaving `Y`,
`Z` and the translation alone. Cycles keeps the camera matrix verbatim — `Camera::set_matrix` is
the generated socket setter over `NODE_SOCKET_API(Transform, matrix)`
(`src/scene/camera.h:151`) — so the negative determinant reaches the kernel unaltered.

**it is applied where the camera is written, not where it is specified.** both routes into a
camera — an eye and a target, or a transform written by hand — mean the same thing in this
engine's axes, and only the writer knows that Cycles reads that frame mirrored.

measured after the change, `landscape` at 1024×1024, 32 samples on CPU. pearson correlation
against the rasterised frame is the measure that cannot confuse a bright ridge with a bright
sky:

| band | before, as-is | before, flipped | after, as-is | after, flipped |
|---|---|---|---|---|
| whole frame | `+0.0392` | `+0.1160` | **`+0.5119`** | `+0.1987` |
| ground band | `+0.1488` | `+0.6510` | **`+0.6499`** | `+0.1478` |

and the markers again, placed by `cross(forward, up)` — the rule `standard::math::look_at` uses,
which states nothing about handedness:

| marker | rasterised | Cycles, after |
|---|---|---|
| camera's right | `0.794` right half | `0.775` right half |
| camera's left | `0.190` left half | `0.228` left half |

the emitted camera's `localX` is now `cross(forward, up)` exactly, where it was
`cross(up, forward)` — the opposite — and `up · world-up` is still `+0.9981`, so the reflection
took the one column it was meant to and nothing else.

the measurements, the sources and the dead ends are in
[investigate/cycles-camera-frame-handedness.md](../../investigate/cycles-camera-frame-handedness.md).

### what is actually left is shading, not the camera

**corrected 2026-10-10: this was half wrong.** the eye, the target and the angle were indeed
right on both sides, and they still are. but the two frames also disagreed about **which side
of the picture is which**, which is a camera defect and was not in view until the two pictures
were compared against each other rather than against an expectation — see
[the camera frame is left-handed](#the-camera-frame-is-left-handed-and-the-reflection-is-not-optional).
what follows is about the remaining difference, which is shading.

`landscape`, `lanterns`, `spotlight`, `sunlight` and `tiles` still do not match. all five write a
final eye, a final target and a final angle through `camera_placed`, which is the same thing both
backends read. measured on `landscape`:

- **the geometry is all there.** `RANGES` is two entries and the plain is the third mesh, so the
  scene has three meshes and the emitted document has three `<mesh>` elements. nothing is dropped.
- **the camera is the same.** the document's eye comes out as
  `translate="0, 2837.4650972885993, 771.224941505395"` and the angle as `fov="1.24"`, which is
  what `camera_placed` wrote.
- **so the difference is in the picture, and the picture is code.** the massif's surface body
  (`crates/gallery/src/landscape.rs:1756-1801`) builds its colour per fragment out of
  `noise(in.world_position.xz * 0.02, 3)`, a snowline driven by `in.world_position.y`, a soil
  mix keyed on height, a skyward ambient term, and an aerial-perspective mix toward `air_colour`.
  the plain's body carries **the river**, which the scene's own comment calls the brightest thing
  in the frame.

the exporter maps a material to one `principled_bsdf` whose `base_color` is the material's `tint`.
**`landscape` emits three shaders and therefore three constants**, and a valley cannot be drawn
with three constants: what is missing is the snowline, the rock striation, the haze and the river,
and each of them reads as a change of framing because each of them is a large part of the picture.

this is not an exporter defect. the exporter is doing the only thing a five-number material row
allows, and doing it correctly. it is the same defect as the water — **logic that lives in code
rather than in the document cannot cross a backend boundary** — one level up, in the shading
rather than in the material. a `MeshMaterial` carries `tint`, `metallic`, `roughness`, `emissive`
and `reflect`, and a rasterised shader is free to compute anything from those five and the world
position; a path tracer is handed five numbers.

recorded rather than fixed: expressing a procedural surface as data is a schema change, and the
honest first step is knowing how many surfaces depend on it. `landscape`, `lanterns`,
`spotlight`, `sunlight` and `tiles` are the measured five.

**`landscape` has since been withdrawn from this list** — see
[the geometry pipeline ships geometry, and the detail is baked later](#the-geometry-pipeline-ships-geometry-and-the-detail-is-baked-later).

**and the count of four is itself wrong: measured 2026-10-10, the number is one.** `spotlight`
(`crates/gallery/src/spotlight.rs:79`) and `sunlight`
(`crates/gallery/src/sunlight.rs:100`) have passed `""` as the lighting stage's surface argument in
**every revision since the commits that created them** (`65012c2`, `f616eb9`), so neither ever wrote
a procedural surface and neither can be withdrawn from a list of them. the earlier four was an
undercounted reading of which scenes *carry a shader*, not of which ones carry a body.
`tiles` (`crates/gallery/src/tiles.rs:52`) asks for `shaders::textured` with an empty helpers block,
and the body that returns is a `textureSample` of the image the document already carries
(`crates/gallery/src/shaders.rs:2151`) — a bound image, not arithmetic in a fragment. **its
cross-backend gap is a different one**: the emitter drops both the `texture` point attribute and the
`image` material field, and reports each, so what the path tracer draws is a flat floor. that is a
limitation of `scene::cycles_export` in carrying an image at all, and no edit to `tiles` reaches it.

that leaves `lanterns` as **the one scene still writing a procedural surface**, with two bodies
(`crates/gallery/src/lanterns.rs:312` and `:336`). **neither is vertex displacement, and neither is
convertible into it.** `WATER_SURFACE` mixes by `pow(1 - |dot(normal, view)|, 4)` and by distance
from the eye; `GLOW` scales by `|dot(normal, towards)|` and by a `sin` of a world position plus the
clock. every term in both is a function of where the eye is or of when it is, and a vertex attribute
is written once when the mesh is built — baking either would freeze a moving quantity onto geometry
and move the defect rather than remove it, which is the fitting this chapter already undid. both
stay, and are recorded as the shading gap they are.

all four scenes build and render on **both** backends today — measured 2026-10-10 at 1024x1024, 32
samples on CPU, no `ERROR` on stderr — and the two scenes that carry no procedural surface are the
two that already agree. pearson correlation of luma against the rasterised frame, both sides put
through the same Reinhard + gamma the gallery's own grade applies:

| scene        | surface bodies of its own | correlation |
|--------------|---------------------------|-------------|
| `spotlight`  | none                      | `0.938`     |
| `sunlight`   | none                      | `0.969`     |
| `lanterns`   | two, both view-dependent  | `0.876`     |
| `tiles`      | one, a texture sample     | `0.115`     |

so the honest reading of "do not match" for three of these four is **not** a missing body: it is the
brightness gap of [no view transform](#no-view-transform) and a sky the document declares no
background for, both already recorded below. `tiles` is the one that genuinely does not compose,
and the emitter's own report says why.

what is *not* open is whether the target format can express it. measured 2026-10-10: it can, natively,
with no compiler in the loop — `<shader>` takes an arbitrary node graph and
`noise_texture`, `voronoi_texture`, `wave_texture`, `checker_texture`, `brick_texture`, `sky_texture`,
`rgb_ramp`, `bump`, `math`, `vector_math`, `map_range`, `layer_weight` and the `mix_*` family are all
registered and all render with zero errors. OSL, which sounds like the intended mechanism for this,
is **not compiled into the executable we ship**; the only `osl` substring in it is the `osl.svm`
device-type identifier, and a shader using `osl_shader` is rejected as an unknown node. so the gap is
entirely on our side of the boundary and needs no external component. evidence, the socket names that
fail silently, and the boundary of the OSL claim are in
[investigate/cycles-procedural-materials.md](../../investigate/cycles-procedural-materials.md).

### the geometry pipeline ships geometry, and the detail is baked later

decided 2026-10-10, superseding the two-form material plan recorded on the abandoned
`procedural-surfaces` branch. a geometry pipeline's job is geometry. what a surface looks like in
detail — the snowline, the per-block stone, the gravel sorting, the braided channels — belongs to a
**later stage that bakes textures**, not to a per-fragment shader in the scene that generated the
mesh. a scene that shades procedurally is a scene whose picture cannot be reproduced by a path
tracer, and that is the defect this chapter exists to prevent.

so `landscape` now draws **both meshes with no surface body at all**. the massif's stone and the
plain's gravel are their materials' own `tint` values, and nothing else decides their colour. a
scene is not required to be interesting: it is required to be honest about what it is, and a mesh
that came out of a geometry crate is a mesh, not a mountain.

**a body came back once, and was measured off again.** `e447f85` gave the massif a per-fragment
body, `mountain_shade`, whose own documentation gave the reason: *"the shared lighting stage's
ambient term is fed by the scene's environment light, and this scene names none — which the stage
answers with black."* the scene names one. `mod.rs:671` declares
`environment_light("air", "air", [1.0; 3], 0.55)`, and that line arrived in `5fb5c12`, which is
**not** an ancestor of `e447f85`: `git merge-base 5fb5c12 e447f85` is `77b3fc2`, and
`git show e447f85:crates/gallery/src/landscape/mod.rs` contains zero occurrences of
`environment_light`. `feature/dissected-plateau` forked before the sky work, so on the branch that
wrote it the body was a fair answer to a black flank, and `d993bc7`'s merge left main holding both.

the cost, measured with the geometry held fixed so the tree rewrite is not inside the number:

| raster massif | whole frame | lower 62% | lower 38% |
|---|---|---|---|
| body present | +0.2339 | +0.0740 | +0.0508 |
| body removed, same tree | **+0.6029** | **+0.5159** | **+0.5854** |

so the tree is worth about **0.02** and the body about **0.37**. `MOUNTAIN_SKY` and `MOUNTAIN_SUN`
are literals in the raster shader and `principled_bsdf` has no socket for either, so there is no
reading of that body the tracer can run. the flank it was written to fix measures 93 mean luma
with the environment light on it — a grey, which is what a shadowed flank should be. the evidence
and the reproduction are in
[investigate/landscape-agreement-drop.md](../../investigate/landscape-agreement-drop.md).

**atmosphere belongs to the post stage**, as a rule about **stages** and not about names. restated
2026-10-13: *an atmospheric effect may not be implemented on the geometry's own material; it must
be a separate post process.* a surface shader that paints its own fog is a surface whose fog a post
stage cannot correct, a tone curve cannot, and a second backend cannot reproduce.

**the wording this clause used to carry named `air_colour()` and `air_closure()`, and a rule about
two spellings is not a rule about the fault.** a survey of the whole gallery, measured 2026-10-13,
found `monoliths` and `tidewater` both satisfying that wording while both did the thing:
`monoliths` mixes toward a local literal `wet`, `tidewater` toward its own `aerial_colour()`, and
**neither file so much as names `air.rs`**. what the clause is about is what a body **computes** —
a distance from the eye to this fragment, and a colour decided by it — which is the shape of aerial
perspective whatever a scene chooses to call it. the three bodies that were doing it, and the
four blind spots in the survey that found them, are in
[investigate/where-atmosphere-is-painted.md](../../investigate/where-atmosphere-is-painted.md).

**all three have moved, measured 2026-10-13 at 1024x1024.** `tidewater`'s two were the shore
mixing toward `aerial_colour` at the lighting stage and the lake doing the same at the emission
stage; the law is now the integral of the density along the line of sight rather than the
density at the far end with `slant = 1 + facing * 2.4` bolted on, which is what let it go — the
slant term needed a normal, and the integral needs only the ray and the endpoint's height:

| `tidewater` camera 0 | bodies | post |
|---|---|---|
| whole | 0.920 | 0.886 |
| sky | 1.059 | **1.059** |
| lower 62% | 0.769 | 0.745 |
| ground | 0.699 | 0.680 |

**the sky is unchanged to the digit, and that is the check the move is verified by**: rows 0 to
84 agree to four decimals between the two raster frames, because the post stage applies the air
only where the depth buffer says geometry was drawn. **the ground ratio falls, and that was
predicted before the change was made** — `slant` was worth up to 3.4x at grazing incidence and a
horizontal ray's integral is worth about 1x, so the air thinned exactly where the old law had
inflated it.

`monoliths` had **no row in the census at all** and still cannot have one: it declares no
environment light, its sky is post-only literals, and the tracer draws it black (traced sky
median `0.0000`, ground `0.0039`), so a ratio against it would measure the absence of a world.
What it does have is a picture, and the move is visible in the picture:

| `monoliths` | before | after |
|---|---|---|
| sky rows, median | unchanged to four decimals | unchanged to four decimals |
| ground rows, median | — | +1% to +10%, growing to the near floor |
| row 264 dark tail | 0.2040 | **0.4885** |
| row 276 dark tail | 0.0890 | **0.4953** |

**the median says almost nothing about that and the dark tail says all of it.** the stones are a
small fraction of the rows they stand in, so the row median moves by one or two percent while the
tenth percentile — what the darkest thing in the row is made of — moves by a factor of five.
Before, the monoliths were `0.089` against a `0.60` median: hard black edges with the morning
pasted behind them. That is the defect this clause exists to prevent, and it is now a measurement
rather than an assertion.

**what the air costs is a constant the scene owns, and it was left alone.** both scenes' density
constants are unchanged; they were tuned against a law that had a fudge in it, and whether a
scene wants more air than an honest integral gives is a question about the picture, not about
where the air is written down.

**what post has, and what it has not, is why the move is not mechanical.** a post stage recovers
where a fragment was: `world_position(uv, depth)` (`shaders.rs:2674`) inverts the depth buffer
against the whole view-projection, and `ray_through` (`shaders.rs:2619`) is what it inverts along.
the world position, and its height above the water, follows from those and the camera uniform.
**that recovery is the shared stage's and not a scene's** — `foldyard` and `terraces` each carried
a copy of it, character for character, and a third scene then wrote a fourth spelling, so four
answers to a question the depth buffer has one of; all three now call the one beside
`ray_through` and `distance_to`, and the relocation is measured to be byte-identical on every
frame of both scenes that had a copy. `distance_to` stays because it answers a different question,
a distance **along the view**, which is what `stilllife`'s depth of field wants.

it has **no normal**: the post pass binds a colour target, a depth buffer and the environment map, and
nothing else. a law whose weight is a function of the surface normal therefore cannot move as
written. `tidewater`'s is — `facing = 1 - clamp(dot(normal, view), 0, 1)` feeding
`slant = 1 + facing * 2.4` — and it is **replaced rather than dropped**: an exponential-height
medium has a closed form for the path length along a segment, and integrating it is what the
`facing` factor was standing in for. the post version needs the ray and the endpoint's height,
both of which it has, so this is a reason to prefer the post statement rather than a concession.

this does not need an exception to **a document carries no logic**. the scene schema already carries
a shader's own text, and says why: *"a shader carries its own text so that a compiled scene renders
wherever it is sent."* an empty body is the limit case of that, not a departure from it.

measured on `landscape` after the change, 1024×1024, 32 samples on CPU:

| claim | measured |
|---|---|
| composition and silhouette | **identical on both backends** — the camera section above is satisfied for this scene |
| surface brightness | raster 162.3 mean luma, cycles 70.9, over 3295 paired surface samples |
| sky | raster painted by the grade, cycles black — the document declares no `<background>` |

**this table is dated; two of its three rows have since been answered.** the sky row is history:
`5fb5c12` gave the scene the world the note below says it wants — an `Environment` light over a
`256 x 128` latlong map, which the tracer reads as readily as the rasteriser does. and the surface
brightness row predates the floating shaded target, which moved `landscape` from **+0.6029** to
**+0.7812** against the same tracer frame. re-measured on `main` at `c8c7ec9`, 1024×1024, 32 samples
on CPU:

| scene | `rgba8unorm` shaded target | `rgba16float` shaded target |
|---|---|---|
| landscape | +0.6029 | **+0.7812** |
| spotlight | +0.9283 | +0.9282 |
| sunlight | +0.9729 | +0.9763 |
| tiles | +0.0221 | +0.0221 |
| lanterns | +0.8352 | +0.8571 |
| foldyard | +0.1050 | +0.1168 |

**this attribution was wrong, and a measurement has replaced it.** an earlier revision of this file
recorded the remaining brightness gap as *the two lighting models — the rasteriser's own Lambert and
specular against the tracer's integral* — and as the same class of difference as
[no view transform](#no-view-transform). it is not that. compared in **linear radiance**, with the
raster scene's own tone curve undone on one side and the tracer's linear file read on the other, a
scene carrying one directional light and nothing else was out by a **uniform factor of `pi`**:
`sunlight` measured 2.817 whole and 2.948 on its ground, `landscape` 1.841 and 2.317, and the factor
held across every region of the frame.

a uniform factor is a conversion and not a model. `light_contribution` returned
`diffuse * lambert * radiance` with no `1 / pi`, and its two callers pass different quantities
through the one parameter: an `Environment` light declares a **radiance**, where the omission is
correct because a uniform hemisphere of radiance `L` delivers irradiance `pi L` so the division
cancels, and a directional light declares an **irradiance**, where a Lambertian surface returns
`albedo * E * cos / pi` and it does not.

**fixed in `a957642`.** the division goes in `directional_radiance` and not in the shared function,
because the clustered loop's brightness has already been through `1 / (4 pi^2)` and is a radiance.
measured after it: `sunlight` 2.817 to **0.896**, `landscape` 1.841 to **0.947**, and `dunes`, which
names no directional light, 1.004 before and after. **neither scene's declared irradiance changed
and neither needed to.** the 6% the raster is still short is the tracer's indirect bounce, which a
rasteriser with no global illumination does not compute.

**the correlation moved the other way on one scene, and the reason matters.** landscape's whole
frame pearson went +0.9633 to +0.9292 with its sky band unchanged at +0.5444, while `sunlight` went
+0.9709 to +0.9785. a pearson is insensitive to a gain, so that number was partly propped up by both
frames being bright together for the wrong reason. **no correlation in this chapter can see a units
error.** the measurement that can is
[investigate/a-directional-irradiance-is-used-as-a-radiance.md](../../investigate/a-directional-irradiance-is-used-as-a-radiance.md).

the stripes along the massif's flanks were **geometry** — the rib construction — and both backends
drew them. that was measured on the older massif, and **it no longer describes these frames**. a
scanline across one lit flank at `y = 620` reads `190 190 191 191 191 191 191 191 191 191` on the
rasteriser across 275 pixels and `255 243 245 236 242 179 220 238 247 243` on the tracer across the
same span: **raster resolves one value per face, the tracer resolves within the face.**

**this is now established, and it is two emitter defects rather than a shading difference.**
an earlier revision of this file left the cause open and guessed that a different interpolated
normal field or the tracer's self-occlusion might explain it. neither does. the full evidence,
every number and every source line, is in
[investigate/cycles-mesh-normals.md](../../investigate/cycles-mesh-normals.md); the summary:

- **the normal array was written under a name the loader never looks for.** the loader reads it
  as `Attribute::standard_name(ATTR_STD_VERTEX_NORMAL)`, which is **`"N"`**
  (`cycles_xml.cpp:470`, `scene/attribute.cpp:460-462`). the emitter wrote
  `standard_vertex_normal` — the enum name, not the wire name. measured: removing the array,
  replacing it with face normals, and leaving it as written all render **byte-identical**. a
  missing attribute is not an error, so nothing ever said so. **now written as `N`**, with the
  state's `interpolation="smooth"` — see [the array is written as `N`](#the-array-is-written-as-n-and-the-state-carries-interpolationsmooth).
- **and the normals we do write face the opposite way from the winding — on 447 of 620 meshes.**
  **corrected: the first count here was the mean dot of a normal against its own triangle's face
  normal, one number per mesh, and it put the gallery in three groups when there are two.** the
  distribution says what it should have said from the start — a mesh is opposed only when *not
  one* of its triangles agrees, and 447 of the 620 are, with 61 more split and 2 carrying a
  normal perpendicular to their own surface. `landscape` at −0.9897, −0.9897 and −0.9991 was
  never a special case; it was three members of the majority. the per-mesh evidence is in
  [investigate/cycles-mesh-normals.md](../../investigate/cycles-mesh-normals.md).

  **the gallery's mesh builders do not share one winding convention**, and neither backend agreed
  with all of them. not the axis conversion — one proper rotation is applied to positions and
  normals alike, and a rotation preserves dot products. not the rasteriser either: it renders
  correctly from these normals and reads no winding at all, so it never had a convention to agree
  with. the convention is stated now, in
  [offline_backend.md](offline_backend.md#winding): the authored normal is the outward direction
  and the winding obeys it.

renaming the array to `N` makes it take effect at once and the massif goes **black**, which is
what an inward normal looks like. all three emitter changes were then implemented and measured
across the whole gallery, and **they are not a net win**:

| scene | as emitted | with all three |
|---|---|---|
| landscape | +0.7812 | **+0.7941** |
| tiles | +0.9740 | +0.9740 |
| spotlight | +0.9282 | +0.8735 |
| lanterns | +0.8571 | +0.7338 |
| sunlight | +0.9763 | +0.8105 |
| foldyard | +0.1168 | **−0.2411** |

one scene gains 0.013 and four lose, one of them catastrophically.

**and the reason that table gives was also corrected by the same re-measurement.** it was read as
*a mapper applies one rule to every mesh it writes and this gallery has no single rule to apply*,
so no change at the mapper layer can be right. The rule is not unavailable; it was applied to the
wrong half of the gallery. A reversal at the mapper inverts the 110 meshes that were already
right and fixes the 447 that were not, which is exactly what the table shows. The mapper cannot
decide **which** way a given mesh goes — that is the document's to carry — but the emitter
writing `N` and `interpolation="smooth"` is right on its own terms and lands with the
document-side correction, not instead of it. recorded rather than deleted, because the reasoning
is what makes the next attempt shorter. the survey is in
[proposals/cycles-mesh-normal-orientation.md](../proposals/cycles-mesh-normal-orientation.md).

**neither of the two unambiguous changes may land on its own** — *this has since been settled, and
the blocker was the winding.* renaming the array is a name that does not match the format, and it
is a defect whatever the shading turns out to be — but with the orientation unsettled it dropped
the massif to **−0.6122**, because the normals suddenly took effect and pointed inward. once every
mesh in the gallery agrees with its own normal the block is gone, and both land together.

### the array is written as `N`, and the state carries `interpolation="smooth"`

measured, all thirteen documents re-exported and the whole gallery re-rendered, 1024x1024 and 32
samples on CPU, against raster frames re-rendered from the same branch:

| scene | before | after | |
|---|---|---|---|
| landscape | +0.7826 | **+0.7938** | the only scene the massif is in, and the one that was blocked |
| monoliths | −0.1594 | −0.1682 | −0.0088 |
| tiles | +0.9740 | +0.9740 | |
| sunlight | +0.9763 | +0.9763 | |
| spotlight | +0.9282 | +0.9282 | |
| lanterns | +0.8571 | +0.8571 | |
| foldyard | +0.1168 | +0.1168 | |
| tidewater | +0.3435 | +0.3435 | |
| dunes | −0.6584 | −0.6587 | |
| stilllife | +0.4887 | +0.4884 | |
| terraces | −0.8264 | −0.8265 | |

**it is worth almost nothing, and the massif no longer goes black.** the first is the finding and
the second is the reason it lands anyway: the array is written under a name the loader never reads,
which is a defect whatever it is worth, and it cannot be worth less than a name that matches.

the reason it is worth nothing is the same one that made the winding worth nothing — **the tracer
flips its shading normal towards the ray it is shading**, so a surface lit from the front is lit
from the front whichever way its corners were written. Correlation is not a sensitive measure of
which of two normals points outward.

`interpolation="smooth"` is read by `xml_read_state` and copied into `mesh->get_smooth()`
(`cycles_xml.cpp:462`), and its default is **flat** — so without it the array is read, stored and
ignored. both attributes are on the same `<state>`, and the mesh's state is the one that wraps a
`<mesh>`; a document has several states and only that one carries them.

**and it does not close the facet difference, which was never an attribute defect.** a scanline
across one lit flank of the massif at `y = 620`, rasterised against both documents:

| x | 275 | 300 | 325 | 350 | 375 | 400 | 425 | 450 | 475 | 500 |
|---|---|---|---|---|---|---|---|---|---|---|
| raster | 222 | 222 | 216 | 217 | 218 | 214 | 204 | 207 | 210 | 214 |
| tracer, flat | 255 | 255 | 243 | 245 | 236 | 242 | 179 | 220 | 238 | 247 |
| tracer, smooth | 255 | 255 | 243 | 243 | 237 | 229 | 191 | 196 | 205 | 235 |

smooth interpolation takes the excursion down from `255 … 179 … 247` to `255 … 191 … 235` and
leaves it there. the residual variation is what a **smoothly interpolated normal does across a
triangle** — it is supposed to vary. the rasteriser shades per face, so its values step at face
boundaries, and no attribute reaches across that difference: it is a flat-versus-smooth shading
semantic, and the earlier reading of this scanline — *raster resolves one value per face, the
tracer resolves within the face* — described a real difference and attributed it to a missing
attribute that was never the reason.

**welding is not available**, which is worth recording because it looks like the obvious
repair. the loader consumes `verts` as per-triangle vertex indices and never writes the
loop-to-vertex map, so a mesh with shared vertices indexes past its own vertex array: welding
mesh 0 from 2256 vertices to 887, with every array length self-consistent, **crashes Cycles**
(`0xC0000005`, inside `update_mesh()`), and dropping the UV array does not help.

those raster values are **not** clipped — 191 is well under 255 — so the eight bit ceiling measured
in
[investigate/shaded-image-eight-bit-ceiling.md](../../investigate/shaded-image-eight-bit-ceiling.md)
is not what hides the tracer's detail here.

what is **not** claimed is that this makes the two backends agree. it makes them agree about
**framing, geometry and the meaning of a document**, which is what the geometry pipeline and the
emitter are responsible for. per-pixel agreement was never available — see
[no view transform](#no-view-transform) — and is not what this change sets out to buy.

### how many scenes the fitting affected, corrected

an earlier revision of this file said **nine** of the twelve gallery scenes were fitted. that was
wrong, and it is recorded here rather than deleted, because the way it was wrong is the useful
part: it came from reading which constructor each scene calls, and `camera_placed` also fills the
role of "a camera that is not fitted" without writing anything the reader has to interpret.

the true count is **four**: `dunes`, `monoliths`, `nurbs` and `terraces` are the only scenes
calling `camera()`. `camera_placed` and `camera_placed_on_lens` both write `near` and `far`, so the
reader kept their eye and there was nothing to disagree about.

`landscape`, `lanterns`, `spotlight`, `sunlight` and `tiles` do not compose identically across the
backends, and the fitting was never why — see **what is actually left is shading, not the camera**
above. the last sentence of the previous revision said the cause was still open; it was isolated
in the same pass that corrected the count above.

## Environment

### the world is indexed by direction, and the direction has a projection

an environment map is read by a direction, and how a direction becomes a coordinate is the
**projection**. Cycles' `environment_texture` has no octahedral option at all: its
`TextureMapping` projection enum is flat, cube, tube and sphere, and its world lookup is
equirectangular. so a fold is not something this backend can read.

measured: `foldyard` handed its octahedral sky over as an octahedral map and came back with the
left half of the sky black, the right half blue, and a hard vertical seam down the middle. nothing
warned.

so the emitter **refuses** an octahedral map in a document bound for Cycles rather than converting
it. converting would mean the emitter holding a second definition of a layout, which is the class
of bug that produced the seam in the first place. the rasterised path keeps both projections and
defaults to latlong, so a scene that says nothing is carried by every backend.

measured, and worth not repeating: adding a `<mapping>` node to `environment_texture` changes
nothing. the equirectangular world lookup is already the identity on `(u, v)`, and declaring the
mapping is decoration.

see [.agents/investigate/cycles-world-axes.md](../../investigate/cycles-world-axes.md) for the
measurements.

### the map is the same sky as the shader, and it is one law rather than three

a scene whose sky lives in a shader stage has **no sky at all** on this backend, because there is
no shader stage to draw it: `landscape` carried its whole sky in its post grade and rendered here
as an empty black field above the horizon. the fix is not a new kind of environment light — the
format has had that all along — it is for a scene to declare the sky it already has **as an
image**, and for that image to be the same sky.

**it is evaluated from the same fields, term for term.** `air::Atmosphere` owns zenith, horizon,
ground, the sun and its glow, and it now carries `environment_pixels`, which is the `WGSL`
`air_sky` written in Rust: the same `pow(clamp(y, 0, 1), CURVE)` ramp, the same
`glow * pow(toward, reach) * strength`, the same disc inside it, and the same below-horizon
branch that **replaces** the sky rather than tinting it, so the glow cannot leak under the horizon
and light the ground.

**three call sites had this written out separately and all three disagreed with the shader.** one
ramped on `(1 - up)^2`, one on a `smoothstep`, and one hardcoded its glow colour and its glow
exponent rather than reading the sun the scene had already named. an ambient from one afternoon and
a sky from another is exactly the seam `air.rs` exists to make impossible, and it was being
produced by the module meant to prevent it. `foldyard`'s copy is gone; it calls the one law now.

**a lamp is not a substitute and the scene says so.** `landscape` lit the massif's shadow side
with a point light called `sky`, dim and blue, standing in for the dome. it cannot be a dome: a
point light's brightness is the square of its distance, and this scene's subject is two thousand
metres across on a plain nine times that, so the fill that lit the massif left the far plain
black. it is now an `Environment` light over a `256 x 128` latlong map, which is **replacement
rather than addition** — the rule `SPEC.align.md` states for the ambient term.

measured on `landscape`, 1024×1024, 32 samples on CPU. pearson correlation against the
rasterised frame:

| band | before, as-is | before, flipped | after, as-is | after, flipped |
|---|---|---|---|---|
| whole frame | `+0.5119` | `+0.1987` | **`+0.6223`** | `+0.1025` |
| ground band | `+0.6499` | `+0.1478` | **`+0.6701`** | `+0.1632` |

`foldyard` is **unchanged** by the shared law, which is the check that matters for it: measured
before and after on both backends, `+0.1044` against `+0.1050` on the whole frame, a difference
inside the noise of sixteen samples. `foldyard` composes poorly across the two backends for its
own reasons and did before this; it is a separate finding, not a regression here.

### `terraces` keeps its own sky law, and `dawn::Sky` is why that is not a second `Atmosphere`

`terraces` was the last scene drawing a sky nothing else could see. Its law was a `WGSL`
function — `sky_colour(ray)` — so the post stage drew it and the tracer had no sky at all: the
rasterised frame carried the morning and the traced one carried black, at **−0.8264**.

**the law moved into rust, as `crates/gallery/src/dawn.rs`, and is read out twice.** `dawn::Sky`
holds zenith, horizon, ground, the sun, its glow and its disc as fields; [`wgsl`](../../crates/gallery/src/dawn.rs)
emits the shader the stages that ask for a ray read, and `radiance` answers the same law as a
number, which `shaders::environment_pixels_of` maps into the world. every field is read by both.

**it is not an `Atmosphere` with three more setters, and that is a decision rather than an
omission.** this sky's horizon **follows the sun around the compass** — east burns, west holds the
night's cool — and its glow is **two lobes** rather than one. `air.rs` grants a scene the right to
write its own law, and the alternative was pushing one scene's hour into a type four others read.
`shaders::environment_pixels_of` exists so the part that is *not* the law — the texel decode and
the clamp before the quantise — is written once for both.

**the two stages are given different skies on purpose.** the mesh stage has no map to read, so the
water that mirrors the sky mirrors `sky_base` and the deck over it; the post stage is handed the
map and the deck alone, and is **not given `sky_base` at all**. a stage that could draw its sky
either way will one day draw the other one, and the two would differ by exactly the amount nobody
measured.

**the stratus deck stays in the raster stage.** it drifts on `uniforms.time` and a static map has
no clock, so it is weather laid over a world rather than part of it. it takes the sky as an
argument rather than evaluating one, which is what lets the map's sky and the law's sky wear the
same deck.

**the `sky` lamp is replaced rather than joined**, on the reasoning its own comment already gave —
a point light cannot be a dome, and a valley 380 units to the far corner is too far for one. the
pass count follows from the scene rather than by hand: two global lamps drew twelve shadow maps
over the grid and one draws six.

measured 1024×1024, 32 samples on CPU, against the frame drawn before:

| | before | after |
|---|---|---|
| whole frame, pearson | `−0.8264` | **`+0.8281`** |
| sky radiance ratio | traced exactly black | **`0.856`** |
| whole frame radiance ratio | `2.619` | **`0.530`** |
| ground radiance ratio | `1.880` | `0.499` |

**`0.856` puts `terraces` in the same class as the four scenes that already had a world** — and
the worst of the five. `dunes` `1.001`, `landscape` `1.040`, `tidewater` `1.059`, `foldyard`
`1.085` (`.agents/investigate/every-sky-agrees-and-the-ground-does-not.md`, 2026-10-12) are
inside nine percent; `terraces` is inside fifteen, and it is inside that **without a fitted
number**: the map is stored at the sky's own scale and the environment light declares a radiance
of **one**, because any other number would be scaling the picture rather than describing it. the
raster sky read out of that map correlates **+0.9802** against the same sky read from the old
`WGSL` law.

**the ground is still wrong and for a reason that is not this scene's.** `0.499` says the tracer's
ground is twice the raster's, and the cause is the one already recorded above it: the terrain's
albedo is decided per fragment — water, earth, crop, forest — and a path tracer is handed the
**material row**, which says white. fitting the declared albedo to the picture's mean would close
the number and open a lie, so the gap stays.

### the `environment` radiance sets the sky as well as the light, and `foldyard`'s was chosen as light

`environment_sky` returns `textureLoad(...).rgb * uniforms.environment.rgb`, so the **one** field
a scene declares is read by the stage that draws the sky and by the stage that lights from it.
that is right — in a path tracer the background and the world's radiance are the same quantity,
and separating them would put the two backends a factor apart.

**but it couples two decisions a scene author makes for different reasons**, and one scene has
already been bitten by it. `foldyard` declares `environment_light("air", "air", [1.0; 3], 0.26)`,
and the comment above it says why: *"at a quarter of the sun's strength; the sun is what makes the
shape."* that is an **ambient lighting** argument, and it is a good one. the same `0.26` also
multiplies the sky its post stage draws, and it did not until the post stage began reading the map
— `air_sky` is a law and carries no radiance.

measured on the rendered frame 2026-10-13, `foldyard` camera 0: the sky rows read **41 / 45 / 55**
of 255, against about **152** for the same rays under its own `Atmosphere` through its own grade
(`HORIZON` 0.600 at ten degrees, exposure 0.42, ACES, gamma). the factor is `0.26`.

**this is not a bug and the frame is not wrong** — the tracer draws the same world at the same
`0.26`, which is why the census has `foldyard`'s sky at `1.085` rather than the `3.8` it would read
if the raster were still drawing the law. what it is, is a number chosen under one name being
read under another, and it is worth knowing which scenes are affected before anyone tunes an
ambient and silently dims a sky:

| scene | declared radiance | who chose it |
|---|---|---|
| `dunes` | 1.06 | measured against a raster sky |
| `terraces` | **1.0** | the sky is the world, so the radiance is one |
| `foldyard` | **0.26** | *"a quarter of the sun's strength"* — an ambient argument |

### the map is read with `textureLoad`, and the artefact that was expected was not found

**the mechanism, from the code.** the map declares **no sampler**
(`emit.rs::environment_binding`, and its comment gives the reason: a filtered read would bleed
across the octahedral fold's seam), and both readers use `textureLoad` at a texel the shader
computes — `environment_sky` at level zero, and `environment_radiance` at
`level = clamp(int(roughness * (levels - 1)), 0, levels - 1)`. for `terraces` the terrain row is
`0.55` and a nine-level chain gives **level four, which is sixteen texels around the whole
sky**, read without filtering. (derived from the code, not measured.)

**what was looked for, and not found.** two frames were measured for the staircase a nearest read
must produce: the step between consecutive sky rows, and a crop of a lit face at one to one.

| frame | sky rows that repeat | median step between rows | worst |
|---|---|---|---|
| `dunes`, top tenth | 284 of 306 | **0.09** | 0.49 |
| `terraces`, top tenth | 198 of 306 | **0.09** | 0.19 |

and a 200×200 crop of `terraces`' mountain at one to one is smooth, with no hard-edged texel
boundary anywhere in it. **a smooth ramp quantised to eight bits is exactly what this measures**,
and the banding one can see in a downscaled view of `dunes` is the eye's, not the file's — a long
dark ramp reads as bands whether or not there is anything to band about.

**so the negative result stands and is the finding: nothing visible came of it.** both the bands
and the blocks were read off a scaled-down frame and neither is in the pixels, and the honest
conclusion is that a latlong map at these sizes carries these skies well enough that the
unfiltered read costs nothing measurable.

**what it would take to become visible, which is why the mechanism is still recorded.** a texel
step has to exceed a quantisation level before a ramp becomes a staircase, and it scales with how
fast the sky changes across one texel. a map a quarter of the size, or a scene whose sky has a
sharper feature — a disc, a hard horizon — puts it over. `foldyard` below is that scene, and it
is not a banding case but a worse one.

**and the octahedral seam is narrower than it looked.** of the environments the gallery declares,
**none is octahedral** — `dunes`, `landscape`, `foldyard` and `terraces` all pass
`Projection::Latlong`, and `Projection::Octahedral` is reached by
`device-rasterize/tests/environment_map_render.rs` and nothing else. the fold therefore constrains
a test, not a rendered scene, and the work a filtered read would actually be is a latlong `u` that
wraps and a `v` that clamps.

### the map is `rgba8unorm`, so a radiance above one is stored as one — and almost nothing is

`environment_pixels_of` clamps every channel into `0..=1` before it quantises, because the format
has no headroom. **that reads as a live defect in three scenes and is not one.** measured
2026-10-13 by evaluating each scene's own law over the rectangle its own map is built at:

| scene | map | upper bound | brightest texel | texels over one |
|---|---|---|---|---|
| `dunes` | 256×128 | 0.700 | 0.687 | 0.000% |
| `landscape` | 256×128 | 1.390 | 0.813 | 0.000% |
| `foldyard` | 512×256 | 2.042 | 1.716 | 0.002% |
| `terraces` | 256×128 | 19.360 | **2.580** | **0.754%** |

**the upper bound is each term's maximum, added.** the terms do not all peak on one ray, so it is
what a clamp has to be sized against rather than a measurement of any texel — and on three
scenes it is far above what any texel reaches, because the disc is small and no sample lands in it.

**`terraces` is the first scene whose map is actually clamped**, over **2.37%** of the sphere and
at **2.58** rather than one. what that costs the picture is small and was measured: the corner the
sun's bearing falls in reads **214/255** with the old analytic law and **204/255** with the map,
**five percent**, because the grade's ACES fit had already rolled that corner off long before the
map could have mattered.

**so the clamp is real, it is confined to one scene, and it is not what is wrong with the maps.**
the fix is the `environment` radiance field — a map stored at one over that radiance, so the
shader's multiply brings it back and the headroom the format lacks is the radiance itself — and it
is not applied here because it would change three scenes already measured against a raster frame.

### a map can miss a feature the shader draws, and `foldyard`'s sun was one

**`foldyard`'s map carried no sun at all.** its disc is `smoothstep(0.99995, …)`, a cap of about
**0.573°**, and a `192 × 96` latlong texel is about **1.875°** across, so no sample could land
inside it. measured by sweeping the size and reading the disc term the brightest texel received:

| map | texel | the disc term |
|---|---|---|
| 192 × 96 | 1.875° | **0.0000** |
| 256 × 128 | 1.406° | **0.0000** |
| 512 × 256 | 0.703° | **1.0000** |

**the map is now `512 × 256`.** that is the first size whose texel centres can fall inside a
`0.573°` cap, and it restores the invariant the module states — the map is the same law as the
shader, term for term. the sun's own pixels then clip (`0.002%` of texels, brightest **1.716**),
which is the format's limit and not a scene's error.

**and no picture changed, which is the part that corrects an earlier claim here.** all three
`foldyard` cameras were rendered at both sizes and the sky band's maximum is the same to within a
level — **108 against 108**, **167 against 168**, **155 against 155**. the sun is at **28°
elevation** and **no `foldyard` camera frames it**, so the map was carrying a sun the picture had
never shown and never lost. what the defect actually was is a divergence between two readings of
one law, which is worth fixing on its own terms and is not worth a frame.

**one resolution does not serve both questions, and the two answers here disagree.** the lighting
stage wants a *blurred* sky — that is what a roughness-selected mip is for — and the post stage
wants the sharpest one it can carry, because it is drawing a picture. `terraces` was measured at
both `256 × 128` and `1024 × 512` and **the larger map bought nothing**, while `foldyard` needs
four times the linear size to hold a disc. a size cannot be chosen once for the gallery.

### the exported world is written in this engine's axes, and Cycles' are not the same

this engine is y-up and Cycles is z-up, and the format has no element that says which way up a
scene is. so **every coordinate the emitter writes is carried across**, by `cycles_export::axes`
and by nothing else:

```text
(x, y, z)  ->  (x, -z, y)
```

which sends this engine's `+y` (up) to `+z` and stays right-handed.

**positions and normals** are points and take it directly. a normal is not a point, but the basis
change is a rotation and a rotation carries a direction the same way it carries a point, so the
same permutation is right for both.

**a rotation takes it two different ways**, and this is the part that is easy to get wrong:

- a rotation that moves **points** — an instance's — is **conjugated**, `A R A^-1`, because the
  point is being placed in a world that has moved. On a quaternion that is the permutation
  `(qx, qy, qz) -> (qx, -qz, qy)` with the scalar part alone.
- a rotation that carries a **local frame both renderers already read the same way** — a
  camera's view, a light's beam — takes it on the **world side only**, `A R`. Conjugating one of
  these is not a near miss: the result is a valid rotation aimed `90` degrees from the place the
  document named.

`A R` is applied to the quaternion rather than to the Euler angles. the document's three angles
compose as `Rz(c) Ry(b) Rx(a)`, so a turn put on the left does not land beside the turn about
`X` it shares an axis with and cannot be folded into one of the three numbers.

measured before the change: a camera's up landed on Cycles' `+Y`, which is why the geometry read
correctly while the sky — which Cycles indexes by `+Z` — came out turned a quarter turn, and a
camera's pitch arrived as a roll. the rasterised path is in this engine's own axes and is
untouched by this; only what crosses the boundary moves.

### the latlong convention is the path tracer's

with the world converted, the path tracer's `v = acos(dir.z) / pi` puts **`v = 0`, which is row
zero, at the zenith**. this engine's latlong had row zero at the nadir and the WGSL reader agreed
with it, so a map was self-consistent here and upside down there. the two halves move together:

- **the row** runs zenith to nadir, and the poles clamp;
- **the column** is `floor(u * width)` with a wrapping modulo, where `u = atan2(-z, x) / 2pi`, so
  `u = 0` — the direction `+x` — is column zero and the seam is where the path tracer's is.

measured: with the seam shifted by half a turn the sky carried a hard vertical step where a
scanline jumped from `(28, 51, 108)` to `(76, 83, 123)`; with it moved, the same scanline is
symmetric and continuous.

see [.agents/investigate/cycles-world-axes.md](../../investigate/cycles-world-axes.md) for the
measurements and their sources, including the two sign mistakes that rendered without an error.
