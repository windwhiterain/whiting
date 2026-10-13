# The Cycles emitter has no tests, and the regression that proves it was not a compile error

## What happened

`a81b203` changed the camera the exporter writes: it rewrote the rotation as `S * R * S`, in the
axis-angle form `(w, v_x, -v_y, -v_z)`, to cancel a world-side scale that the loader does not
compute. Every camera in the gallery was turned away from its subject by
`2 * atan2(forward.x, forward.z)`. `sunlight` went from drawing 59.597% of its frame to 10.372%;
`foldyard` lost its courtyard entirely.

**It was reverted in `afb0c0b` and the gallery is back to its 2026-10-12 census figures to the
digit.** What is written here is not the fault — that is recorded in
[do-the-backends-read-one-environment-the-same-way.md](../investigate/do-the-backends-read-one-environment-the-same-way.md).
This is about why nothing caught it.

## Why nothing caught it

`crates/scene/src/cycles_export/` has **no test anywhere in the workspace**. Not the camera, not
the axes, not the transform composition, not the geometry, not the lights, not the materials. The
module is reached from exactly two places, `crates/scene/examples/emit_cycles.rs` and
`crates/device-cycles/src/render.rs`, and neither asserts anything.

So the change was made, **110 tests stayed green**, and the only thing that noticed was a manual
census of eight rendered scenes — which reported the damage as an **improvement**, because five
cameras turned away from their subjects put different ground in the frame.

**The census was not fooled by chance.** It compared a ratio and a correlation between two
rendered frames, and both are insensitive to the specific thing that had gone wrong in the way it
was being read. A test on the emitted document would have caught it before any render ran.

## What the test would be, and why it needs no Cycles

The fault was a claim about **how a third-party loader composes a transform**. That claim is
checkable as arithmetic on the emitted XML, with no Cycles process and no render:

```text
the loader:  tfm = tfm * translate;  tfm = tfm * rotate;  tfm = tfm * scale
so a top-level transform is  T * R * S
the camera's basis is the columns of R * S, which for S = diag(-1,1,1) is
    forward = R * (0,0,1)
    up      = R * (0,1,0)
    right   = -R * (1,0,0)
```

**A test that composes the emitted `translate`/`rotate`/`scale` the way the loader does, and
compares the camera's forward against the eye and target the document names, fails on `a81b203`
and passes now.** It is about fifteen lines, it runs in milliseconds, and it needs a hand-written
scene document rather than a gallery scene — the same shape as
`out/probe-camera-calibration.cycles.xml`, which settled the same question by rendering.

## What was measured, and the correction it forced

The first version of this section claimed that **three** properties fail during the regression and
one of them was the determinant. **Run against both documents — the current export and the
regression's, which differ only in the camera's rotation — that is wrong.**
(`out/probe-camera-basis-properties.py`.)

| | current export | the regression's |
|---|---|---|
| `det(basis)` | **−1.0** | **−1.0** — *the same* |
| `up · world_up` | **+0.9809** | **+0.9809** — *the same* |
| forward against `normalize(target - eye)` | **+0.0000°** | **+66.9472°** |

**Two of the three held throughout the regression, and only the forward identifies it.** That is
worth stating plainly because the two that held are the two that look like the diagnosis:

- **`det == -1` guards the mirror, not the aim.** The determinant is −1 whenever a scale of
  `diag(-1, 1, 1)` is written at all, whatever the rotation says. It is the right assertion for
  *"did anyone drop the mirror"*, and it would have stayed green through the whole of `a81b203`.
- **`up · world_up` guards a tipped frame**, and `a81b203` tipped nothing: it mirrored in `x`,
  which is a turn about the view axis, not a roll.

**The assertion that catches it needs the scene's own eye and target**, so it cannot be a property
of the transform alone. It is a comparison between two things, which is also why no amount of
looking at the emitted camera's own numbers would have found this.

## What else in the emitter deserves the same treatment

The camera is the case where a wrong answer still looks like a picture. The others are cheaper and
should be pinned for the same reason — a wrong answer there is a wrong document:

- **the basis change** (`axes.rs`). `position` is `(x, -z, y)`; `rotation_frame` and
  `rotation_points` are different on purpose, and `Rotation::{Points,Frame}` exists because getting
  that wrong aims a camera 90 degrees off. Both are worth a round-trip property.
- **a mesh round trip.** Every vertex and normal through `into_cycles_axes` and back must be what
  went in. One facet of a box is enough and catches a transposed permutation immediately.
- **a dropped field is reported.** The emitter has a `LossyReport` and nine fields are dropped on
  `sunlight`; a test that asserts the report is non-empty for a scene that declares them keeps the
  report from rotting into always-empty.

## What is not proposed

**No test renders.** A test that starts `cycles.exe` is a test that needs the binary, a GPU or a
long timeout, and this repository already has eight rendered scenes that only a person checks.
Every assertion above is arithmetic on a document.

**No golden files of whole documents.** A whole document changes whenever any scene changes, and
the failure message would be a diff rather than a property. The three assertions in the table above
say what actually went wrong and survive everything else.

## What has been written, and what has not

**One of these is now a test: `crates/scene/tests/cycles_camera_forward.rs`, merged as `252043c`.**
It is the forward assertion and nothing else, on the terms this file argued for. It composes the
emitted transform the way `xml_read_transform` does and compares the camera's `+Z` column against
`normalize(target - eye)`, and it was checked against both documents by checking `a81b203`'s
`scene.rs` out and running it:

```text
current export      +0.00000000 degrees
the regression's    66.94721815 degrees
```

The second figure is the one `out/probe-camera-basis-properties.py` reached by hand, reached again by
a different route — the probe composed vectors, the test composes matrices and compares directions,
and they agree to eight figures. `whiting-scene` is 111 tests, from 110.

**The other two are still not written, and the reasons they were not are not all the same.**

- **`det(basis)` and `up · world_up`.** The table above says these hold through the regression, so
  writing them now would be pinning two properties nobody has watched fail. They guard two real and
  different things — *did anyone drop the mirror* and *did anyone tip the frame* — and the honest
  order is to write them when one of them has been caught failing once.
- **the `det(basis)` assertion is the wrong shape anyway.** A determinant is a property of the
  transform alone, so it can be asserted on the emitter's own output without naming the scene's eye
  and target. That is what makes it cheap and it is also why it would have stayed green through the
  whole of `a81b203`.

**The other three items in this file — the basis change round trip, the mesh round trip and the
non-empty `LossyReport` — are untouched and would need permission again.**