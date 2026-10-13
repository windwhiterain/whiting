# Which geometry-stage bodies decide what a fragment looks like from its distance to the eye?

**Question.** `SPEC/render/cycles_backend.md` decides that atmosphere belongs to the post stage,
and on 2026-10-13 that decision was restated as a rule about stages rather than about names: *an
atmospheric effect may not be implemented on the geometry's own material; it must be a separate
post process.* A rule about names cannot be applied to that question, because a scene is free to
name its air whatever it likes. So: **which bodies in the gallery decide a fragment's appearance
from the distance between that fragment and the eye, and what do they decide?**

**Answer, measured 2026-10-13** against `.worktrees/terraces`, the worktree that rendered the
frames in [every-sky-agrees-and-the-ground-does-not.md](every-sky-agrees-and-the-ground-does-not.md).
27 source files under `crates/gallery/src`, walked recursively. **Three bodies, in two scenes:**

| scene | body | stage | what it decides from the distance |
|---|---|---|---|
| `tidewater` | `SHORE` | lighting | mixes toward `aerial_colour` by `aerial_penetration`, clamped at `0.985` |
| `tidewater` | `water_body()` | emission | mixes toward `aerial_colour` by `aerial_penetration`, clamped at `0.97` |
| `monoliths` | `SURFACE` | lighting | mixes toward the local literal `wet` by `1 - exp(-from_eye * 0.004)`, weighted by the material's own `amount` |

**Reproduction.** `out/survey-distance-mix.py`, gitignored, so it is not the evidence — the
source lines it prints are. It reads the gallery's source rather than the built scenes, because
the question is which bodies *mention* a distance, and a body that mentions it in a comment is
worth knowing about too.

## Why the earlier wording could not be applied

`cycles_backend.md:501` used to forbid a surface body reaching for `air_colour()` or
`air_closure()`, the two names `air.rs` exports. **`monoliths` and `tidewater` both satisfied that
wording while both did the thing**, and neither file so much as names `air.rs`:

- `tidewater` reaches for `aerial_colour()` and `aerial_penetration()`, its own helpers in its own
  `AERIAL_WGSL` (`crates/gallery/src/tidewater.rs:560`).
- `monoliths` reaches for neither: it mixes toward `wet`, a `vec3` literal declared two lines
  above the mix (`crates/gallery/src/monoliths.rs:220`).

A survey that searched for the shared module's two spellings returned "compliant" for both. The
clause's own three reasons — a post stage cannot correct it, a tone curve cannot, a second backend
cannot reproduce — applied to both all the same, so the document was stating reasons it did not
enforce.

## Three blind spots in the survey, each of which would have reported a clean scene

These are recorded because each produced a **negative** result, and a negative result is the kind
that has to be earned.

1. **Only `const NAME: ... = r#"` bodies were candidates.** `tidewater`'s water shader is built
   inside a function and handed to `wgsl!`, so it was not scanned at all — the lake did not appear
   on the first run although its own emission body mixes toward `aerial_colour` four lines from the
   end. Bodies are now found by the raw string itself, whatever precedes it.
2. **The walk listed the top level and kept the `.rs` files.** `landscape` is a **directory**
   (`crates/gallery/src/landscape/mod.rs`, named in the SPEC's own account of `e447f85`), so
   `landscape` — the scene with the most measured surface-body history in this chapter — was never
   a candidate. The walk is now `os.walk`.
3. **A bare `to_eye` was counted as a distance.** `stilllife` writes `to_eye` in four bodies as a
   normalised **direction**, reflected off the surface, and mixes colour by `grain`, `woven`,
   `pool` and `skin` — material patterns. Four reflections were reported as four pieces of
   atmosphere. `to_eye` is no longer a distance signal; `from_eye`, a distance against the eye
   position, and `length(eye - in.world_position)` are.

The search is also deliberately not `mix(` alone: a body can multiply colour by a distance factor
rather than mix toward a colour, and that is the form most likely to be written by someone who is
not thinking of what they are writing as atmosphere. The reported lines are every line that decides
`color` or `emitted`.

## What post has, and what it has not

This is why moving the three bodies is not mechanical, and it is measured from the shared template
rather than assumed.

A post stage **recovers where a fragment was**. `ray_through(in.uv)` (`shaders.rs:2619`) gives the
direction through the pixel, `distance_to(depth)` (`shaders.rs:2642`) gives the distance along the
view from the depth buffer and `uniforms.depth_range`, and `uniforms.camera.position` gives the
eye. The world position, and therefore its height above the water, follows from those three.

A post stage has **no normal**. There is no normal attachment on the post pass's inputs —
`color_target` at binding one, `depth_buffer` at two, the environment map at three
(`shaders.rs:2586-2589`). So a law whose weight is a function of the surface normal cannot move as
written.

`tidewater`'s is: `let facing = 1.0 - clamp(dot(normal, view), 0.0, 1.0);` feeding
`slant = 1.0 + facing * 2.4` (`tidewater.rs:579`). **It is replaced rather than dropped.** An
exponential-height medium has a closed form for the path length along a segment, and integrating
that is what the `facing` factor was standing in for; the post version needs only the ray and the
endpoint's height, both of which it has. That is a reason to prefer the post statement, not a
concession.

## Not settled here

**Whether the ground rows move.** `monoliths` measures `0.971` and `tidewater` `0.699` on the
census, so neither is currently paying a visible price for painting its own air, and the tracer
cannot have air over geometry in either case: this build registers no `volume_absorption` or
`volume_scatter`, and `principled_volume` leaves a 360000 pixel frame bit-identical at a density of
8.0 ([this-build-cannot-express-a-volume.md](this-build-cannot-express-a-volume.md)). Moving the
law is therefore a statement about **where the air is written down**, not a measurable gain against
the other backend, and this note does not claim one.
