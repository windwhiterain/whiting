# Mountain

crate `mountain` (publish name `whiting-mountain`).

An agent-driven editing and authoring system that expands a polyline tree/network into a polygon mesh, for authoring complex realistic mountains.

## Motivation

- The ridge-line distribution of a real mountain must be *planned*: it has hierarchy (main ridge, spur ridges), direction, and rhythm. Noise functions cannot plan.
- Noise also cannot express the block faces of real rock mass: a real mountain flank reads as a set of planar facets meeting at sharp edges.
- Therefore the ridge distribution is authored as a polyline tree/network, and the polylines are then expanded into a polygon mesh. Triangles are the natural primitive for this: a triangle is exactly one planar block face, and the two flanks of a ridge are two fans of triangles meeting at the ridge line.

## Requirements

### R1: a polyline tree/network plans the ridges

- The ridge distribution of a massif is authored as a set of polylines forming a tree or network.
- Each polyline is one ridge line. Branching expresses the hierarchy from main ridge down to spur ridges.
- The polylines carry elevation, so the skeleton is a 2.5D object: plan-view polylines plus heights.

### R2: dual-line control — ridges and valleys

- Both ridge lines (high) and valley lines (low) are authored and editable.
- The ridge/valley line pair is the control structure of the massif: the block faces are the surfaces spanned between them.

### R3: derivable default lines, no guarantee

- Default lines may be auto-generated (e.g. valley lines derived from the authored ridges, or a generated initial ridge tree).
- Generated lines carry no correctness guarantee. Their only purpose is to produce an initial state for continued editing. Once generated they are ordinary editable lines.

### R4: expansion into a polygon mesh

- The polyline tree/network is expanded into a polygon mesh of triangles.
- Each triangle is one planar block face; ridge flanks read as facet fans meeting at the ridge line.
- The mesh is the expanded form of the edited lines: every face traces back to the skeleton, and editing the skeleton re-expands the mesh.

### R5: pure rust crate, rust api

- The system is a pure rust crate with a rust api. It has no editor frontend and no runtime service of its own.
- Agents author by writing and modifying rust construction code against this api.
- A DSL for authoring is planned for the future and is out of scope now.

### R6: the output mesh mirrors the raw edit

- Building outputs a mesh that reflects the raw editing result of the skeleton, without simulation or beautification passes.
- Erosion and all further processing live in other, downstream pipelines and are out of scope for this crate.

## Out of scope

- DSL and any serialization format.
- Erosion, simulation, shading, texturing, and any other downstream processing.
- Editor frontend, undo/redo service, renderer integration.
