# Why two gpu test frames were black, and why neither was a shadow defect

**Measured 2026-10-16** on machine B, `whiting` at `main` `7d43a77`, RTX 3060 Laptop GPU, DX12
(no Vulkan ICD registered). Sources: `git blame` / `git show` on `crates/gallery/src/shaders.rs`,
the two test files, and `.agents/SPEC/render/light_units.md`; and the two test binaries run on the
adapter. Every number below was read off a run, not inferred.

## The question

`clustered_shadow_render` reported `0 darker pixels and 0 brighter ones` and
`environment_map_render` reported every probe at `0`. Both read as a broken page or a broken
lookup. Neither was one.

## Answer 1 - the stand-in is gone and two tests still asked for it

`environment_radiance` returns black for a scene naming no `Environment` light:

    crates/gallery/src/shaders.rs:2096
    fn environment_radiance(normal: vec3<f32>, roughness: f32) -> vec3<f32> {{
        if uniforms.environment.w < 0.5 {{
            return vec3<f32>(0.0);
        }}

`git blame` puts that line in `d36b4cb` (2026-10-08, "Render the gallery through Cycles, and
state what a light's power means"). It is a decision, not a defect:
`.agents/SPEC/render/light_units.md:60-66` says a surface receives the environment only when the
scene names one, that **there is no stand-in**, and that this was measured to be a large term.

**The frame is not black where the environment is present.** `a_rougher_surface_reads_a_coarser_
level_of_the_map` passed throughout and asserts `sharp_ground > blurred_ground + 100`, so the
ground probe is above 100 in both of its renders. The failing probes were the ones on the
environment-less scene and the ceiling. "The whole frame is black" is true of the fourth render
and false of the first.

## Answer 2 - two hand-built scenes were never re-derived for the new light units

`d36b4cb` put `1 / (4 pi^2 d^2)` into the shading:

    crates/gallery/src/shaders.rs:2008
    let attenuation = FLUX_TO_RADIANCE / (light_range * light_range);
    crates/scene/src/standard/photometry.rs:64
    pub const FLUX_TO_RADIANCE: f64 = 1.0 / (4.0 * PI * PI);      // 0.025330295910584444

so a lamp's `power` became a radiant flux. It re-derived every **gallery scene** against that,
including `terraces` and `tiles` which were not even on the old rule.

**Its diff on the two test files is an import reorder and nothing else.** Both build their scenes
by hand, so neither was re-derived and neither was re-run. `clustered_shadow_render` stated
`power = 3.0` and `2.4`, brightness multipliers of the model that no longer exists.

## The arithmetic, which is the whole of it

Lamps at `[-30, 20, -24]` and `[26, 18, -20]`; a ground at the origin with albedo 0.8.

| | distance | `power / (4 pi^2 d^2)` | x `N.L` x albedo | of 255 |
| --- | --- | --- | --- | --- |
| key, before | 43.31 | `4.05e-5` | `1.50e-5` | **0.004** |
| key, `flux()` after | 43.31 | `0.45` | `0.168` | **43 to 196** |

Measured before the fix, by instrumenting the existing test: `nonzero = 0 / 1048576`,
`max_channel = 0`, `mean_lum = 0.000`, on **both** frames. That is why the assertions read `0
darker and 0 brighter` — they were comparing two black frames, and a shadow cannot darken what is
already nothing. The test's own comment had already named this trap: *"a scene of ambient alone
is a frame that renders and casts no shadow - which is the same symptom as a broken page and the
opposite diagnosis."* The ambient it meant was removed the same week.

Measured after, at 1024x1024, three channels summed:

| frame | lit px | max | mean | clipped |
| --- | --- | --- | --- | --- |
| key lamp alone | 447475 | 586 / 765 | 83.4 | 0 |
| block vs no block | — | — | — | 70467 darker, 0 brighter |
| second lamp added | 505618 | 765 / 765 | 174.7 | 422 |

The floor each assertion asks for is 2000.

## Not a regression in the window it was pinned to

The two files were last touched in `5546ee3` (2026-10-10) and that is where the fault was looked
for. Built and run at `5546ee3` in a separate worktree: **identical failures, word for word.**
The cause is `d36b4cb` (2026-10-08), which is an ancestor of `main` (`merge-base --is-ancestor`
exits 0) and two days earlier.

## One stale record, not acted on

`.agents/SPEC.align.md:1729-1746` still heads its section "The environment **replaces** the
stand-in, and the stand-in is the stated fallback", and says a scene naming no environment "binds
a one texel black map ... The shader branches on `present` and takes the stand-in." The branch is
still there; what it takes is `vec3(0.0)`. That paragraph contradicts
`SPEC/render/light_units.md` and contradicts `shaders.rs:2245-2248` in the same function, which
also still describes the hemisphere. Not edited here: SPEC.align is an alignment note, and
`light_units.md` is the record that governs.
