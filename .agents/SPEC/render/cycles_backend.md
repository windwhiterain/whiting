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
