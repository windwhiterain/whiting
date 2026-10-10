# Which way up is the exported world, and what does Cycles do with it

**Measured 2026-10-10 on `cycles.exe` 5.3.0**, built from `blender/cycles` at
`a456b761034dda42c32eef9f4aae0fa5a5c9f604`, at `C:\resource\cycles\install\cycles.exe`.
Sources are the built binary's own behaviour, the emitted `foldyard` document, and the source
tree the binary was built from; where a claim is inference rather than measurement it says so.

## The question

The exporter writes a scene's coordinates into the document with no axis change at all. A camera
at `(28, 14.244, 34)` is written `translate="28, 14.244078695546408, 34"`, and the first three
vertices of the ground mesh are `-150 0 -150`, `-149 0 -150`, `-150 0 -149` — a plane of constant
`y = 0`. This engine is y-up. Cycles is z-up. What does that actually do to a render?

## What was measured

### 1. The sky map comes out turned a quarter turn

`out/latlong_probe.py` writes a 192 x 96 image whose **red channel is the column** and whose
**green channel is the row**, and the sky is sampled back out of a render of `foldyard` at 480 x
270. Nine sky samples, screen coordinates in fractions of the frame:

| screen | R (column) | G (row) |
|---|---|---|
| top-left | 0.33 | 0.83 |
| top-centre | 0.16 | 0.66 |
| top-right | 0.10 | 0.51 |
| bottom-right, at the horizon | 0.00 | 0.52 |

The green channel — the image's own **row** — runs 0.83 to 0.51 **along one horizontal line of the
frame**. In an equirectangular lookup the row is a function of elevation alone, and elevation is a
function of the screen's vertical axis, so a row of the frame can only ever carry one value of it.
The camera is level; the horizon is level in the render.

### 2. An explicit `<mapping>` node changes nothing

The same render with

```xml
<environment_texture name="world_texture" filename="ramp.png">
  <mapping node_type="ShaderNodeMapping">
    <vector_type type="TEXTURE" />
    <projection type="POINT" />
    <interpolation type="LINEAR" />
  </mapping>
</environment_texture>
```

returns **byte-identical** sky samples. So the lookup was already the identity on the
equirectangular `(u, v)` and declaring the mapping would have been decoration. Worth knowing
before anyone writes it down as the fix.

### 3. The numbers are exactly what a z-up world would predict

`foldyard`'s camera is emitted as `rotate="135.4789, -0.00621, -0.99987, -0.01518"` — an axis and
an angle, so 135.48 degrees about `(-0.0062, -0.9999, -0.0152)`, which is about **-Y**. Rodrigues
on that axis-angle:

- camera up `(0, 1, 0)` becomes `(0.0213, 0.9987, 0.0217)` — **+Y**, level, no roll;
- camera forward `(0, 0, 1)` becomes `(-0.7017, 0.0304, -0.7120)`.

Cycles' world lookup is `u = atan2(dir.y, dir.x) / 2pi`, `v = acos(dir.z) / pi` — **z-up**. The
scene's up is y. So Cycles computes elevation against an axis that lies across the frame. Feeding
the camera basis above into that lookup:

| corner | predicted `v` | measured `v` |
|---|---|---|
| top-left | 0.798 | 0.83 |
| top-right | 0.498 | 0.51 |

Agreement to the accuracy of the assumed 90 degree field of view and the sample position.

## Why the geometry reads correctly anyway

It is a coincidence that holds for the camera and not for the world. The camera's rotation is
almost entirely about Y, so its up stays on +Y, and Cycles draws +Y upwards in the image. The
ground is a plane of constant `y`, whose normal is +Y — so it reads as a floor. Nothing converted
any axes; the framing hid it.

The pitch does not hide it. `camera_rotation` returns Euler `[-pitch, yaw, 0]`, and the pitch is a
turn about **X**, which for this camera is a turn about its forward axis: a roll. `foldyard`'s
emitted pitch is `-1.74` degrees, so the error is small there, but it is not zero and it grows with
the pitch. A camera with any real downward tilt comes out tilted by that much in Cycles and level
in the rasterised path.

## What it costs to fix

The basis change that carries this engine's axes onto Cycles' is

```text
(x, y, z)  ->  (x, -z, y)
```

which sends this engine's `+y` (up) to `+z` and keeps the basis right-handed: the determinant of
`[[1, 0, 0], [0, 0, -1], [0, 1, 0]]` is 1. Checked on `foldyard`: the ground `(x, 0, z)` becomes
`(x, -z, 0)`, a plane of constant Cycles-z, which is the floor; the camera at `(28, 14.24, 34)`
becomes `(28, -34, 14.24)`, above it; its forward `(-0.605, -0.308, -0.734)` becomes
`(-0.605, 0.734, -0.308)`, which points down and towards the fold.

Rotations conjugate rather than translate. With `A` the matrix above and `R` this engine's rotation,
the emitted one is `A R A^-1`; a quaternion's vector part maps by `A` and its scalar part does not,
so the whole conversion is `(qx, qy, qz) -> (qx, -qz, qy)` in `transform::axis_angle` once the
quaternion is already built. In Euler terms `Rx(a) Ry(b) Rz(c)` becomes `Rx(a) Rz(b) Ry(-c)`.

## Also measured: the row convention disagrees too

Once the world is converted, Cycles' `v = acos(z) / pi` puts `v = 0` — and therefore **row 0** — at
the **zenith**. `shaders::latlong_direction` puts **row 0 at the nadir**, and the WGSL `latlong()`
reader agrees with it, so this engine's latlong is self-consistent and is the path tracer's upside
down. The two halves move together or not at all:

- `latlong_direction`: `polar = ((row + 0.5) / height) * pi` instead of `(1 - ...) * pi`;
- the WGSL reader: `down` loses its `1 -`, becoming `atan2(horizontal, direction.y) / pi`.

The azimuth mirrors with them. `latlong_direction` turns with `atan2(z, x)`; after the basis change
Cycles' is `atan2(-z, x)`, so the generator turns the other way, and the reader's
`atan2(direction.z, direction.x)` becomes `atan2(-direction.z, direction.x)`.

**Not measured**: the render after any of this. Nothing here has been applied yet.

## Sources

- `out/latlong_probe.py` — writes the axis ramp and reads a frame back, in pure python.
- `out/probe-latlong.png`, `out/probe-map.png` — the two probe renders.
- `out/proj-foldyard.cycles.xml` — the emitted `foldyard` document the probes were derived from.
- `blender/intern/cycles/kernel/shader/node_tex_environment.cc`, `equirectangular()`.
