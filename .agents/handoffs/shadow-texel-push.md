# The receiver's push, in the units of the recording

Follows [shadow-probes.md](shadow-probes.md), which found the fault, and
[virtual-shadow-maps.md](virtual-shadow-maps.md), which is the design the fix holds to.

Status: **the pages path is fixed and measured; the pool path is untouched and carries the same
constant.** One file, `crates/gallery/src/shaders.rs`, one function: `shadowed_pages`. The
second hand-written copy in `crates/scene/src/standard/example.rs` did not have to move, and
this is worth stating plainly because it was expected to: **that copy has no pages path at
all.** Its `shadowed` is the pool path only, and it carries no normal offset whatsoever — it
takes a larger depth bias (`0.002 + 0.04 * slope` against the gallery's `0.0006 + 0.003`) and
pushes nothing. So the two copies never carried the same number, and there was no second copy
of this defect to keep in step.

## What the number was, in the units that matter

`shadowed_pages` read the shadow at `world_position + normal * 1.6`. A grid eight pages a side
over a cone seventeen units wide holds `8 * 128 = 1024` texels across the light's plane, so a
texel of that plane is `17.63 / 1024` world units **at the far plane** and finer everywhere
nearer. Measured against the probe scene's own lamp, where the shadow falls, one texel is
**0.0116 to 0.0127 world units** — so the push was **a hundred and thirty texels**, or 126 to
166 across the four points sampled.

That is the whole defect in one ratio: a push far larger than the kernel it protects. A caster
shorter than the push is not between the lamp and the point being read, so it casts nothing —
not softly, not in the wrong place, not at all.

## The fix

One texel of the plane **at the fragment**, read out of the light's own projection rather than
written down:

    one texel = 2 / (side * per_unit) world units

`per_unit` is what one world unit is worth in the plane's own normalised coordinates, taken
from the projection as `(span.xy - ndc.xy * span.w) / w`. The step is measured along whichever
of the two world axes lies **further across** the light's view, not along the normal: a step
along the normal of a surface standing square to the lamp does not move in the plane at all,
and the reciprocal of no movement is not a distance. Measuring across the plane is what makes
the number finite everywhere, and it is why the guard is a `max` on a denominator rather than a
branch on a degenerate case.

The push stays in **level zero** texels and does not follow the level, because the level is not
known until the page is found and the page is found from this point. A coarse page needs more,
and the bias already scales with the level for exactly that reason.

## What the measurement caught that the arithmetic did not

**Naming the page from one point and the texel from another wraps the walk's own arithmetic.**
The first version projected twice: once to pick the page from the un-pushed point, once to read
the depth from the pushed one, and took the texel within the page from the second. Crossing a
page boundary wraps `(texel % shift) >> level` and reads a texel up to a page away. It put hard
black bands across the 3.4 wall, and **the bands were as wide as the push that made them** —
thin at one texel, thick at eight, which is what named the mechanism. One point now names both,
as it did before the change. Worth remembering as a shape of bug: a walk whose page and whose
texel come from different points is not a walk.

## The numbers

Probe six, one box, one standing, one density, one grid, varying only height, 1024², real gpu.
`shadow px` and `soft` are the harness's own; the last two columns are a pixel diff of the two
frames, which is the only thing that describes the 3.4 row honestly (see below).

| height | shadow px before → after | soft before → after | leaks before → after | pixels changed (all but 95 darker) |
| --- | --- | --- | --- | --- |
| 1.3 | 665 → **3909** | 387.1% → **4.2%** | 0 → 0 | 5416, confined to x 759..908 y 564..636 |
| 1.7 | 912 → **5797** | 417.2% → **3.1%** | 0 → 0 | 8270 |
| 2.1 | 749 → **7915** | 627.4% → **2.6%** | 0 → 0 | 10108, 23 lighter |
| 3.4 | 3024 → **0** | 76.5% → — | 0 → 0 | 10674, 95 lighter |

**The 3.4 row's zero is the harness skipping the caster, and it is not a shadow that
disappeared.** `shadow_footprint` projects the box's top face onto the floor, and for a 3.4-tall
box at that standing that quad lands about **13.8 world units from the lamp, outside the cone's
ground pool**, where the floor is unlit anyway: `lit - darkest` falls under the probe's own
minimum spread and the caster is counted as nothing. The 1.3, 1.7 and 2.1 footprints land 6.4,
7.6 and further in and are measured normally. The number that row carried before the fix —
3024 pixels, 76.5% of them partial — was a shadow *displaced* by the old push, landing where the
footprint and the pool still overlapped. The frame itself is verified by eye and by the diff: a
complete, hard-edged, acne-free shadow of the 3.4 box, reaching the frame's right edge, with
**zero lighter pixels anywhere on the caster**.

## The rest of the budget

The control row is unmoved: 8 pages a side, 4 levels, 85 pages, an atlas 10 to a side, 252
passes. `spotlight` casts a shadow for the first time — its 1.3-tall wide box throws a clear
hard-edged one to the right, and the tall and thin boxes throw theirs behind the boxes standing
in front of them, so the scene's composition hides two of its three. `stilllife` and `monoliths`
are **bit for bit identical** to `main` (`b4829019…`, `1e58e1ac…`), and `out/stilllife-before.png`
from before the virtual shadow work hashes the same as both.

## What is deliberately not here

**The pool path's 1.6 is untouched.** `shadowed` carries the same constant and the same latent
fault, and how much of it bites depends on a page fitted to the whole scene rather than to a
cone, so a scene with a small pool and a big floor pays more for it than this one did. It is
left alone because `stilllife` — thirty five point lights, all on the pool — is the control that
says this change did not move the pool, and moving the pool's constant moves that control. It
is a one-line change of the same shape when the owner wants it, and it wants its own
measurement rather than this one's.

**`shadow_probe.rs`'s own documentation for probe six still describes 1.6 as present.** It is
the harness, it is the evidence, and its wording is its author's; the note belongs there rather
than in a commit that is not the author's.
