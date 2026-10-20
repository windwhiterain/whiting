## Spec

- [SPEC](.agents/SPEC.md), [SPEC/](.agents/SPEC/): the only source of truth, human editable only.
- [SPEC.align](.agents/SPEC.align.md), [SPEC.align/](.agents/SPEC.align/): agents' understanding and amends, never merged, can be altered.

## Investigate

- [investigate](.agents/investigate/): survey notes and external-technology research. never merged, never a basis for implementation on its own.
- each file answers exactly one question, records the date of every measurement, and states its sources.
- when a finding becomes a decision, it moves into SPEC. when it becomes a change to this repository, it moves into proposals.

## Proposals

if you have some ideas: new feature, fix, refactor, create a `<name>.md` under [proposals](.agents/proposals/).

## Render

- **Never draw a frame through the GPU.** The machine's gpu belongs to the owner, so
  `whiting-device-rasterize`'s `render` harness (wgpu) is not to be run. A frame is path-traced on the
  cpu instead: `scene_document` writes the document, then
  `render_cycles --document <path> <out.png> [samples] [CPU]` hands it to `cycles.exe` through
  `device-cycles`. Its reader takes the json document, so a plant plate is written as `.json` for a
  picture and as `.whiting` only for a measurement that needs no picture.

## Handoffs

- when start to implementation, create or follow a handoff under [handoffs](.agents/handoffs/). 
- during implementation, keep the handoff synced so other session can follow-up your implementation. 
- after an implementation is finished, delete the handoff.
