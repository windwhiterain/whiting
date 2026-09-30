# Real camera and light matrices in the standard pipeline

## What the compiler emits today

Every pass the standard pipeline emits carries an identity view projection, and a
shadowing pass carries the unit vector from the origin toward its light. The arithmetic is
in `scene/src/standard/math.rs` and it is deliberately the smallest thing that makes the
uniform well formed: a pass has a uniform, and the uniform has numbers in it.

The reason it stops there is that a real projection is not arithmetic, it is a decision.
An orthographic light frustum needs an extent and a near and far plane; a perspective
camera needs a field of view and a near and far plane; the scene document has none of
those, and inventing them in the compiler would put a camera where the author did not ask
for one.

## What the scene document would need

- a camera, with a projection and a transform, either in the scene document or derived
  from a light when there is no camera;
- a near and far plane, or a convention that reads them from the shadow extent;
- the light frustum, which is an orthographic box around the shadow extent.

The `shadow_extent` a light already carries is the one input that is there, and it is
enough for a light frustum if the near and far planes follow a stated rule. That rule is
the thing to decide: fixed planes, planes scaled by the scene's bounds, or planes the
author writes per light.

## What it buys

Without a real projection the emitted pipeline is structurally right and visually wrong:
every mesh lands on the same plane and every light is a point light in the identity view.
The pipelines that read these documents are the thing this workspace is for, so the gap
is worth closing, but it is a scene schema change and therefore the superior's call.
