# Adapter limits and the shadow page pool: what a ceiling should be

`WgpuRenderer::required_limits` used to build its request from `wgpu::Limits::downlevel_defaults()` and
override only the two vertex counts, so every other limit the device was created with was the
downlevel floor — `max_texture_dimension_2d` of 2048, `max_texture_array_layers` of 256 — whatever
the hardware could actually do. It now builds from the **adapter's own** `Limits` and keeps the two
vertex overrides, which are the pipeline's requirement rather than a ceiling.

That change is one line and it is finished. What it leaves is a question about the shadow page
pool, and the question is one this project already got wrong once and should not get wrong again.

## What `MAX_TEXTURE_ARRAY_LAYERS` was, and what it is now

`lighting::buffers::MAX_TEXTURE_ARRAY_LAYERS` is 256, and the compiler refuses a scene naming a
deeper pool than it. While the device asked for `downlevel_defaults()`, that constant was a
**transcription of a hardware fact**: the floor really was 256, and the number in the compiler
agreed with the number the device was created with. Raising the device's request and leaving the
constant would have been a lie in the other direction — a scene asking for 1024 pages would have
been refused by the compiler on a machine that could hold 4096 of them.

So the constant stayed where it was, and its documentation now says what it is: **a floor this
compiler enforces, not a ceiling the hardware imposes**. The two are not the same kind of thing
and they fail differently:

- A **ceiling the hardware imposes** is a fact about the machine. A document that exceeds it
  cannot be drawn there, and the error says so at resource creation.
- A **floor the compiler enforces** is a decision about what this project is willing to stand
  behind. A document refused here is refused on *every* machine, including the ones that could
  have held it.

That is a defensible thing for a compiler to do, and it is what the constant now is. It is also
not the same as knowing what the hardware can do.

## Why a const that tracks the hardware is the wrong shape

The obvious "fix" is to make the constant a statement about the machine: read the adapter's
`max_texture_array_layers` and refuse above that. Two things go wrong with it, and the first is
fatal.

**The compiler does not know the adapter.** `lighting::buffers::read_light_pool` is handed a
`&Value` and nothing else. A const that "tracks the hardware" can only track it if the hardware's
number is compiled in, and a number compiled in is a number that was right on the day somebody
ran the program on their machine and is wrong everywhere else. The document is the portable thing
in this system and the constant is not; putting a machine's answer in the compiler makes the
compiler's refusals depend on which machine last rebuilt the binary.

**A refusal is the wrong answer to a document that is merely larger than this machine.** A scene
that asks for 1024 pages is a legitimate scene. On an adapter that can give 256, the useful answer
is "you got 256, here is which lights went without a page and how to make room" — which is a note,
not an error. Refusing stops the frame entirely, and refusing is a decision the compiler has no
standing to make about a document it cannot see a machine for.

## The honest shape: a device-side clamp that emits a note

The device already knows the adapter — it asked it for the device. The compiler does not, and the
compiler is where the pool is read, where the pages are assigned, and where notes are born. So the
ceiling has to travel from the device into the compile rather than the other way round.

**`CompileContext` gains the limits the compile is being made against.** It already carries a
`camera`, for the same reason: something the emitter cannot know on its own, supplied by the host
that is about to render. A host holding a `WgpuRenderer` has `adapter.limits()`; the harness in
`crates/device/examples/render.rs` creates the renderer before it compiles, so the number is
available at the point the `CompileContext` is built. A host that has no adapter passes nothing
and gets today's behaviour.

**`read_light_pool` clamps rather than refuses.** `pages` becomes
`min(declared, limits.max_texture_array_layers)`, and a clamp emits a `CompileNote` naming the
field, the number asked for, the number given, and the limit it hit. The note joins the pool
overflow note `pool_notes` already emits, and a host that prints notes — which
`crates/device/examples/render.rs` now does — shows both: the pool was smaller than the document
asked for, and here is which lights it could not shadow. The frame still draws. That is the whole
point: the note is what distinguishes a smaller frame from a wrong one.

**The constant goes away when the clamp lands**, rather than becoming a second, smaller ceiling
beside the real one. Two numbers that both claim to bound the pool is one too many; the compiler
refuses above the clamp only if the clamp is not a thing a machine can move, and this one is.

## The page dimensions have the same hole, and nobody has tested it

`shadow_page_width` and `shadow_page_height` were moved to the `frame` block in this checkpoint
because a page's size is a property of the frame, and with them they picked up a ceiling that is
now the adapter's `max_texture_dimension_2d` rather than 2048. Nothing enforces it. A scene may
declare `shadow_page_width: 32768`, and `LightPool::total_bytes` will cheerfully report 256 GiB of
depth texture, and the device will fail to create it with a message about texture size.

This is the same problem and it wants the same answer, and it wants it in the same place: clamp
both dimensions to the adapter's `max_texture_dimension_2d` at read time, note it, draw. It is
worth noting that this dimension is the one that *benefits* from asking the adapter — a scene that
wants a 4096 page on a machine that can give it is now allowed to, and the refusal that used to
stop it is gone. The clamp is what makes that safe, because the refusal was never the thing making
it safe; the downlevel floor was an accident of how the limits were built.

## What this costs

- One field on `CompileContext` and one on `Renderer` to read, so a host can pass the limits down.
- A ceiling on each of the three counts in `read_light_pool`, and a `CompileNote` for each clamp.
- Removing `MAX_TEXTURE_ARRAY_LAYERS`, and the two tests that assert it
  (`crates/scene/tests/wgsl_validation.rs`, `crates/scene/tests/shadow_pages.rs`) rewritten to
  assert the **clamp** rather than a constant: a scene asking for more than the limit still
  compiles, still draws, and still emits a note naming what it lost.
- A decision this project has not made: whether a clamped pool is a note or an error when the
  document asked for pages by name. The argument above is for a note, and the argument against is
  that a document naming a budget and silently receiving less is the exact failure the
  `pool_notes` machinery exists to prevent — which is answered by the note existing at all, and
  not answered at all if the host does not print them.
