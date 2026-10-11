# Offline Backend

a `Scene` document rendered by a subprocess, for the frame that ships. the rasterised path is for
iterating while authoring; this is for the frame, and it is a path tracer's light transport rather
than a rasteriser's at any sample count.

see [SPEC/cycles_backend.md](cycles_backend.md) and [SPEC/light_units.md](light_units.md).

## Terms

### `Backend`

one way of turning a `Scene` document into a frame. more than one, and they answer the same
question about different documents rather than about the same one.

### `LossyMode`

what to do about scene data the target format cannot express.

- `Reject`: refuse to render. the default, because a silently dropped field shades as though it
  were zero and the picture looks like an authoring mistake rather than an export one.
- `Degrade`: render without it, and report it. for an author who knows the backend is narrower
  and wants the frame anyway.

a distinct mode rather than the absence of a flag, so that "I did not choose" and "I chose
strict" cannot be confused when a default changes.

### `LossyReport`

what a `Degrade` render left out. returned even on success, because a knowingly approximate
frame is a legitimate result and a silent one is not.

### `Resolution`

the size of the image, in pixels. **not** carried by the document: see
[cycles_backend.md](cycles_backend.md) for why the target format has nowhere to put it and the
`Backend` has to be told.

### `Frame`

the answer every `Backend` gives, in one type. a `Backend` that writes a file decodes it, so
that a caller holding a rasterised frame and a path traced one is holding the same thing.

### `Winding`

**the authored normal is the outward direction, and the winding obeys it.** a triangle's three
corners are written in the order that makes `cross(second − first, third − first)` point the way
the normals those corners carry point. this is stated once, here, because it was stated nowhere:
before it, 447 of the gallery's 620 meshes were wound the other way and 61 had no consistent
order at all, and the only reason either was visible is that one backend happened to disagree
about which side of a surface is lit.

**the normal is authoritative and the winding is the notation.** geometry is the asset; the order
of three corners is how the asset records that it has an outside. a reader may therefore rely on
`cross(b − a, c − a)` agreeing with the authored normal, and may not have to guess which of the
two is the one that was meant.

what the two are not:

- **not a culling rule.** a `Backend` need not reject a back-facing triangle. the rasterised path
  does not — there is no `cull_mode`, no `FrontFace` and no `CullMode` anywhere under `crates/`,
  and no `front_facing`, `faceforward` or `face_normal` in any `.wgsl` — so it shades from the
  authored normal and draws both sides. the convention is about what the winding *says*, and a
  backend that reads only the normals is not violating it.
- **not a shading rule.** where a surface is lit comes from the normal. the winding says which
  side the surface claims to be, and a `Backend` that reads no winding is not wrong, only silent.

**a normal perpendicular to its own surface is a malformed document.** no winding satisfies the
convention for such a mesh, so there is nothing to choose and the authoring mistake cannot be
papered over by re-ordering corners. two meshes in `studio` are in this state: the triangle is
built in the plane `z = 0` and carries the normal `(0, 1, 0)`, which lies inside that plane.
measured, and the evidence is in
[investigate/cycles-mesh-normals.md](../../investigate/cycles-mesh-normals.md).

**a mesh whose triangles do not share one orientation is not in a third state.** it is in the
same state as any other mesh that has not yet been brought into line, and it is brought into line
by the generator that wrote it. a `Backend` applies no rule of its own to guess which way a
reversal goes, because a reversal applied to a document that holds both kinds of triangle inverts
the ones that were already right.
