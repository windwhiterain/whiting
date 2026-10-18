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
//! loop, a cluster that puts several leaves at one station and a fan that spreads them, and the pitch,
//! twist, size and jitter laws that orient and scale them. Every leaf of the field is one row in that
//! arithmetic, so *"fewer, and smaller, above six metres"* is a curve edit rather than four thousand
//! edits, and a node's numbers come from its own [`crate::rng`] stream so inserting a bough elsewhere
//! leaves this crown where it was.

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
///
/// # A template may be a sprig
///
/// **One template is one instanced mesh, and a mesh may hold several blades.** A shoot of oak carries
/// nine leaves in a fixed arrangement, and instancing them as nine placements of one blade costs nine
/// rows where one row and a nine-blade template would do; at the instance counts a crown wants, that
/// ratio is the difference between a plate that renders and one that does not. So a template is a list
/// of parts, each a blade in its own frame inside the template's, and [`LeafTemplate::sprig`] joins
/// them. A single blade is a sprig of one part at the identity, which is every template this crate had
/// before the sprig existed.
#[derive(Debug, Clone, PartialEq)]
pub struct LeafTemplate {
    parts: Vec<Part>,
}

/// One blade of a template, and the frame it stands in within the template's own coordinates.
#[derive(Debug, Clone, PartialEq)]
struct Part {
    sweep: Sweep,
    stations: usize,
    around: usize,
    at: Placement,
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
            parts: vec![Part {
                sweep,
                stations,
                around,
                at: Placement::IDENTITY,
            }],
        }
    }

    /// Several blades in one template: a shoot, a fascicle, a pinna's leaflets — whatever a crown
    /// instances as one unit. Each part keeps its own frame inside the template's, so a sprig is
    /// authored as blades placed where they grow and instanced as one mesh.
    ///
    /// # Invariants
    ///
    /// **The parts' frames compose the way placements do**: a part already carrying a frame inside a
    /// template that is itself a part of a larger sprig keeps both, because `at` is `outer.then(inner)`.
    /// An empty list is refused by construction of every caller — a template with no blade has no mesh,
    /// and [`LeafTemplate::loft`] would have nothing to join.
    pub fn sprig(parts: Vec<(LeafTemplate, Placement)>) -> Self {
        Self {
            parts: parts
                .into_iter()
                .flat_map(|(blade, at)| {
                    blade
                        .parts
                        .into_iter()
                        .map(|part| Part {
                            at: at.then(&part.at),
                            ..part
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
        }
    }

    /// How many blades the template holds.
    pub fn blades(&self) -> usize {
        self.parts.len()
    }

    /// The template in its own coordinates: base at the origin, forward `+x`, face `+z`.
    ///
    /// A width law that reaches zero collapses that ring to a point and the cap stays, so a blade
    /// pointed at both ends is a closed solid without a special case in the walk. A sprig is the join
    /// of its blades' solids, which [`shape::Loft::joined`] states is not a grid: the walk that packs
    /// it reads positions and triangles and never asks for a ring.
    pub fn loft(&self) -> Loft {
        let mut joined: Option<Loft> = None;
        for part in &self.parts {
            let at = &part.at;
            let loft = part
                .sweep
                .sample(part.stations, part.around)
                .moved(at.origin, at.x, at.y, at.z);
            joined = Some(match joined {
                Some(held) => held.joined(loft),
                None => loft,
            });
        }
        joined.expect("a template holds at least one blade")
    }

    /// The first blade's own spine, in the template's coordinates.
    pub fn midrib(&self) -> &Curve {
        &self.parts[0].sweep.spine
    }

    /// How many rings the first blade's loft draws.
    pub fn stations(&self) -> usize {
        self.parts[0].stations
    }

    /// How many vertices each ring of the first blade holds.
    pub fn around(&self) -> usize {
        self.parts[0].around
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
    /// How many leaves share one station, `1` for one leaf to a station.
    ///
    /// A cluster is the fascicle a pine's needles come in, the leaflets a kapok's palmate leaf spreads
    /// from one point, the two bracts under a dove tree's flower head, and the florets of a rhododendron's
    /// truss: **`n` leaves at one station, `1/n` of a turn apart**. It is a count on this field and not a
    /// node of its own, because a sheaf is not wood and a plant whose every fascicle were a node would
    /// spend its whole node budget on placements.
    pub cluster: u32,
    /// Radians of extra pitch the members of one cluster fan through, centred on the pitch the field
    /// already gives them, `0` for a cluster whose leaves all leave the same way.
    ///
    /// A fan of nought is the identity whatever the cluster is, and a cluster of one leaves every member
    /// at the centre of the fan — so the two dials are independent and neither can move a leaf alone.
    pub fan: Scalar,
    /// How a cluster's members differ in size: a law over the member's rank as a fraction of the
    /// cluster, multiplying [`LeafField::size`], constant one for members of one size.
    ///
    /// A dove tree's bract pair is one bract of sixteen centimetres and one of eight, and a cluster with
    /// one size law can only draw the pair at the larger one; this is the law that says the second
    /// member is half the first. It is evaluated at `member / (cluster - 1)`, so a cluster of one reads
    /// it at nought and a constant law of one leaves every leaf exactly where the size law put it.
    pub cluster_size: Scalar,
    /// Radians from the parent's outward direction toward the parent's tip.
    pub pitch: Scalar,
    /// Radians about the leaf's own forward.
    pub twist: Scalar,
    /// The world axis the leaf's midrib is turned toward, when [`LeafField::aim`] weighs it.
    pub aim_axis: Vec3,
    /// How much of the way the midrib is turned toward [`LeafField::aim_axis`], nought to one.
    ///
    /// It is the last term of the design's composition — *"`aim` (a world axis and a `Scalar` weight)"* — and it is
    /// the law grass wants: a blade aimed at the sky stands up whatever the rule that placed it, and a crown's
    /// leaves aimed at it shade as a volume rather than as a bag of leaves.
    pub aim: Scalar,
    /// The leaf's scale in the world.
    pub size: Scalar,
    /// The fraction of `size` the node's own numbers may add or remove, `0` for none.
    pub jitter: f32,
    /// Which template the group instances.
    pub template: usize,
    /// How many generations **up** the frame is read from, `0` meaning the parent.
    ///
    /// It is SpeedTree's five anchor levels as one number: `0` is its *Local*, `1` its *Intermediate*, and a
    /// plant's own depth its *Global* — one degree of freedom with one home rather than five cases.
    pub reference: u32,
}

impl LeafField {
    /// A field on `generation` of template `template`, with the defaults: the parent's whole length,
    /// eight leaves per unit, a quarter-turn of pitch, one leaf to a station and no fan, no twist, no
    /// jitter, and a leaf of a tenth.
    pub fn new(generation: usize, template: usize) -> Self {
        Self {
            generation,
            band: (0.0, 1.0),
            density: Scalar::constant(8.0),
            phyllotaxy: GOLDEN_ANGLE,
            cluster: 1,
            fan: Scalar::constant(0.0),
            cluster_size: Scalar::constant(1.0),
            pitch: Scalar::constant(std::f32::consts::FRAC_PI_4),
            twist: Scalar::constant(0.0),
            aim_axis: [0.0, 1.0, 0.0],
            aim: Scalar::constant(0.0),
            size: Scalar::constant(0.1),
            jitter: 0.0,
            template,
            reference: 0,
        }
    }

    /// The frame this field's leaves are read in: the ancestor `reference` generations up, or `None` for the
    /// parent's own surface frame.
    ///
    /// # Invariants
    ///
    /// - **`reference = 0` is the identity**, returning `None` so the caller composes exactly what it composed
    ///   before the field existed: a default field cannot move a leaf.
    /// - **A reference deeper than the ancestors a leaf has reads the root.** A field is one law over a whole
    ///   generation and a plant's nodes stand at a range of depths, so the number cannot be a per-leaf report;
    ///   the clamp is the only rule that keeps one generation-wide number one law.
    fn reference_frame(&self, plant: &Plant, handle: &Handle) -> Option<Placement> {
        if self.reference == 0 {
            return None;
        }
        let mut at = handle.clone();
        for _ in 0..self.reference {
            match at.parent() {
                Some(parent) => at = parent,
                None => break,
            }
        }
        plant.placement(&at).ok()
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
            // **The frame the leaves are read in is an ancestor's**, `reference` generations up; the point they
            // stand on is always the parent's own surface, so a reference turns a leaf's orientation and never
            // moves it off the bark.
            let reference = self.reference_frame(plant, &handle);
            let spread = Density::of(node.spine().length(), &self.density, self.band);
            if spread.count == 0 {
                continue;
            }
            // **A cluster is a whole number of leaves, so the last station may stand short of one.** The
            // density law counts leaves and every leaf it counted is drawn; how many stations they stand
            // on follows from the cluster. At a cluster of one the stations are the leaves and the
            // member loop runs once, which is exactly the walk this was before the cluster existed.
            let cluster = self.cluster.max(1) as usize;
            let stations = spread.count.div_ceil(cluster);
            let mut rng = plant.rng(&handle);
            let phase = rng.unit();
            let mut leaves = Vec::with_capacity(spread.count);
            'station: for place in 0..stations {
                let members = (spread.count - place * cluster).min(cluster);
                // The station of the cluster's own middle leaf, so a cluster stands where the leaves it
                // replaces stood rather than where its first one would have.
                let station = spread.station(place * cluster + (members - 1) / 2);
                for member in 0..members {
                    let rank = place * cluster + member;
                    // The cluster's members stand `1 / cluster` of a turn apart, so a fascicle spreads
                    // around the parent's loop and a cluster of one advances by the phyllotaxy alone.
                    let slot =
                        (phase + place as f32 * self.phyllotaxy + member as f32 / cluster as f32)
                            .rem_euclid(1.0);
                    // A negative scale is not a leaf: it would stand the template's winding inside out, and
                    // the template's volume is the check that would see it. The cluster's own size law is
                    // read at the member's rank and draws nothing, so a field whose cluster_size is a
                    // constant one is the field it was before the law existed.
                    let rank_fraction = if cluster > 1 {
                        member as f32 / (cluster - 1) as f32
                    } else {
                        0.0
                    };
                    let scale = (self.size.at(station)
                        * self.cluster_size.at(rank_fraction)
                        * (1.0 + self.jitter * rng.signed()))
                    .max(0.0);
                    // A handle from the walk resolves, so this arm is the staleness case the API allows
                    // for; the group ends there rather than carrying a leaf with no anchor.
                    let Ok(surface) = plant.surface(&handle, Attachment::new(station, slot)) else {
                        break 'station;
                    };
                    let anchor = match &reference {
                        Some(up) => Placement {
                            origin: node_placement.then(&surface).origin,
                            x: up.x,
                            y: up.y,
                            z: up.z,
                        },
                        None => node_placement.then(&surface),
                    };
                    leaves.push(LeafInstance {
                        index: rank as u32,
                        station,
                        slot,
                        placement: aim(
                            frame(
                                &anchor,
                                self.pitch.at(station) + fan(member, members, &self.fan, station),
                                self.twist.at(station),
                            ),
                            self.aim_axis,
                            self.aim.at(station),
                        ),
                        scale,
                    });
                }
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

/// Turns a leaf's midrib toward a world axis: the last term of the design's composition.
///
/// # Invariants
///
/// - **A weight of nought is the identity and a weight of one points the midrib at the axis**: the turn is a rotation
///   about the axis the two directions span, carried through the leaf's side and face, so the frame stays
///   right-handed and the blade is never mirrored.
/// - **A midrib already at the axis is the identity at every weight**, which is what lets a grass field aim every
///   blade at the sky without moving the ones the rule already stood up.
/// - **A direction of no length is no axis**: a zero axis, or one the midrib already lies along, leaves the frame
///   alone rather than turning it somewhere arbitrary.
fn aim(frame: Placement, axis: Vec3, weight: f32) -> Placement {
    let far = length3(axis);
    if weight == 0.0 || far < 1.0e-9 {
        return frame;
    }
    let toward = scale(axis, 1.0 / far);
    let along = dot3(frame.x, toward).clamp(-1.0, 1.0);
    let turn = along.acos() * weight;
    let across = cross(frame.x, toward);
    let reach = length3(across);
    if turn < 1.0e-9 || reach < 1.0e-9 {
        return frame;
    }
    let axis = scale(across, 1.0 / reach);
    let (sin, cos) = turn.sin_cos();
    let swing = |vector: Vec3| -> Vec3 {
        let along = dot3(axis, vector);
        add(
            add(scale(vector, cos), scale(cross(axis, vector), sin)),
            scale(axis, along * (1.0 - cos)),
        )
    };
    Placement {
        origin: frame.origin,
        x: swing(frame.x),
        y: swing(frame.y),
        z: swing(frame.z),
    }
}
/// The extra pitch the member `index` of `count` gets from a field's fan law, at the station the cluster
/// stands on.
///
/// # Invariants
///
/// **Centred, so a fan turns a cluster and does not move it**: the members spread evenly across the law's
/// own value at the station, the first by minus half of it and the last by plus half of it. One member
/// stands at the centre whatever the law says, which is what makes a fan of any size the identity on a
/// field whose cluster is one, and a law of nought the identity on any cluster.
fn fan(index: usize, count: usize, law: &Scalar, station: f32) -> f32 {
    if count <= 1 {
        return 0.0;
    }
    (index as f32 / (count - 1) as f32 - 0.5) * law.at(station)
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

fn length3(vector: Vec3) -> f32 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}

fn dot3(left: Vec3, right: Vec3) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: Vec3, right: Vec3) -> Vec3 {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn add(left: Vec3, right: Vec3) -> Vec3 {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn scale(vector: Vec3, factor: f32) -> Vec3 {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}
