# The pass-count guard has been red on main, so it has been guarding nothing

Measured 2026-10-12, on `main` at `d2ac734` and on `feature/environment-projection`.

## The question

`crates/scene/tests/clustered_lights.rs` fails on `main` with three rows out. Two of them are
scenes this work touched and one is not. Is the table stale, or is the emitter wrong?

## What the test counts

Its own documentation states the rule, and the rule is what makes the answer checkable rather than
a matter of taste:

> The **shadowing** number is the lights the pool bought pages for, times the groups of *shadow
> casting* geometry, so it is bounded by the pool rather than by the scene's light count.

> A shadowing pass is one per (light, shader, mesh) […] and the sun counts. […] a light that were
> wrongly given no page would emit **no** shadowing pass here at all, and the scene would render
> with no shadow in it and nothing to say so.

So for each scene the expected number is derivable from the lights it declares and the geometry it
draws, and the table should not need to be remembered.

## The three rows

| scene | expected | measured | derivation |
|---|---|---|---|
| `dunes` | 2 | **1** | one shadow-casting lamp (`light_with_radius`, named `sun`) and one shadow-casting group. The environment light casts none. |
| `landscape` | 4 | **2** | one directional sun, two shadow-casting groups. The environment light casts none. |
| `nurbs` | absent | present | the scene exists in the gallery and the test enumerates it |

**The measured numbers are the derivable ones in both cases, so the table is stale and the emitter is
right.** The assertion is not protecting the thing it was written to protect, and it has not been
protecting it for long enough that nobody noticed it was red — which is the more expensive half of
this.

The same failure appears identically on `main` and on the feature branch, so nothing this work did
caused it and nothing this work did fixed it.

## Why it was left alone

`nurbs` is the other session's scene: `feature/nurbs-multipatch` is checked out alongside, and that
row would be theirs to write. Correcting the other two leaves the test red for a reason that is
visibly not ours, which is the same state it is in now and one commit away from green. Correcting
all three writes a row for a scene another branch is still building.

Both are defensible and the choice is not one to make without being asked.

## A trap for whoever writes the row

`dunes` names a light `"sun"` and it is **not** a directional light — it is a
`light_with_radius`, a lamp with a position. Searching a scene for `directional(` finds `landscape`
and `sunlight` and misses `dunes` entirely. A pass-count derivation that trusted the name would get
this scene wrong in the opposite direction from the stale row.

## Not measured

- Whether the three rows are the only stale ones. The assertion stops at the first difference, so
  rows after `nurbs` in the list have not been checked against the emitter at all.
- Whether the test was ever green after `98226c3`, which is the last commit to touch it. **Not
  measured** — the history would answer it and nobody has looked.

## Sources

- The rule and the table: `crates/scene/tests/clustered_lights.rs`, the `Stages::of` documentation
  and the expected list at `:350`.
- The two scenes' lights: `crates/gallery/src/dunes.rs:144` and `:185`;
  `crates/gallery/src/landscape/mod.rs:679`.
- The failure on `main`, measured before and after this work with
  `cargo test -p whiting-scene --test clustered_lights`, identical both times.
- The parallel session: `git worktree list`, which shows `feature/nurbs-multipatch` checked out.