# The lighting pass is not depth tested, and every piece of it except the attachment says it should be

A depth prepass runs for every gallery scene, writes a depth buffer, and the shading stage never
reads it. Nothing in the renderer reports this, and the resulting images look like a deliberate
style until a scene puts a saturated surface in front of a brightly lit one.

Found while building the spot light gallery scene. It is not caused by spot lights and it is not
new; it is a pipeline defect that the spotlight scene makes visible and that `dunes` makes
unmissable.

## What is wrong

`lighting_pass` in `crates/scene/src/standard/emit.rs` names one target:

```rust
vec![COLOR_TARGET],
COLOUR_CLEAR,
lighting_state(),
```

`WgpuRenderer::attachments` resolves a pass's attachments from its **targets**, and
`raster.rs` builds the pipeline with `depth_stencil: depth.map(...)`, where `depth` is the
attachment whose format is a depth format. `COLOR_TARGET` is a colour format, so a lighting
pass has **no depth attachment and therefore no depth state at all**, however it declares one.

## The three pieces that already say the opposite

Every part of the design that is not the `targets` line assumes a depth tested pass. Taken
together they are not a plausible oversight of one line; they are a design that was written down
and then not finished.

- **`lighting_state`** (`emit.rs`) declares `depth_write: false` and `depth_compare: LessEqual`,
  with a paragraph arguing at length why `Less` would reject every fragment of a prepassed
  surface and why the pass must not write. A pipeline with no depth attachment cannot honour
  either field.
- **The WGSL template reserves the binding.** `lighting_bindings` in `crates/gallery/src/shaders.rs`
  opens with `cluster_light_counts`, `cluster_light_indices`, and then an **empty placeholder**
  whose comment reads: *"The depth buffer is next in name order, so it takes binding three. The
  pass binds it because a fragment is tested against it and no fragment here reads it, so it has
  no line of WGSL: the empty name is the number it consumes and nothing else."* The template is
  written for a pass that is depth tested.
- **The prepass is emitted and paid for.** Every scene runs one depthing pass per group into
  `DEPTH_BUFFER` at full cost, and the result is never sampled.

## What it looks like

Rendered on this machine, with the gallery as it stands:

- `dunes` — the dark foreground boxes are drawn **additively over** the lit sand with no depth
  test, so the dune is visible straight through them. They read as translucent panels.
- `spotlight` — a single box, one prepass group, no accumulation to blame. Its lower half washes
  out to near white and its own far interior wall shows through its near wall. The upper half,
  which sits against the black void, looks correctly solid, because adding red to black is red.
- `monoliths` — looks right, and only because the spires are small and distant. The defect is
  present in this scene too.

The tell is the boundary between washed and solid, which tracks where the background changes
from lit ground to black void rather than tracking any property of the surface.

## The evidence that settled it

Pixel samples from the spotlight render, against a material of `[1.0, 0.1, 0.1]`:

| sample | RGB | reading |
| --- | --- | --- |
| red box, face away from the light | 74, 28, 38 | correct dark red |
| red box, lower half against lit floor | 186, 157, 152 | near neutral — the floor is showing through |
| blue box `[0.1, 0.1, 1.0]` | 34, 34, 103 | correct, because it is last and nothing is behind it |

Reducing the light intensity to 0.22 made the boxes read as clean saturated solids above the
horizon and washed out below it, which is additive blending over a bright background rather than
a depth failure: if depth were rejecting fragments, intensity would not change the shape of the
object.

## Why the one line fix does not work

Adding `DEPTH_BUFFER` to the lighting pass's targets is the obvious change and it is **wrong on
its own**, because it produces a different failure:

```
the shader for `lighting/lighting/floor` declares `environment_map` at binding 4 as a two
dimensional texture, and this pass binds a buffer there.
```

`WgpuRenderer::bind_pass` routes a texture that is one of the pass's targets into
`bindings.targets` and **not** into `bindings.bound` (`bindings.rs`), while `binding_numbers`
numbers only `bound`. Making the depth buffer a target therefore removes it from the numbering
and every binder after it moves down one — straight into the slot the template reserved for it.

The fix is two coupled changes:

1. `lighting_pass` names `DEPTH_BUFFER` among its targets, so an attachment exists and wgpu builds
   the depth state the pass already declares.
2. `binding_numbers` gives a target the same number it would have had as a bound resource, so a
   resource that is both bound and written keeps its place. This is the device's numbering rule
   changing, and it moves the binding number of **every** resource in **every** pass: shadowing,
   depthing, lighting and post. Every WGSL template reserves numbers in the same name order, so
   all of them move together and the two sides stay in agreement — but the whole gallery is
   re-rendered and re-baselined as a result, and the change is better reviewed on its own.

## What landing it will change

Occlusion starts working, so:

- `dunes` stops being transparent and the sand stops showing through the boxes.
- `spotlight` stops washing out; the boxes become solid and the cone shaped shadows they cast
  become the thing the scene is for.
- Every scene gets correct mutual occlusion, which may expose **new** problems that were
  invisible while nothing occluded anything: shadow acne where a surface shadows itself, missing
  shadows where a caster is outside the prepass, and geometry that was relying on additive
  overdraw to be visible at all.

That last list is the reason this is a proposal and not a change. Turning occlusion on is a
correctness fix whose blast radius is every image the renderer produces, and the first render
after it should be expected to need work rather than to be a clean win.

## The one line that would have caught it

Nothing in the emitter can assert that a pass which declares a depth state has a depth
attachment, because the two are built in different crates from different inputs. A check in
`raster.rs` that refuses to build a pipeline whose `state.depth` is present while no depth
attachment was resolved would have turned this into an error at the first scene rather than a
style choice nobody questioned. The `check_resources` machinery already exists for exactly this
class of disagreement between what a pass declares and what it is built from, and this is the
same class: a declaration that nothing honours.
