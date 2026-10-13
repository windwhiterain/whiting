# Name the environment map's colour space

## The question

`cycles_export::scene::write_world` writes

```xml
<environment_texture name="world_texture" filename="air.png" />
```

with no `colorspace`. The loader answers for an unnamed space by falling back to scene linear, so an
eight-bit sRGB PNG is used directly as linear radiance. On `landscape` this costs the traced sky
forty six per cent of its level, and it is a defect in the exporter rather than in the renderer: the
exporter knows the bytes it is writing are sRGB-encoded and is not saying so.

The measurement, the three wrong explanations it took to find, and the sources are in
`investigate/the-air-image-reaches-cycles-with-no-colour-space.md`. In one line: naming the space
takes `landscape`'s whole-frame mean from **0.831** of the rasterised frame to **0.993**.

## What changes

**One attribute, one site.** `crates/scene/src/cycles_export/scene.rs`, in `write_world`, on the
`environment_texture` leaf:

```rust
writer.leaf(
    "environment_texture",
    &[
        ("name", "world_texture".to_owned()),
        ("filename", name_for(label)),
        ("colorspace", ENVIRONMENT_COLORSPACE.to_owned()),
    ],
);
```

There is no second site to keep in step. A grep of the whole emitter finds one `filename` attribute
and one image-bearing node; `image_texture` is never written, because nothing in the schema carries a
texture onto a surface. A constant is still worth naming rather than inlining, because the name is a
fact about the loader that the comment has to state and the code should not have to repeat.

## Why `__builtin_srgb` and not `sRGB`

`sRGB` is what a person would write and this build **rejects** it — `Color space 'sRGB' could not be
found` — falling back to exactly the scene-linear read the change exists to remove. The result is a
frame byte-identical to the shipped one.

`__builtin_srgb` is the string Cycles binds to `u_colorspace_srgb` (`src/util/colorspace.cpp:28`),
and that socket is the one `EnvironmentTextureNode` declares (`src/scene/shader_nodes.cpp:542`).
`colorspace.cpp:121-123` routes it through `to_scene_linear`, which is the sRGB electro-optical
transfer function — the decode the PNG format already implies.

The better property is that it does not depend on the machine. A name that resolves only where a full
OpenColorIO configuration happens to be installed is the wrong thing for an exporter to hard-code; a
builtin name is the same on every machine and needs no environment.

## What this does not do

**It does not close `landscape`.** After the change the sky is still 0.82 to 0.87 of the rasterised
frame and the near ground is 1.17 to 1.24 hot, in opposite directions. The whole-frame 0.993 is an
average over two errors that do not cancel cleanly, and must not be reported as agreement. The two
residuals are a separate piece of work and are not folded into this one.

**It does not silence every colour space warning.** The same empty-name warning is emitted by a
document with no environment texture in it at all, so a second, unidentified lookup is also unnamed.
Chasing that is not this change's job; recording it is.

## Verification

Done, in `e75a18e`.

1. `cargo run --release -p whiting-device-cycles --example render_cycles -- landscape out/cs-landscape.png 32`
   and the row-band table from `out/probe-world-colorspace.py`, against the same rasterised frame
   `f7c00ad9cc65d009`.
2. **The emitted document is byte identical to the hand edited variant** the measurement was taken
   on (`out/cs-landscape.cycles.xml` and `out/wc-named.cycles.xml` are the same sha256), and so is
   the frame (`e1c3b08c5ab3e6cd` both). The number above is therefore what the shipped emitter
   produces, not what a hand edited file produces.
3. `cargo test --release -p whiting-scene` passes.
4. Re-run `out/probe-census-both-graded.py` over the whole gallery. **Not done** — `dunes` and
   `terraces` carry skies as well and their bounce-free ratios were 0.952 and 0.928; they are
   expected to move and it is not known which way, so they are measured rather than predicted.

### The check that was planned and turned out to be worthless

The first version of this file said the warning pair "must shrink to one line — the one belonging to
the other lookup — and not to zero", on the reasoning that it named the environment map's space.

It does not move at all. `out/wc-named.log` still carries the same two lines that
`out/wx-a-shipped.log` does, and so does `out/wx-c-unconnected.log`, which is a document with **no
environment texture in it**. `colorspace.cpp:400` caches the miss under the empty name, so the first
node to ask warns and every later one is silent. The count is independent of what was fixed.

**A check whose result cannot move is not a check.** The image is the evidence here, and the image
moved.

## Not included

**No new test.** None was requested, and there is no existing test that reads back an emitted
document's world block. If one is wanted later it has to earn its place against a failure this change
actually prevents, which is a scene rendering a sky that is 46 per cent too dark with no error
anywhere — a test that asserts the attribute is present is a test of the emitter against itself.

**The strength is untouched.** The document writes `strength` from the light's own tint and radiance
and that mapping is not what is wrong here; `out/probe-world-colorspace.py` shows the ground already
reading high, so raising the strength would make that worse.