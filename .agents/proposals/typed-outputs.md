# Typed outputs: a scene declares what it renders out, and the compiler checks it can

Status: **approved for implementation.** Scope is the **output side only** — the value a shader
computes is still written by the scene's own WGSL. The value side (a shader graph) is explicitly
out of scope and explicitly *not* shelved as a whole; see *What this is not* at the end.

**No backward compatibility is required.** This supersedes the additive property every other change
in this project has held to, at the superior's instruction. What that costs is listed under
*What this breaks*, so it is a decision on the record rather than a surprise at review.

Supersedes the framing in
[agent-scene-iteration-loop.md](agent-scene-iteration-loop.md) item 3 and the *Shelved* section of
[handoffs/gbuffer-channels.md](../handoffs/gbuffer-channels.md). The multi-colour-attachment
renderer work that landed for `Depth` is **prerequisite**, not detour: a pass with several named
colour targets is what a declared output *is*.

## The taxonomy, which is the whole design

The three landed channels and the two shelved ones are not two tiers of the same thing. They are
two kinds, and the difference is **where the value comes from**.

| | value comes from | scene-independent? | who fills it | type is checked against |
| --- | --- | --- | --- | --- |
| `position` | the depth buffer, plus the camera's projection | **yes** | **the compiler** | the compiler's own declared type |
| `depth` | the depth buffer, verbatim | **yes** | **the compiler** | the compiler's own declared type |
| `grade` | the shaded image, the frame before the post grade | **yes** | **the compiler** | the compiler's own declared type |
| `normal` | a value the scene's fragment shader computed | no | **the scene** | naga's parsed fragment outputs |
| *anything else* | ditto | no | **the scene** | ditto |

A scene that has a camera and a depth buffer **has** a position image. That is a property of the
pipeline, not of the scene, so no scene edit can change it and there is nothing for the scene to
declare or get wrong. Those are **compiler shortcuts**, and they keep their hand-written WGSL
because that WGSL is the compiler's own code, not a special case bolted onto a general mechanism.

`Normal` is a different thing. It exists only because a particular fragment shader decided to
compute it. That is the case that has no general answer today, and this document is the general
answer to it.

**Both kinds land in one `outputs` declaration and a caller looks them up the same way.** The
difference is who fills the target, and which type is the authority.

## The declaration

`outputs` exists today and already does half the job. From `graphics/src/lib.rs`:

> `outputs` says which of the images a document draws are **answers**: a render returns what it
> names, rather than whichever target the last pass to write one happened to fill.

What it does not carry is a **type**, and without a type there is nothing to check. It maps
`{ name: <binding point> }` — a name to an address, with nothing said about what lives there.

It becomes `{ name: <output declaration> }`, where a declaration names three things:

1. **the point** — which binding point the image is rendered into
2. **the type** — the schema of a texel, in the workspace's own schema language
3. **the location** — which `@location` of the writing shader's fragment output carries it

```ron
outputs: {
    present: { point: "present_target", location: 0, type: vec4<f32> },
    normal:  { point: "debug_normal",  location: 1, type: vec3<f32> },
}
```

**The location is explicit and is not inferred from position.** This is the one place where
inferring would be actively wrong: a pass writes `@location(0..N)`, and a declaration that derived
its location from its order in the map would silently mis-bind the moment a name was inserted,
removed or reordered. The failure would be a wrong picture rather than an error, which is the
failure class this project refuses everywhere. `texture-channel.md` records the same lesson from
the other direction — a rule that *inferred* a sampler's need from a texture's format broke six
gallery scenes at once, and the fix was to make the document **declare** it.

## The type check

### Where it lives, and why it needs no gpu

`crates/device/src/wgpu_renderer/shader_interface.rs` already reads what a shader declares and
compares it against what the pass built, and it is the best piece of engineering in the workspace.
`declared_vertices()` is the shape to copy:

```rust
let Some(entry) = module.entry_points.iter().find(|entry| entry.stage == naga::ShaderStage::Vertex)
else { return Vec::new(); };
// entry.function.arguments, filtered by naga::Binding::Location { location, .. },
// each read with components_of(&module.types[...].inner)
```

A fragment output is an entry point's **return value** rather than one of its arguments, and
`naga` 30.0.1 is already a dependency. So the read is the same walk with the return type instead of
the argument list.

**The check is therefore pure.** `check_resources()` is already a free function over
`&naga::Module` precisely so it *"can be checked on a machine with no gpu, and it is why
`WgpuRenderer::check_bindings` is a thin wrapper over it rather than the other way round."* The
output check gets the same treatment, which means **it is unit-testable without an adapter** —
the single most valuable property it could have, in a project whose scene work happens on a
laptop.

### What it checks, and what it says

Three failures, and **the message must name the shader's own variable**, because that is what a
reader opens:

| failure | message names |
| --- | --- |
| the writing shader has no output at that location | the output name, the location, and the locations the shader *does* write |
| it has one, of a different type | both types, in the workspace's spelling and naga's |
| it has one of a type no format can hold | the type, and the formats that can |

The last one is real and not hypothetical. `read_back` **refuses a depth format** (`raster.rs:329`)
and knows `r32float` but **not `r32uint`** — so an integer identifier cannot be read back at all
today, and a declaration that type-checks perfectly would still produce an unreadable image. The
check must therefore be **"is there a format that can hold this type"**, not "is this a type",
and an integer output is a refusal until `r32uint` is supported rather than a silent empty image.

### The hole this closes

`photographic-scenes.md` item 5 records the shape, from the multisample analysis:

> **A pass that loads its target reads uninitialised memory.** […] Every lighting pass uses
> `load: Load` on the colour target […] With more than one sample the second light would add to
> whatever was in a texture nobody wrote.

A declared output that no shader writes is exactly that: a target the render returns, holding
whatever was in a texture nobody wrote. Today `DebugChannel` cannot express it, because the closed
enum guarantees every channel has a pass behind it. With a general declaration, **"declared and
never written" becomes reachable**, and the type check is what makes it an error naming the shader
rather than a picture of uninitialised memory.

There is a second, closer instance of the same class found while landing the exposure field:
a scene naming a uniform field its shader does not declare compiled clean and drew a frame that
silently ignored it, because the uniform is uploaded with the layout the shader itself declares
and an undeclared field is dropped by the upload rather than caught. That one is now refused by
name. **This is the same bug on the write side**, and the reason the check is worth building rather
than leaving to review.

## Format comes from the type

`DebugChannel::format()` today is a `match` on the channel's kind:

```rust
Self::Position | Self::Depth | Self::Grade => "rgba16float",
```

That is a format inferred from what a channel *is*, which is the same mistake
`texture-channel.md` records. Under a declaration, the type is given and the format is derived —
with a documented table and a refusal when there is no entry, because a `vec3<f32>` and a
`vec4<f32>` are not the same texture and a `u32` is not a float one.

## The closed set becomes a registry

`DebugChannel` as a closed enum was the right shape for a project that had exactly three
frame-resource channels and no way to express a fourth. It is the wrong shape now, because the set
is open by construction.

It splits, and the two halves are different things:

- **A registry of named built-in outputs** — `position`, `depth`, `grade`, and whatever else the
  pipeline can derive without the scene's help. Each carries its own type, its compiler-supplied
  WGSL, and its coverage contract. These are shortcuts, and they are allowed to be shortcuts
  because they are scene-independent.
- **Scene-declared outputs** — anything else, type-checked against the shader.

A caller sees one list. The distinction is real and load-bearing but it is the compiler's business,
not the caller's: both are looked up by name, both come back in the format they were drawn in, and
both are written into the same manifest and sidecar machinery that already exists.

**The coverage contract now has two sources of truth, and the errors must say which.**

- a built-in is bounded by **what the pipeline has**: a scene with no instance under `depthing`
  has no depth buffer, so a position image would be empty. That is the note that already exists,
  and it keeps firing.
- a scene-declared output is bounded by **which buckets the writing pass enumerated**, which is a
  property of the emitter's enumeration rather than of a buffer.

`gbuffer-channels.md` already wrote the second contract into `debug.rs` *before* the channel
existed, so it is written rather than discovered. It should be kept, and the two must not be
allowed to blur into one vague statement about "coverage".

## What this breaks

No backward compatibility was required, so this is a deliberate list rather than a diff:

1. **Every `outputs` declaration in the workspace is rewritten** to the new shape. The `dunes`,
   `terraces`, `tidewater` and other gallery scenes, plus the doctests in `graphics` and the
   hand-written documents in `example.rs`.
2. **The `DebugChannel` enum dissolves** into a registry plus a declaration path. `--debug
   position` becomes a name that resolves to a registered output rather than an enum variant.
3. **The type check can now fail documents that render today.** That is the point, but it means
   the first full-gallery run is the moment the workspace learns what it had wrong. Expect to fix
   gallery scenes, and do not weaken the check to make them pass.
4. **`r32uint` read-back support is now on the critical path** for any integer output, and does not
   exist yet.

## What this is not

**This is not a shader graph, and nothing here forecloses one.** The point of doing the output side
first is that it is **the output port a graph would plug into**: a graph's job is to decide what a
node's value *is*, and the declared output is where that value *goes*. Build the port now and the
graph later supplies values into the same declarations without the check changing by a line.

The value side stays where it is: the scene writes WGSL. An agent that can modify the compiler can
also write a shader, and the type check is what makes that safe to do — the plumbing is verified
even though the math is the scene's.

Two things a graph will have to answer, recorded now so they are not discovered late:

- `striking-3d-scenes.md` lists among the framework's real strengths that *shaders carry their own
  source, so a scene is self-contained and renders wherever it is sent*. A graph is not source, so
  it has to compile to WGSL to keep that property, and whether a scene carries a graph or carries
  compiled WGSL has an RPC dimension: a document sent to the device must be one the far end can run.
- The emitter blocker in `gbuffer-channels.md` — whether a pass's shader is a lighting one or a
  post one — is **dissolved** by a graph, because a node knows what it is and a hand-written WGSL
  string does not. The other half of that fix (carrying the stage name through the emitter to the
  device) is unrelated to the graph and still owed.

## What is deliberately not proposed

- **A change to how the frame is chosen.** `present` is still the one called `present`.
- **Moving any value-side work into the compiler.** A scene-declared output is a declaration the
  compiler *verifies*, not one it implements.
- **Any weakening of the type check to accommodate a document that does not type.** That is the
  whole change.
