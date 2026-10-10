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
