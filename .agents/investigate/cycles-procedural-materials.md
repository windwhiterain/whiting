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
- Connect targets are validated and an unknown one raises `Unknown input socket name "x" on "y"`,
  but connect *sources* are not: an unknown source socket raises nothing and quietly leaves the
  destination at its default. A graph can therefore be half-connected with no error at all.
- **A target that is a real socket with its space removed is accepted, says nothing, and does
  nothing.** This is the one spelling a writer is most likely to reach for, and it is the only one
  that fails silently. Measured 2026-10-12 against `principled_bsdf`, connecting a `mix_color` into
  its base colour:

  | target spelling | what Cycles says | what the frame shows |
  |---|---|---|
  | `Base Color` | `ERROR: Invalid from or to value for connect node` | unaffected |
  | `base_color` | **nothing at all** | the BSDF's default colour, not the mixed one |
  | `BaseColor` | `ERROR: Unknown input socket name "BaseColor" on "bs"` | unaffected |

  So "a socket whose name contains a space cannot be connected at all" is **true** — the space is
  what the parser splits on — and the consequence for us is that removing the space does not get
  around it, it only gets around the error. See
  [a-material-image-cannot-be-carried.md](a-material-image-cannot-be-carried.md).
- Every `ERROR:` line matters because the process still exits 0.

## What this does not settle

Whether whiting should gain a document-level expression for a procedural surface is a design
decision, not a finding, and belongs in a proposal. This note only establishes that the target
format can express the thing and that the capability is native rather than compiler-backed.

## Reproducing

The probe scripts that produced every number above are in `out/`, which the repository ignores:
`probe-basecolor.py`, `probe-connect.py`, `probe-colorspace.py`, `probe-emission.py`,
`probe-mixfloat.py`, `probe-link.py`, `probe-explicit.py`, `probe-sockets.py`, `probe-inputs.py`,
`probe-inputs2.py`, `probe-direct.py`, `probe-showcase.py`.