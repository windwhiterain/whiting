# Can this Cycles build render a procedural material, and is OSL the mechanism?

Recorded 2026-10-10. Every measurement below was taken on that date against the one executable
described under "Build under test"; re-measure before relying on any of it, because both the node
set and the XML spelling of sockets are properties of this build.

## Question

The gallery computes surfaces per fragment in WGSL, so a path tracer needs a node graph rather than
one constant per material. Does the Cycles build we ship have a way to express that, and is OSL —
the optional compile-time feature that sounds like it exists for exactly this — the answer?

## Answer in one line

**Yes, Cycles has native procedural texture and shading nodes and does not need OSL for them; OSL
is not present in this build at all.** Everything whiting would need for `landscape`, `lanterns`,
`spotlight`, `sunlight` and `tiles` can be expressed as a `<shader>` node graph with no compiler
involved. The obstacle is on our side of the boundary, not Cycles': whiting currently emits one
constant `principled_bsdf` per material and has no document-level vocabulary for a graph.

## Build under test

- `C:\resource\cycles\install\cycles.exe`, version 5.3.0, 7,400,448 bytes
- SHA-256 `8316E615CB9046D4320F375085CBF24783D09C9955513F759F579E1A54F2D28C`
- Rendered on `--device CPU`; exit status is 0 even when the scene is wrong, so **stderr must be
  read every time** — `ERROR:` lines are the only failure signal.

## OSL is not in this build

Method: scan every printable ASCII run of length >= 4 in the executable for `osl_shader`, `WITH_OSL`,
`osl_path`, `osl_name`, `openshading`, `OSLShader`. **All zero hits.**

The scan method itself was verified against strings that must be present, so a zero is a real zero
and not a broken search: `principled_bsdf` 2, `noise_texture` 4, `voronoi_texture` 2.

The single `osl` substring in the binary sits inside the device-type enumeration next to `osl.svm`,
which is a CPU instruction-set identifier, not a sign of the shading language.

Confirmed by rendering: a shader containing `<osl_shader>` reports
`ERROR: Unknown shader node "osl_shader"`.

**Limit of this claim:** this is a statement about *this executable*. A build compiled with
`WITH_OSL=ON` would behave differently, so this is not evidence about OSL the technology.

## Native procedural nodes are present and work

Registered node type names found in the same binary, all confirmed to render with zero `ERROR:`
lines: `texture_coordinate`, `noise_texture`, `voronoi_texture`, `wave_texture`, `checker_texture`,
`gradient_texture`, `magic_texture`, `brick_texture`, `gabor_texture`, `sky_texture`,
`image_texture`, `mix_color`, `mix_float`, `mix_vector`, `mix_closure`, `rgb_ramp`, `bump`, `math`,
`vector_math`, `combine_xyz`, `separate_xyz`, `map_range`, `layer_weight`, `fresnel`,
`diffuse_bsdf`, `glossy_bsdf`, `transparent_bsdf`, `principled_bsdf`.

Names that sound right but **are not** registered in 5.3.0 and each raise `Unknown shader node`:
`tex_coord`, `color_ramp`, `mix_rgb`, `mix_shader`.

## Measured proof

`out/proc-pure.png`, rendered from `out/proc-pure.xml` on 2026-10-10 with 128 samples at 900x900 and
zero `ERROR:` lines: one wall, no image textures, no OSL. A `noise_texture` mixes between red and
blue into `principled_bsdf.base_color`, and the same noise drives a `bump` node into the surface
normal. Both the colour and the relief are computed per fragment by the renderer.

The render's pixel statistics agree with the eye: mean RGB `[0.180, 0.015, 0.167]` (red and blue
both high, green near zero, which is what mixing red into blue must look like), and a mean
four-neighbour colour spread of 0.146, so the surface is genuinely spatial rather than one constant.

## XML spellings that cost the most time to find

These are all measured, and each one fails *silently* rather than loudly, which is why they are worth
writing down. Unless stated otherwise a wrong name is ignored and the socket keeps its default.

| Thing | Name that works | Name that is accepted and does nothing |
|---|---|---|
| Mix node operands | `a`, `b` as node attributes | `A`, `B`, `factor1`, `factor2`, `color1`, `color2` |
| Mix node operand, as a connect target | `A`, `B`, `fac` | `factor1`, `color1`, `value1` |
| Mix node output | `result` | `value`, `out`, `output`, `color` |

- `<link from=... to=...>` is **not** an element in this build: it reports `Unknown shader node
  "link"`. `<connect from="node socket" to="node socket" />` is the correct form, and it also works
  when the target is spelled out as `from_node`/`from_socket`/`to_node`/`to_socket` only if the
  shorthand `from`/`to` attributes are also present.
- `color1` and `color2` do exist in the binary, but they belong to `brick_texture` (the mortar
  colours), not to the mix nodes. Their presence in a string scan is not evidence about the mix node.
- Connect targets **and sources** are both validated. An unknown target raises
  `Unknown input socket name "x" on "y"`; an unknown source raises
  `Unknown output socket name "y" on "x"`. Socket names are matched case-insensitively: `value` and
  `Value` are the same socket.

  **An earlier revision of this file claimed the opposite**, that an unknown connect source raises
  nothing and silently leaves the destination at its default. That was a misreading of a truncated
  error line, corrected on 2026-10-10 by a direct test. What *is* silent is narrower and worth
  stating exactly:

  - a node **attribute** whose name the node does not have is ignored, with no error — writing
    `color1` on a mix node does nothing, and writing `a` works;
  - an operation whose inputs are left at zero **computes zero**, which renders as a black surface
    with a specular highlight on it. That is what the mix node was doing before its operand names
    were right, and it looks like a lighting fault rather than a typo.
- Every `ERROR:` line matters because the process still exits 0.

## The arithmetic node vocabulary, measured 2026-10-10

An emitter cannot be written from the names above alone. Each arithmetic node has its own input
socket names, and **they are not the arithmetic node's** — the convention does not carry across node
types. Input sockets were enumerated by connecting a source whose output name is already known
(`result` on the mix family) into each candidate and keeping the ones that do not raise; output
sockets were settled by their effect, wiring each into an emission strength and comparing against an
emission of the value it should produce.

| Node | Inputs | Output | Tried and **not** a socket |
| --- | --- | --- | --- |
| `math` | `value1`, `value2`, `value3` | `value` | `value`, `value_001`, `a`, `b`, `factor` |
| `vector_math` | `vector1`, `vector2`, `scale` | `value` | `value`, `value1`, `value2`, `vector` |
| `map_range` | `value`, `from_min`, `from_max`, `to_min`, `to_max`, `steps` | `value` | `value1`, `value2`, `scale`, `factor`, `min`, `max` |
| `bump` | `height`, `normal`, `strength`, `distance` | `normal` | `value`, `value1`, `scale`, `invert` |
| `combine_xyz` | `x`, `y`, `z` | `vector` | `value`, `vector1` |
| `separate_xyz` | `vector` | `x`, `y`, `z` | `value`, `value1`, `vector1` |
| `mix_color` | `fac`, `a`, `b` | `color` | `color1`, `color2`, `factor` |
| `geometry` | none | `position`, `normal`, `true_normal`, `incoming`, `tangent`, `parametric`, `random` | `pointiness`, `backfacing` |

`vector_math` reporting that it has no `value1` is the clearest single result here: the socket names
a developer would carry over from `math` are the wrong ones on the very next node.

**A node's parameter attributes use the same names as its input sockets, and that was confirmed
rather than assumed.** `math` with `operation="ADD"`, `value1="0.3"` and `value2="0.4"` renders
byte-for-byte what an emission of strength 0.7 renders. The same node with `value="0.3"` and
`value1="0.4"` does not, because `value` is its *output* and setting it does nothing.

**Socket names are matched case-insensitively.** `from="n value"` and `from="n Value"` both work and
produce identical images.

### A blind spot in the output-socket method, found by checking the documentation

Deciding an output name by wiring it to an emission strength and looking for light **cannot
distinguish a wrong name from a node whose value is legitimately zero** on the test surface. On a
flat quad facing the camera, `backfacing` is zero and `pointiness` is zero, and both look exactly
like a name that was refused.

The Blender manual settles it: the Geometry node does have `Backfacing`, and the Attribute node has
`Position`, `Normal`, `Tangent`, `True Normal`, `Incoming`, `Parametric` and `Backfacing`. **Two
names this repository's earlier pass recorded as absent are in fact documented as present.** The
method has to feed the node something that is not zero before it can say anything about an output.

### The attribute node, settled from documentation

The manual states the Attribute node has **no inputs**, one property called **Name**, and four
outputs: `Color`, `Vector`, `Fac` and `Alpha`. It also states what happens when the lookup fails:

> If a suitable property is not found, all sockets of the node, including Alpha, output 0.

That is exactly what this repository's render measured — every candidate spelling returned zero. So
the node and its socket names were never the problem.

The node has a **Type** — Geometry, Object, Instancer or View Layer — and Cycles' XML has no way to
write it, so it is Geometry, which reads mesh attributes. And this repository has already recorded
what that means, quoting the loader:

> we don't yet support arbitrary attributes, for now add vertex attributes

The loader produces **exactly five**: position, normal, UV, UV tangent and tangent sign.

**So the two independent lines agree, and the answer is definite: a per-face authored id cannot
cross to this renderer.** The `landscape` massif varies its stone colour per block by reading
`in.authored_id`; there is no attribute node that can carry it, because there is no attribute to
read. What replaces it has to be built from what the node *does* have — the shading point's position
and normal — or the variation is dropped and the massif reads as one uniform stone. That is a
design decision about the picture, not about the file format.

Source: the Blender manual's Attribute Node and Input Nodes pages, read 2026-10-10. Version drift is
likely and unmeasured here; the loader's five-attribute limit is quoted in this repository's own
source and is the stronger of the two claims for our purposes.

### The one rule this imposes on an emitter

**Write every input explicitly.** `math` has three, and an input left unset keeps the node's own
default rather than becoming zero — which is how `SUBTRACT` of 0.7 and 0.3 can arrive as something
other than 0.4. An emitter that writes only the inputs it has values for inherits defaults nobody
chose. Writing all of them makes the node's defaults irrelevant and removes a whole class of
"works on one scene and not on another".

### How a value was measured

The response of an emission strength to its input is linear only below roughly 1.2 and saturates
above it, so a ladder of known strengths can only decode that range. Comparisons are therefore made
between two *images* rather than against a decoded number: render the operation with known inputs,
render an emission of strength equal to what it should produce, and compare pixels. Both images go
through the same transfer, so nothing has to be known about it.

## What this does not settle

Whether whiting should gain a document-level expression for a procedural surface is a design
decision, not a finding, and belongs in a proposal. This note only establishes that the target
format can express the thing and that the capability is native rather than compiler-backed.

## Reproducing

The probe scripts that produced every number above are in `out/`, which the repository ignores:
`probe-basecolor.py`, `probe-connect.py`, `probe-colorspace.py`, `probe-emission.py`,
`probe-mixfloat.py`, `probe-link.py`, `probe-explicit.py`, `probe-sockets.py`, `probe-inputs.py`,
`probe-inputs2.py`, `probe-direct.py`, `probe-showcase.py`.