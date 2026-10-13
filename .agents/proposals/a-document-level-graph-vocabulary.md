# A document-level graph vocabulary for materials

## The question

`cycles-procedural-materials.md` established that this Cycles build registers and renders
`noise_texture`, `voronoi_texture`, `wave_texture`, `gradient_texture`, `mix_color`, `mix_float`,
`mix_closure`, `rgb_ramp`, `bump`, `math`, `vector_math`, `map_range`, `layer_weight`, `fresnel`,
`sky_texture`, `glossy_bsdf`, `diffuse_bsdf` and `transparent_bsdf`, and that **OSL is not present
in this build at all** — the capability is native, not compiler-backed. It named the obstacle
exactly:

> whiting currently emits one constant `principled_bsdf` per material and has no document-level
> vocabulary for a graph.

That was written as a design decision with no instance attached. **`terraces` is the instance.**
Its terrain declares no albedo at all — `crates/gallery/src/terraces.rs:446` is
`surface([1.0, 1.0, 1.0], 0.0, 0.55, [0.0; 3])` with the comment "the terrain's albedo is decided per
fragment by the surface" — and its colour, its water and its Fresnel mirror towards an analytic sky
function are all in `TERRAIN_SURFACE` at `terraces.rs:809-897`. The document can carry
`base_color = 1, 1, 1` and nothing else, so the traced frame is white snow.

## What the emitter would gain

A scene could say what its material *does* rather than what colour it is, and the document could
carry it. The vocabulary does not have to be Cycles' node set; it has to be small enough that a
scene author can hold it and expressive enough that a material which needs a Fresnel mix towards a
reflected sky can be written down.

**The smallest thing that carries `terraces` is four concepts**: a colour, a scalar, a ray, and a
closure. Fresnel is a scalar from a ray; the mirror is a closure mixed by that scalar towards a sky
colour sampled along the reflected ray; the ripple is a vector from two noises. Every one of those
has a node in the build.

## What the emitter would not gain

**A procedural material would stop being comparable for a different reason.** `terraces` is marked
not comparable because the document carries `base_color = 1, 1, 1`. With a graph it would carry the
albedo and the water — and the rasterised side would still be drawing a water mirror in WGSL while
the traced side derives one from `fresnel` and `glossy_bsdf`. The two would then agree on a picture
neither had before, which is the point, and the census would have to re-measure `terraces` rather
than inherit anything from the figure in
[the-census-and-what-each-baseline-answers](../investigate/the-census-and-what-each-baseline-answers.md).

**Nothing about the emitter's mapping of a constant becomes different.** A scene that names a flat
colour keeps naming a flat colour, and one constant per material stays the degenerate case of a
graph of one node.

## Decided

**The vocabulary is adopted as the direction; no code is written here.** The four painted layers
are kept — `cycles-procedural-materials.md`'s successors and the census's own table record why each
one exists and what it costs — and the four scenes are marked not comparable rather than being
changed to become comparable.

**A scene that wants its material carried writes a graph.** The emitter carries one
`principled_bsdf` per material today and would carry a `<shader>` node graph after this, with a
single-node graph meaning exactly what it means now.

## Not settled here

**The vocabulary itself** — its terms, its value form, whether a material names a graph or an
inline block, how a graph refers to a light or the camera. This is the design work the decision
opens, not part of the decision.

**Whether it lands in the scene schema or in the graphics schema.** The gallery has two and a
material belongs to one of them; which is a question about what else a material may reference.

**`monoliths` is not addressed by this.** Its problem is that the scene declares no environment
light and the post stage paints a sky, which is a statement about the scene rather than about its
material. Its materials do carry albedo.

**The three other painted scenes.** `landscape`'s ranges and shafts are not geometry, so no graph
carries them; `tidewater`'s aerial perspective is a fog law, which is a graph only in a sense that
has to be argued. Only `terraces` is a material, and only `terraces` is what this is about.