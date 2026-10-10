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

**atmosphere belongs to the post stage**, for the same reason. haze, aerial perspective, the sky
itself and the far ranges were already the grade's alone; what changed is that no surface body is
allowed to reach for `air_colour()` or `air_closure()` and mix toward it. a surface shader that
paints its own fog is a surface whose fog a post stage cannot correct, a tone curve cannot, and a
second backend cannot reproduce.

this does not need an exception to **a document carries no logic**. the scene schema already carries
a shader's own text, and says why: *"a shader carries its own text so that a compiled scene renders
wherever it is sent."* an empty body is the limit case of that, not a departure from it.

measured on `landscape` after the change, 1024×1024, 32 samples on CPU:

| claim | measured |
|---|---|
| composition and silhouette | **identical on both backends** — the camera section above is satisfied for this scene |
| surface brightness | raster 162.3 mean luma, cycles 70.9, over 3295 paired surface samples |
| sky | raster painted by the grade, cycles black — the document declares no `<background>` |

the two rows that still differ are **not** shading defects and are recorded as open rather than
fixed here. the brightness gap is the two lighting models — the rasteriser's own Lambert and specular
against the tracer's integral — and it is the same class of difference as
[no view transform](#no-view-transform). the missing sky is the grade's, and a scene that wants a
world declares one.

the stripes along the massif's flanks are **geometry** — the rib construction in
`crates/gallery/src/massif.rs` — and both backends draw them. they were briefly mistaken for
residual striation from the deleted per-block stone hash; they are not, and a crop of both frames
settles it.

what is **not** claimed is that this makes the two backends agree. it makes them agree about
**framing and geometry**, which is what the geometry pipeline is responsible for. per-pixel agreement
was never available — see [no view transform](#no-view-transform) — and is not what this change
sets out to buy.

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
