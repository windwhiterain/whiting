# Investigate

Survey notes and external-technology research. Nothing here is authoritative for
implementation. When a finding becomes a decision, it moves into
[SPEC](../SPEC.md). When it becomes a proposed change to this repository, it
moves into [proposals](../proposals/). When it becomes work in progress, it
moves into [handoffs](../handoffs/).

## Contents

| File | Question it answers | Recorded |
| --- | --- | --- |
| [rust-offline-renderers.md](rust-offline-renderers.md) | What offline ray tracing renderers exist in the Rust ecosystem, and how mature are they? | 2026-10-07 |
| [offline-renderer-backend.md](offline-renderer-backend.md) | Which mature open source renderer should produce final output, and how should we drive it from a scene compiler? | 2026-10-07 |
| [cycles-standalone-scene-format.md](cycles-standalone-scene-format.md) | What does the Cycles Standalone XML scene format accept, and can `StandardScene` be emitted into it? Format read from source, and `principled_bsdf`, polygon triangulation, sun orientation and the `exposure` multiplier confirmed by rendering. | 2026-10-07 |
| [cycles-standalone-windows-build.md](cycles-standalone-windows-build.md) | Can Cycles Standalone be built on this machine, and what does it cost? **Built and rendering** — `cycles.exe` 5.3.0, including the two traps that cost time. | 2026-10-07 |
| [cycles-procedural-materials.md](cycles-procedural-materials.md) | Can this Cycles build render a procedural material, and is OSL the mechanism? **Yes, natively** — the texture and shading nodes are all registered and render with zero errors, and OSL is not compiled into the executable at all. | 2026-10-10 |
| [cycles-world-axes.md](cycles-world-axes.md) | What does the y-up to z-up change actually do to a render? The sky, the pitch and the row convention. **Left and right were never in this file's scope** — see [cycles-camera-frame-handedness.md](cycles-camera-frame-handedness.md). | 2026-10-10 |
| [cycles-camera-frame-handedness.md](cycles-camera-frame-handedness.md) | Which way round is the exported camera, and why does Cycles need a mirrored frame? **Cycles draws the camera's local `+X` on the right, so its camera frame is left-handed and a camera transform that is a rotation cannot be right.** | 2026-10-10 |
| [shaded-image-eight-bit-ceiling.md](shaded-image-eight-bit-ceiling.md) | How much light does the rasterised pipeline lose before its tone curve runs? **19.6% of the drawn surface is already clipped at one, and it comes out as a plateau 17 levels wide.** The curve exists, the output does not clip, and `exposure` cannot reach it. | 2026-10-10 |
| [hierarchical-spline-bases.md](hierarchical-spline-bases.md) | Which spline bases support genuinely local refinement and coarsening, and what has reached production? **Largely superseded** — the hierarchy here is a storage and editing decomposition over one flat spline, not a nested spline space. | 2026-10-07 |
| [nurbs-izing-arbitrary-polygons.md](nurbs-izing-arbitrary-polygons.md) | How do you turn an arbitrary polygon mesh into editable NURBS, and what do kernels support afterwards? **Superseded** — that is fitting, and this project does not fit. | 2026-10-07 |
| [sparse-nurbs-representations.md](sparse-nurbs-representations.md) | Where do NURBS-based and learned representations win and lose at sparse, locally editable geometry? **Largely superseded** by the same finding. | 2026-10-07 |
| [nurbs-libraries.md](nurbs-libraries.md) | Which Rust/C++/Python libraries can implement this, and what do the licences cost? Current, including the multi-patch continuity gap. | 2026-10-07 |

## Conventions

- Each file answers exactly one question. If a file answers two, split it.
- Record the date of every measurement. Star counts and last-push dates decay.
- Every factual claim states its source. Anything read from a secondary source,
  a mirror site, or an AI summary page is marked as unverified and must not be
  used as a basis for implementation.
- Record negative results. "We looked at X and rejected it, for reason Y" is
  as valuable as "we chose X", and it stops the next agent from repeating the
  search.
- Write in English. Keep identifiers, file paths, and command lines verbatim.