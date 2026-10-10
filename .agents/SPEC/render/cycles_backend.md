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
`emissive` and `reflect` have none that means the same thing and are reported rather than
dropped, because a tint that silently failed to apply is the same invisible-wrong-image failure
this backend is arranged to prevent.

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

### what is actually left is shading, not the camera

`landscape`, `lanterns`, `spotlight`, `sunlight` and `tiles` still do not match, and the camera is
**not** why. all five write a final eye, a final target and a final angle through
`camera_placed`, which is the same thing both backends read. measured on `landscape`:

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

a second sweep on 2026-10-10 confirmed the node the design leans on is really there, rather than
assuming it from the ones already exercised: `geometry`, `attribute`, `camera`, `object_info`,
`white_noise_texture`, `combine_xyz` and `separate_xyz` are all registered alongside the nodes listed
above. `new_geometry`, `musgrave_texture`, `fractal_noise`, `cell_noise`, `bright_contrast` and
`hsv_rgb` are **not**, so nothing in the design may rest on them.

### how a procedural surface is carried, and what the fallback is

decided 2026-10-10: the route is a **backend-neutral shader IR, compiled to the target's own node
graph**. OSL is recorded as the fallback, not the plan.

the IR is deliberately backend-neutral, because that is what keeps the fallback cheap. its node names
describe what a node *does* and never name a Cycles node, so the same graph can be compiled to a
Cycles node graph today and to OSL source text later. anything that puts `principled_bsdf` or
`noise_texture` into the IR has made the fallback a rewrite instead of a second backend, and must be
treated as a design error rather than a shortcut.

the reason OSL is not the first choice is measured, not aesthetic. it is real and maintained — the
Academy Software Foundation project, with 1.15.7.0 released 2026-09-07 — and Cycles lists it as an
optional dependency that ships inside the precompiled libraries, so enabling it is a CMake flag
rather than a patch. what it costs is not the flag. it is that whiting would have to **rewrite** the
five shaders rather than compile them: WGSL and OSL are different languages, OSL has no light loop
and requires the surface to be built from closures, and `n`/`p` reach the shader by different routes.
The IR is the part that survives either choice, so it is built first.

two limits of the OSL fallback are worth stating now rather than discovering later. Cycles runs OSL on
**CPU only** — the GPU path is an unenabled patch — which costs us nothing, because we render on CPU
already. And `oslc` is a real compiler with real diagnostics, which is a genuine advantage over the
node graph, where a mistyped **node attribute** is ignored rather than refused.

### a material has two forms, and they are two implementations rather than one compiled twice

decided 2026-10-10: a material may declare a **rasterised** form and a **traced** form separately.
The rasterised form is what already exists — WGSL text in a shader's `source`. The traced form is a
node graph, carried beside it. A material that declares no traced form is emitted as the single
constant `principled_bsdf` it is emitted as today, so every existing scene keeps working and this is
purely additive.

**this needs no exception to the rule that a document carries no logic**, because the scene schema
already carries a shader's own text, and says why:

> The shader source. A shader carries its own text so that a compiled scene renders wherever it is
> sent.

A node graph is the same kind of thing for the same reason. The field is separate rather than a
variant of `source` because the two are different languages of different shapes, and one field
holding both would be a schema about their union rather than about either.

the traced form is written in this repository's own operation set, held in `crates/shader`, which is
built as Rust and **serialised into the document**. That split matters: the Rust side is where a
surface is authored, and it is where a mistyped operation or a wire between two mismatched types is
refused at build time — because the alternative was measured, and the target ignores what it does
not recognise rather than complaining. The document side is where it travels, so that a compiled
scene still renders anywhere a rasterised one does. `crates/shader` names no node from any target,
which is what keeps a future OSL form a second backend rather than a rewrite.

**the two forms will not agree pixel for pixel, and that is not a defect to be fixed.** This is the
load-bearing sentence of this decision. A rasteriser and a path tracer resolve a surface by different
means — one interpolates across a triangle and shades once per fragment, the other samples the
surface repeatedly per ray — so **the same material definition still renders two different pictures**.
The two forms buy that the *structure* crosses the boundary: the snowline is driven by height in
both, the rock is mixed by steepness in both, and a person changing one can see what the other is
doing. A single shared representation would not make the images agree either, and would cost the
rasteriser its written shader, its comments and its bindings.

the corollary is that **"the two backends agree" is not a testable claim at the pixel level**, and
neither is a difference between them a defect. Comparing the two frames is for seeing whether a
surface has the right shape, and for telling a change of shading from a change of framing.

this also settles the noise. A `fract(sin(dot(p, k)) * 43758.5453)` hash over lattice cells near 56
gives `sin` an argument near 24,600 radians, whose `f32` representation already carries about 0.002
of error; the multiply amplifies that to about 87 before `fract` turns it into an unrelated number.
Two implementations agreeing to one ULP still produce uncorrelated noise, so **porting that hash buys
nothing and costs the stability it appears to have.** The traced form uses the target's own noise
node, and the rasterised form keeps the hash it has always had.

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
