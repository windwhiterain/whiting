# `sphere` builds a hemisphere, twice, and has never been a sphere

> **Fixed.** `phi` is now the polar angle measured from `+y` and the winding is one fixed
> order with no test in it. All 620 meshes still agree with their own normals, and the
> doubled surface is gone with it: the same-direction edge count falls to **zero** in all
> three scenes that use a sphere, and the branched count with it — `monoliths` 9440 → **840**,
> `stilllife` 5592 → **486**, `terraces` 288 → **66**. Agreement moves by nothing worth
> reporting: `monoliths` **−0.1682 → −0.1525**, `stilllife` **+0.4884 → +0.4720**,
> `terraces` −0.8265 → −0.8264, and the eight scenes that use no sphere are byte-identical.
>
> What follows is what was measured before the change, kept because the caller question at
> the end turned out to be the real one.
>
> Found while bringing the gallery into line with the winding convention in
> [SPEC/render/offline_backend.md](../SPEC/render/offline_backend.md#winding). The winding fix
> made the defect visible rather than causing it.

## What it does

`crates/gallery/src/geometry.rs:359`:

```rust
let vertex = |theta: f64, phi: f64| -> ([f64; 3], [f64; 2]) {
    let normal = [theta.sin() * phi.cos(), phi.sin(), theta.cos() * phi.cos()];
    (scale(normal, radius), [theta / TAU, phi / PI])
};
```

with `phi` swept from `0` to `pi` across `divisions` rows.

Two facts fall out of that one line, and neither is a matter of taste.

**`sin(phi)` is never negative over `[0, pi]`, so no point of the mesh is below the equator.**
Measured on `sphere(10, 1.0)`: all 600 corners have `y >= 0`. The function documented itself as
*"Builds a sphere"*, with the invariant *"the normal of a point is the direction it sits in from
the centre, so a sphere lit by its own normals is round rather than faceted"* — and half of it
was a hemisphere.

**`cos(phi)` changes sign at `pi / 2`, so the second half of the sweep retraced the first with
`theta` shifted by a half turn.** Same measurement: `sphere(10, 1.0)` emitted **600 corners at 51
distinct positions**, and the 60 corners that lie on the equator were only **10 distinct points** —
the rim was written six times over.

So the mesh was one hemisphere drawn twice, with a degenerate band at the pole between the two
copies. That is where the 800 same-direction edges in the survey's coherence column came from,
and it is why `monoliths` was 40 copies of an 800-triangle mesh with only 288 distinct vertices
welded.

## What it cost

- **Twice the triangles for the same picture.** A closed `sphere(20, r)` is 800 triangles; this
  one was too — but half of them landed on a surface that was already there, and they shaded it.
- **A pole band that carried no orientation.** The row whose ring was `phi = pi / 2` had four
  coincident corners per cell, so its second triangle had two identical corners. Measured on
  `monoliths`: those triangles sat at an area of **3.7e-19** where the mesh's smallest real face
  is **1e-3**.
- **A shape nobody chose.** `terraces` calls `sphere(10, 1.0)` for a fir's `crown` and
  `stilllife` calls `sphere(36, ·)` for a `lemon`. A lemon is not a hemisphere drawn twice, and
  neither is a crown a reason to spend 2592 triangles on.

## What the fix was

Take `phi` as the polar angle from `+y`, with the azimuth measured from `+x`:

```rust
let normal = [phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin()];
```

`dP/dtheta x dP/dphi = sin(phi) * n` is outward for every `phi` in `(0, pi)`, so the grid is
uniformly wound, the two ends of the sweep are the two poles, and the conditional the winding
fix had introduced goes away.

**the spelling is not interchangeable.** `(sin theta sin phi, cos phi, sin theta cos phi)` reads
the same and comes out as `-sin(phi) * n`, which is a sphere whose hemispheres are wound against
each other — measured at 56 triangles agreeing and 52 reversed where the correct spelling gives
180 agreeing and 0 reversed.

## the question this does not answer

`sphere` now returns the sphere its name and its invariants have always claimed, and **three of
its four callers may not have wanted a sphere.** `terraces` places a fir's `crown` at
`ground + 1.4 * scale` with a scale of `(1.9, 1.5, 1.9)`, above two stacked cones; as a
hemisphere it sat on top of them, and as a ball it hangs half into them. `stilllife`'s
`spoon-bowl` is named for a bowl. Whether each wanted the whole ball is a question for those
scenes, and it is now a question that can be asked — the shape is honest either way, which is
what made the difference between the two answers invisible before.