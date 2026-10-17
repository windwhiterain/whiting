//! Leaves: one blade authored once, and the rigid frames that instance it.
//!
//! # The one idea
//!
//! **A leaf is a template and a frame, and the only number between them is one uniform scale.** The
//! blade is a [`shape::Sweep`] of a midrib by a lens section — a **closed solid**, so both of its
//! faces carry their own outward normal and a leaf lit from behind needs no shading trick
//! (`.agents/proposals/agentic-vegetation-generation.md` D18) — and every leaf of a field is that one
//! mesh placed by a [`LeafInstance`]. **No instance bends and no instance has a shape of its own**: a
//! bend is geometry, so it is one material per leaf, and this design pays for that only where a field
//! is explicitly named. Variety across leaves is a *second* template, named by
//! [`LeafField::template`], rather than a number per instance.
//!
//! # The template's coordinates are the contract
//!
//! The blade is authored in its own frame: the midrib starts at the origin with its tangent `+x`, the
//! blade lies in the `xy` plane and its face looks along `+z`. So an instance's `x` is forward, `y`
//! is side and `z` is the face normal, and a template point reaches the world as
//! `instance.placement.place(point) * instance.scale` — one multiplication, because the placement is
//! rigid and the scale is uniform. [`LeafTemplate::of`] states the frame rather than leaving it to
//! the midrib's own default, so the three axes mean the same thing for every authored blade.
//!
//! # What a field owns
//!
//! [`LeafField`] is the edit surface: a band of the parent's length, a density law that counts the
//! leaves and places them within it, a phyllotaxy that advances each leaf's slot around the parent's
//! loop, and the pitch, twist, size and jitter laws that orient and scale them. Every leaf of the
//! field is one row in that arithmetic, so *"fewer, and smaller, above six metres"* is a curve edit
//! rather than four thousand edits, and a node's numbers come from its own [`crate::rng`] stream so
//! inserting a bough elsewhere leaves this crown where it was.

use shape::math::{Vec3, clamp};
use shape::{Curve, Loft, Scalar, Section, Sweep};

use crate::handle::Handle;
use crate::node::Attachment;
use crate::plant::{Placement, Plant};

/// The blade's width axis in the template's own coordinates: the section's `x` half axis, which
/// [`shape::Curve::set_initial_normal`] places on `+y`.
const SIDE: Vec3 = [0.0, 1.0, 0.0];

/// A blade authored once: a midrib, the section it sweeps, and the law that widths it.
///
/// # Invariants
///
/// **Its own coordinates are the contract** (see the module docs): the midrib starts at the origin
/// with its tangent `+x`, and [`LeafTemplate::of`] sets the frame's initial normal to `+y`, so the
/// section's `x` half axis — the blade's width — runs along the template's `y` and its `y` half axis —
/// the blade's thickness — along the template's `z`, the face.
///
/// The sweep carries **no twist of its own**: rolling a blade about its own midrib is per-instance
/// ([`LeafField::twist`]), which is a rotation of a frame rather than a change of a baked mesh.
/// `stations >= 2` and `around >= 3` are [`Sweep::sample`]'s assertions, so they are made there and
/// are not checked twice here.
#[derive(Debug, Clone, PartialEq)]
pub struct LeafTemplate {
    sweep: Sweep,
    stations: usize,
    around: usize,
}

impl LeafTemplate {
    /// A blade: `thickness` is the lens's half axis across, `width` the half axis out.
    pub fn blade(
        midrib: Curve,
        width: Scalar,
        thickness: f32,
        stations: usize,
        around: usize,
    ) -> Self {
        Self::of(
            midrib,
            Section::flat(1.0, thickness),
            width,
            stations,
            around,
        )
    }

    /// The same with an arbitrary section.
    ///
    /// # Invariants
    ///
    /// The blade is **capped**, which is the difference between a solid and a tube: an uncapped sweep
    /// has two boundary loops and no interior, so [`shape::mesh::signed_volume`] — the check D18 makes
    /// the leaf's own — has nothing to measure. `width` is the sweep's `scale`, so it multiplies both
    /// of the section's axes and a section that must change *shape* along the midrib is a
    /// [`Section::outline`] rather than a second law.
    pub fn of(
        mut midrib: Curve,
        section: Section,
        width: Scalar,
        stations: usize,
        around: usize,
    ) -> Self {
        // `shape` puts a section's `x` half axis on the frame's `normal`, so `+y` is what makes the
        // blade's width the template's side. A curve's own default is a perpendicular of its first
        // tangent's smallest component, which for a `+x` midrib is `-z` — so the blade's width would
        // run along `z` and stand the blade on its edge. The contract is stated here, not defaulted.
        midrib.set_initial_normal(SIDE);
        let mut sweep = Sweep::new(midrib, section);
        sweep.scale = width;
        Self {
            sweep,
            stations,
            around,
        }
    }

    /// The blade in the template's own coordinates: base at the origin, forward `+x`, face `+z`.
    ///
    /// A width law that reaches zero collapses that ring to a point and the cap stays, so a blade
    /// pointed at both ends is a closed solid without a special case in the walk.
    pub fn loft(&self) -> Loft {
        self.sweep.sample(self.stations, self.around)
    }

    /// The blade's own spine, in the template's coordinates.
    pub fn midrib(&self) -> &Curve {
        &self.sweep.spine
    }

    /// How many rings [`LeafTemplate::loft`] draws.
    pub fn stations(&self) -> usize {
        self.stations
    }

    /// How many vertices each ring holds.
    pub fn around(&self) -> usize {
        self.around
    }
}

/// One placement of a template on one node.
///
/// # Invariants
///
/// **`(station, slot)` is the whole placement of the leaf on its parent** — the same [`Attachment`] a
/// branch uses, resolved on the parent's swept surface rather than searched for, so
/// `placement.origin` is that attachment in the world. `index` is the leaf's rank among its node's
/// leaves, which is its last step in a report; `placement` is rigid and `scale` is one uniform
/// number, so the two cannot disagree about the blade's size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeafInstance {
    /// The leaf's index among its node's leaves, which is its last step in a report.
    pub index: u32,
    /// Where on the parent it hangs, and where around the parent's loop: the two numbers that place it.
    pub station: f32,
    pub slot: f32,
    /// Forward `x`, side `y`, face `z`, in the world; `origin` is the anchor point on the parent's skin.
    pub placement: Placement,
    /// One uniform number, applied **in the template's own frame**: the world point is
    /// `placement.place(point * scale)`, never `placement.place(point) * scale`.
    ///
    /// # Invariants
    ///
    /// The two orders differ by `(scale − 1) · origin`, which is up to two metres at the top of a plant —
    /// the second one drags a leaf toward the **root** in proportion to how small it is drawn, so a small
    /// leaf would not sit on its twig at all. Scaling in the template's frame keeps the blade's own size
    /// the only thing the number changes, and it is what leaves the anchor at the anchor for every scale.
    pub scale: f32,
}

/// One node's leaves, all of one template: the group a report numbers, and one instanced mesh.
///
/// # Invariants
///
/// The leaves are in rank order, and `index` counts them, so a row can name a leaf as
/// *twig 3, leaf 2* without a separate table. One group is one instanced mesh: the template is the
/// same mesh for every group that names it.
#[derive(Debug, Clone, PartialEq)]
pub struct LeafGroup {
    /// The node the leaves hang from.
    pub node: Handle,
    /// Which template the group instances.
    pub template: usize,
    /// The leaves, in rank order.
    pub leaves: Vec<LeafInstance>,
}

/// The golden angle as a fraction of a turn: the default `phyllotaxy`.
pub const GOLDEN_ANGLE: f32 = 0.381_966_01;

/// How many uniform steps the midpoint rule takes over a field's band.
const DENSITY_STEPS: usize = 64;

/// Where a field puts its leaves.
///
/// # Invariants
///
/// **Every leaf of a field is one row in these numbers**, so the field is the edit surface: the
/// band, the density law, the phyllotaxy, and the four laws that orient and size. `generation` is a
/// [`Handle::depth`] — the generation the leaves hang *from* — so a field named on the twigs costs one
/// edit and reaches every twig at once.
#[derive(Debug, Clone, PartialEq)]
pub struct LeafField {
    /// The generation the leaves hang from: `Handle::depth` of the parent node.
    pub generation: usize,
    /// The band of the parent's arc length the leaves are spread over.
    pub band: (f32, f32),
    /// Leaves per unit of the parent's length, along the band.
    pub density: Scalar,
    /// How far around the parent's loop each leaf advances from the last, as a fraction of the loop.
    pub phyllotaxy: f32,
    /// Radians from the parent's outward direction toward the parent's tip.
    pub pitch: Scalar,
    /// Radians about the leaf's own forward.
    pub twist: Scalar,
    /// The leaf's scale in the world.
    pub size: Scalar,
    /// The fraction of `size` the node's own numbers may add or remove, `0` for none.
    pub jitter: f32,
    /// Which template the group instances.
    pub template: usize,
}

impl LeafField {
    /// A field on `generation` of template `template`, with the defaults: the parent's whole length,
    /// eight leaves per unit, a quarter-turn of pitch, no twist, no jitter, and a leaf of a tenth.
    pub fn new(generation: usize, template: usize) -> Self {
        Self {
            generation,
            band: (0.0, 1.0),
            density: Scalar::constant(8.0),
            phyllotaxy: GOLDEN_ANGLE,
            pitch: Scalar::constant(std::f32::consts::FRAC_PI_4),
            twist: Scalar::constant(0.0),
            size: Scalar::constant(0.1),
            jitter: 0.0,
            template,
        }
    }

    /// One [`LeafGroup`] per node of this generation that carries a leaf, in [`Plant::walk`] order.
    ///
    /// # Invariants
    ///
    /// **A node with no leaf of its own is not a group**, so a report's row is a node that has leaves
    /// and the walk order is the order the tree was built. A node whose handle does not place is
    /// **skipped rather than panicked on**: a stale handle is [`crate::plant::HandleOrphaned`]'s fact
    /// to report, and this walk is not where it is reported.
    ///
    /// **The three draws per node are in one order**: the phase first, before any jitter, then one
    /// jitter number per leaf in rank order. The phase is drawn first so two nodes of one field do not
    /// align their spirals; the jitter number is drawn whether `jitter` is zero or not, so editing the
    /// jitter leaves every slot where it was and every other node's stream untouched.
    pub fn groups(&self, plant: &Plant) -> Vec<LeafGroup> {
        let mut groups = Vec::new();
        for (handle, node) in plant.walk() {
            if handle.depth() != self.generation {
                continue;
            }
            let Ok(node_placement) = plant.placement(&handle) else {
                continue;
            };
            let spread = Density::of(node.spine().length(), &self.density, self.band);
            if spread.count == 0 {
                continue;
            }
            let mut rng = plant.rng(&handle);
            let phase = rng.unit();
            let mut leaves = Vec::with_capacity(spread.count);
            for rank in 0..spread.count {
                let station = spread.station(rank);
                let slot = (phase + rank as f32 * self.phyllotaxy).rem_euclid(1.0);
                // A negative scale is not a leaf: it would stand the template's winding inside out, and
                // the template's volume is the check that would see it.
                let scale = (self.size.at(station) * (1.0 + self.jitter * rng.signed())).max(0.0);
                // A handle from the walk resolves, so this arm is the staleness case the API allows
                // for; the group ends there rather than carrying a leaf with no anchor.
                let Ok(surface) = plant.surface(&handle, Attachment::new(station, slot)) else {
                    break;
                };
                let anchor = node_placement.then(&surface);
                leaves.push(LeafInstance {
                    index: rank as u32,
                    station,
                    slot,
                    placement: frame(&anchor, self.pitch.at(station), self.twist.at(station)),
                    scale,
                });
            }
            groups.push(LeafGroup {
                node: handle,
                template: self.template,
                leaves,
            });
        }
        groups
    }
}

/// The instance's own frame at `anchor`: forward `x`, side `y`, face `z`, at the anchor's origin.
///
/// # Invariants
///
/// **`pitch` turns `x` toward `y` and `twist` rolls `(side, face)` about `x`**, and both are rotations
/// of a frame that is already right-handed, so the result is too: `forward × side` is the anchor's
/// `z` after the pitch, and the twisted `face` after the twist. Neither stage re-derives the other's
/// answer, and neither can scale — a frame that scaled would make the template's own size a per-leaf
/// number.
fn frame(anchor: &Placement, pitch: f32, twist: f32) -> Placement {
    let (pitch_sin, pitch_cos) = pitch.sin_cos();
    let forward = add(scale(anchor.x, pitch_cos), scale(anchor.y, pitch_sin));
    let side = add(scale(anchor.x, -pitch_sin), scale(anchor.y, pitch_cos));
    let (twist_sin, twist_cos) = twist.sin_cos();
    Placement {
        origin: anchor.origin,
        x: forward,
        y: add(scale(side, twist_cos), scale(anchor.z, twist_sin)),
        z: add(scale(anchor.z, twist_cos), scale(side, -twist_sin)),
    }
}

/// One node's leaves counted and placed: a density law's own table over a field's band.
///
/// # Invariants
///
/// **One table answers both numbers, from the same `DENSITY_STEPS` midpoint samples**: the count is
/// the rounded integral over the band, and the leaf at rank `k` stands where the *cumulative* density
/// reaches `(k + 0.5) / count` of the total, inverted linearly inside the step it lands in. So a
/// density law that rises toward the tip really does put more leaves there, an even law gives evenly
/// spaced stations, and the two cannot disagree about how many leaves there are.
///
/// The samples are accumulated in `f64` because the inversion subtracts two cumulative values that are
/// close whenever the step it lands in is small — the same reason `shape`'s ellipse table is.
struct Density {
    /// How many leaves the band carries, which may be zero.
    count: usize,
    /// The band's `t` at its first step.
    start: f64,
    /// The width of one step of the band's table.
    width: f64,
    /// Cumulative density at each step's start, plus the total at the end: `DENSITY_STEPS + 1`.
    cumulative: Vec<f64>,
}

impl Density {
    /// The count a node of length `length` carries, and the table its stations come from.
    ///
    /// The band's two ends are clamped into `[0, 1]` and **ordered**, so a band authored outside the
    /// parent's length is the band it names and an inverted one is empty rather than negative.
    fn of(length: f32, density: &Scalar, band: (f32, f32)) -> Self {
        let low = clamp(band.0, 0.0, 1.0);
        let high = clamp(band.1, 0.0, 1.0);
        let (from, to) = if low <= high {
            (low, high)
        } else {
            (high, low)
        };
        let width = (to - from) / DENSITY_STEPS as f32;
        let mut cumulative = Vec::with_capacity(DENSITY_STEPS + 1);
        cumulative.push(0.0);
        let mut total = 0.0f64;
        for step in 0..DENSITY_STEPS {
            let midpoint = from + (step as f32 + 0.5) * width;
            total += f64::from(density.at(midpoint)) * f64::from(width);
            cumulative.push(total);
        }
        Self {
            count: (f64::from(length) * total).round().max(0.0) as usize,
            start: f64::from(from),
            width: f64::from(width),
            cumulative,
        }
    }

    /// The station the leaf at `rank` stands at.
    ///
    /// Its caller asks only when `count` is positive, which is what makes the division by the count —
    /// and by the total it reaches — sound: a count above zero is a total above zero.
    fn station(&self, rank: usize) -> f32 {
        let total = self.cumulative[DENSITY_STEPS];
        let target = (rank as f64 + 0.5) / self.count as f64 * total;
        let index = self
            .cumulative
            .partition_point(|&value| value < target)
            .saturating_sub(1)
            .min(DENSITY_STEPS - 1);
        let span = self.cumulative[index + 1] - self.cumulative[index];
        let within = if span > 0.0 {
            (target - self.cumulative[index]) / span
        } else {
            0.0
        };
        (self.start + (index as f64 + within) * self.width) as f32
    }
}

fn add(left: Vec3, right: Vec3) -> Vec3 {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn scale(vector: Vec3, factor: f32) -> Vec3 {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}
