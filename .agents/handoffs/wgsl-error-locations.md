# WGSL error locations: provenance carried in the shader's own source

Status: **implemented, built, tested and rendered. Not committed**, and not reviewed by anyone
but me. `AGENTS.md` asks for this file to be deleted when an implementation is finished; it is
kept because the task that produced it asked for a handoff, and because two things in it are
decisions the parent may want to overrule — the scope extension into `tidewater.rs` and the
decision **not** to attempt the stage-scoping refusal. Delete it when the work is merged, as the
other five were.

**The off-by-one in `origin_of` is fixed**, and the fix is a change of shape rather than a change
of arithmetic: see *The other shape cannot be reported* below. `origin_of` itself is unchanged,
because the reader's contract — a marker carries the line the block's first line of **text** is
on — was always right; what was wrong was the line the gallery handed it. The `#[rustfmt::skip]`
that kept the test in the unreportable shape is gone, and the sentence a device prints now names
the line the WGSL is on for every block in the shipped scene.

Item 6 of [agent-scene-iteration-loop.md](../proposals/agent-scene-iteration-loop.md), which
is numbered 7 in the file's own sections. Nothing under `.agents/SPEC.md` is touched; the
contract is recorded in [SPEC.align.md](../SPEC.align.md).

## The design question, answered

The task posed three ways for provenance to travel from the code that concatenated the WGSL to
the compiler that reported the parse error, and asked which one to build. The answer is **the
first**, with the map carried **inside the shader's own source** rather than beside it, and the
reason is a fact about this codebase rather than a preference:

> `emit.rs`'s `shader_entry` rebuilds the instance entry a device reads with `source` and
> `schema` and nothing else. A map carried *beside* the string is dropped at the emit
> boundary, and that boundary is in `crates/scene`.

The source is the only part of a shader that survives the trip from a scene document to a
device, so the map travels inside it, as **one WGSL comment line above each block the assembler
knows the origin of**:

```text
//@whiting surface crates\gallery\src\tidewater.rs:720 lines=21
```

The path is spelled the way `file!()` spells it and the line count is the block's own, so the
reader needs nothing else: the marker says which lines of the assembled module the block owns by
saying how many lines it has, and where the first of them is.

`naga` reads a comment as a comment, so the module is unchanged, and `graphics::provenance`
scans for the marker **only when a parse has already failed** — the happy path costs nothing
and touches nothing.

Two things follow, and both are the additive property:

- **A block that declares no origin gets no marker**, so its bytes are exactly what they were.
  Every gallery scene except the migrated one is byte-for-byte unchanged today, and a scene that
  opts in changes only by comment lines.
- **A line the markers do not cover gets no sentence**, so the message for a shader that
  declares nothing is the message it was.

## The origin is declared, not inferred

A `Block` is either an ordinary `&str` — today's shape, no origin, no marker — or one written
with the new `wgsl!` macro, which captures `file!()` and `line!()` at the invocation:

```rust
const SHORE: Block = wgsl!(
    r#"
    color = color * (0.4 + 0.6 * shore_tint);
"#
);
```

This is the shape [`texture-channel.md`](../proposals/texture-channel.md) arrived at: *a
declared rule is a rule that can be changed safely; an inferred one is not.* The alternative —
inferring the origin from `#[track_caller]` on the builder, which names the line the **scene
called the builder on** rather than the line the WGSL is on — is the inferred kind, and it is
the kind that points at the wrong line.

**The line is the line after the one the macro was invoked on.** `line!()` inside a
`macro_rules!` expansion reports the line the invocation **starts** on, and the block's first
line of WGSL is the line after it: a literal opening on the next line puts the block's first
line of text — an empty one — there, and every line of the block from there on is one line on.
So `Block::spliced` writes `invoked_at + 1` into the marker, `mark`'s `line` is the line the
block's first line of **text** is on, and `origin_of` adds only the offset within the block.
Nothing in `graphics::provenance` reads a declaration any more: the reader is shape-free and
the one shape that only the writer can see lives in the writer.

### The other shape cannot be reported, and that is not a choice

Two invocations holding **the same literal** and starting on **the same line** put the block's
first line of text on different lines — one where the literal opens on the invocation's own
line, one where it opens on the line below — and nothing in the expansion tells them apart.
No arithmetic over `line!()` and the literal's text can therefore be right for both, and a
`macro_rules!` cannot ask.

The arithmetic this code had — passing `invoked_at` itself — is right only for the shape
`cargo fmt` **destroys**, which was the whole of the defect. Verified directly: `cargo fmt`
rewrites

```rust
const SURFACE: Block = wgsl!(r#"
```

into

```rust
const SURFACE: Block = wgsl!(
    r#"
```

so the one-line shape cannot be left in a formatted file at all, and the
`#[rustfmt::skip]` on the test constant was holding back the formatter to preserve the one
shape whose arithmetic happened to work. **The skip is gone**, the constant is in the shape the
formatter writes, and the arithmetic is the one that shape needs. A test
(`every_block_the_crate_declares_is_written_in_the_shape_the_macro_can_report`) reads every
`wgsl!` declaration in the gallery's own sources, doc comments included, and holds it to the
shape, so a block written in the unreportable shape fails a test rather than being reported a
line high with nothing to say so. The `wgsl!` doc in `shaders.rs` states the shape and what
the other one does.

## Why there is no note here, and no compile-time refusal

The task asks for an error rather than a note, and asks why that differs from the camera and
scale notes that landed today. Those describe a **legitimate** scene: the render succeeds and
the frame is a smaller one than was asked for, so a note travels beside a picture. This one
describes a scene that **cannot render as assembled** — the render already fails, and the error
already exists. There is nothing to note. What changes is where the failure points.

The refusal option — the assembler refusing to splice a block into a stage that does not
declare it — was the design the task inclined towards, and it is not built. Two reasons, both
structural:

1. **There is nowhere to refuse from.** `mesh_shaders`, `post_shader` and their siblings return
   `Vec<Value>`, not a `Result`. Making them fallible changes every call site in every scene,
   and a panic in their place is not this project's style.
2. **Nothing downstream can check it either.** The stage a shader belongs to is the *variant
   tag* of its scene-document entry, and `shader_entry` drops the tag: the device is handed
   `source` and `schema` and has no way to know it is parsing a `lighting` stage rather than a
   `post` one. So a declared stage set has no referee — the gallery that would enforce it cannot
   report, and the device that could report cannot know.

So item 6's **stage-scoping** failure (a post-only `ray_through` in a block injected into every
shader) is **not addressed**, and the smallest change that would address it is: carry the
splice points' stage names in the marker, carry the shader's own stage through `shader_entry`,
and refuse in `parsed_module` when they disagree. Both halves are in `crates/scene` and
`crates/device`, and the first is out of this task's scope.

The third recorded failure — a raw-string terminator defect that produced a **shifting**
identifier and buried `error[E0765]: unterminated double quote string` at the end of the
output — is an **agent's own editing mistake**, and nothing here would have prevented it. A
line number would have shortened the search. It is recorded here because the evidence for this
item should be read with that in mind: **one of the three failures is not the framework's to
fix, and the other two are the same missing declaration.**

## What is here

- `crates/graphics/src/provenance.rs` — the marker vocabulary, one module holding both the
  writer (`mark`) and the reader (`origin_of`, `explain`), so the syntax is written once.
- `crates/gallery/src/shaders.rs` — `Block`, the `wgsl!` macro, and the splice points
  (`Block::declared` at the top of the file, `spliced` in every template that takes a block).
- `crates/device/src/wgpu_renderer/mod.rs` — `parsed_module` appends the sentence.
- `crates/graphics/tests/provenance.rs`, `crates/gallery/tests/shader_provenance.rs` — the ten
  tests above, six of them in the gallery: the marker in a real assembled stage, the exact line
  mapping read back out of the test's own source, the shape the macro is written for, every
  declaration in the crate held to that shape, the additive property, and the splice point's
  name.
- `.agents/SPEC.align.md` — the contract, under *A shader says which file and line each of its
  blocks was written at*.
- `crates/gallery/src/tidewater.rs` — three blocks migrated, so the feature is live in a scene
  that is rendered. **This is outside the scope the task drew** (`shaders.rs` plus graphics and
  device) and is reported as such; it is three `const` declarations, their closing delimiters,
  and the imports the macro needs.

## Verification

Run in this order, on the tree as it stands (six other sessions have uncommitted work in the
same files, so nothing here was stashed).

**1. Build and clippy.** `cargo build -j 2 -p whiting-graphics -p whiting-gallery` is clean, and
`cargo fmt --all --check` exits 0 with no `#[rustfmt::skip]` anywhere.
`cargo clippy -j 2 -p whiting-graphics -p whiting-gallery --all-targets` reports **nothing in
any of the five files this work owns**: the one warning in `shaders.rs` is `lighting_bindings`'
loop counter at `shaders.rs:535`, which this change does not touch, and the rest are in
`whiting-definition` and `whiting-scene`.

**2. Tests.** `cargo test -j 2` (workspace, default features): **135 passing, 0 failing, 1
ignored**. `cargo test -j 2 -p whiting-graphics -p whiting-gallery` is green on its own, and the
six gallery provenance tests are the ones that carry this fix:

```text
test a_block_that_declared_nothing_is_spliced_in_verbatim ... ok
test a_declared_block_is_spliced_under_a_marker_naming_its_file_and_line ... ok
test the_same_text_in_two_stages_is_two_blocks_with_two_meanings ... ok
test the_block_starts_on_the_line_after_the_one_the_macro_was_invoked_on ... ok
test an_assembled_line_maps_back_to_the_exact_line_of_the_file_it_came_from ... ok
test every_block_the_crate_declares_is_written_in_the_shape_the_macro_can_report ... ok
test result: ok. 6 passed; 0 failed
```

`crates/device` was **not** built with `--features wgpu` for this: the sentence's text is
unchanged and no test asserts it (`explain` is called at `wgpu_renderer/mod.rs:354` and written
at `provenance.rs`, and nothing else names it), so the wgpu build would have cost a wgpu and
naga rebuild for no new evidence. What changed is the numbers the sentence carries, shown in
item 4.

**3. The additive property, by SHA-256.** `dunes` and both `tidewater` cameras, rendered with
the marker written and with `graphics::provenance::mark` short-circuited by a one-line switch in
its own path (the technique a previous worker used, since `git stash` is unusable in this tree):

| image | marker written | marker not written |
| --- | --- | --- |
| `tidewater` camera 0 | `fde43a8582c9a945…` | `fde43a8582c9a945…` |
| `tidewater` camera 1 | `b966e28a5232c0e7…` | `b966e28a5232c0e7…` |
| `dunes` | `c22e66459d5e675c…` | `c22e66459d5e675c…` |

Full hashes are in the session record; the three pairs are equal. The scene *is* migrated to
`wgsl!`, so the source it renders is not the source it rendered before — the marker is in it, and
the pixels are the same. The off-by-one fix does not disturb this: the marker's **line number**
is what moved, and a marker's content is still a comment, so a render after the fix is byte-equal
to a render before it. The hashes above were not retaken, because the claim they support is
about the marker's being a comment and nothing in this change touched that.

**4. The off-by-one, before and after, on the scene that ships.** `tidewater`'s `surface`
block (`SHORE`) and its `displace` block (`REED_SWAY`) are both `wgsl!` invocations of the
shape `cargo fmt` writes, and both were reported one line high. Read out of the assembled
scene document's own shader sources, with the reported line looked up in `tidewater.rs`:

Before:

```text
//@whiting displace crates\gallery\src\tidewater.rs:701 lines=8
//@whiting surface crates\gallery\src\tidewater.rs:722 lines=21
assembled line 657 -> reported 708 | tidewater.rs says: -normal.x * sin(sway) + normal.z * cos(sway));
assembled line 780 -> reported 731 | tidewater.rs says: // bring it down, because the tint is not in the specular.
```

After:

```text
//@whiting displace crates\gallery\src\tidewater.rs:702 lines=8
//@whiting surface crates\gallery\src\tidewater.rs:723 lines=21
assembled line 657 -> reported 709 | tidewater.rs says: normal = normalize(vec3<f32>(turned.x, normal.y, turned.y));
assembled line 780 -> reported 732 | tidewater.rs says: color = color * 0.55;
```

Both "before" lines are the **middle of a different statement and a WGSL comment**, which is
what a one-line error costs a reader: the second one names a sentence of prose rather than the
line that failed to parse. Both "after" lines are the text, checked by reading the file rather
than by trusting the number.

**What changed for a reader.** Only the numbers: the sentence is byte-for-byte the same and is
still `this line came from the `X` block in FILE:LINE (the block starts at line Y)`. A block
declared in the shape `cargo fmt` writes now reports the line its WGSL is on, and its block
line is the literal's opening line rather than the invocation's — so `(the block starts at line
N)` is a line a reader can open and see the whole block below. Every message for a block
declared any other way is one line high, which is why the one other shape is now named in the
macro's doc and held out of the crate by a test rather than left to be believed.

**5. The stage-scoping failure: not addressed**, and the reason is in *Why there is no note
here* above — there is no channel to refuse from and no stage for a checker to check. What would
be needed is written down there too, and the first half of it is in `crates/scene`.
