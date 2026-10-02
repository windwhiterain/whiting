# Typed outputs: a document says what it renders out, and the compiler checks it can

Status: **in progress.** The specification is
[proposals/typed-outputs.md](../proposals/typed-outputs.md). **No backward compatibility is
required** for this change, at the superior's instruction; nothing here accepts the old shape.

## The declaration

```ron
outputs: {
    present: { point: "present_target", location: 0, type: "vec4<f32>" },
    normal:  { point: "debug_normal",   location: 1, type: "vec3<f32>" },
}
```

Three fields, all required: the **point** a pass renders into, the **location** the writing
shader's fragment output carries it at, and the **type** of a texel.

## Decisions taken where the proposal is open

1. **The type is a string in the workspace's own type vocabulary, borrowed from the document.**
   The proposal writes it unquoted (`type: vec4<f32>`) but the document is a `Value` tree and
   `vec4<f32>` is not a value this language has. It is read as `Value::String`, and
   `graphics::TexelType::parse` is the one place that spelling is known. `graphics::Output::ty`
   stays a `&'document str` rather than a decoded `TexelType`, so the **refusal for a type no
   format can hold is raised by the device's check**, where it can name the location and the
   shader, rather than by the document reader, which can name neither.
2. **The format-from-type table lives in `graphics`, beside the vocabulary it is derived from,
   and its unit is "a format this pipeline can create *and read back*".** An entry that a
   backend could hold but this device cannot read is **not an entry**, and the refusal says so.
3. **`r32uint` is refused, by name.** Two reasons, both load-bearing: `WgpuRenderer::format_of`
   has no such format, so the target cannot be created; and `texel_bytes`/`decode` in
   `render.rs` do not know it, so a read back image could not be decoded at all. Supporting it is
   not a table row — it is a change to `RenderedImage::texel`'s contract, because a `u32` above
   2^24 is not representable in the `f32` that method returns, and the identifier work the
   gbuffer handoff already scoped is where that belongs.
4. **`vec3<f32>` is a row, and the check is the width of the image rather than a width of the
   declaration.** *(Changed twice on 2026-10-02 at the superior's instruction. The first decision
   refused `vec3`; the second allowed a fragment output *at least as wide* as the declaration; both
   are superseded.)* The declaration layer speaks the abstract type and the **image** speaks the
   concrete one, so the two are asked as two questions against two different numbers: **can the
   fragment output fill the image the declaration resolves to** — against
   `TexelType::image_width`, and the answer must be **exact**, because an image is filled one whole
   texel at a time — and **how many numbers does the declaration claim mean something** — against
   `TexelType::components`, and never more than the image holds. Everything from the declared count
   onward is padding. The reason is the one the decision was made in — a scene says what its value
   *is*, and a rule about how an image is written belongs to the layer underneath. The previous
   version asked the width question of the **declared count**, which is what let a `vec3<f32>`
   declaration filled by a `vec3<f32>` fragment output pass the check and then fail at pipeline
   creation; that pair was confirmed on the adapter to be undrawable, so the row that draws is
   "declare three, write four" and the fourth is padding.
5. **`DebugChannel` survives as the registry of built-in outputs rather than dissolving into a
   table of constants.** It already *is* a registry: three named, scene-independent entries,
   each with its own type, its own compiler-supplied WGSL and its own coverage contract. What
   changes is that it stops being the closed set of *all* outputs — the scene-declared half is
   `graphics::Output`, type-checked, and a caller looks both up by name out of one `outputs` map
   and one list of images. Melting the enum into constants would touch six landed test files and
   buy no behaviour, which is why it was not done. **A reviewer should look at this hardest.**
6. **A declaration's `location` is cross-checked against the pass's attachment order.** *(Changed
   2026-10-02; previously recorded as not done.)* The mapping was established before the check
   was written, and it is real — see *Change B* below.
7. **Padding versus marker, decided.** A `vec3` declaration's surplus channel is **padding,
   never a marker**, and no declaration can ask for one: a marker is a *reading convention*
   rather than a value, and a declaration is where a document says what its value is. The
   built-ins keep `vec4<f32>` **because** they own a marker — `Position` and `Depth` fill their
   own fourth channel with "no surface was drawn here", which only compiler code can know — and
   their own `description()` is what tells a reader what that channel means. The shape is
   therefore *not* "four components when it wants a marker and three when it does not", which
   would make a scene's declaration a statement about the compiler's conventions. A `marker`
   field and a dedicated type were both considered and rejected for the same reason: both move a
   convention of `DebugChannel` into the document.

## Change A — the width rule, and the table after it

`check_outputs` asks **how wide the image is**, not **which type**:

| declared | image is | shader writes | verdict |
| --- | --- | --- | --- |
| `vec3<f32>` | 4 | `vec4<f32>` | allowed — the fourth is padding. **This is the row that draws** |
| `vec3<f32>` | 4 | `vec3<f32>` | refused, "a reader would find 1 number the scene never wrote" |
| `vec3<f32>` | 4 | `f32` | refused the same way, and the message says it supplies 1 of the 4 |
| `vec4<f32>` | 4 | `vec4<f32>` | allowed |
| `vec4<f32>` | 4 | `vec3<f32>` | refused the same way as row two |
| `f32` | 1 | `f32` | allowed — the one row with no padding at all |
| `f32` | 1 | `vec4<f32>` | refused, "3 of the 4 components the shader writes have nowhere to go" |
| any | any | a type that is not a float at all | refused as it always was |
| any | — | a type no format can hold | refused as it always was |

The format table is three rows: `f32` → `r32float`, `vec3<f32>` → `rgba16float`,
`vec4<f32>` → `rgba16float`, and **each row now carries two numbers** —
`TexelType::components` (what the declaration claims) and `TexelType::image_width` (what the image
it is drawn in holds). `TexelError::Unattachable` is **gone** — it existed only to refuse
a `vec3` and its message named a backend rule. `vec2<f32>` is now an unknown spelling, which is
true of the vocabulary, and the message does not invent a reason to make it more interesting.

**The second question is an invariant of the table, and it is held where a row is added.**
`components() <= image_width()` holds for all three rows, so a document cannot currently trip it;
`every_row_claims_no_more_numbers_than_the_image_it_is_drawn_in_holds` in
`crates/scene/tests/output_declarations.rs` walks `TexelType::ALL` and fails loudly if a fourth
row breaks it, and the check's own refusal for it is written out rather than left to the table.

## Change B — the attachment order, and what it turned out to be

**A positional `location` is well defined, and the mapping was established before the check was
written.** `graphics/src/read.rs`'s `Pass::targets()` returns the document's own list,
`bindings.rs` collects it with no sort and no deduplication, `raster.rs` builds one
`ColorTargetState` per attachment in that order, and a shader's `@location(i)` resolves against
that list. The only filtering is dropping the depth attachment, which is the same thing the
colour list is. So **a point's index in the pass's colour target list is the index its
declaration must claim**, and the check is that the two agree.

The consequence, stated in the message because a reader will hit it: **a point more than one
pass writes has to sit at the same location in each of them**, since one declaration has to name
all of them. Nothing in the workspace does otherwise, and the first full gallery run is where
that would show up.

- `crates/graphics/src/output.rs` (new) — `TexelType`, its spelling, the format table, and
  `OutputDeclaration` for the writer. **Changed**: each row now carries **two** numbers —
  `components()` (the declared count) and `image_width()` (the width of the image the
  declaration resolves to) — and the module doc states the two questions separately.
- `crates/graphics/src/lib.rs` — the schema, the three field names, the doc.
- `crates/graphics/src/read.rs` — `Output` carries `location` and `ty`; both are borrows out of
  the same tree as the name and the point.
- `crates/device/src/wgpu_renderer/shader_interface.rs` — `check_outputs`, a free function over
  `&naga::Module`, and the `WgpuRenderer::check_outputs` wrapper that only finds and parses.
  **Changed**: the location cross-check, `FragmentOutput.floats` (a count, not a table-row
  lookup), and **the width rule** — the comparison is against `image_width()` and is an
  **equality**, `width_sentence` names the image's width and the shader's supply rather than the
  declared count, and the declared-count question has its own refusal.
- `crates/device/src/wgpu_renderer/raster.rs` — the call, before the pipeline is built, filtered
  to the colour points this pass owns. The `points` it passes were already in attachment order.
- `crates/device/src/wgpu_renderer/mod.rs` — `execute` reads `outputs` once and threads it; the
  `written_labels` helper.
- `crates/device/src/render.rs` — `UnwrittenOutput`, the second coverage contract's error.
  **Changed**: its message said "a colour target" and "a texture nobody wrote", which the
  forbidden-word list refuses; it now says "some bucket asked it to" and "the image nobody wrote".
- `crates/scene/src/standard/debug.rs` — `ty()`, `location()`, `format()` derived,
  `declaration()`, and `declared_description` for the scene-declared sidecar header. **Changed**:
  the padding-versus-marker decision, and `surplus_of`, which is what makes a `vec3<f32>` header
  and a `Position` header say opposite things about their fourth channel. `surplus_of` now reads
  `TexelType::parse` rather than matching the spelling, and has a **third** sentence for a one
  component image — an `f32` declaration used to be handed the `vec4<f32>` header, which was
  false of an image that has no fourth component at all.
- `crates/scene/src/standard/emit.rs` — `debug_outputs` writes the new shape for the frame and
  for every built-in.
- `crates/device/examples/render.rs` — the sidecar header is read from the document, the two
  contracts are chosen between by name in one function, and **`marks_alpha`** decides whether an
  image's fourth component is a marker or padding. **Changed**: `viewable`, `covered_range`,
  `probe` and `unclaimed` take that fact, so a scene-declared image is measured over every texel
  and a `-` line is printed only for a built-in. **The declared count does not drive it**, and
  the function says why: the count is what decides where the padding starts, so keying the marker
  on it would call a `vec4<f32>` scene output a marker and an `f32` one not, which is the
  opposite of what either is.
- Tests: `crates/device/tests/typed_outputs.rs` (new, 19) and
  `crates/scene/tests/output_declarations.rs` (new, 9).

## Verification

| check | result |
| --- | --- |
| `cargo build --workspace` (and `--examples --features whiting-device/wgpu`) | clean |
| `cargo fmt -p whiting-graphics -p whiting-device -p whiting-scene -- --check` | clean. **Not `--all`**: the defect at the end of this file is caused by `cargo fmt --all` reaching into `whiting-gallery`'s tests. |
| `cargo clippy --workspace --all-targets --features whiting-device/wgpu` | **no warning in any line this change added.** The one hit in a file it touches is `examples/render.rs:182` (`Gallery::build(&name)`), which is pre-existing and outside every region edited. |
| `cargo test --workspace` | **133 passed, 0 failed** (baseline 130 when the handoff was written; 132 at the start of this correction) |
| `cargo test --workspace --features whiting-device/wgpu` | **181 passed, 0 failed** on a real adapter (baseline 178) |
| purity | `check_outputs` still takes `(&naga::Module, &[Output], &[&str], &str)` — no renderer, device or queue. Fourteen of the nineteen tests in `typed_outputs.rs` need no adapter at all. |
| mutation, width | `every_row_of_the_width_table_gives_its_own_verdict` walks all **seven** rows, reads the resolved width back out of the table, and asserts what each refusal *says* — not only that it fired |
| mutation, messages | `the_four_width_refusals_are_four_different_messages_that_name_no_protocol` compares all four whole messages pairwise and asserts the two numbers each has to carry |
| mutation, words | `no_refusal_in_the_output_path_names_a_word_from_underneath` — every refusal the output path can produce, ten of them, against one word list. **It found two real leaks**: "whatever the target was cleared to" in the writes-nothing message, and "a colour target" in `UnwrittenOutput`. Both are fixed. |
| mutation, table | `every_row_claims_no_more_numbers_than_the_image_it_is_drawn_in_holds` walks `TexelType::ALL`, so the declared-count question cannot be violated by a row added later |
| mutation, location | `a_location_that_is_not_its_points_is_named_before_the_type_is_ever_read` (pure) and `a_swapped_pair_of_same_typed_declarations_is_refused_before_the_draw` (adapter) |
| end to end, `vec3` | `a_three_component_output_is_written_read_back_and_the_frame_is_unchanged` on the adapter: declared `vec3<f32>`, read back, every component of the beauty frame within one eight bit step of what the shader wrote |
| end to end, `vec3` refused | `a_three_component_declaration_filled_by_three_components_is_refused_before_the_draw` — the pair the previous rule allowed, refused as a `RenderError::Backend`, which is how the test knows the **check** refused it and not the pipeline |
| padding versus marker | `a_three_component_sidecar_says_its_fourth_channel_carries_nothing` prints both headers under `--nocapture` |
| real render | `tidewater --debug position`, two cameras: beauty png, channel png, and a sidecar whose header is the channel's own |
| end to end | `a_scene_declared_output_is_written_read_back_and_carries_its_own_type` on the adapter |

## Two things this correction found that are not fixed here

1. **A document may declare a type its target does not match, and the check's authority is the
   declaration.** The width comes from the row the *declared type* resolves to, while the format
   the image is actually created with is the instance payload's own field (recorded as **not
   done** further up). A document declaring `f32` over an `rgba16float` target would now be
   refused for asking a scalar to fill a one wide image that is really four wide — a document the
   hardware would have accepted. Nothing in the workspace does this: every emitter writes
   `vec4<f32>`, and the table's own rows agree with each other. It is the same seam the "not done"
   note names, and closing it means checking the declared type against the bound target's format,
   which is a different check and not this one's.
2. `check_outputs`' refusal for a declared count larger than its image is **written but not
   reachable today**, because `TexelType::parse` cannot produce such a row. It is a guard on the
   table, and the row walk above is what holds the invariant it guards.

## A defect this change exposed — now fixed

`cargo fmt --all` normalised `const SURFACE: Block = wgsl!(...)` in
`crates/gallery/tests/shader_provenance.rs` from one line to three, and the provenance line
arithmetic was **wrong for that shape**: `origin_of` computed
`declared.line + (line - first)`, and `declared.line` is `line!()` at the invocation, so a literal
whose first line of WGSL is on the *next* line had every origin one line too high. The `wgsl!` doc in
`shaders.rs` claimed both shapes report the same line; they did not.

**Fixed.** The `+1` belongs at the writer rather than the reader — given `mark`'s contract `origin_of`
was always right, and the wrong number was the one `Block::spliced` handed it. It went into
`shaders.rs`, the one place that can see the invocation's shape. See
[wgsl-error-locations.md](wgsl-error-locations.md) for the repair and for why the same-line shape is
not achievable rather than merely unhandled: `cargo fmt` breaks it, so it cannot exist in formatted
source, and the `#[rustfmt::skip]` was holding back a state the formatter will not produce. The
skip is gone, `cargo fmt --all --check` is clean, and a new test reads every `wgsl!` declaration in
the gallery's sources and fails on the other shape.

On the real `tidewater` scene the reported line moved from a continuation line to the statement it
was meant to name. The `explain` sentence's text is unchanged; only its numbers moved, which is the
fix rather than a regression.

