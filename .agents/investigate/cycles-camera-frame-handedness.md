# Which way round is the exported camera, and why does Cycles need a mirrored frame

**Measured 2026-10-10 on `cycles.exe` 5.3.0** at `C:\resource\cycles\install\cycles.exe`,
SHA-256 `8316E615CB9046D4320F375085CBF24783D09C9955513F759F579E1A54F2D28C`, against the source
tree it was built from at `C:\resource\cycles\src`. Every number below is either read out of a
render or read out of that source; where a step is inference it says so.

## The question

The `landscape` scene came out of the two backends as two pictures that were each other's mirror
image. The frame's **up** was right, its **forward** was right, the sun's travel was right and
every vertex was in the right place. What, exactly, was wrong?

## What was measured

### 1. The two frames really are exchanged, not merely different

Pearson correlation of the rasterised frame against the path-traced one, on luminance, and then
against the same frame with its columns reversed:

| band | as-is | flipped |
|---|---|---|
| whole frame | `+0.0392` | `+0.1160` |
| ground, below the sky | `+0.1488` | `+0.6510` |

The sky is reported apart because it is a different kind of thing — the rasterised path draws it
from a gradient and Cycles has no environment to sample yet — so it drags the whole-frame number
down for a reason that has nothing to do with the camera.

### 2. Two markers, one on each side, land on exactly opposite sides

The only check that needs nobody to agree about a convention. Two emissive triangles are put in
world space on the **rasterised** camera's right and left, and both backends render:

| marker | rasterised | Cycles |
|---|---|---|
| camera's right | `x = 0.794`, right half | `x = 0.240`, **left half** |
| camera's left | `x = 0.190`, left half | `x = 0.818`, **right half** |

Not "roughly swapped". Swapped.

### 3. Cycles draws the camera's local `+X` on the right

A document holding nothing but a camera and three triangles, with the camera at `(0, 0, 10)` and
`rotate="180, 0, 1, 0"`:

| triangle | screen `x` |
|---|---|
| at the origin | `0.498` |
| at world `+X` | `0.027` — **left half** |
| at world `-X` | `0.969` — **right half** |

So with the camera turned to face `-Z`, its own `+X` — which now points along world `-X` — comes
out on the **right**. Cycles is drawing local `X` to the right of the frame.

### 4. Nothing on the way from the raster to the screen flips a sign

`Camera::update`, `src/scene/camera.cpp`:

- `:236-249` — `ndctoraster`, `screentondc` and the two `rastertoscreen` terms are built from
  `transform_from_viewplane` and `transform_scale`, both diagonal and both positive. The view
  plane's two components are positive (`src/scene/camera.cpp:206-209`).
- `:265` — `rastertocamera = screentocamera * rastertoscreen`.
- `projection_perspective`, `src/util/projection.h:265-266` — sets `w = z` and leaves the `x` and
  `y` rows as the identity, so only `z` is signed by the near plane.

This was the one link in the chain still unverified when the answer turned up, and it is clean:
**the mirror is not introduced inside Cycles.** It is in the basis handed to it.

### 5. The camera matrix is stored verbatim

`Camera::set_matrix` is the generated socket setter over `NODE_SOCKET_API(Transform, matrix)`
(`src/scene/camera.h:151`). There is no normalisation, no orthonormalisation and no rotation
extraction anywhere on the path, so a matrix whose determinant is negative reaches the kernel
unchanged. This matters: it is what makes the fix possible at all.

### 6. The emitted basis had the opposite handedness, and it must

On `landscape`, in Cycles' own axes:

| column | emitted | `cross(up, forward)` |
|---|---|---|
| `localX` | `[0.9913, -0.1313, -0.0000]` | `[0.9914, -0.1313, -0.0000]` |

`localX` was `cross(up, forward)` to four decimals, and `det(localX, localY, localZ) = +1`.

That is the wrong sign, and it is forced to be. For orthonormal columns `a, b, c` of a rotation,
`b × c = det · a`. A camera that looks along its own `+Z` and whose `+X` is its own right-hand
side needs

```
a = cross(c, b) = cross(forward, up)
```

which is `-cross(up, forward)`, so `det = -1`. **A camera whose `+Z` is forward and whose `+X` is
screen-right is necessarily left-handed.** This is not a defect in the exporter's arithmetic; it
is the shape of the thing Cycles reads. Blender never has to state it because Blender's own
camera looks along `-Z`, and Cycles applies the flip internally — `blender_camera.cpp` does
`result = tfm * transform_scale(1, 1, -1)` — which is why every stock Cycles scene is fine.

## Why the direction checks could not see it

Every direction quantity read correct while the picture was mirrored: camera forward against the
scene's intent `+0.9997`, camera up against world up `+0.9981`, the sun's travel `+1.00000`, and
the vertices read back with `y` as height and the right magnitude. `forward` and `up` together
fix two of the three columns. The third is fixed only up to a sign, and the sign is handedness —
the one quantity none of those checks carries.

## What was tried and rejected

Each of these was built and rendered. None is the fix:

| attempt | measured outcome |
|---|---|
| half a turn about the view axis | `right` and `up` both flip; `up · world-up = -0.9981`, the frame comes back upside down |
| half a turn about local `Y` | `up` is kept and the sides line up (`dot = +0.9998`), but the camera turns away and the markers leave the frame |
| half a turn about local `Z` | markers to the left and inverted |
| conjugating by the basis change instead of left-multiplying | markers at `0.129` and `0.649` — worse |
| flipping only the geometry's `x`, then `y`, then `z` | markers stay in the left half |
| rebuilding the basis from the original forward and up | `dot = +1.0000` against the emitted value: the original basis was already standard, which is the tell that the answer is not in the rotation |

Negating `right` while holding `up` is a **reflection**, determinant `-1`. No rotation is a
reflection, which is why every entry above failed: a half turn cannot flip one column and leave
the other two.

## The fix, and what it measured

The reflection rides in the transform's `scale`. The loader composes `matrix`, then `translate`,
then `rotate`, then `scale` (`src/app/cycles_xml.cpp:684-711`), so a scale written last
multiplies the rotation's columns from the right — it renames the local `X` and leaves `Y`, `Z`
and the translation alone. `scale="-1, 1, 1"` is written on the camera and on nothing else;
lights keep proper rotations, because a light has no left and right (`src/scene/light.cpp:251`,
`:312` take `-column 2` and the other two columns are free).

Measured after, `landscape` at 1024×1024, 32 samples on CPU:

| | before, as-is | before, flipped | after, as-is | after, flipped |
|---|---|---|---|---|
| whole frame | `+0.0392` | `+0.1160` | **`+0.5119`** | `+0.1987` |
| ground band | `+0.1488` | `+0.6510` | **`+0.6499`** | `+0.1478` |

The two columns exchange places rather than merely improving, which is what "this is the same
picture, no longer mirrored" looks like numerically.

And the markers, this time placed by `cross(forward, up)` — the rule `standard::math::look_at`
uses, which says nothing about handedness — so the check no longer rests on any assumption:

| marker | rasterised | Cycles, after |
|---|---|---|
| camera's right | `0.794`, right half | `0.775`, right half |
| camera's left | `0.190`, left half | `0.228`, left half |

`localX` is now `[-0.9913, 0.1313, 0.0000]` and `cross(forward, up)` is
`[-0.9913, 0.1313, 0.0000]` — equal — where before they were exact negatives. `up · world-up` is
still `+0.9981`, so the reflection took the one column it was meant to.

## Sources

- `out/probe-agreement.py` — correlation against the column-reversed frame, band by band.
- `out/probe-marker.py` — injects two emissive markers into the emitted document and reads back
  the column each one lands in. A `<mesh>` needs `nverts` and `verts` or Cycles does not
  recognise it, and a marker at emission strength 40 clips to white and cannot be told from sky;
  strength 1.0 and a per-channel threshold read it cleanly.
- `out/mf-cycles.cycles.xml` — the emitted `landscape` document the probes were run against.
- `C:\resource\cycles\src\scene\camera.cpp` — `:236-249`, `:265`, `:276`, `:331-336`.
- `C:\resource\cycles\src\scene\camera.h:151` — the camera matrix socket.
- `C:\resource\cycles\src\util\projection.h:265-266` — `projection_perspective`.
- `C:\resource\cycles\src\app\cycles_xml.cpp:684-711` — `xml_read_transform`, including `scale`.
- `C:\resource\cycles\src\scene\light.cpp:251`, `:312` — a light travels along `-column 2`.
- `developer.blender.org`, `blender_camera.cpp` — the internal `-Z` flip, read from a search
  result and **not** re-read from source here; it is context for why stock scenes are unaffected,
  and nothing in this file's conclusion rests on it.
