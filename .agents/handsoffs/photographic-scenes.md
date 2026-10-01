# Photographic scenes: `tidewater`

Worktree: `./.worktrees/photographic`, branch `feature/photographic`, based on `main` at `1ee2ddc`.

A lake at first light, built against the standard pipeline, and the write-up of what the
framework could not say while building it:
[proposals/photographic-scenes.md](../proposals/photographic-scenes.md).

## Do not overlap `feature/lighting`

`.worktrees/lighting` is mid-way through a five stage effort that owns the lighting path.
This branch does not touch the per light pass generation, clustered assignment, the shadow
page pool or IBL. The one finding that reaches the lighting path — a mirror drawn once per
light — is answered by the emission stage that already exists, not by anything stage 3 or 4
changes.

The `lighting` handoff also moves `crates/gallery/src/*` to `crates/scene/src/gallery/*`.
`tidewater.rs` is a pure addition, so the rename merges; the files this branch *edits* are
listed in the proposal's *Touched* section and carry a real conflict.

## Status

| step | state |
| --- | --- |
| `tidewater` scene | renders; coherent, not finished |
| `stilllife` scene | **not started** |
| proposal | written, seven findings, the first four for review |
| `SPEC.align.md` | not synced yet |
| gallery render test | 2 passed on the adapter |

```text
cargo run --release -p whiting-device --features wgpu --example render -- tidewater out/tidewater 0
```

Both cameras are rendered; the wide one is the picture and the long lens is the treeline.

## What `tidewater` is

A flat lake, a far shore in silhouette, mist lying on the water and a sun behind the ridge.
Almost nothing in it is an object: the whole frame is water, shore, air, and the decisions
are about light rather than geometry.

- **The water is drawn by the emission stage, not the lighting stage.** Its colour is almost
  entirely a reflection, and a lighting pass adds that colour once per light — under two
  lights the lake came out twice as bright as the sky above it and went white. This is the
  framework working as designed and the stage being undiscoverable; proposal item 3.
- **The water is not in the `shadowing` bucket.** A flat plane a thousand units across at two
  degrees off the light is the worst case for shadow acne there is, and a flat lake casts no
  shadow anyone could see. What the lookup then returns is the far shore's shadow on the lake.
- **One sky, written once**, handed to the lighting stage, the emission stage and the post
  stage. `dunes` has this bug already and `SPEC.align.md` already records the symptom.
- **The camera is placed, not fitted.** At four units up, every visible part of the lake is at
  a grazing angle and the whole frame is one flat mirror; at nine the near water is seen
  steeply, goes dark, and the frame runs from a dark foreground to a bright horizon. This was
  the single change that made the scene a photograph, and it took the longest to find.

## What is not finished

Stated rather than left to be discovered:

- **The sun's reflection bands.** The glitter is a tight lobe on a wave-modulated normal, and
  where the waves line up it breaks into hard horizontal steps rather than sparkling. A
  wider lobe, or waves faded sooner, would fix it; neither is in.
- **The near bank reads as a sandbar, not as a corner.** It is a low mound at the right
  distance and the right size, but it is too far into the frame and its shoreline is too soft.
  It was replacing a bed of 340 reeds, which was worse: a thin object seen edge on takes the
  whole of the standard shading's grazing Fresnel and renders as a pale stick whatever tint
  it carries. Proposal item 4 is that bug.
- **A bright sliver at the left edge** of the long lens frame, at the water's boundary. Not
  diagnosed.
- **The sun disc shows through the treeline** in the wide frame. It should be behind the ridge;
  the shore's crest is lower than intended where `ridged_noise` spends most of its range low.
- **`stilllife` was not started.** The session went on `tidewater` for far longer than planned
  and the budget went there. It is the better half of the pair in principle — a placed camera
  and a real depth of field — but nothing of it exists.

## Findings

The seven are in the proposal. In one line each, because they are the durable output:

1. The surface body is spliced in after the shading, so it cannot displace the normal.
2. No value reaches two stages, so the same air is written twice and disagrees.
3. A mirror in a lighting pass is added once per light; the emission stage is the answer and
   nothing says so.
4. The specular is not normalised by roughness and the Fresnel goes to one at grazing.
5. `samples` is a field of the graphics schema the standard emitter never writes.
6. No exposure, no falloff a scene can set, and a lens is a field of view and nothing else.
7. The harness wrote both cameras to the same file.

Items 1 and 7 are fixed on this branch. Item 4 is a change to a template and would improve
every frame in the gallery at once.
