# Light Units

what a light's stated brightness means, in the units of the world rather than of a renderer.

see [SPEC/render/offline_backend.md](render/offline_backend.md).

## Terms

### `power`

a point or spot light's **total radiant flux**, in the units of the world. it is a flux and never
a radiance, so it is a property of the lamp alone and not of anything it lights.

a scene calibrated so that `power = 4 pi^2 d^2`, where `d` is the distance from the lamp to its
subject, puts that subject at a radiance of 1.0. that is the convention the gallery uses, and the
`d^2` reads as a squared distance, which is what the number is.

### `irradiance`

a directional light's **flux per unit area on a perpendicular surface**. it has no distance to be
a power over, which is the whole of what makes it the sun rather than a lamp, and why it is
attenuated by nothing.

### `radiance`

an environment light's **outgoing radiance**: light leaving the image per area per unit solid
angle. what a surface faces is then this, sampled by the direction it faces and blurred by its
own roughness.

one field cannot carry three units whose meaning depends on a tag two fields away, so a document
carrying the wrong one is refused.

### `Flux To Radiance`

`1 / (4 pi^2)`, and the only place it is written.

two divisions, and a document's `power` is upstream of both:

- a lamp's flux becomes an irradiance by spreading over a sphere of area `4 pi d^2`;
- a Lambertian surface returns radiance by dividing a surface's irradiance by `pi`, because the
  cosine-weighted integral of its lobe is `pi`.

so a white surface square to a lamp at distance `d` returns `power / (4 pi^2 d^2)`.

a `Directional` light needs `1 / pi` and not `1 / (4 pi^2)`: it has no sphere.

**a renderer that omits the BRDF's `1 / pi` must apply this**, or a light's stated flux will not
mean what it says. measured against a path tracer whose own normalisation is already correct —
see [.agents/investigate/cycles-light-units.md](../../investigate/cycles-light-units.md).

### a light's own size is not its reach

a scene's `radius` is **how far a light reaches** for cluster culling. a renderer's `radius` is
the **physical size of the source**. they are different quantities: a reach of sixty units would
become a lamp a hundred and twenty units across.

a reach is therefore not mapped. a renderer given a zero radius produces a delta light with hard
shadows, which is the honest rendering of a document that says nothing about source size.

### no ambient the scene did not ask for

a surface receives the environment **only** when the scene names an `Environment` light, and
**nothing** when it does not.

there is no stand-in. a scene naming no environment has no sky, in any backend, and a frame's
darkness is a property of the document rather than of a texture somebody bound.

this was measured to be a large term: on one gallery scene the rasterised floor sat at a linear
radiance of 0.132 and the scene's single lamp accounted for 0.011 of it, so the rasterised image
was mostly a picture of the stand-in.
