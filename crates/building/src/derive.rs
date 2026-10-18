//! Derivation: a starting point that carries no guarantee and is immediately editable.
//!
//! # Contract of this module
//!
//! - **A derived element is an ordinary element.** Nothing here stores a rule: a starter returns a
//!   [`Building`] whose masses and roofs are the same values an author would have written by hand,
//!   and editing one is editing the document (`mountain`'s R3, kept).
//! - **Randomness is a visible alternative, never jitter.** A choice is one of a small published
//!   set — [`RoofStyle`], a ridge proportion — picked from a seeded stream, so a derived default
//!   looks like a default and an authored choice looks authored. What is left open about a
//!   rectangle is exactly one thing, and it is published as three values rather than a range: the
//!   hip's ridge inset is [`RIDGE_INSETS`] of the eave's short span, so a reader can see which of
//!   the three a building used without asking. There is no per-node jitter anywhere: a number that
//!   moved on every run would be a number nobody could edit.
//! - **The seed forks per node along the ancestry path.** A choice made for a wing is a function of
//!   that wing's own key, not of how many choices were made before it, so editing one wing re-rolls
//!   that wing alone and the same seed always derives the same building. The rule is [`fork`].
//! - **Derivation reports rather than guesses** when the structure it is handed is outside the
//!   domain it supports, the way `mountain::derive_valleys` returns a count of what it could not
//!   place instead of inventing lines.
//!
//! # The one thing a derivation has to decide, and the winding it hands over
//!
//! A roof is a **strip of rows**, a row being a crest point paired with an eave station. Two
//! neighbouring planes are closed against each other because they name **the same row** — the same
//! crest index and the same station — rather than because two constructions met by luck, and this
//! module keeps that true by building every shared row **once** and putting it in both planes.
//!
//! The winding of a face is this: **the quad between two consecutive rows is wound
//! `[crest_i, crest_{i+1}, eave_{i+1}, eave_i]`**, which puts its right-handed normal out of the
//! solid. `roof.rs` fixes what a row means and does not fix this order, so it is written down here
//! rather than left to the expander to guess, and the probe measures the normals it produces.
//!
//! # Why the planes come out planar, and why that is a naming rule
//!
//! A quad is planar exactly when its two runs are parallel, so **the stations a plane names decide
//! it**: a long-side plane whose eave run is corner-to-corner along one edge of the loop and whose
//! crest run is the segment between two crest points on the long axis spans two parallel edges, and
//! an end plane names one crest point twice against the corners of one edge, which is a triangle.
//! Both are readings of *which nodes a plane names*, not arithmetic afterwards — which is why this
//! module spends its care on the nodes and spends no tolerance anywhere. Measured by
//! `examples/roof_planarity_probe.rs` on the expanded mesh: every plane of every starter departs its
//! own fitted plane by under 3e-7 m, and each skin's plan area equals its eave loop's to the last
//! digit, which is the other half of the same claim — the planes tile the loop.

use crate::mass::{Mass, MassId, PlanNode};
use crate::math::{Vec2, cross, difference, distance, dot, length};
use crate::ops::closed_spans;
use crate::roof::{CrestPoint, EaveStation, Roof, RoofPlane, RoofRow};
use crate::skeleton::{Building, BuildingSkeleton};
use crate::slab::Slab;

/// Which roof a starter builds.
///
/// # Invariants
///
/// There is deliberately no `Flat` here yet, and the reason is a representation fact rather than a
/// shortage of effort: a plane in this crate is a **strip of transverse rows**, each row a crest
/// point paired with an eave station. A flat slab has no transverse extent — its skin *is* the eave
/// polygon — so it needs a second representation (a plane bounded by a loop) rather than a
/// degenerate row set that expands to nothing. Recorded as an open question for P2 instead of faked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoofStyle {
    /// Four planes meeting at a ridge, with hips running to the corners.
    Hip,
    /// Two planes meeting at a ridge, with gable ends.
    Gable,
    /// A plate over the eave outline: the third primitive, not a zero-rise roof.
    Flat,
}

/// What a starter building is made of.
#[derive(Debug, Clone, PartialEq)]
pub struct Starter {
    /// Plan width, east to west.
    pub width: f32,
    /// Plan depth, north to south.
    pub depth: f32,
    /// Wall top elevation above the ground.
    pub height: f32,
    /// Wall thickness.
    pub wall: f32,
    /// Floor slab thickness.
    pub slab: f32,
    /// How far the eaves stand outside the walls.
    pub overhang: f32,
    /// How far the eave sits below the wall top, so the roof is seated rather than balanced on it.
    pub sink: f32,
    /// How far the crest rises above the eave.
    pub rise: f32,
    /// Roof slab thickness.
    pub roof_thickness: f32,
    /// Which roof to build.
    pub roof: RoofStyle,
    /// The seed every derived choice is forked from.
    pub seed: u64,
}

impl Default for Starter {
    fn default() -> Self {
        Self {
            width: 6.0,
            depth: 9.0,
            height: 3.2,
            wall: 0.3,
            slab: 0.4,
            overhang: 0.5,
            sink: 0.1,
            rise: 1.6,
            roof_thickness: 0.25,
            roof: RoofStyle::Hip,
            seed: 1,
        }
    }
}

/// The published ridge insets of a hip, as a proportion of the eave's short span.
///
/// # Invariants
///
/// **Three values, not a range.** A derivation that chose a float in `[0.4, 0.6]` would produce a
/// roof whose ridge nobody could name afterwards, and the module's own rule is that a derived
/// element is an ordinary one: an author reading the crest back has to be able to say which of the
/// published alternatives it is. The middle value is the even hip — both hips at 45° — which is why
/// [`hip_roof`] is the middle one and not a choice at all.
///
/// Published, because a reader of a derived building has to be able to read the choice back off the
/// document, and [`starter`] draws from this list rather than from a range of its own.
pub const RIDGE_INSETS: [f32; 3] = [0.4, 0.5, 0.6];

/// Builds a house: one mass, and the roof its starter asks for.
///
/// # Invariants
///
/// - **The outline is a rectangle centred on the origin, wound so its signed plan area is
///   positive** — the solid on the walker's right, which is the condition under which `up × along`
///   points away from it (see [`Mass::plan_area`], which states the rule and the frame it was
///   measured in). A reversed outline is not a visible fault but a missing building, every wall's
///   outer face looking inward, so the winding is part of what a starter decides rather than
///   something left to the checker to report.
/// - **The one seeded choice is the hip's ridge inset**, drawn from [`RIDGE_INSETS`] through a
///   stream forked at [`NodeKey::ROOT`]. A gable starter draws nothing: the stream belongs to the
///   node that has a choice, so a building with nothing to choose does not advance one.
/// - **A shape that cannot carry the drawn inset does not get it.** The draw is one step of the same
///   stream either way, and the value used is the first of the published list at or after the drawn
///   one that the eave admits ([`Eave::admits`]) — so the document still names one of the three,
///   and a square, which admits only the first two, never stands its hip feet on the wrong corners.
/// - **The plate is the mass's inner loop, its top is the ground, and every roof style carries it.** A
///   style decides what stands on the wall top; nothing it can be takes the floor away, which is the
///   reading that once did not hold for [`RoofStyle::Flat`]. The floor is seated on the
///   same loop the walls are built on — the way [`floor_slab`] seats a first floor — rather than
///   under the whole footprint. A footprint-wide plate puts its four edges in exactly the wall ring's
///   outer-face planes, so the floor is a depth tie with the wall it carries; the inner loop leaves the
///   wall ring's base annulus standing outside the plate, which is the ring's own floor and not the
///   plate's. The cost of the seating is that the plate is **buried**: on level ground nothing outside
///   the building can see it, and a frame that has to show a floor shows it through an opening.
/// - Everything it returns is written out in full. The caller gets masses and roofs, not a recipe.
pub fn starter(starter: &Starter) -> Building {
    let mass = Mass::new(rectangle(starter.width, starter.depth), starter.height)
        .with_wall(starter.wall)
        .with_slab(starter.slab);
    // The body is the first mass, which is the whole of what makes `MassId(0)` the honest host for
    // the roof below and for the plate.
    let host = MassId(0);
    // **The plate the mass stands on, seated on the mass's own inner loop** — `derive::floor_slab`'s
    // seating, at the ground rather than above it, and for the same reason it is right there. A plate
    // spanning the whole footprint puts its four edges in exactly the planes of the wall ring's outer
    // faces, so the floor is a depth tie with the wall it carries and takes only the texels the tie
    // happens to give it. Inset by the wall, the plate's own sides stand clear of every wall face, the
    // wall ring's base annulus stands outside the plate as the ring's own floor, and the plate is the
    // room's inside.
    let plate = {
        let lowest = mass.lowest();
        // A wall the miter cannot offset answers `None`, and a mass whose plate cannot be seated is a
        // building with no floor rather than a plate standing somewhere else.
        inner_loop(&mass, host)
            .map(|inside| Slab::plate(inside, lowest - starter.slab, lowest).with_host(host))
    };
    let Some(eave) = Eave::read(&mass, starter.overhang, starter.sink) else {
        // Unreachable for the rectangle this function just built: four nodes is a quadrilateral.
        // Returning the mass on its own is the honest answer if that ever stops being true — but it
        // is an answer **nothing else checks**: `validate` has no violation for a mass that carries
        // no roof (its list is `OutlineReversed`, `OutlineTooShort`, `ZeroHeight`,
        // `UnattachedOverlap`, `MassHostMissing`, `RoofHostMissing`, `RoofEaveTooShort`,
        // `RoofPlaneTooShort`, `RoofCrestMissing`, and the four solid faults with
        // `FaceNotPlanar`), so a mass without a roof would be silent rather than reported.
        return with_top(vec![mass], None, plate);
    };
    let roof = match starter.roof {
        RoofStyle::Hip => {
            let mut stream = Stream::at(ROOT_KEY, starter.seed);
            let drawn = stream.pick(RIDGE_INSETS.len());
            let count = RIDGE_INSETS.len();
            let inset = (0..count)
                .map(|step| RIDGE_INSETS[(drawn + step) % count])
                .find(|inset| eave.admits(*inset))
                // Never empty, and not a guess: the middle inset is half the short span, and the
                // short span is at most the long run by the way `Eave::read` picks its axis.
                .expect("the middle published inset is admissible on every eave this reads");
            Top::Roof(hip_over(
                &eave,
                host,
                starter.rise,
                starter.roof_thickness,
                inset,
            ))
        }
        RoofStyle::Gable => Top::Roof(gable_over(
            &eave,
            host,
            starter.rise,
            starter.roof_thickness,
        )),
        // **A flat roof is a plate, not a roof**, so this arm builds a slab and the document holds no
        // roof at all: the roof model's skin is a strip between a crest row and an eave row, and zero
        // rise would make those rows coincide and every facet of the strip have no area.
        //
        // **What the style decides is what stands on the wall top, never what stands under the wall.**
        // The plate the mass's inner loop carries was derived above and it is carried here like every
        // other style's, so a flat-roofed starter is a room with a floor: a mass is a wall ring plus a
        // plate ([`SlabRole`](crate::slab::SlabRole)), and an arm that returned early on its roof would
        // have left the ring standing on nothing.
        RoofStyle::Flat => Top::Plate(flat_over(&eave, host, starter.roof_thickness)),
    };
    with_top(vec![mass], Some(roof), plate)
}

/// What a starter's roof style leaves on the wall top: a pitched [`Roof`], or the plate a flat roof is.
///
/// # Invariants
///
/// **The one arm that builds the wrong kind of element is named rather than spelt out twice.** A hip and
/// a gable are skins, so both land in the document's `roofs`; a flat roof is a [`Slab`], so it lands in
/// its `slabs` beside the mass's own plate — which is why this is a two-kinded value at all, and why
/// [`with_top`] is the single place that knows where each kind goes. One arm building the document itself
/// is what let a flat starter drop the plate derived above it: the arm that returned a whole
/// [`Building`] had to remember every other thing the function puts in one, and it remembered the roof.
#[derive(Debug)]
enum Top {
    /// A pitched roof, over the eave loop.
    Roof(Roof),
    /// The plate a flat roof is, over the same eave loop.
    Plate(Slab),
}

/// Returns a starter's building: the mass, whatever its style left on the wall top, and the plate the
/// mass's inner loop could carry.
///
/// # Invariants
///
/// **A refused plate is a building with no floor rather than a plate standing somewhere else.** The
/// walk [`inner_loop`] refuses is the wall ring's own, so a plate placed anywhere but on it would be a
/// second opinion about where the inside of the building is — which is the one thing this module's
/// floors exist to say once.
///
/// **The plate is carried by every style, and it is the last slab in the list.** A flat starter holds a
/// roof plate and then the plate its mass stands on, so `SlabId(0)` is the roof and `SlabId(1)` the floor
/// whichever way a reader counts; that order is what `examples/slab_probe.rs` and
/// `examples/validate_probe.rs` read their roof out of.
fn with_top(masses: Vec<Mass>, top: Option<Top>, plate: Option<Slab>) -> Building {
    let mut skeleton = match top {
        Some(Top::Roof(roof)) => BuildingSkeleton::new(masses, vec![roof]),
        Some(Top::Plate(slab)) => {
            let mut skeleton = BuildingSkeleton::new(masses, Vec::new());
            skeleton.slabs.push(slab);
            skeleton
        }
        None => BuildingSkeleton::new(masses, Vec::new()),
    };
    skeleton.slabs.extend(plate);
    Building::new(skeleton)
}

/// Builds the plate a flat roof is: a slab over the eave outline, `thickness` thick, seated on the eave.
///
/// # Invariants
///
/// Its underside is the eave's own elevation, so the plate sits on the wall top with the same `sink` the
/// pitched roofs get, and its top is one thickness above it. It carries its `host`, which is what makes
/// seating move it with the building instead of leaving the roof behind at the old level.
fn flat_over(eave: &Eave, host: MassId, thickness: f32) -> Slab {
    let bottom = eave.elevation;
    let outline = eave
        .nodes
        .iter()
        .map(|node| node.position)
        .collect::<Vec<_>>();
    Slab::roof(outline, bottom, bottom + thickness).with_host(host)
}

/// Builds a hip roof over a mass's outline, standing `overhang` outside it.
///
/// # Invariants
///
/// - **The eave loop is the outline offset outward by `overhang`** — a miter, so each edge slides
///   along its own outward normal and neighbouring edges meet at their intersection — and every
///   eave node stands at `mass.top - sink`.
/// - **The ridge is inset from each end by half the short span**, which is what makes the hips run
///   at 45°: the run from a crest end to the corner it sheds to is then equal across the roof as it
///   is along it. [`starter`] is the only thing here that chooses a different inset, and it chooses
///   it from the published [`RIDGE_INSETS`].
/// - **Every plane is planar, and it is planar because of which corners it names, not because of
///   arithmetic.** A long-side plane's eave run is corner-to-corner along one edge of the loop and
///   its crest run is the segment between the two crest points, which lie on the long axis and on
///   the middle line between the long sides: two spans of two parallel edges, so the quad between
///   them is one plane. An end plane names one crest point twice against the two corners of an end
///   edge, and three points are one plane. The eave run is never a diagonal of the loop, which is
///   the one way these quads could twist — see [`long_plane`] and [`end_plane`].
/// - **A ridge with no length is one crest point.** A hip whose inset reaches half the long run —
///   which is every hip over a square — has both crest ends standing on the same spot, and two
///   points at one spot are two welded vertices and a quad between them with no area. The four
///   planes are then all one triangle against the same apex.
/// - **The planes that meet share the row they meet on**: the same crest index, the same eave
///   station, written twice from one value. No test compares coordinates anywhere in this file.
///
/// Returns `None` when the outline is not a quadrilateral with parallel long sides: see
/// [`read_eave`], which is where the domain is stated.
///
/// `host` is the mass the roof sits on, taken as an argument rather than assumed: a `Roof` names its
/// host, a mis-named host silently authorizes an intersection the author did not make, and a mass
/// handed over on its own cannot say which index it holds. A caller that does not know passes the id
/// it obtained when it added the mass — never a guess.
pub fn hip_roof(
    host: MassId,
    mass: &Mass,
    overhang: f32,
    rise: f32,
    sink: f32,
    thickness: f32,
) -> Option<Roof> {
    let eave = Eave::read(mass, overhang, sink)?;
    Some(hip_over(&eave, host, rise, thickness, 0.5))
}

/// Builds a gable roof over a mass's outline, standing `overhang` outside it.
///
/// # Invariants
///
/// - **The ridge is not inset**: its two ends stand at the ends of the eave loop's long axis, so
///   the roof is a pair of rectangles and the gable ends are triangles the expander's fascia band
///   closes rather than rows anybody authored.
/// - **Both planes are planar by the same naming rule as a hip's**: each names the two corners of one
///   long edge against the two crest points on the axis between them, and the crest ends stand at
///   the eave ends, so each plane's crest run and eave run are spans of two parallel edges.
/// - The two planes share the **crest edge** — both name crest 0 and crest 1 — and no row, which is
///   the gable's version of the hip's shared row. The probe measures both kinds.
///
/// Returns `None` for the same reason [`hip_roof`] does, and takes its host the same way.
pub fn gable_roof(
    host: MassId,
    mass: &Mass,
    overhang: f32,
    rise: f32,
    sink: f32,
    thickness: f32,
) -> Option<Roof> {
    let eave = Eave::read(mass, overhang, sink)?;
    Some(gable_over(&eave, host, rise, thickness))
}

/// The key a building's body derives from: the root of every ancestry.
///
/// # Invariants
///
/// A free-standing mass has no host, so its key is a constant rather than a hash of anything. It is
/// published because a caller composing wings needs to name the body to fork a wing from.
pub const ROOT_KEY: u64 = 0;

/// Returns the key one node's derived choices are read from, forked from its host.
///
/// # Invariants
///
/// **This is the fork rule, and it is the whole of the seed discipline.** A node's key is a
/// function of *where it hangs* and of nothing else:
///
/// - the same seed derives the same building, because the key of a node in a given place is the
///   same number on every run;
/// - editing one wing re-rolls that wing alone, because the wing's key is its host's key with the
///   station it hangs at mixed in — not "the third choice drawn", which would re-roll every wing
///   after an edit. That is the failure the rule exists to prevent, and it is why this is a
///   function of the ancestry rather than a counter.
///
/// The station goes in as **bits**, mixed with the host's key rather than added to it, so two wings
/// of the same host at different stations get unrelated keys and a host and its wing do not get
/// correlated ones.
pub fn fork(host: u64, station: f32) -> u64 {
    host ^ u64::from(station.to_bits()).rotate_left(17)
}

/// A SplitMix64 stream, forked from one node's key and the building's seed.
struct Stream {
    state: u64,
}

impl Stream {
    /// The golden gamma of SplitMix64, and the constants that mix it.
    const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
    const MIX_FIRST: u64 = 0xBF58_476D_1CE4_E5B9;
    const MIX_SECOND: u64 = 0x94D0_49BB_1331_11EB;

    /// Opens the stream one node's choices are read from.
    ///
    /// The state is a function of `(key, seed)` alone — not of the building, the shape, or how many
    /// choices have been drawn — so a node derives the same choice whether it is the first node of a
    /// building or the ninth.
    fn at(key: u64, seed: u64) -> Self {
        Stream {
            state: key ^ seed.wrapping_mul(Self::GAMMA),
        }
    }

    /// Returns the next value of the stream.
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(Self::GAMMA);
        let mut mixed = self.state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(Self::MIX_FIRST);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(Self::MIX_SECOND);
        mixed ^ (mixed >> 31)
    }

    /// Returns one of `count` alternatives, without the bias a remainder would carry.
    ///
    /// The multiply is by a `u128` and the shift takes the high half, so every alternative is
    /// equally likely however small `count` is — a derivation is not a place to be off by one in
    /// fifty.
    fn pick(&mut self, count: usize) -> usize {
        ((self.next() as u128 * count as u128) >> 64) as usize
    }
}

/// A quadrilateral's eave loop and the geometry a roof reads off it.
///
/// # Invariants
///
/// Everything here is read **once** and then shared by the planes, which is what makes two planes
/// that share a row share a value rather than a calculation: the stations are one number each,
/// computed once in [`Eave::read`].
struct Eave {
    /// The offset loop, standing at `mass.top - sink`.
    nodes: Vec<PlanNode>,
    /// The unit direction of the loop's long axis.
    axis: Vec2,
    /// The point on the axis where `along` zero sits.
    origin: Vec2,
    /// Where the two ends of the long axis stand, near first.
    ends: (f32, f32),
    /// The distance between the two long sides.
    span: f32,
    /// The distance between the two ends along the axis, which is at least `span`.
    run: f32,
    /// The index of the node the long side's planes are read from.
    long_side: usize,
    /// Each node's station on the closed loop.
    stations: [EaveStation; 4],
    /// The elevation every eave node stands at.
    elevation: f32,
}

impl Eave {
    /// Reads the eave loop of a mass and the geometry a roof needs from it, or `None` outside the
    /// domain.
    ///
    /// # Invariants
    ///
    /// **The domain is a quadrilateral with parallel long sides, and it is narrower than "four
    /// nodes" on purpose.** Two reasons, and both come from what a plane is rather than from taste:
    ///
    /// - A plane here is a quad of the form `[crest_i, crest_{i+1}, eave_{i+1}, eave_i]`, and
    ///   `roof.rs` calls that face **planar**. The quad is planar exactly when the crest line and
    ///   the eave edge are parallel, so the two long sides of the loop have to be parallel or the
    ///   derivation would hand the expander a "planar" face with four points that are not on one
    ///   plane — a document that says something untrue about itself, which is the one failure this
    ///   workspace pays for twice.
    /// - Which way is "long" is only a fact when the opposite sides are parallel. On a general
    ///   quadrilateral the answer is a measurement, and a derivation that measures where a shape
    ///   does not support measuring returns a guess.
    ///
    /// A parallelogram passes both, and so does every rectangle, which is the shape a starter and
    /// an author reach for first.
    fn read(mass: &Mass, overhang: f32, sink: f32) -> Option<Self> {
        let count = mass.outline.len();
        if count != 4 {
            return None;
        }
        let elevation = mass.top - sink;
        let nodes = offset_outward(&mass.outline, overhang, elevation);
        let position = |index: usize| nodes[index].position;

        // Which pair of opposite edges is longer: the pair that is, is the axis the roof runs along.
        let pair = |first: usize, second: usize| {
            distance(position(first), position((first + 1) % 4))
                + distance(position(second), position((second + 1) % 4))
        };
        let long_side = if pair(0, 2) >= pair(1, 3) { 0 } else { 1 };
        let near = (long_side + 3) % 4;
        let far = (long_side + 1) % 4;
        let across_side = (long_side + 2) % 4;

        let run = difference(position(far), position(long_side));
        let long = length(run);
        if long <= f32::EPSILON {
            return None;
        }
        // The diagonals of a parallelogram bisect one another, which is the whole of the test and
        // the cheapest one that says it; a bow-tie and a general quadrilateral both fail it.
        let slack = 1.0e-4 * (long + distance(position(far), position(across_side)));
        let first_diagonal = [
            position(0)[0] + position(2)[0],
            position(0)[1] + position(2)[1],
        ];
        let second_diagonal = [
            position(1)[0] + position(3)[0],
            position(1)[1] + position(3)[1],
        ];
        if distance(first_diagonal, second_diagonal) > slack {
            return None;
        }

        let axis = [run[0] / long, run[1] / long];
        let along = |node: usize| dot(position(node), axis);
        let across = |node: usize| cross(axis, position(node));
        // The ends are the two edges that are not the long ones, and they stand `long` apart measured
        // along the axis; the crest line runs midway between the long sides.
        let ends = [
            (along(near) + along(long_side)) * 0.5,
            (along(far) + along(across_side)) * 0.5,
        ];
        let (first, second) = if ends[0] <= ends[1] {
            (ends[0], ends[1])
        } else {
            (ends[1], ends[0])
        };

        let (spans, total) = closed_spans(&nodes);
        if total <= f32::EPSILON {
            return None;
        }
        let mut stations = [EaveStation::new(0.0); 4];
        for (index, station) in stations.iter_mut().enumerate() {
            *station = EaveStation::new(spans[index] / total);
        }

        let middle = (across(long_side) + across(across_side)) * 0.5;
        let span = (across(across_side) - across(long_side)).abs();
        // `across(x)` is `cross(axis, x)`, so the point at a distance `middle` along the axis's left
        // side is `[-axis[1], axis[0]] * middle`: the crest line stands there.
        let origin = [-axis[1] * middle, axis[0] * middle];

        Some(Eave {
            nodes,
            axis,
            origin,
            ends: (first, second),
            span,
            run: second - first,
            long_side,
            stations,
            elevation,
        })
    }

    /// Returns a node's station on the closed loop.
    fn station(&self, node: usize) -> EaveStation {
        self.stations[node % 4]
    }

    /// Returns whether a ridge inset, as a fraction of the short span, leaves the two crest ends in
    /// the order the two end walls are in.
    ///
    /// # Invariants
    ///
    /// **The bound is half the long run, and it is a bound on geometry rather than on taste.** The
    /// near crest end stands at `ends.0 + inset` and the far one at `ends.1 - inset`, so the two
    /// cross exactly when `2 · inset` passes `run`: past that the near foot would stand beyond the
    /// far end, each hip would run to the corner on the **other** side, and the two long-side planes
    /// would shed inside out while still looking like a hip in a section drawing. A hip is the roof
    /// whose feet are at its own ends, and this is the test of that.
    ///
    /// At the bound itself the ridge has no length, which is not a fault but a pyramid: see
    /// [`hip_over`].
    fn admits(&self, inset: f32) -> bool {
        inset * self.span <= 0.5 * self.run
    }

    /// Returns the point on the crest line `along` the long axis from the middle of the short span.
    fn crest_at(&self, along: f32) -> Vec2 {
        [
            self.origin[0] + self.axis[0] * along,
            self.origin[1] + self.axis[1] * along,
        ]
    }
}

/// Offsets a closed loop outward by a distance, a miter at every node.
///
/// # Invariants
///
/// Each **edge** slides along its own outward normal and the node between two edges sits where the
/// two slid edges meet, which is the offset a wall's outside corner actually is — a normal offset
/// would round the corner and a mitre offset by the node would leave a gap on the inside.
///
/// "Outward" is **`up × along`**, the same direction `gallery::built` gives a wall ring's outer
/// face, and it points away from the interior exactly when the loop's signed plan area is positive —
/// that is, when the loop puts the solid on the walker's **right** (see [`Mass::plan_area`], which
/// states the rule in the frame it was measured in). An outline wound the other way is offset
/// inward here rather than repaired, because an op does not fix what it was handed, and the probe
/// measures the difference: a reversed rectangle's eave comes out *smaller* than the outline it
/// came from.
fn offset_outward(nodes: &[PlanNode], amount: f32, elevation: f32) -> Vec<PlanNode> {
    let count = nodes.len();
    let normal = |from: usize, to: usize| -> Vec2 {
        let step = difference(nodes[to].position, nodes[from].position);
        let length = length(step);
        if length <= f32::EPSILON {
            return [0.0, 0.0];
        }
        [step[1] / length, -step[0] / length]
    };
    (0..count)
        .map(|index| {
            let before = normal((index + count - 1) % count, index);
            let after = normal(index, (index + 1) % count);
            let summed = [before[0] + after[0], before[1] + after[1]];
            let length = length(summed);
            let delta = if length <= f32::EPSILON {
                // A node where the loop doubles back: one edge's normal is the best answer there is.
                [after[0] * amount, after[1] * amount]
            } else {
                // The bisector, scaled so that it reaches `amount` along either edge's normal. The
                // two lengths cancel: the offset is the *sum* of the normals divided by its own dot
                // with either of them, which is the intersection of the two slid edge lines.
                let reach = amount / dot(summed, after).max(1.0e-3);
                [summed[0] * reach, summed[1] * reach]
            };
            PlanNode::new(
                [
                    nodes[index].position[0] + delta[0],
                    nodes[index].position[1] + delta[1],
                ],
                elevation,
            )
        })
        .collect()
}

/// Builds the four planes of a hip over a read eave loop.
///
/// # Invariants
///
/// - **The inset is held to what the loop admits.** [`Eave::admits`] is the rule; applying it here
///   as well as in [`starter`] is the one place the value is used, so a caller that asks for an
///   inset of its own gets a hip whose feet are at its own ends rather than a crossed one.
/// - **A ridge of no length is one crest point.** The two ends coincide when the inset is half the
///   long run, which every hip over a square reaches, and two crest points at one position are two
///   welded vertices: the quad between them is a facet with no area, and the expander's guard
///   compares *welded point indices*, so it cannot see that two crest indices are one place. Four
///   triangles against one apex say the same thing without the dead facet.
fn hip_over(eave: &Eave, host: MassId, rise: f32, thickness: f32, inset: f32) -> Roof {
    let inset = (inset * eave.span).min(0.5 * eave.run);
    let near = eave.crest_at(eave.ends.0 + inset);
    let far = eave.crest_at(eave.ends.1 - inset);
    let elevation = eave.elevation + rise;
    if length(difference(far, near)) <= f32::EPSILON {
        let crest = vec![CrestPoint::new(near, elevation)];
        // One apex and the loop's four edges in loop order: neighbours meet on a row written twice.
        let planes = (0..4)
            .map(|step| end_plane(eave, 0, (eave.long_side + step) % 4))
            .collect();
        return Roof::new(host, eave.nodes.clone(), thickness)
            .with_crest(crest)
            .with_planes(planes);
    }
    let crest = [near, far]
        .iter()
        .map(|position| CrestPoint::new(*position, elevation))
        .collect();
    let planes = vec![
        // The long side the axis was read from, low crest to high.
        long_plane(eave, 0),
        // The other long side, high crest to low: the run reverses so both normals face out.
        long_plane(eave, 1),
        // The far end: one crest point against the two nodes of that end's edge.
        end_plane(eave, 1, (eave.long_side + 1) % 4),
        // The near end.
        end_plane(eave, 0, (eave.long_side + 3) % 4),
    ];
    Roof::new(host, eave.nodes.clone(), thickness)
        .with_crest(crest)
        .with_planes(planes)
}

/// Builds the two planes of a gable over a read eave loop.
///
/// # Invariants
///
/// The same two long planes a hip has, and no end planes: the gable ends are closed by the
/// expansion's fascia band, which is a decision that belongs to the solid and not to the author.
fn gable_over(eave: &Eave, host: MassId, rise: f32, thickness: f32) -> Roof {
    let crest = [eave.crest_at(eave.ends.0), eave.crest_at(eave.ends.1)];
    let elevation = eave.elevation + rise;
    let crest = crest
        .iter()
        .map(|position| CrestPoint::new(*position, elevation))
        .collect();
    let planes = vec![long_plane(eave, 0), long_plane(eave, 1)];
    Roof::new(host, eave.nodes.clone(), thickness)
        .with_crest(crest)
        .with_planes(planes)
}

/// Builds one long side's plane: the two rows that shed from one crest point to one long edge.
///
/// # Invariants
///
/// `side` is `0` for the edge the axis was read from and `1` for the one opposite it, and **the two
/// sides run the opposite way**: the near crest end pairs with the first node of its edge and the
/// far end with the second on one side, and reversed on the other. That reversal is what puts both
/// normals out of the solid, and the probe measures it rather than taking this sentence's word.
///
/// **The two eave stations are corners of the loop** — `first` and `second` are the two nodes of one
/// edge — which is what makes the facet planar: its crest run and its eave run are spans of the
/// crest line and of that edge, and those are parallel by construction. A station that fell between
/// two corners would still be a point on the loop, and the quad would be twisted, which is the one
/// failure this function exists to not have.
///
/// The rows are written from **one** pair of stations, so the neighbour that shares the end row
/// writes the same `EaveStation` value out of the same `stations` entry rather than recomputing it.
fn long_plane(eave: &Eave, side: usize) -> RoofPlane {
    let first = (eave.long_side + 2 * side) % 4;
    let second = (first + 1) % 4;
    RoofPlane::new(vec![
        RoofRow::new(side, eave.station(first)),
        RoofRow::new(1 - side, eave.station(second)),
    ])
}

/// Builds one end's plane: the same crest index against the two nodes of that end's edge.
///
/// # Invariants
///
/// The facet is a **triangle** — one crest point named by both rows against the two corners of the
/// end edge — and three points are one plane, so an end plane is planar by construction rather than
/// by an argument about parallel runs. `node` and `node + 1` are the two corners of one edge of the
/// loop, in loop order, which is also what puts the normal out of the solid.
fn end_plane(eave: &Eave, crest: usize, node: usize) -> RoofPlane {
    let here = eave.station(node);
    let there = eave.station((node + 1) % 4);
    RoofPlane::new(vec![RoofRow::new(crest, here), RoofRow::new(crest, there)])
}

/// Returns a rectangle centred on the origin, on the ground, wound with the solid on the walker's
/// right — the winding whose signed plan area is positive, which is what [`Mass::plan_area`] states
/// and [`starter`] needs.
fn rectangle(width: f32, depth: f32) -> Vec<PlanNode> {
    let half_width = width * 0.5;
    let half_depth = depth * 0.5;
    [
        [-half_width, -half_depth],
        [half_width, -half_depth],
        [half_width, half_depth],
        [-half_width, half_depth],
    ]
    .into_iter()
    .map(|position| PlanNode::new(position, 0.0))
    .collect()
}

/// Builds an intermediate floor inside a mass's own walls, at `elevation`, `thickness` deep.
///
/// # Invariants
///
/// - **The plate is the mass's inner loop**, which is what `expand::offset_loop` computes for the walls:
///   the floor meets its walls rather than overlapping them or leaving a gap, and it is the same inset
///   the walls themselves are built on, so the two cannot disagree about where the inside is.
/// - **It is hosted by the mass**, so seating moves it with the building.
/// - **The elevation must be strictly inside the mass's own height.** A floor at or below the ground is under
///   the building and one at or above the wall top is the roof's business; both answer `None` rather than a
///   plate that would be invisible or sticking out.
/// - A wall the miter cannot offset answers `None`: the mass's own expansion reports that fault, and a floor
///   derived from a wall that has no inside has no inside either.
///
/// **The guard above is about where a floor is, not about what it is made of**, which is why
/// [`starter`] cannot call this function and does not try: its plate is the same inner loop at the
/// ground rather than above it. What the two share is [`inner_loop`].
///
/// `O(nodes)` to offset, once.
pub fn floor_slab(mass: &Mass, id: MassId, elevation: f32, thickness: f32) -> Option<Slab> {
    let lowest = mass.lowest();
    if !(elevation > lowest && elevation < mass.top && thickness > 0.0) {
        return None;
    }
    Some(Slab::floor(inner_loop(mass, id)?, elevation - thickness, elevation).with_host(id))
}

/// Returns the loop a mass's walls stand on: its outline offset inward by the wall's own thickness.
///
/// # Invariants
///
/// **One walk for both floors.** An intermediate floor ([`floor_slab`]) and the ground plate
/// [`starter`] gives a mass are both seated on it, so the two cannot disagree about where the inside
/// of a building is — and they are the same loop by construction rather than by two offsets that
/// happen to agree.
///
/// `None` when the wall ring's own walk refuses: a mass whose wall cannot be mitred has no inside to
/// put a plate in, and the refusal is the mass expander's to report.
///
/// [`validate`](crate::validate) holds a ground plate to this same loop, because it is the floor the
/// wall ring leaves — so the loop that seats a floor and the one a plate is measured against are one
/// walk rather than two offsets that happen to agree.
pub(crate) fn inner_loop(mass: &Mass, id: MassId) -> Option<Vec<Vec2>> {
    let mut problems = Vec::new();
    let inside = crate::expand::offset_loop(
        &mass.outline,
        crate::mass::AuthoredLoop::Outline { mass: id },
        mass.wall,
        mass.plan_area(),
        crate::expand::OffsetSide::Inward,
        &mut problems,
    )?;
    if !problems.is_empty() {
        return None;
    }
    Some(inside)
}

/// A style: the **relations** a building's numbers obey, rather than the numbers themselves.
///
/// # What problem this solves
///
/// A [`Starter`] is ten numbers an author writes, and coherence between them is the author's problem: a wall
/// 0.3 thick is a bunker in a 3 m shed and a tent in a 12 m hall, and a 1.6 m rise is a roof over a 4 m
/// half-span and a spire over a 1 m one. A style states the relations once — *a wall is a nineteenth of the
/// smaller span and never thinner than 220 mm* — and every size the style is asked for comes out coherent for
/// **that** size.
///
/// # Where a style lives, which is D11's answer
///
/// **In `derive`, never in the skeleton.** The document-is-the-skeleton invariant says a skeleton holds what
/// an author *authored*: a wall's thickness, a rhythm, a roof's rise. A style is a **rule for computing**
/// those numbers, not a fact about a building, and keeping it in the skeleton would make a document's geometry
/// move when a constant changed — the "derivations have no guarantee" rule run backwards. So the style is a
/// `derive` type, and a [`Starter`] is the record of what it produced: a document built from a style holds the
/// numbers, not the rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// What it is called, so a report reads.
    pub name: &'static str,
    /// Wall thickness: (share of the smaller plan span, minimum in metres, **maximum** in metres).
    ///
    /// **A share alone is not enough, and the probe found it**: at a twelfth of a 18 m span a cottage's wall is
    /// 0.63 m, which is a fortress rather than a cottage. A relation between a share and a span needs both ends
    /// bounded — the floor keeps a small building standing and the ceiling keeps a large one a building.
    pub wall: (f32, f32, f32),
    /// Floor slab depth as a share of the wall.
    pub slab: f32,
    /// Eave overhang as a share of the smaller span.
    pub overhang: f32,
    /// How far the eave sinks below the wall top, as a share of the overhang.
    pub sink: f32,
    /// Roof rise as a share of the **half** of the smaller span: the pitch, stated as the style's own.
    pub rise: f32,
    /// Roof thickness as a share of the rise.
    pub roof: f32,
    /// The roof form the style wears.
    pub form: RoofStyle,
    /// One storey's height.
    pub storey: f32,
    /// One bay's width across a facade.
    pub bay: f32,
    /// A window in its bay: (share of the bay's width, sill above the bay's floor, head as a share of the bay).
    pub window: (f32, f32, f32),
}

impl Style {
    /// Returns a starter of this style for a footprint and a storey count.
    ///
    /// # Invariants
    ///
    /// - **Every number is a function of the span it has to be compatible with**, so the style is coherent at
    ///   any size rather than at one: the wall, the slab, the overhang, the sink, the rise and the roof's
    ///   thickness all scale, and the minimum is the only constant.
    /// - **The rise is read from the *shorter* span**, because the ridge runs along the longer axis and the
    ///   roof's pitch is set across it.
    /// - **The height is the storey times the count** and not a number of its own: a style that fixed a height
    ///   would decide the storey count for the author.
    pub fn starter(&self, width: f32, depth: f32, storeys: usize) -> Starter {
        let span = width.min(depth);
        let wall = (self.wall.0 * span).clamp(self.wall.1, self.wall.2);
        let rise = self.rise * span * 0.5;
        Starter {
            width,
            depth,
            height: self.storey * storeys as f32,
            wall,
            slab: self.slab * wall,
            overhang: self.overhang * span,
            sink: self.sink * self.overhang * span,
            rise,
            roof_thickness: self.roof * rise,
            roof: self.form,
            seed: 1,
        }
    }

    /// Returns how many bays this style gives a facade `span` metres wide.
    ///
    /// # Invariants
    ///
    /// **At least one**, and rounded to the nearest bay rather than truncated, so a facade a little shorter
    /// than two bays is two bays of a slightly narrow rhythm rather than one bay and a stub.
    pub fn bays(&self, span: f32) -> usize {
        ((span / self.bay).round() as usize).max(1)
    }

    /// Returns a window's inset in a bay of this style's own rhythm.
    ///
    /// # Invariants
    ///
    /// The **same shape as [`Opening`](crate::openings::Opening)'s inset**: from each jamb, then the sill and
    /// the height, so a scene can cut the windows a style implies without a second table of numbers.
    pub fn glazing(&self, bay_width: f32, bay_height: f32) -> (f32, f32, f32, f32) {
        let inset = (bay_width * (1.0 - self.window.0)) * 0.5;
        let sill = self.window.1.min(bay_height * 0.5);
        let head = (bay_height * self.window.2).min(bay_height - sill);
        (inset, inset, sill, head)
    }
}

/// A cottage: thick walls, a low hip, a deep eave, chunky floor plates and small windows.
///
/// Every number here is a relation, so a 4 m cottage and a 9 m one are both cottages: the wall stays a
/// nineteenth of the span with a 220 mm floor, the eave stays just under a twelfth of it, and the pitch stays a
/// little over half the half-span.
pub const COTTAGE: Style = Style {
    name: "cottage",
    wall: (1.0 / 19.0, 0.22, 0.45),
    slab: 1.15,
    overhang: 0.08,
    sink: 0.22,
    rise: 0.55,
    roof: 0.16,
    form: RoofStyle::Hip,
    storey: 2.6,
    bay: 2.2,
    window: (0.38, 0.95, 0.42),
};

/// A hall: thinner walls, a steep gable, a shallower seat, a thin roof and tall windows in wide bays.
pub const HALL: Style = Style {
    name: "hall",
    wall: (1.0 / 26.0, 0.18, 0.50),
    slab: 1.05,
    overhang: 0.09,
    sink: 0.14,
    rise: 0.85,
    roof: 0.10,
    form: RoofStyle::Gable,
    storey: 3.4,
    bay: 3.0,
    window: (0.52, 1.15, 0.55),
};
