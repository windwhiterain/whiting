# Procedural surfaces: a backend-neutral shader IR, compiled to a Cycles node graph

Status: **in progress.** Branch `procedural-surfaces`, worktree `.worktrees/procedural-surfaces`,
based on `5a973b3`. Delete this file when the work is merged, as the other handoffs are.

The gap being closed is recorded in
[SPEC/render/cycles_backend.md](../SPEC/render/cycles_backend.md) under *what is actually left is
shading, not the camera* and *how a procedural surface is carried, and what the fallback is*.
`landscape`, `lanterns`, `spotlight`, `sunlight` and `tiles` are the measured five scenes.

## What was decided, and by whom

Two questions were put to the user on 2026-10-10 rather than answered here, because both are
expensive to reverse:

- **Fidelity: same pixels.** The gallery's own `value_noise` is ported into the IR and rebuilt from
  `math` nodes, rather than reaching for Cycles' `noise_texture`. The reason is that
  `noise_texture` is a *different function*; using it would give a picture of the same kind, not the
  same picture.
- **Home: its own crate.** The IR must not live inside `cycles_export`, because the whole point of
  it being backend-neutral is that the OSL fallback stays a second backend rather than becoming a
  rewrite.

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

Measured, and it is the single most important thing to carry into the implementation:

- a connect **target** with an unknown name raises `Unknown input socket name "x" on "y"`;
- a connect **source** with an unknown name raises **nothing at all** and leaves the destination at
  its default — in practice zero, which renders as black plus a specular highlight;
- node **attributes** with unknown names are ignored silently too.

The mix node takes `a` and `b` as attributes, `A`/`B` as connect targets, and `result` as its
output. `color1`/`color2` look right and belong to `brick_texture`.

**Consequence for this work:** an IR that names its own nodes and sockets and lets the backend
compiler map them is not a convenience, it is the only thing standing between us and a black render
that reports success. The IR therefore must be typed — a mis-wired edge has to fail in *our* code,
with the node and socket named, rather than survive into the XML.

## Open, and the reason it is open

**The `value_noise` hash cannot survive a port as written, and this is the thing to decide next.**

`crates/gallery/src/landscape/mod.rs:896`:

```wgsl
fn hash(point: vec2<f32>) -> f32 {
    return fract(sin(dot(point, vec2<f32>(127.1, 311.7))) * 43758.5453);
}
```

The lattice cell components reach roughly 56 at landscape scale, so the argument to `sin` is around
24,600 radians. In `f32` that argument carries about 0.002 of absolute error before `sin` is even
called. A difference of one ULP in `sin` becomes roughly 0.002 there, times `43758.5453` becomes
about 87, and `fract` of that is a completely different number. **Two implementations that agree to
one ULP still produce uncorrelated noise.** Cycles and a WGSL JIT will not agree bit-for-bit, and
Cycles may use a fast `sin`.

So "same pixels" is not reachable by porting this hash. It has to be *replaced* with a hash whose
output depends only on exactly-representable arithmetic — and the replacement has to land in the
gallery's WGSL too, or the two backends are running different functions rather than the same one
differently.

The constraint on the replacement: **Cycles' `math` node has no bitwise operations**, so the usual
32-bit integer hash is not expressible. The candidate space is therefore float-only hashes that keep
every intermediate inside `2^24` and use no `sin`.

This is not a decision to make inside this handoff. It changes what the gallery's existing scenes
look like, and that is the parent's call.

## Shape of the work, and where it stands

1. **done** — the IR crate `crates/shader`: `socket.rs` (what flows), `op.rs` (the operation set
   and the one table describing it), `graph.rs` (the graph and its checks), `error.rs`. Builds with
   no warnings, and the crate doc example compiles.
2. **not started** — the Cycles compiler: IR → `<shader>` node graph. This is blocked on step 4.
3. **not started** — proving the chain end to end on the landscape massif surface.
4. **the measurement the compiler is waiting on.** The node vocabulary is *not* uniform and cannot
   be assumed:

   | Node | Input sockets measured | Tried and not a socket |
   | --- | --- | --- |
   | `math` | `value1`, `value2`, `value3` | `value`, `value_001`, `a`, `b` |
   | `vector_math` | `vector1`, `vector2`, `scale` | `value`, `value1`, `value2`, `vector` |
   | `map_range` | `value` | `value1`, `value2`, `scale`, `factor` |
   | `separate_xyz` | `vector` | `value`, `value1`, `vector1` |
   | `bump` | none found yet | `value`, `value1`, `scale`, `height`, `normal` |
   | `combine_xyz` | none found yet | `value`, `value1` |

   `vector_math` raising `Unknown input socket name "value1" on "pick"` is the result that matters:
   the convention a developer carries over from `math` is wrong on the next node.

   Still to measure before an emitter can be written: `map_range`'s four bound sockets, `bump`'s two
   inputs and two parameters, `combine_xyz`'s three inputs, and **the attribute names for every
   node, which are not the socket names**. Writing `value="-0.4"` on a `math` node yields the node's
   own default, silently; `value1` is read. An emitter written before this is done renders black
   and reports success.

   The method is in `out/probe-socketnames.py` and `out/probe-opnames.py` in the main tree: compare
   two renders rather than decoding a number, because the emission response is linear only below
   about 1.2 and saturates above it.

5. not yet — the other four scenes.