# Which way round is a gallery mesh's normal?

> **Outcome: option 3 was taken, and it was taken in full.** The convention is stated in
> [SPEC/render/offline_backend.md](../SPEC/render/offline_backend.md#winding) — the authored
> normal is the outward direction and the winding obeys it — and nine builders were brought
> into line. All 620 meshes now agree with their own normals: 0 opposed, 0 split, 0
> malformed, measured by `out/probe-orientation.py` over freshly exported documents.
>
> **and the honest part is that it bought almost nothing.** every rendered frame changed and
> no scene moved: `tiles`, `sunlight`, `spotlight`, `lanterns` and `foldyard` are identical to
> four decimals, `landscape` gained 0.0014. A path tracer flips its shading normal towards
> the ray it is shading, so which way the corners were written is not a question it asks.
> With that settled the blocked emitter changes could land, and they are worth about the same
> — see [SPEC/render/cycles_backend.md](../SPEC/render/cycles_backend.md).
>
> **the survey that made this possible was itself wrong once.** The 111 / 127 / 382 split this
> proposal was written from came from one scalar per mesh. The distribution says 110 / 447 /
> 61 / 2, and 557 of the 620 are unanimous. The reasoning below is kept because the way it was
> wrong is the useful part: three times this gallery was summarised into one number and a
> story told about it.

## The question

Measured in [investigate/cycles-mesh-normals.md](../investigate/cycles-mesh-normals.md): the
mean dot between a mesh's exported normals and its own triangles' geometric face normals is
**−0.99 on some meshes and +1.00 on others**, and both appear within a single scene. The
rasteriser reads the normals and the tracer reads the winding, so the two backends currently
disagree about which side of every surface is lit.

Fixing the attribute name and the `interpolation` flag makes the normals take effect — and on a
mesh whose normals are negated, the surface turns black, which is what an inward-pointing
normal looks like. Something has to give. Three ways.

## The choices

### 1. Reverse the winding the emitter writes

`crates/scene/src/cycles_export/geometry.rs` emits triangle indices in the document's order.
Reversing two indices per triangle flips what Cycles computes for `Ng` without touching a
single number the document owns.

- **for**: it changes nothing the rasteriser sees, so its picture cannot move. It makes the
  *format's* geometric normal agree with the document's authored normal, which is the relation
  every Cycles tool assumes and the one our data violates.
- **for**: it is one index swap in one place.
- **against**: it hard-codes Cycles' winding convention into a module whose stated job is a
  mapping. If the format ever accepted the other order, this becomes a second thing to know.
- **against**: it is invisible in review. A reversed triangle list reads exactly like a correct
  one.

### 2. Negate the normals on export

`into_cycles_axes(normals)` gains a sign, alongside the permutation it already applies.

- **for**: the code says what it does at the point where the numbers are produced, and the
  comment at `geometry.rs:387-390` already argues that positions and normals need the same
  mapping — this is the same argument with its sign restored.
- **for**: it is one function, one testable expression.
- **against**: it leaves the two backends on opposite conventions forever. Every future
  contributor has to know that "our normals are the winding's negation, and that is fine".
- **against**: it papers over the real question. The disagreement is about what a document's
  winding *means*, and answering it in the exporter answers it in the wrong layer.

### 3. Change the document, and both renderers follow

The gallery's mesh builders emit triangles in an order that our own rasteriser reads as
front-facing and Cycles reads as back-facing. Whichever is the intended convention, the
document should state it once, and each backend should honour it.

- **for**: it is the only option where the two backends agree for a *reason* rather than by
  coincidence. It is also the only one that survives someone reading the exporter later.
- **for**: it fixes the class, not the instance — any future mesh built the same way is right.
- **against**: it touches the gallery's generators, which several sessions work on at once.
- **against**: it moves every scene's appearance on one backend, so it needs the whole gallery
  re-rendered before it can be called an improvement. That is a real cost, not a formality.

## What is recommended now

**Option 3 alone, and it is no longer a preference — options 1 and 2 are both measured wrong.**

The measurement above is what decides it. A mapper applies one rule to every mesh it writes; the
gallery does not have one convention to apply. Options 1 and 2 therefore cannot be right, and
the numbers say so. The remaining question is only *which* mesh builders are the minority.

So the work is now a survey, not a change:

- run the per-mesh dot statistic — it is written, `out/probe-mesh-normals.py` — over **every**
  gallery scene's exported document, not one scene;
- the builders on the **−1** side and those on the **+1** side will be different code, and the
  fix is whichever group is wrong;
- one scene is not evidence of a convention. `landscape` was −1 on all three meshes and that is
  exactly why it looked like a global fact and cost an afternoon.

## What has to be measured before this is called done

- every gallery scene rendered on **both** backends, before and after, at the same resolution;
- the mean dot reported **per mesh, per scene**, and the builders named by the pattern that falls
  out — that naming is the deliverable;
- at least one scene whose surface is **open and one-sided**, because a closed mesh hides an
  inverted normal from the eye far better than a heightfield does. `landscape`'s massif is
  closed; its plain is not, and the plain's number is the one to watch.

## What must not be done meanwhile

**Do not land changes 1 and 2 on their own.** Each looks harmless and neither is: with the array
renamed to `N` but no orientation settled, the massif renders at **−0.6122** against a baseline
of +0.7812, because the normals suddenly take effect and point inward. That is why change 1,
which is on its own an unambiguous name fix, is listed here as blocked rather than ready.

