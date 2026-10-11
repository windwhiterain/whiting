# Which way round is a gallery mesh's normal?

## The question

Measured in [investigate/cycles-mesh-normals.md](../investigate/cycles-mesh-normals.md): every
mesh in `landscape` exports a vertex normal array whose mean dot with the triangle's own
geometric face normal is **−0.99**. The rasteriser renders correctly from those normals, which
means its front-face convention is the opposite of the document's winding. Cycles follows the
document's winding and so shades every surface from the wrong hemisphere.

Fixing the attribute name and the `interpolation` flag makes the normals take effect — and
turns the massif black, which is what an inward-pointing normal looks like. Something has to
give. Three ways, and they are not equally good.

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

## What is recommended

**Option 3, with option 1 as the fallback if the gallery cannot be changed yet.**

The decision that is being made is *what a document's winding means*, and that belongs to the
document, not to a mapper. Option 2 records the disagreement in the exporter; option 1 records
it in an index order nobody will read. Option 3 removes it.

Whichever is taken, it must land **after** the two unambiguous changes, which are defects
regardless of this one:

1. the normal array is written as `standard_vertex_normal` and the loader reads `"N"`;
2. no `<state>` carries `interpolation="smooth"`, so the array would be read and then ignored.

Neither of those is a convention question. Both are a name and a flag that do not match the
format.

## What has to be measured before this is called done

- every gallery scene rendered on **both** backends, before and after, at the same resolution;
- the mean dot of exported normal with face normal reported **per mesh**, not for one scene —
  the statistic is what makes this checkable, and it is already written
  (`out/probe-mesh-normals.py`);
- at least one scene whose surface is **open and one-sided**, because a closed mesh hides an
  inverted normal from the eye far better than a heightfield does. `landscape`'s massif is
  closed; its plain is not, and the plain's number is the one to watch.
