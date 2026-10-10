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

## What it cost to get right

Three things were wrong on the first attempt and all three rendered without an error.

### the azimuth seam is where the path tracer's is

The path tracer's column is `floor(u * width)` repeating, with `u = atan2(y, x) / 2pi` running
`(-0.5, 0.5]` — so `u = 0`, the direction `+x`, is **column zero** and the seam is there. A
reader that adds a half turn first puts the seam at `-x` and every texel half a turn from where
the path tracer looks for it. Measured: with the seam shifted, the sky carried a hard vertical
step where a scanline across the frame jumped from `(28, 51, 108)` to `(76, 83, 123)`. Moving
the generator's seam to `turn = 0` and taking `floor` rather than truncating in the rasterised
reader made the same scanline symmetric and continuous: `(60, 77, 116)` down to `(27, 50, 107)`
and back to `(54, 72, 115)`.

### a quarter turn does not fold into one of the three Euler angles

The basis change is `Rx(90)`, and the obvious way to apply it is to add ninety degrees to the
first angle. That is wrong: the document's three angles compose as `Rz(c) Ry(b) Rx(a)`, so a
turn put on the **left** does not land beside the turn about `X` it shares an axis with. Applied
to the quaternion instead — `quarter_turn ⊗ q` — it is exact and does not care what order the
angles were composed in.

Two smaller variants of the same mistake, both of which ran and rendered:

- adding `FRAC_PI_2` to a field that holds **degrees** is a turn of one and a half degrees;
  the camera rotation moved from `135.4789` to `135.4736` degrees, which looks like nothing
  happened, and the frame came back of a wall because the geometry around it had moved.
- reading the composition as `Rx(a) Ry(b) Rz(c)` instead of `Rz(c) Ry(b) Rx(a)` swaps two
  components of the result. Both readings are self-consistent and differ only in which axis a
  two-axis rotation is taken about.

### the two kinds of rotation really are two kinds

A rotation that moves **points** — a mesh's vertices, an instance's translation — is
conjugated by the basis change, because the point is being placed in a world that has moved. A
rotation that carries a **local frame both renderers already read the same way** — a camera's
view axis, a light's beam, forward along `+Z` and up along `+Y` in both — takes the basis change
on the world side only. Conjugating a camera is not a near miss; the result is a valid rotation
aimed `90` degrees from the place the document named.

## The row convention moves with it

Once the world is converted, the path tracer's `v = acos(z) / pi` puts `v = 0` — and therefore
**row 0** — at the **zenith**. This engine's latlong had **row 0 at the nadir**, and the WGSL
reader agreed with it, so the map was self-consistent and was the path tracer's upside down.
The two halves move together or not at all:

- `latlong_direction`: `polar = ((row + 0.5) / height) * pi`, and the `z` component negated so
  that `atan2(-z, x)` is the azimuth;
- the WGSL reader: `turn = atan2(-direction.z, direction.x) / 2pi` with `floor` and a wrapping
  modulo, and `rise = atan2(horizontal, direction.y) / pi` with a clamping modulo.

## Verified

Measured 2026-10-10, after the change above:

- `foldyard` through `cycles.exe`: the sky is deep blue at the zenith fading to pale at the
  horizon, no seam, no hard edge, horizon level, shadows consistent with the sun.
- `tidewater`: a cold zenith, purple, an orange band along the horizon and dark ground below —
  which is what `sunset_pixels` draws, and what it did not draw before.
- the axis-ramp probe: `v` now varies with the screen's **vertical** axis and is constant along
  a horizontal line, which is what an equirectangular lookup must do. Before, it ran from
  `0.83` to `0.51` along one horizontal line.
- all twelve gallery scenes render through `device-cycles` with no failure and a non-zero
  maximum.
- the rasterised path assembles, validates and renders the new WGSL, and its sky now runs the
  same way as the path tracer's.

## Sources

- `out/latlong_probe.py` — writes the axis ramp and reads a frame back, in pure python.
- `out/probe-latlong.png`, `out/probe-map.png` — the two probe renders.
- `out/proj-foldyard.cycles.xml` — the emitted `foldyard` document the probes were derived from.
- `blender/intern/cycles/kernel/shader/node_tex_environment.cc`, `equirectangular()`.
