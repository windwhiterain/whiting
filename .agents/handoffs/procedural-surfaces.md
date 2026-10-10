# Procedural surfaces: a backend-neutral shader IR, compiled to a Cycles node graph

Status: **in progress.** Branch `procedural-surfaces`, worktree `.worktrees/procedural-surfaces`,
based on `5a973b3`. Delete this file when the work is merged, as the other handoffs are.

The gap being closed is recorded in
[SPEC/render/cycles_backend.md](../SPEC/render/cycles_backend.md) under *what is actually left is
shading, not the camera* and *how a procedural surface is carried, and what the fallback is*.
`landscape`, `lanterns`, `spotlight`, `sunlight` and `tiles` are the measured five scenes.

## What was decided, and by whom

Two questions were put to the user on 2026-10-10 rather than answered here, because both were
expensive to reverse. **Both answers changed afterwards**, and the revisions are the interesting part:

- **Fidelity: same pixels — later revised to same picture.** The first answer was to port the
  gallery's own `value_noise` rather than reach for the target's noise node, so the two backends
  would agree exactly. Measuring the hash showed that was never reachable, and reasoning about what
  a rasteriser and a path tracer each *do* showed it should never have been wanted. See
  *The noise hash* below.
- **Home: its own crate. Unchanged.** The IR must not live inside `cycles_export`, because the whole
  point of it being backend-neutral is that a second backend stays a second backend rather than
  becoming a rewrite.

The third thing decided on the same day is the shape: **a material carries a rasterised form and a
traced form separately**, they are two implementations rather than one compiled twice, and the two
are expected to render differently. That is recorded in
[SPEC/render/cycles_backend.md](../SPEC/render/cycles_backend.md).

OSL is recorded as the **fallback**, not the plan. It is real, maintained (Academy Software
Foundation, 1.15.7.0 on 2026-09-07), and a CMake flag away in Cycles. It is not first because the
five shaders would have to be **rewritten**, not compiled: no light loop, closures instead of light
sampling, different routes to `n` and `p`.

## The node set the design rests on, and how that was established

Not assumed — every name below was found by scanning printable ASCII runs of the executable for an
exact match, with `principled_bsdf` / `noise_texture` / `voronoi_texture` as controls that must be
found. `out/probe-nodes.py` in the main tree is that scan.

Present: `geometry`, `attribute`, `camera`, `object_info`, `math`, `vector_math`, `map_range`,
`mix_float`, `mix_vector`, `mix_color`, `combine_xyz`, `separate_xyz`, `bump`, `white_noise_texture`,
`noise_texture`, `voronoi_texture`, `wave_texture`, `invert`, `clamp`, `fresnel`, `layer_weight`,
`texture_coordinate`, `diffuse_bsdf`, `glossy_bsdf`, `principled_bsdf`.

**Absent, and so unusable:** `new_geometry`, `musgrave_texture`, `fractal_noise`, `cell_noise`,
`bright_contrast`, `hsv_rgb`. `fractal_noise` is an SVM/OSL internal, not a node — its absence does
not mean fBm is unavailable, only that it must be built.

## The silent-failure trap, which shapes the IR

Corrected 2026-10-10 — an earlier version of this file said connect sources are not validated. They
are. A direct test settles it:

| Written | Result |
| --- | --- |
| `from="n value"` | works |
| `from="n Value"` | works — socket names are case-insensitive |
| `from="n nonexistent_socket"` | `ERROR: Unknown output socket name "n" on "nonexistent_socket"` |
| `from="n vector"` on a `math` node | `ERROR: Unknown output socket name "n" on "vector"` |
| `from="n result"` on a `math` node | `ERROR: Unknown output socket name "n" on "result"` |

Connect **targets** are validated too: an unknown one raises
`Unknown input socket name "x" on "y"`.

What is genuinely silent is narrower:

- a node **attribute** whose name the node does not have is ignored — `color1` on a mix node does
  nothing where `a` works, and `value` on a `math` node does nothing where `value1` works;
- an operation whose inputs are left at zero **computes zero**, which renders as a black surface
  with a specular highlight on it and reads as a lighting fault.

**Consequence for this work:** the IR still types its sockets and still refuses a mismatched edge,
because that turns a silent wrong picture into a named build failure. But the IR is not protecting
against unvalidated socket names — the target does that. It is protecting against a mistyped
operation or a wire that changes the meaning of a surface without saying so.

## The noise hash: settled, and not by porting it

**Fidelity: same picture, not the same pixels.** The earlier goal of porting the gallery's own
`value_noise` so the two backends would agree exactly is **abandoned**, and the reason is in
`.agents/SPEC/render/cycles_backend.md`: a rasteriser and a path tracer resolve a surface by
different means, so the same definition still renders two different pictures. Chasing bit equality
would have cost a lot to buy something that was never reachable.

The hash could not have been ported anyway. `crates/gallery/src/landscape/mod.rs:896` is

```wgsl
fn hash(point: vec2<f32>) -> f32 {
    return fract(sin(dot(point, vec2<f32>(127.1, 311.7))) * 43758.5453);
}
```

The lattice cells reach about 56 at landscape scale, so `sin`'s argument is near 24,600 radians and
already carries about 0.002 of `f32` error before `sin` is called; the multiply by 43758.5453 turns
that into about 87, and `fract` of that is an unrelated number. **Two implementations agreeing to one
ULP still produce uncorrelated noise.** The traced form therefore uses the target's own noise node
and the rasterised form keeps the hash it has always had.

**What the traced form loses by doing this:** the massif's `patches`, `wobble`, `lobes` and `threads`
terms are tuned against a particular noise's statistics. A different noise function will not honour
those constants, so the bands will need re-tuning against the traced frame. That is expected work,
not a defect, and it is the part of this that cannot be done ahead of looking at the picture.

## Shape of the work, and where it stands

The design settled on 2026-10-10: a material carries a **rasterised** form (the WGSL text it already
carries) and a **traced** form (a node graph) separately. They are two implementations rather than
one compiled twice, the two are expected to render differently because the two rendering principles
differ, and `crates/shader` is the Rust construction and schema for the traced form, serialised into
the document. A material with no traced form still emits the constant `principled_bsdf`, so nothing
that works today stops working.

1. **done** — `crates/shader`: `socket.rs`, `op.rs`, `graph.rs`, `error.rs`. Builds with no
   warnings; the crate doc example compiles.
2. **the node vocabulary is measured.** Input sockets were enumerated by connecting a known source
   into each candidate and keeping the ones that do not raise; outputs were settled by wiring each
   into an emission strength and comparing against an emission of the value it should produce.

   | Node | Inputs | Output |
   | --- | --- | --- |
   | `math` | `value1`, `value2`, `value3` | `value` |
   | `vector_math` | `vector1`, `vector2`, `scale` | `value` |
   | `map_range` | `value`, `from_min`, `from_max`, `to_min`, `to_max`, `steps` | `value` |
   | `bump` | `height`, `normal`, `strength`, `distance` | `normal` |
   | `combine_xyz` | `x`, `y`, `z` | `vector` |
   | `separate_xyz` | `vector` | `x`, `y`, `z` |
   | `mix_color` | `fac`, `a`, `b` | `color` |
   | `geometry` | none | `position`, `normal`, `true_normal`, `incoming`, `tangent`, `parametric`, `random` |

   **Attribute names equal socket names**, confirmed by rendering: `math` with `operation="ADD"`,
   `value1="0.3"`, `value2="0.4"` matches an emission of strength 0.7 exactly.

   **The emitter must write every input.** `math` has three, and an unset one keeps the node's own
   default rather than becoming zero. Writing only the inputs it has values for inherits defaults
   nobody chose.

3. **not started** — the `traced` field in the scene schema, and the reader that turns it into a
   graph. The field's documentation should answer why a shader already carries its own text: so that
   a compiled scene renders wherever it is sent. The node graph is the same kind of thing.
4. **not started** — the compiler from that graph to `<shader>` XML, which refuses to emit an
   operation it cannot spell.
5. not yet — the other four scenes.

### Still unmeasured, and marked as such

`attribute` produced nothing under any of `attribute_name` / `name` / `attribute` crossed with
`fac` / `color` / `vector` / `alpha`. That node is what would carry a per-face authored id, and the
massif's stone colour depends on one, so it has to be resolved before `landscape` can match. The
likely cause is that the mesh carries no such attribute, which would be an emitter problem rather
than a spelling one — Cycles' loader reads exactly five vertex attributes and has no way to add
another.