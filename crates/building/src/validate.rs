//! Validation: what the author has got wrong, reported and never repaired.
//!
//! # Contract of this module
//!
//! - **Everything wrong is returned**, in a deterministic order, so two runs diff. A check that
//!   stops at the first fault turns a fix into a sequence of runs.
//! - **The two halves are separate functions.** [`document`] reads the skeleton alone and needs no
//!   expansion, so it answers even for a skeleton that cannot be built; [`mesh`] reads a built mesh
//!   and answers the closure and orientation questions a document cannot. `Building::validate`
//!   composes them.
//! - **A violation is never an error and never a repair.** An intermediate state that violates
//!   something is ordinary — the ops do not refuse edits — and this is the instrument that reads
//!   it.
//! - **Staged checks**: a check that would read a value the document cannot support is skipped
//!   rather than guessed at, the way `mountain`'s validate stages its own.
//!
//! # The order the answer comes back in
//!
//! `order_key` sorts by the variant first and by what the variant is **about** second, and it is
//! a total order: a `f32` or `i64` field is folded to the `u64` `f32::total_cmp` would have ordered
//! it by, so two runs over one input give byte-identical lists rather than two orders of a hash map.
//!
//! # The three numbers here
//!
//! `PLANARITY_TOLERANCE`, `TOUCH_TOLERANCE` and `COVERAGE_TOLERANCE`, each justified where it
//! is declared. None is imported from `mountain`: this crate depends on `shape` for `math` alone
//! (the handoff for `building-p1` says so), and a weld tolerance from `mountain::sampling` would be
//! a second dependency on a crate this crate does not otherwise read.

use std::array;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::expand::Problem;
use crate::grid::{FaceRef, GRID_TOLERANCE, LoopSide, increasing};
use crate::mass::{Mass, MassId, PlanNode};
use crate::math::{Vec2, Vec3, distance, dot3, subtract};
use crate::mesh::{BuildingMesh, Element, RoofSurface, SolidId};
use crate::openings::Extents;
use crate::parts::Target;
use crate::roof::{Roof, RoofId};
use crate::skeleton::BuildingSkeleton;

/// How far an authored surface's vertices may stand off that surface's own fitted plane.
///
/// # Invariants
///
/// It is **absolute**, in metres, because an authored surface is authored in metres and a departure
/// is a length. The value is a millimetre, and it is chosen between two measurements rather than
/// by taste:
///
/// - The noise it has to absorb is `f32` rounding. A group of vertices at building scale carries
///   about `1e-7` of relative error, and the normal is a sum divided by an area that may be a
///   square metre, so anything under about `1e-5` is arithmetic rather than geometry.
/// - The slip it has to catch is an **authored** one: a crest elevation written in centimetres, an
///   eave station that resolved onto the wrong node, a corner welded to the wrong row. The
///   smallest of those is a centimetre, an order of magnitude above this.
///
/// A millimetre therefore keeps the two apart with a decade of head-room on each side, and is the
/// same kind of number as `mountain`'s `WELD` rather than a value tuned per building.
pub(crate) const PLANARITY_TOLERANCE: f32 = 1.0e-3;

/// How close in plan two masses have to stand before their outlines count as touching rather than
/// overlapping.
///
/// # Invariants
///
/// Two masses whose walls stand flush against each other share a plane and no area, so a test that
/// counted a shared edge as an overlap would report every pair of masses an author had butted up
/// without attaching — which is a **different** fault with a different fix, and is not what
/// [`Violation::UnattachedOverlap`] names. The margin is a millimetre for the reason
/// [`PLANARITY_TOLERANCE`] has one: it is a length, it is above the rounding of plan coordinates,
/// and it is far below the gap between two masses that were meant to stand apart.
const TOUCH_TOLERANCE: f32 = 1.0e-3;

/// How much plan area two masses may share before it is an overlap, in square metres.
///
/// **A square of the touch tolerance**: a shared plane contributes zero area, and a shared strip wider than a
/// millimetre contributes more than this at any length the crate draws.
const AREA_TOLERANCE: f32 = TOUCH_TOLERANCE * TOUCH_TOLERANCE;

/// How far a skin may miss its own eave loop, in metres of edge, before the missing area is a
/// defect.
///
/// # Invariants
///
/// **It is a width, and the check converts it to an area by the loop's own length**: the area
/// allowed to be missing is `COVERAGE_TOLERANCE × eave_length`, so a roof is held to the same
/// millimetre whether its eave is a shed's or a warehouse's. A gap of `w` metres along an edge `L`
/// long is an area of `w·L`, which is what makes one linear number the honest statement of a
/// tolerance about a surface nobody tiles exactly.
///
/// The value is a millimetre for the reason [`PLANARITY_TOLERANCE`] is one: it is a length, it sits
/// above the `f32` rounding of plan coordinates at building scale (about `1e-6` m over a span of
/// tens of metres), and it is far below the slip it has to catch. The smallest authored slip is a
/// station landing a centimetre off the node it meant — an eave corner skipped, a row paired with
/// the wrong neighbour — and the largest is a whole corner of the loop, which is metres.
const COVERAGE_TOLERANCE: f32 = 1.0e-3;

/// The smallest area a facet may have and still be a facet, in square metres.
///
/// # Invariants
///
/// **It is an area and not a length, and it is the square of a length this module already uses.**
/// A facet of no area is a facet of no surface: it draws nothing, it casts no shadow worth naming,
/// and it is invisible to every other reading here — its edges are shared with their neighbours
/// twice over, so closure, manifoldness, Euler characteristic and volume are all satisfied by a
/// zero-area triangle exactly as they are by a real one. Measured on this tip, a square hip's
/// collapsed ridge emits **four** of them, and the census read that solid as closed, manifold,
/// outward, euler 2 and of positive volume.
///
/// The number is chosen between two measurements rather than by taste:
///
/// - **The noise it has to absorb is `f32` rounding on a cross product.** Three corners that are
///   collinear in a document written at metre scale do not measure zero: measured `1.97e-6` m² for
///   a triple at `(0, 3.1, −6)`, `(2, 4.7, 6)`, `(4, 6.3, 18)`, because the cross product of two
///   metre-scale differences carries about `1e-7` of relative error and `1e-6` m² is the square of
///   a thousandth of a metre. A threshold under that would report a rounding artefact as a defect.
/// - **The slip it has to catch is exactly zero.** Two rows naming one crest point emit a facet of
///   `0.000000000` m², and so does a face that names one of its own corners twice. Measured, not
///   argued: both readings are exact zeros, four decades under this threshold.
///
/// Between them sits the smallest facet any starter emits — `0.500` m² for the gable's gable-end
/// triangle, `0.960` m² for the hip's — which is three decades above it. So the value is a
/// centimetre on a side: it is the same kind of number as [`TOUCH_TOLERANCE`] and the same kind of
/// reasoning, read in the units the quantity is in.
const DEGENERATE_AREA: f32 = 1.0e-4;

/// How much plan area two voids may share before it is an overlap, in square metres.
///
/// **It is the same tolerance [`Violation::VoidNotInside`] allows between a void's own area and the
/// area of it the outline encloses**, because both are the same measurement of the same relation —
/// two loops that should coincide, compared through [`crate::clip`], whose flush case answers zero
/// but not to the last bit of a float. A courtyard's wall within a millimetre of a wall's face is a
/// touch; a courtyard's wall a centimetre inside one is metres of area and is reported.
const VOID_AREA_TOLERANCE: f32 = 1.0e-3;

/// How close two of a facet's corners have to be before they are called one point.
///
/// # Invariants
///
/// It separates the two classes of a facet of no area ([`Violation::FaceDegenerate`]) and it is a
/// **coincidence** test, not a proximity test: the fault is a point two elements each authored at one
/// place, which is exact — the phase's square hip stood two crest points at *the same* coordinates —
/// so this only has to clear `f32` rounding at building scale. Ten micrometres is four decades above
/// that and five below the shortest edge any starter emits (`0.3` m for a wall's thickness), so no
/// structural corner of a real building can be mistaken for a coincidence.
const COINCIDENT_TOLERANCE: f32 = 1.0e-5;

/// Where a loop meets itself, which is the reading that decides
/// [`Violation::VoidNotSimple`] and [`Violation::OutlineNotSimple`].
///
/// One payload for both, because both are the same question asked of a different loop: **a loop
/// that meets itself has no triangulation**, so every area reading about it is a measurement of
/// whichever clipper was asked rather than of the shape.
///
/// # Invariants
///
/// **Two cases and not one, because they are two different edits.** A crossing names two edges, and
/// an author fixes it by moving one node or by reordering the loop; a touch names an edge and a node
/// standing in its interior, and an author fixes it by moving that one node. A count of meetings
/// would tell them neither.
///
/// **The touch is read first, and the reason is that a touch and a crossing are the same contact read
/// two ways.** A segment that runs *through* another's endpoint answers true to
/// `crate::expand::segments_cross_properly` whenever the angle is
/// not degenerate, because one side of the product is exactly zero; naming two edges for it would ask
/// the reader to go and find which of the pair's four ends is the point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfMeeting {
    /// The loop's two edges cross, and the point they meet at is in the interior of both.
    Crossing {
        /// The first edge, counted from the loop's first node.
        left: usize,
        /// The second.
        right: usize,
    },
    /// One of the loop's own nodes stands inside another of its edges, so the loop touches itself at a
    /// point its author did not write down.
    Touch {
        /// The edge that runs through the node.
        edge: usize,
        /// The node it runs through.
        node: usize,
    },
}

/// One thing wrong with a building.
#[derive(Debug, Clone, PartialEq)]
pub enum Violation {
    /// A mass's outline is wound so its walls face inward, which is a building that is not drawn.
    OutlineReversed {
        /// The mass.
        mass: MassId,
        /// The signed plan area that came out negative.
        area: f32,
    },
    /// A mass's outline has fewer than three nodes, so it encloses nothing.
    OutlineTooShort {
        /// The mass.
        mass: MassId,
        /// How many nodes it has.
        nodes: usize,
    },
    /// A mass's outline meets itself at a point its author did not write down, so it is not the
    /// simple loop the walls are walked from.
    ///
    /// # Invariants
    ///
    /// **This is the loop [`clip`](crate::clip) could be asked about and nothing was standing in
    /// front of it.** A void loop's simplicity was asked and a void loop's fault was named
    /// ([`VoidNotSimple`](Self::VoidNotSimple)) while the overlap subtraction was still reaching
    /// [`clip`](crate::clip) with any outline at all — measured, a needle cut into a mass's north
    /// edge read `12.5000 m²` shared with its neighbour against a true `50.0000`, and an outline
    /// carrying an edge of no length read `0.0000` and reported two buildings as touching at nothing.
    /// Both were measurements of the clipper rather than of the plan, because **a loop that meets
    /// itself has no triangulation for any ear clipper to be exact about**.
    ///
    /// **It is a parallel reading and not a widened [`VoidNotSimple`](Self::VoidNotSimple), because
    /// a mass has no void.** The two faults are the same statement about the same predicate — a loop
    /// that meets itself is a loop the expander's own offset walk cannot resolve — and they differ
    /// in what an author has to edit: one node of a courtyard, or the mass's own plan. Carrying the
    /// void index in this one as an `Option` would have made the payload say "there may be no void
    /// here", which is a variant that is true of a mass rather than one that is false of it, and
    /// every reader of the first kind pays for the second.
    ///
    /// **`meeting` is the same [`SelfMeeting`] the void reading carries, and it is read by the same
    /// `self_meeting`**, so the two cannot come apart: a loop that meets itself is named, and the
    /// subtraction that needs a triangulation is not asked of it. A point two of the outline's own
    /// nodes stand at is **not** this violation and that is measured rather than assumed — see
    /// `self_meeting`'s own argument, which is about a wall and not about an area.
    ///
    /// **Nothing is lost by naming it here, because [`DidNotExpand`](Self::DidNotExpand) still
    /// reaches the same loop** carrying the walk's own refusal. What changes is that the name no
    /// longer depends on which entry point ran: [`document`] says the shape, and
    /// [`Building::validate`](crate::Building::validate) says it too, instead of the shape arriving
    /// only as a mitre past a join limit.
    OutlineNotSimple {
        /// The mass.
        mass: MassId,
        /// Where the outline meets itself, read off the authored loop.
        meeting: SelfMeeting,
    },
    /// The wall top is not above the outline, so the mass has no height.
    ZeroHeight {
        /// The mass.
        mass: MassId,
        /// The authored wall top.
        top: f32,
        /// The highest elevation of the outline.
        highest: f32,
    },
    /// A crown carries a different number of elevations than the outline has nodes.
    ///
    /// # Invariants
    ///
    /// **Neither empty nor one per node is a wall.** Empty is the level top and is the state every
    /// mass in this gallery is in; a crown is a per-node elevation, so a count that disagrees with the
    /// outline cannot be read node by node — and indexing past the end of one would be a mass whose
    /// walls are right up to the node that is missing.
    CrownCountMismatch {
        /// The mass.
        mass: MassId,
        /// How many elevations its crown holds.
        crowns: usize,
        /// How many nodes its outline has.
        nodes: usize,
    },
    /// A crown at or below the elevation its node stands on, so the wall has been cut into its base.
    CrownBelowBase {
        /// The mass.
        mass: MassId,
        /// Which node.
        node: usize,
        /// The crown there.
        top: f32,
        /// The elevation the node stands on.
        base: f32,
    },
    /// A face with a rhythm is on a crowned edge, which is a trapezoid and not a rectangle.
    GridOnACrownedFace {
        /// The face the rhythm names.
        face: crate::grid::FaceRef,
    },
    /// Two masses' plans overlap and neither names the other as a host.
    UnattachedOverlap {
        /// One mass.
        mass: MassId,
        /// The other.
        other: MassId,
    },
    /// Two masses' plans could not be compared, so whether they overlap is not known.
    ///
    /// # Invariants
    ///
    /// **It is the answer the subtraction had no way to give, reported rather than dropped.** A pair
    /// whose bounding boxes meet and at least one of whose plans [`crate::clip`] cannot triangulate is
    /// **not** a pair that shares no area: it is a pair this module never compared, and a reader told
    /// nothing about it reads the silence as a measurement. Measured on this tree, two `3 × 3` squares
    /// sharing an authored corner standing inside a neighbour that wholly contains them really share
    /// `18.0000 m²`, and the check said nothing at all.
    ///
    /// **It is not a second name for [`OutlineNotSimple`](Self::OutlineNotSimple), and the gate is
    /// what keeps the two apart.** An outline that meets itself never reaches this comparison at all:
    /// `check_overlap` skips the pair and `check_masses` has already named the loop, which is the
    /// same argument `check_voids` makes about [`VoidNotInside`](Self::VoidNotInside) against
    /// [`VoidNotSimple`](Self::VoidNotSimple). **What arrives here is the one shape that gate is
    /// silent about by design** — a point two of the loop's own nodes stand at, where two `3 × 3`
    /// squares sharing a corner build a sound courtyard for a void loop and a mass the expander's
    /// inward offset walk refuses outright (`OffsetSelfIntersecting`). There is no other reading of
    /// that loop to arrive under, so it arrives here.
    ///
    /// **It carries the mass whose plan could not be compared, and the neighbour it stands against.**
    /// The first is the loop an author can fix and the second is only the plan it was measured
    /// against, and where both plans are unmeasurable the lower index takes the first — so the
    /// payload always names a loop this module could not triangulate. Each pair is reported once, in
    /// the same order as [`UnattachedOverlap`](Self::UnattachedOverlap). The expander still names the
    /// loop itself through [`DidNotExpand`](Self::DidNotExpand), so nothing is lost by this reading
    /// being about the comparison: it says the question is open, and the other says which loop closed
    /// it.
    ///
    /// **A plan that cannot be measured beside a neighbour it provably does not touch is still
    /// silent**, because bounding boxes that miss are a measurement rather than a refusal, and this
    /// variant is only about the pair the subtraction could not run.
    OverlapNotMeasured {
        /// One mass.
        mass: MassId,
        /// The other.
        other: MassId,
    },
    /// A mass's own outline has no triangulation, so nothing about its plan can be measured.
    ///
    /// # Invariants
    ///
    /// **It is the name the document half owes a loop the expander already refuses.** A mass whose
    /// outline `self_meeting` cannot name reaches [`clip`](crate::clip) with nothing standing in
    /// front of it, and [`clip`](crate::clip) answers `None` for it: [`Building::validate`](crate::Building::validate)
    /// then reports the expander's own [`DidNotExpand`](Self::DidNotExpand) carrying the walk's name,
    /// and [`document`] reported **nothing at all**. **The name must not depend on
    /// which entry point ran**, which is the rule the mass-outline gate was built on, and a loop with
    /// no triangulation is what was left over from it.
    ///
    /// **What arrives here is the one loop both readings are silent about**, and
    /// [`OutlineNotSimple`](Self::OutlineNotSimple) is the other half of that pair: an outline that
    /// meets itself is named there and is never asked the question below, and this is asked exactly
    /// where that name is absent. One predicate, read twice, rather than two tests of one fact — and
    /// **neither widens the other**, because a loop that meets itself is a statement about a shape and a
    /// loop with no triangulation is a statement about a measurement, and a report carrying both would say
    /// twice what one of them says once.
    ///
    /// **It carries the mass and no node, because what decides it is not a node.** The deciding test is
    /// [`clip`](crate::clip)'s own coverage test — are the triangles the loop? — and **that is not the ear
    /// walk stopping**: measured, a `1 m` slit stops the walk and the triangles it stopped with read `12.0000`
    /// against a loop of `12.0000`, so there is no corner the triangulation "stuck on" to name. The corner the
    /// walk *did* stop at is [`Problem::MiterUnresolved`]'s, and it
    /// arrives through [`DidNotExpand`](Self::DidNotExpand) beside this name — so an author who wants the node
    /// reads it from the entry point that expands, and an author who only wants to know the plan is
    /// unmeasurable is served here. **A second reading of where the walk stopped would be a second test that can
    /// disagree with the first**, which is the fault every staging in this module exists to prevent.
    ///
    /// **It does not replace [`OverlapNotMeasured`](Self::OverlapNotMeasured), and a pair it applies to carries
    /// both.** That variant is about a *pair* whose question went unanswered, and a mass standing alone asks no pair
    /// question at all — which is the boundary the overlap gate was drawn around, and the shape above is the answer
    /// to it. The two name one loop from the two sides it has: this says the mass's own plan cannot be measured, and
    /// that says a comparison against it could not be run.
    ///
    /// **A plan the clipper can measure is never named by it**, and the edge of no length is the row that says so:
    /// [`ops::split_outline`](crate::ops::split_outline) writes one on purpose, the clipper takes it out before the walk
    /// and measures the loop exactly, and a check that named a refusal there would be reporting a measurement as a
    /// fault.
    OutlineNotMeasured {
        /// The mass whose own plan has no triangulation.
        mass: MassId,
    },
    /// A mass names a host that does not exist, so the attachment it authorizes is not one.
    MassHostMissing {
        /// The mass.
        mass: MassId,
        /// The host it named.
        host: MassId,
    },
    /// A roof names a host mass that does not exist.
    RoofHostMissing {
        /// The roof.
        roof: RoofId,
        /// The mass it named.
        host: MassId,
    },
    /// A roof's eave loop has fewer than three nodes, so no station on it names a point.
    RoofEaveTooShort {
        /// The roof.
        roof: RoofId,
        /// How many nodes it has.
        nodes: usize,
    },
    /// A roof plane has fewer than two rows, so it spans nothing.
    RoofPlaneTooShort {
        /// The roof.
        roof: RoofId,
        /// The plane.
        plane: usize,
    },
    /// A roof plane's purlin profile is not a profile: a step whose fraction or rise does not
    /// increase, or whose fraction is outside `(0, 1)`.
    ///
    /// # Invariants
    ///
    /// **This and [`Violation::ProfileReachesTheCrest`] are**
    /// [`Roof::profile_fault`](crate::roof::Roof::profile_fault)'s own two answers, and
    /// [`expand`](crate::expand) reads the same function as a problem. One derivation, three names:
    /// a check written twice would be two opinions about what a step is.
    ProfileOutOfOrder {
        /// The roof.
        roof: RoofId,
        /// The plane.
        plane: usize,
        /// Which step, counting from the eave.
        step: usize,
    },
    /// A profile step stands at or above the crest of a row that carries it, which makes it a ridge.
    ProfileReachesTheCrest {
        /// The roof.
        roof: RoofId,
        /// The plane.
        plane: usize,
        /// Which step, counting from the eave.
        step: usize,
    },
    /// A roof's covering is not a covering: a period that is not a length, a fill outside `(0, 1)`, or
    /// a height that is not a rise.
    ///
    /// # Invariants
    ///
    /// It is [`Covering::fault`](crate::covering::Covering::fault)'s own answer and
    /// [`expand`](crate::expand)'s own problem, so a covering that a document check refuses is a
    /// covering the expansion will not draw either — one reading with two names, the way the profile's
    /// fault is.
    CoveringFault {
        /// The roof.
        roof: RoofId,
        /// What is wrong with it.
        fault: crate::covering::CoveringFault,
    },
    /// A roof row names a crest point that does not exist.
    RoofCrestMissing {
        /// The roof.
        roof: RoofId,
        /// The plane.
        plane: usize,
        /// The row.
        row: usize,
        /// The index it named.
        crest: usize,
    },
    /// A skeleton does not expand at all, so nothing else can be said about its solids.
    ///
    /// # Invariants
    ///
    /// **The problems travel with the violation, untranslated.** `expand::Problem` names the thing
    /// that could not be built — a mitre past the join limit, an offset edge folded back on itself —
    /// and re-spelling each of them here would be a second list of the same faults that drifts from
    /// the first the moment the expander grows a variant. One list, carried whole, is also the whole
    /// of what this module owes a skeleton it cannot build: the mesh half has no mesh to read, and a
    /// skeleton that expands to nothing used to read as a building with nothing wrong with it.
    ///
    /// It is reported once per [`Building::validate`](crate::Building::validate) and last in the
    /// list, because it is the composition's own answer and neither half can make it.
    DidNotExpand {
        /// Every problem the expansion found, in the expander's own deterministic order.
        problems: Vec<Problem>,
    },
    /// A solid is not closed: its faces leave a boundary.
    SolidOpen {
        /// The solid.
        solid: SolidId,
        /// How many boundary loops it left.
        loops: usize,
    },
    /// A solid has an edge that is not shared by exactly two of its faces.
    SolidNonManifold {
        /// The solid.
        solid: SolidId,
    },
    /// A solid's faces point inward: its signed volume came out negative.
    SolidInsideOut {
        /// The solid.
        solid: SolidId,
        /// The signed volume that came out.
        volume: f32,
    },
    /// A solid could not be given an Euler characteristic of a closed surface.
    SolidNotASphere {
        /// The solid.
        solid: SolidId,
        /// The characteristic that came out.
        euler: i64,
    },
    /// A roof's skin does not tile the eave loop the author drew.
    ///
    /// # Invariants
    ///
    /// A roof's skin does not cover its eave loop in area.
    ///
    /// # Invariants
    ///
    /// **The check every closure predicate was blind to.** The expander closes *any* skin by
    /// construction — the skin, its own reverse one thickness below, and a fascia band around the
    /// **skin's own boundary** — and orients it by one rule. A roof whose row stations land
    /// mid-edge, skipping an eave corner, therefore expands to a closed, edge-manifold, outward solid
    /// of positive volume with euler characteristic 2, and nothing in that list can see the hole in
    /// the roof over the missing corner. The design's exit criterion is "closed, outward, **and
    /// covered**", and this is the covered part of it.
    ///
    /// **The comparison is on magnitudes.** An up-facing triangle's plan area is the negative of its
    /// projection in this crate's plan convention, so a correctly tiled skin reads `-96.0000`
    /// against a `+96.0000` eave loop, and the sign that says so is the winding's business —
    /// [`Violation::SolidInsideOut`] owns it. Coverage asks one question, which is how much of the
    /// loop is covered, and a fault that is reported twice under two names is a fault an author has
    /// to triage twice.
    ///
    /// **It is a net area, and a net cannot tell a covering from a hole plus a skirt of the same
    /// size** — a review built exactly that roof and got an empty violation list. The boundary is
    /// what tells them apart, which is [`RoofSkinLeavesTheEave`](Self::RoofSkinLeavesTheEave)'s
    /// business and its only reason to exist; this number stays because it is cheap, because it names
    /// the common case in the units an author thinks in, and because a fault reported under two names
    /// is still better than one reported under none.
    RoofDoesNotCover {
        /// The roof.
        roof: RoofId,
        /// The plan area its skin covers.
        skin: f32,
        /// The plan area of the eave loop it must cover.
        eave: f32,
    },
    /// A roof's skin has a boundary that is not the eave loop: a notch cut into it, or a skirt
    /// reaching past it.
    ///
    /// # Invariants
    ///
    /// **This is the check that a net-area comparison cannot replace**, and it exists because a
    /// review built the counterexample: a hip with its near end plane deleted (a notch, which removes
    /// area) and its crest slid past the far eave edge (a skirt, which adds the same area) nets to the
    /// eave's own area, closes as a solid, keeps every closure predicate green — and is a roof with a
    /// hole over one end and an overhang over the other. The **area** of a skin cannot tell that from
    /// a covering; the **boundary** can, because a covering skin's boundary *is* the eave loop, and
    /// every vertex of a notch or a skirt stands off it.
    ///
    /// It is measured in plan, against the eave loop's own polyline, with `COVERAGE_TOLERANCE` as
    /// the distance a boundary vertex may stand off it. A roof that expanded no skin never reaches
    /// this check: the expansion has already named the reason.
    RoofSkinLeavesTheEave {
        /// The roof.
        roof: RoofId,
        /// How many of its skin's boundary vertices stand off the eave loop.
        off: usize,
    },
    /// A roof's skin is not one piece, so its boundary is not the eave loop alone.
    ///
    /// # Invariants
    ///
    /// A hole entirely enclosed by skin adds a boundary loop, and the count is what tells that apart
    /// from a covering. It is the cheap companion of
    /// [`RoofSkinLeavesTheEave`](Self::RoofSkinLeavesTheEave), which catches the notches and skirts a
    /// strip-shaped skin can actually produce; this one catches the enclosed hole a future
    /// two-dimensional plane would allow.
    RoofSkinNotADisc {
        /// The roof.
        roof: RoofId,
        /// How many boundary loops its skin has.
        loops: usize,
    },
    /// A face carries a rhythm, and the document holds no such face.
    ///
    /// The one grid fault that is about the reference rather than the numbers, and it is staged
    /// exactly like the others: there is nothing to measure the stops against, so this is reported
    /// and the rest of that face's checks are skipped rather than guessed at.
    GridFaceMissing {
        /// The face the rhythm names.
        face: FaceRef,
    },
    /// A face's rhythm has fewer than two stops in one of its lists, so it makes no bay.
    ///
    /// An **empty** grid is not this: no rhythm at all is a state an author passes through, and
    /// `FaceGrid::is_empty` tells the two apart. One stop is the fault, because one stop is a family
    /// with nothing between its ends.
    GridTooShort {
        /// The face.
        face: FaceRef,
    },
    /// A face's stops are not strictly increasing, so some bay has no width or no height.
    GridNotOrdered {
        /// The face.
        face: FaceRef,
    },
    /// A face's rhythm does not span the face: a level outside the wall, or a division off the edge.
    ///
    /// # Invariants
    ///
    /// **Both ends are checked, not only the far one.** A family that starts below the wall's foot has
    /// a gap at the bottom of the facade, and a tiling sum over the bays alone would not see it —
    /// which is why this is a statement about the stops against the face's own extents rather than a
    /// statement about the bays against each other.
    GridOutsideFace {
        /// The face.
        face: FaceRef,
    },
    /// An opening names a bay the document's grid does not hold.
    ///
    /// # Invariants
    ///
    /// **The wall travels with the index**, because a slot is a `(face, row, column)` triple and
    /// "opening 3" does not tell an author which wall is wrong. The face is the one the opening named,
    /// which is the only reading available: the whole point of the fault is that the document holds no
    /// grid for it.
    ///
    /// This is the check that makes slot-naming honest. A placement keyed by coordinates would land
    /// somewhere else after a re-split and read as a building; a placement keyed by slot is orphaned by
    /// one, and this names it.
    OpeningBayMissing {
        /// The opening, by its index in the document's list.
        opening: usize,
        /// The wall whose slot the opening names.
        face: FaceRef,
    },
    /// An opening's insets leave it no rectangle inside its own bay.
    ///
    /// # Invariants
    ///
    /// **It is the refusal [`Opening::extents`](crate::openings::Opening::extents) already makes,
    /// carried rather than re-derived**: an inset that is not a distance inward, and a hole with no
    /// width or no height left in its bay, are the same statement about the same opening, and this
    /// check reads the one answer rather than a second arithmetic that could disagree with the
    /// expander's.
    ///
    /// `ops::add_opening` refuses to write one, so what this catches is a document that went past the
    /// op or had its bay re-split underneath it — a re-split into narrower bays is the ordinary way to
    /// arrive here, and it is why the bay is read through the document's own grid.
    ///
    /// **The index is the whole of what it carries**, because all four insets belong to one opening
    /// and are read off it: which of them was wrong is not a second fault with a second fix.
    OpeningExceedsBay {
        /// The opening.
        opening: usize,
    },
    /// An opening's hole reaches the boundary of the wall face it is cut into.
    ///
    /// # Invariants
    ///
    /// **Reaching the boundary is the fault the cut depends on** ([`crate::openings`]): this phase
    /// cuts rectangles out of a rectangle, and a hole that touches an edge makes the wall a set of
    /// pieces joined at the author's own corners.
    ///
    /// **The strip a hole must stay inside runs `0..the edge's own length` across and the ground line
    /// to the wall top up.** The ground line is `(node elevation − slab)` at each end of the edge,
    /// linear between them: it is the underside of the slab, which is where the expander starts the
    /// wall face (`expand.rs` lifts its `outside_base` to `elevation − slab`). **The floor is the
    /// maximum of the line's two ends of the hole, and the maximum is over the ends because the ground
    /// line is straight**: a linear function attains both its extremes on an interval at that
    /// interval's ends, so the lowest ground under a hole is under one of its two jambs. Reading the
    /// line at the hole's midpoint instead would clear a hole on a slope whose lower end stands over
    /// higher ground than the centre does.
    ///
    /// **A door is not this fault, and on level ground nothing reaches the ground line at all.** The
    /// wall face stands on the slab's underside, one slab below the elevation the mass is authored at,
    /// and a grid's bottom row already starts at that elevation
    /// ([`crate::grid::face_extent`]), so the wall under a bottom-row sill is
    /// the slab's own thickness and a hole there is an interior hole like any other. An inset only ever
    /// moves a hole **inward** — the sill up, never down — so a bottom-row hole cannot be driven onto
    /// the ground line by its own insets; the line is reached where the ground **rises across the
    /// face**, which is a wall on a slope. A door is a hole in row 0 whose sill is at the elevation the
    /// mass was authored at, and it clears this check by exactly the slab's thickness.
    OpeningOutsideWall {
        /// The opening.
        opening: usize,
    },
    /// Two openings in one wall share area.
    ///
    /// # Invariants
    ///
    /// **Touching is not sharing**, and the comparison is [`Extents::overlaps`]'s: two holes meeting
    /// exactly along an edge remove no wall twice. That is the case a divided facade produces on
    /// purpose — a hole in each of two adjacent bays whose insets run them together at the mullion —
    /// and a check that counted the shared line would report every facade that was divided and then
    /// opened.
    ///
    /// **Each unordered pair once, the lower index first.** Three mutually overlapping holes are three
    /// faults and not six, and the order is the document's own, so two runs diff.
    ///
    /// **Openings in different walls cannot share area**, and an opening with no rectangle is not
    /// compared at all: a hole with no extents has no area to share, whatever else is in its wall.
    OpeningOverlaps {
        /// The earlier of the two openings, by its index in the document's list.
        opening: usize,
        /// The later of the two.
        other: usize,
    },
    /// An opening names the **far loop** of a wall: the face a wall's thickness away from the one the
    /// cut is made from.
    ///
    /// # Invariants
    ///
    /// **A hole is cut through both of a ring's faces at once**, which is the whole of why this loop was
    /// unnameable until the far side itself became addressable. A window in a courtyard's wall is one
    /// rectangle on the void loop and it goes through the ring; naming the offset loop as well would be
    /// a second rectangle for one hole, with no second wall to put it in, and the walk would decline to
    /// see it — a filter with no companion, which is the one thing this crate refuses to ship.
    ///
    /// A **rhythm and a part may name the far loop** and are seated on it: a bay is a slot on a face
    /// and a part stands on the face its bay names. Where a hole is cut *from* is a different question
    /// from where a part stands, and this is the answer to the first of them.
    OpeningOnAFarLoop {
        /// The opening.
        opening: usize,
        /// The wall face it names.
        face: FaceRef,
    },
    /// A placement names a template the document does not hold.
    ///
    /// Staged like the others: with no template there is no box, so nothing about where the part
    /// stands can be asked.
    PlacementTemplateMissing {
        /// The placement, by its index in the document's list.
        placement: usize,
    },
    /// A placement names a bay the grid does not hold.
    ///
    /// The face may have no rhythm at all, or the row and column may be past its stops — the same
    /// question [`OpeningBayMissing`](Self::OpeningBayMissing) asks of a hole, because a part and a
    /// hole are both authored against a bay and both are orphaned the same way by a re-split.
    PlacementBayMissing {
        /// The placement.
        placement: usize,
        /// The wall it names.
        face: FaceRef,
    },
    /// A placement's template does not fit in the bay it stands in.
    ///
    /// # Invariants
    ///
    /// **It is a statement about the template's box against the bay's own extents**, both measured in
    /// the bay's coordinates, and it is a fault rather than a refusal: a part whose box overhangs its
    /// bay by a centimetre is a part an author wants placed while they move it, which is why
    /// [`place_part`](crate::ops::place_part) accepts it and this check reports it. `past` is how far
    /// the worst edge stands out, so the answer reads the way an author measures.
    ///
    /// A template with no mass is not this fault: it has no box, and a part of no material is caught by
    /// the fact that it emits no solid rather than by a containment reading.
    PlacementOutsideBay {
        /// The placement.
        placement: usize,
        /// How far the worst edge of its box stands past the bay.
        past: f32,
    },
    /// A placement names a wall run the document does not hold.
    ///
    /// A document that has lost a wall since a part was hung on it is a document whose part stands nowhere;
    /// nothing about the run can be measured, so the placement is reported and skipped.
    PlacementWallMissing {
        /// The placement.
        placement: usize,
        /// The wall it names.
        wall: crate::wall::WallId,
    },
    /// A wall-mounted placement's **pin** stands off its run.
    ///
    /// # Invariants
    ///
    /// **This is a check on the pin and not on the part's box**, which is the one place the crate's two
    /// targets deliberately differ: a gate leaf is hung at the end of one run and closes the gap beyond it, so
    /// its geometry *must* reach past the wall. What may not reach past the wall is the point it hangs from —
    /// a part pinned where its wall is not is a part floating, and `past` is how far the pin stands out along
    /// the run or below its base.
    PlacementOffItsWall {
        /// The placement.
        placement: usize,
        /// The run it names.
        wall: crate::wall::WallId,
        /// How far the pin stands past the run's own line: beyond either end, or below its base.
        past: f32,
    },
    /// A wall-mounted placement is also mounted on a joint.
    ///
    /// Every anchor names a feature of a **bay** — its sill, its head, an opening's head — and a wall run has
    /// none of them, so a joint on a wall-mounted part has nothing to resolve against. Refused by name rather
    /// than resolved against the run's own ends by guesswork.
    PlacementMountOnAWall {
        /// The placement.
        placement: usize,
    },
    /// A placement hangs from a socket its template does not name.
    ///
    /// Staged like the others: with no socket there is no point to mate, so nothing about where the
    /// part lands can be asked. The name is carried because it is what the author wrote.
    PlacementSocketMissing {
        /// The placement.
        placement: usize,
        /// The socket it names.
        socket: String,
    },
    /// A placement hangs from an anchor that names nothing in its own wall.
    ///
    /// The anchor is a feature of the document — an opening's head or sill, a bay's own edge — and a
    /// joint to an opening in another wall (or to an opening whose slot the grid has lost) has nowhere
    /// to land. It is one variant for both because a reader's next move is the same: check the wall
    /// the anchor names.
    PlacementAnchorMissing {
        /// The placement.
        placement: usize,
    },
    /// A wall's two ends coincide, so it has no run and no box.
    WallTooShort {
        /// The wall.
        wall: crate::wall::WallId,
        /// The length that was measured.
        length: f32,
    },
    /// A wall's thickness is not a thickness.
    WallNotThick {
        /// The wall.
        wall: crate::wall::WallId,
        /// The thickness that was given.
        thickness: f32,
    },
    /// A wall's top is at or below the ground it stands on, so it is not a wall.
    ///
    /// # Invariants
    ///
    /// The comparison is against the **higher** of the run's two ground elevations, because a top that
    /// clears the ground at one end and not the other is a wall that disappears into a slope.
    WallTooLow {
        /// The wall.
        wall: crate::wall::WallId,
        /// The top it was given.
        top: f32,
        /// The highest ground under its ends.
        highest: f32,
    },
    /// A slab's two elevations are the wrong way round, so it is not a prism.
    SlabNotThick {
        /// The slab.
        slab: crate::slab::SlabId,
        /// The thickness that came out of its elevations.
        thickness: f32,
    },
    /// A slab's outline is not a polygon.
    SlabNotAPolygon {
        /// The slab.
        slab: crate::slab::SlabId,
        /// How many nodes it had.
        nodes: usize,
    },
    /// A slab names a mass the document does not hold, so the seating it authorizes is not one.
    ///
    /// # Invariants
    ///
    /// **This is the third element's own answer to a fault two of its siblings already name, and the
    /// third is the only one that was silent.** A mass names a host ([`MassHostMissing`](Self::MassHostMissing))
    /// and a roof names a host ([`RoofHostMissing`](Self::RoofHostMissing)), and both were reported; a
    /// slab has carried the same field since it became a prism of its own, and it was **staged out** —
    /// the plate's coverage check returns early on a host the document does not hold, because there is no
    /// outline to measure the plate against, and behind that early return a document that had lost its
    /// mass said nothing at all about the floor or the roof still standing where that mass was.
    ///
    /// **One variant for the three roles, and the role travels in the payload.** The three are seated
    /// differently — [`derive::floor_slab`](crate::derive::floor_slab) and `derive::starter`'s plate on
    /// the host's **inner** loop, `derive::starter`'s `RoofStyle::Flat` arm over the host's **eave**,
    /// which stands outside its outline — so the host is what determines the loop each was written
    /// against, and a roof really is seated on something a plate never was. But **the fault is in the
    /// reference and not in the seating**: a missing host is a statement about an index, and the slab's
    /// own plan and its two elevations are in the document already and are unaffected by the mass's
    /// absence — there is nothing of the host's left to measure. The reading that decides it is one
    /// lookup, `skeleton.mass(slab.host)`, and it is the *same* lookup for all three roles, so three
    /// variants would be one check spelled three times over a field that holds a single [`MassId`]. The
    /// role is carried because **it is the only thing that says what the missing mass was carrying**:
    /// between "the ground plate under a mass that is gone" and "the flat roof over a wall top that is
    /// gone" there is the difference between looking in the mass's list and looking for a roof, and a
    /// reader holding a `SlabId` alone has to go and read the slab back to learn which it is.
    ///
    /// **It and [`PlateCoverage`](Self::PlateCoverage) are the host that is not there and the host that
    /// is.** A plate whose host exists and falls short of its floor is one fault with an area in it; a
    /// plate whose host does not exist has no floor to fall short of, so there is no number for that
    /// check to report and this one reports the fact instead.
    ///
    /// **No host is not this fault**, and that is [`Slab::host`](crate::slab::Slab::host)'s own
    /// documented reading: a step, a plinth and a canopy are plates that stand on nothing and are left
    /// exactly where their author put them.
    ///
    /// **It is not staged by the outline the way the coverage check is.** The other two slab checks read
    /// the plan and the two elevations, so a loop of two nodes has already answered for them; this one
    /// reads nothing but the host, so a slab that is a degenerate polygon *and* names a mass that is gone
    /// is two faults with two fixes rather than one reported twice.
    SlabHostMissing {
        /// The slab, by its index in the document's list.
        slab: crate::slab::SlabId,
        /// The mass it named, which the document does not hold.
        host: MassId,
        /// What the slab is, and therefore what the missing mass was carrying: a ground plate, an
        /// intermediate floor, or the plate a flat roof is.
        role: crate::slab::SlabRole,
    },
    /// A mass's plate covers less of the mass's own outline than the outline is.
    ///
    /// # Invariants
    ///
    /// **It is a reading between two loops the author wrote, and not between a mesh and a document.**
    /// The shell-era `MassDoesNotCover` compared the shell's soffit face set against the outline, and
    /// that face set was the only thing that could fill a coverage bucket. In the volume model the
    /// plate is a [`Slab`](crate::slab::Slab) of its own, so both sides are authored and the reading
    /// is about the **document** — which is why this is one of [`document`]'s and not one of
    /// [`mesh`]'s. A plate too small for its mass is an authoring fault, and an author hears about it
    /// for a skeleton that cannot be expanded at all.
    ///
    /// **A plate is the hosted slab whose [`SlabRole`](crate::slab::SlabRole) is `Plate`, and the
    /// role is what selects it.** `host` alone cannot: [`derive::floor_slab`](crate::derive::floor_slab)
    /// builds an **intermediate** floor on the mass's inner loop — smaller than the outline on
    /// purpose, so that the floor meets its walls rather than running under them — and
    /// [`derive::starter`](crate::derive::starter) builds a **flat roof** on the same host at the
    /// wall top. All three name the mass and all three cover less than its outline, and only one of
    /// them is a plate. This check once told them apart by elevation — *the hosted slab whose top
    /// stands at its host's ground line* — which asked a reader of the document to know where each
    /// slab sits before knowing what it was and needed a millimetre of margin to admit the levels a
    /// plate is really authored at; **a floor `0.0005` above the ground line came through that margin
    /// and was reported as a plate `15.800003` m² short of its floor**, by a rule whose own tolerance
    /// was the defect. The role is on the slab, authored where the slab is built, and the check reads
    /// nothing else.
    ///
    /// **It is a shortfall only, never an overshoot**, which is where it differs in kind from
    /// [`RoofDoesNotCover`](Self::RoofDoesNotCover). That one compares magnitudes because a skin
    /// that skirts its eave has no other reading — the fascia follows the skin's own boundary — and
    /// [`RoofSkinLeavesTheEave`](Self::RoofSkinLeavesTheEave) catches the overshoot as well. A plate
    /// that stands outside its host's outline is a deck, a canopy or a plinth, all three of which
    /// [`Slab`](crate::slab::Slab)'s own documentation names as plates, so there is nothing to
    /// compare an overshoot against.
    ///
    /// **A mass with no plate at all is not this fault.** A plate is optional, and two of the
    /// crate's own documents are the evidence: a part template's skeleton is one mass and no slabs
    /// (the shutter), and the `yard` scene's store carries no plate because its plate was never
    /// carried across with its mass. A check that reported that would report the crate's work, so
    /// this one starts from the slabs a document holds rather than from the masses it wants to see
    /// them under.
    ///
    /// **The tolerance is a width times the loop's own perimeter** (`COVERAGE_TOLERANCE`), the
    /// conversion [`RoofDoesNotCover`](Self::RoofDoesNotCover) already makes, so a plate is held to
    /// the same millimetre of edge whether its mass is a shed or a hall.
    PlateCoverage {
        /// The mass the plate stands under.
        mass: MassId,
        /// The plate.
        slab: crate::slab::SlabId,
        /// The plan area the plate covers.
        plate: f32,
        /// The plan area the plate has to reach: **the host's inner loop**, which is the floor its
        /// wall ring leaves it. A wall the miter refuses has no inner loop, and such a mass is held to
        /// its outline instead.
        outline: f32,
        /// How much of the floor the plate misses, which is `outline - plate`. It is carried so a
        /// reader can see the size of the fault without doing the subtraction, and it is the same
        /// number twice rather than a reading of its own.
        short: f32,
    },
    /// A facet has no area because **two of its corners are one point**, so it is a facet of no
    /// surface.
    ///
    /// # Invariants
    ///
    /// **This is the one fault of the four that every other reading here is blind to**, because a
    /// zero-area triangle satisfies all of them: its edges are used by their neighbours exactly as a
    /// real facet's are, so the solid stays closed, edge-manifold, outward, a sphere and of positive
    /// volume. The phase's own square hip roof emitted four of them — a collapsed ridge gives two
    /// crest points at one spot, and points are welded by authored identity rather than by coordinate,
    /// so the two are two vertices and the triangle between them has no area.
    ///
    /// **A facet of no area is two classes, and only one of them is the author's** — which is a
    /// correction to what this check first said, and it was measured rather than argued:
    ///
    /// - **Two corners at one point** is the fault this reports. A ridge of no length, a wall face
    ///   collapsed onto itself, a bay of no width.
    /// - **Three distinct corners in a line** is *structural*, and it is not reported: an outline
    ///   carrying a collinear node — which is exactly what `ops::attach_mass` inserts when it splits a
    ///   host — must be closed by a facet of no area, because the ring's edges around that node each
    ///   need their second face and the disc has to carry the same vertices the wall does. Removing it
    ///   opens the solid. Reporting it left the phase's own attach verb unable to produce a document
    ///   this validator calls clean: measured, two violations per split host, before this
    ///   discriminator existed.
    ///
    /// The two classes are told apart by their corners and not by their area, since both areas are
    /// zero. The area is still reported rather than a boolean so that the threshold can be read off
    /// the answer: a facet at `1e-8` is a different fault from one at `9.9e-5`, and the second is
    /// within a whisker of the tolerance where the first is six decades under it.
    FaceDegenerate {
        /// The face.
        face: usize,
        /// The area it measured.
        area: f32,
    },
    /// An authored surface — one element's facets — is not one plane within tolerance.
    ///
    /// # Invariants
    ///
    /// **The fault is the element's, not the triangle's.** A triangle is planar by construction, so
    /// there is no such thing as a non-planar face here; what the contract makes planar is the
    /// authored surface, because one wall face and one roof plane are one plane however many
    /// triangles they were cut into, and a facet that does not lie in that plane is either a seam
    /// that will show or a shading group whose normals disagree with its silhouette.
    FaceNotPlanar {
        /// The element's first face, naming the group the fault is in. The reported face is the
        /// group's lowest index so that the number is a stable handle on the group.
        face: usize,
        /// How far the group's farthest vertex stands off the group's own fitted plane.
        departure: f32,
    },
    /// A mass's void is not strictly inside its outline, so the hole it makes would open through the wall.
    ///
    /// # Invariants
    ///
    /// **The subtraction behind it is exact only for a simple loop, so it is asked only of one.** A
    /// symmetric bowtie `+8 − 8` gives `8.0` shared with the outline against `12.0` shared with itself,
    /// and the same bowtie stood on its side as an hourglass gives `8.0` against `8.0` — both measured
    /// inside a mass **twice as wide as the loop**, so neither is outside its outline. What the numbers
    /// were is whatever [`crate::clip`]'s ear clipper made of a polygon that has no triangulation, so
    /// this violation reported that the clipper disagreed with itself **and reported it for one of two
    /// identical shapes**. The gate is `self_meeting`'s own answer: a loop it names is
    /// [`VoidNotSimple`](Self::VoidNotSimple)'s to report, and the subtraction is not asked of it.
    ///
    /// **"Simple" is the gate and it is not the whole of what the clipper can measure.** A loop carrying
    /// an edge of no length does not meet itself, so it is asked — and it is asked **correctly**,
    /// because [`crate::clip`] removes such an edge before it clips. That is measured rather than assumed
    /// and it was the other way round: with the edge left in, a `4 x 4` void with one node written down
    /// twice read `8.0000` against its own `16.0000`, and **this violation then missed the fault at every
    /// leak under `5 cm`** — the difference it read grew as the square of the leak where the truth grows as
    /// four times it. [`DidNotExpand`](Self::DidNotExpand) still names the edge by name, so nothing is
    /// silent because of it.
    ///
    /// **A loop that leaves the outline is still reported, and the fault is measured on one.** A
    /// `5 x 4` rectangle with its east side `2 m` outside a `12 x 12` plan gives `12.0000` against
    /// `20.0000`, and a right triangle over the north-east corner gives `15.5000` against `24.5000` —
    /// both simple, both firing on the difference, both refused by the walk as well.
    ///
    /// **A pinch at a point its author wrote twice is not asked, and the reason is that
    /// [`crate::clip`] cannot measure it — not that it measured it correctly.** `self_meeting` is
    /// silent for it, so the subtraction used to be asked of it, and the sentence in the record that
    /// said the two areas "were measured equal (`9.0000` against `9.0000` for two `3 x 3` squares
    /// sharing a corner)" was true of the **subtraction and false of the clipper**: that loop encloses
    /// `18.0000`, [`crate::clip`] read `9.0000` for it, and both terms came out halved, so the check
    /// stayed silent by cancellation rather than by being right. Measured after the walk learned to say
    /// when it could not cover a loop, the same loop reads "cannot measure" and this declines it —
    /// **still silent, now for the reason it should be**, and the building it stands in is sound either
    /// way: `self_meeting`'s own argument for the silence is a claim about a wall, not about an area.
    VoidNotInside {
        /// The mass.
        mass: MassId,
        /// Which of its voids, in the order it authored them.
        void: usize,
    },
    /// A mass's void loop meets itself at a point its author did not write down, so it is not the
    /// simple loop a courtyard's ring is built from.
    ///
    /// # Invariants
    ///
    /// **This is the fault [`VoidNotInside`](Self::VoidNotInside) reported by accident and
    /// [`DidNotExpand`](Self::DidNotExpand) reports as arithmetic.** Before it, a crossing reached
    /// the author as a subtraction that happened to disagree with itself — a bowtie reads `8.0000`
    /// shared with the outline against `12.0000` shared with itself and fired, while the *same*
    /// bowtie stood on its side as an hourglass read `8.0000` against `8.0000` and said nothing —
    /// and a touch at an unauthored point reached them as `DidNotExpand` carrying
    /// [`Problem::MiterTooLong`], a mitre length rather than a shape. Both loops are refused either
    /// way, so this reports nothing a build refuses today; it says the sentence the reading deserves,
    /// **and [`VoidNotInside`](Self::VoidNotInside) no longer speaks for the same loop**, because the
    /// subtraction is exact only where this check has nothing to say.
    ///
    /// **A point two of the loop's own nodes stand at is not this violation, and that is measured
    /// rather than assumed.** Two `3 × 3` squares sharing exactly one corner name that corner twice,
    /// the loop is not simple by the textbook definition this check uses, and **the ring it builds is
    /// sound**: edge-manifold, no boundary loop, Euler `0`, and a volume of `24.1920 m³` against
    /// `(offset − loop) × height` to `2 × 10⁻⁶`. The two lobes meet along the whole cross-section of
    /// the pinch, which is why the characteristic is `0` and not `-1`. A loop the author pinched at a
    /// point they wrote down is a courtyard, and the walk carries it, so it is deliberately silent.
    ///
    /// **The reading is carried rather than a count**, because the author has to act on it and the two
    /// ways are different edits: a crossing is two edges out of order and a touch is one node in the
    /// wrong place.
    VoidNotSimple {
        /// The mass.
        mass: MassId,
        /// Which of its voids, in the order it authored them.
        void: usize,
        /// Where the loop meets itself, read off the authored loop.
        meeting: SelfMeeting,
    },
    /// Two of a mass's voids share plan area, so the hole they make is one hole with two descriptions.
    VoidOverlaps {
        /// The mass.
        mass: MassId,
        /// The lower-indexed void.
        first: usize,
        /// The other.
        second: usize,
    },
    /// A mass's void has no room for its own wall inside the mass's, so the two occupy the same space.
    ///
    /// # Invariants
    ///
    /// **A void inside the outline is not a courtyard that fits**, because the courtyard's ring stands
    /// `mass.wall` *outside* the void's loop: a void authored within a wall's width of the outline
    /// has its wall standing in the mass's own wall. Both rings would then read closed, edge-manifold,
    /// outward and of the right Euler characteristic — two sound solids in the same material — so no
    /// other check in this module can see it.
    ///
    /// `over` is the plan area of the courtyard's wall that falls outside the mass's inner loop, in
    /// square metres, and it is `0.000000` for a courtyard that fits. Both loops are the expander's
    /// own walks rather than a second derivation, so this cannot disagree with the drawing about
    /// where they are.
    VoidReachesTheWall {
        /// The mass.
        mass: MassId,
        /// Which of its voids.
        void: usize,
        /// How much of the courtyard's wall stands inside the mass's own wall, in plan area.
        over: f32,
    },
}

/// Reports what a skeleton has got wrong, without expanding it.
///
/// Answers for a skeleton that cannot be built, which is why it is the half an agent should call
/// while editing.
///
/// # The four checks here that are quadratic
///
/// [`Violation::OutlineNotSimple`] and [`Violation::VoidNotSimple`] compare every pair of one loop's
/// edges — the mass's own outline and one of its courtyards — [`Violation::UnattachedOverlap`]
/// compares every pair of one mass's plan with every other mass's: `O(n^2)` in the masses and `O(k^2)`
/// in the nodes of a pair. [`Violation::OpeningOverlaps`] compares every pair of holes in one wall. All
/// four are the right cost here and the wrong one anywhere else — this is called by an author asking
/// what is wrong, a few times a second at most, and **never** by `build`, which is why the expansion can
/// stay linear. `mountain`'s validate makes the same trade for its rib-crossing scan and states it in the
/// same terms. The mass one rejects a pair in `O(1)` on their bounding boxes before any edge is read, so
/// the quadratic part is only paid by masses whose plans really are near each other; the opening one walks
/// the walls rather than the document, so the pairs it compares are only the holes that share a wall.
///
/// Everything else here is a pass: the roof checks are `O(rows)` per plane, a plate's coverage is `O(k)`
/// in its host's outline, and the opening checks are `O(openings)` plus the quadratic above.
pub fn document(skeleton: &BuildingSkeleton) -> Vec<Violation> {
    let mut violations = Vec::new();
    check_masses(skeleton, &mut violations);
    check_roofs(skeleton, &mut violations);
    check_grids(skeleton, &mut violations);
    check_parts(skeleton, &mut violations);
    check_walls(skeleton, &mut violations);
    check_slabs(skeleton, &mut violations);
    check_openings(skeleton, &mut violations);
    violations.sort_by_key(order_key);
    violations
}

/// Reports what a built mesh has got wrong: closure, orientation, planarity, coverage.
///
/// # Invariants
///
/// Every solid is measured over **one** face set, gathered once and then handed to the four
/// predicates, so a solid costs `O(faces of that solid)` rather than the four passes
/// [`BuildingMesh::solid_boundary_loops`] and its three siblings would make of it. The four are
/// the same measurements either way — the `solid_*` forms are one-line wrappers over the `_of`
/// forms taken here.
///
/// The solids are the ones the mesh actually holds, read out of the faces' own provenance rather
/// than out of a count the mesh does not carry: a `BuildingMesh` has no idea how many masses or
/// roofs its author wrote, and asking the empty solids of a document would report four violations
/// about solids that were never there.
///
/// **It takes the skeleton as well as the mesh, for the coverage check and nothing else.**
/// Coverage is the one question here with nothing inside the mesh to measure against: the fascia
/// follows the **skin's own boundary** rather than the eave loop, so a mesh carries no record of
/// the loop its roof was meant to tile, and only the document does. Every other check reads the
/// mesh alone, and this is why the signature is not `mesh(&BuildingMesh)` — the alternative was a
/// second public half and a second call site, which is more surface for the same answer.
pub fn mesh(skeleton: &BuildingSkeleton, built: &BuildingMesh) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut solids: BTreeSet<SolidId> = BTreeSet::new();
    for face in &built.faces {
        solids.insert(face.element.solid());
    }
    for solid in solids {
        check_solid(skeleton, built, solid, &mut violations);
    }
    check_planarity(built, &mut violations);
    check_degenerate_faces(built, &mut violations);
    check_coverage(skeleton, built, &mut violations);
    violations.sort_by_key(order_key);
    violations
}

/// Checks the masses: enough nodes, the right winding, some height, a host that exists, and an
/// outline that does not meet itself.
///
/// # Invariants
///
/// **Every mass's outline is read once for its own meeting and that one reading is both the report
/// and the gate.** [`check_overlap`] asks [`crate::clip`] how much plan two masses share, and
/// [`crate::clip`] is exact only for a loop that can be triangulated; a mass outline that meets
/// itself has none, so it reached the subtraction with nothing in front of it and the answer was
/// whatever the ear clipper made of it — measured, `12.5000 m²` against a true `50.0000`, and
/// `0.0000` for the outline carrying an edge of no length. **Asking once and reading it twice is
/// what keeps the name and the gate from coming apart**: there is no second simplicity test to
/// disagree with the first, which is the same argument [`check_voids`] makes about
/// [`VoidNotInside`] and [`VoidNotSimple`].
///
/// **The same one-pass reading now answers the second question too, and it is the one this module's
/// whole refusal thread arrived at.** [`OutlineNotMeasured`](Violation::OutlineNotMeasured) is asked
/// exactly where [`OutlineNotSimple`](Violation::OutlineNotSimple) is not — the two are the
/// alternatives of one `match` on one [`OutlineReading`] — and the pair that a plan
/// [`crate::clip`] declines then carries both, because one is about the loop and the other is about
/// the comparison the loop could not take part in.
///
/// **An outline with fewer than three nodes is not asked, and needs no staging of its own**:
/// [`self_meeting`] walks pairs at two or more apart, so a loop that short has no pair to compare
/// and answers `None` by construction — the same `None` [`OutlineTooShort`](Violation::OutlineTooShort)
/// reports beside, which is staged because [`Mass::plan_area`] has no winding to sign.
///
/// **The cost is a fourth quadratic in this module**, `O(nodes²)` per mass, for the trade
/// [`document`]'s own docs state: it is called by an author asking what is wrong and never by
/// `build`.
fn check_masses(skeleton: &BuildingSkeleton, violations: &mut Vec<Violation>) {
    let readings: Vec<OutlineReading> = skeleton
        .masses
        .iter()
        .map(|mass| OutlineReading::of(&plan_of(&mass.outline)))
        .collect();
    for (index, mass) in skeleton.masses.iter().enumerate() {
        let id = MassId(index);
        // Staged: an outline with fewer than three nodes has no plan area to sign, and
        // `plan_area` answers 0.0 for it, which would be reported as a winding fault when it says
        // nothing about the winding at all. A zero area from three or more nodes is a different
        // fault and is reported below.
        if mass.outline.len() < 3 {
            violations.push(Violation::OutlineTooShort {
                mass: id,
                nodes: mass.outline.len(),
            });
        } else {
            let area = mass.plan_area();
            if area <= 0.0 {
                violations.push(Violation::OutlineReversed { mass: id, area });
            }
        }
        // **One reading, and the two names are the alternatives of one `match` on it.** An outline that
        // meets itself is named by `OutlineNotSimple`, and an outline that does not is asked whether it has a
        // triangulation at all — so a needle cut into this outline reads `12.5000` against a true `50.0000` when it
        // reaches the overlap subtraction, and reaches it silently, because nothing in `document` asked whether a
        // **mass's** outline is simple — only a void loop's was asked.
        match readings[index] {
            OutlineReading {
                meeting: Some(meeting),
                ..
            } => violations.push(Violation::OutlineNotSimple { mass: id, meeting }),
            OutlineReading {
                meeting: None,
                measurable: false,
            } => violations.push(Violation::OutlineNotMeasured { mass: id }),
            OutlineReading { .. } => {}
        }
        // Staged the same way: `highest` of an empty outline is not an elevation, it is a fold
        // over nothing that answers negative infinity. A crowned mass is measured by its **highest**
        // top, because a gable that rises to its roof is a wall and not a zero-height one.
        if !mass.outline.is_empty() {
            let highest = mass.highest();
            let top = mass.highest_top();
            if top <= highest {
                violations.push(Violation::ZeroHeight {
                    mass: id,
                    top,
                    highest,
                });
            }
        }
        // **A crown is one elevation per node, and the two counts are the statement.** A crown that is
        // neither empty nor the outline's length cannot be read node by node, so it is reported rather
        // than indexed past the end of.
        if !mass.crowns.is_empty() && mass.crowns.len() != mass.outline.len() {
            violations.push(Violation::CrownCountMismatch {
                mass: id,
                crowns: mass.crowns.len(),
                nodes: mass.outline.len(),
            });
        }
        // Every crown stands above the wall's own foot, or it is a wall that has been cut into its base.
        for (node, top) in mass.crowns.iter().enumerate() {
            let Some(node_at) = mass.outline.get(node) else {
                break;
            };
            if *top <= node_at.elevation {
                violations.push(Violation::CrownBelowBase {
                    mass: id,
                    node,
                    top: *top,
                    base: node_at.elevation,
                });
            }
        }
        if let Some(attachment) = mass.attachment {
            if skeleton.mass(attachment.host).is_none() {
                violations.push(Violation::MassHostMissing {
                    mass: id,
                    host: attachment.host,
                });
            }
        }
    }
    check_overlap(skeleton, violations, &readings);
    check_voids(skeleton, violations);
}

/// What one mass's own outline is, read once and read twice.
///
/// # Invariants
///
/// **Both fields come from one walk of one loop and are read by two callers**, which is the argument
/// [`check_masses`] makes about the meeting on its own: a report and the gate in front of it are one
/// answer or they are two tests of the same fact, and two tests drift.
///
/// **The second field is asked only where the first is `None`.** An outline that meets itself is
/// already named and [`check_overlap`] never asks about it, so there is nothing for a triangulation
/// to decide; the cost is paid once per mass rather than once per shape.
///
/// **`O(nodes²)` on both fields**, the same trade [`document`] states and never paid by `build`: the
/// meeting is a sweep of edge pairs and the triangulation is an ear walk with a coverage test over the
/// triangles it cut.
struct OutlineReading {
    /// Where the outline meets itself, or `None` when it does not.
    meeting: Option<SelfMeeting>,
    /// Whether the outline's own plan has a triangulation to measure with. **Only read when `meeting`
    /// is `None`**, and `true` for an outline of fewer than three nodes, which has no region whatever and is named
    /// by [`OutlineTooShort`](Violation::OutlineTooShort) beside.
    measurable: bool,
}

impl OutlineReading {
    /// Reads both halves of one outline's own reading, and asks [`crate::clip`] only where the first is silent.
    ///
    /// **The staging is the same one `self_meeting` gets for free, made explicit**: a loop too short to have a pair
    /// of edges two or more apart answers `None` by construction, so the second field is where the length is
    /// asked. Three nodes standing at one place do reach [`crate::clip`](crate::clip), and it answers `None` for
    /// them — which is a true statement about a loop with no region, reported beside the winding fault that names
    /// the same loop, rather than one name doing the work of two.
    fn of(outline: &[Vec2]) -> Self {
        match self_meeting(outline) {
            Some(meeting) => Self {
                meeting: Some(meeting),
                measurable: false,
            },
            None => Self {
                meeting: None,
                measurable: outline.len() < 3 || plan_is_measurable(outline),
            },
        }
    }
}

/// Reports every mass's voids that break the contract a courtyard is authored under.
///
/// # Invariants
///
/// **Five faults, and each is a statement about the document rather than about the drawing:**
///
/// - a void loop that **meets itself** ([`VoidNotSimple`](Self::VoidNotSimple)), which is a statement
///   about the loop's own shape;
/// - a void that is not strictly inside its mass's outline (the hole would open through the wall);
/// - two voids that share plan area (one hole described twice); and
/// - a void whose **wall** has no room inside the mass's own wall
///   ([`VoidReachesTheWall`](Self::VoidReachesTheWall)), which is the only one of the four that is
///   about the ring rather than about the loop.
///
/// **A void loop that is not convex is no longer one of them**, and what replaced it is narrower and
/// is stated by the walk that refuses it rather than by a shape class: a ring stands `wall` out of the
/// loop, so **a bite in the loop has to be wider than `2 × wall`** and every other reflex corner is
/// walked. [`expand::offset_loop`](crate::expand::offset_loop) is what says so, by name, and a loop it
/// refuses reaches the author through [`DidNotExpand`](Self::DidNotExpand).
///
/// **"Inside" is measured, not eyeballed, and only where the loop can be**: the area two plans share
/// must equal the void's own area, which is a computation [`crate::clip`] makes exact for a simple
/// loop. A void that crosses the outline shares less than it encloses. A void that **meets itself**
/// shares something else again, because a loop with no triangulation has no area for the clipper to be
/// exact about — so the subtraction is asked only where the simplicity reading above is `None`, and
/// the two are the same [`self_meeting`] answer rather than two tests of the same fact. See
/// [`VoidNotInside`](Self::VoidNotInside).
///
/// **The two loops a courtyard is built from are read through the same two calls the expander makes**
/// — [`expand::offset_loop`](crate::expand::offset_loop) for the mass's inner loop and for the void's
/// outward offset — so the document half cannot disagree with the drawing by carrying a second idea
/// of where "inside" is. Both are **staged**: a loop that will not offset is a fault the expander
/// already names through [`DidNotExpand`](Self::DidNotExpand), and a check that would read a loop
/// that does not exist is skipped rather than guessed at.
///
/// **The simplicity reading above is the one that is not staged, and it is not staged because it
/// needs nothing the walk has not already got.** It reads the authored nodes and compares them with
/// each other, so it answers for a loop that offsets, one that does not, and one that never reaches
/// the expander at all — which is the only way it can say what it says about a shape the walk would
/// have refused by a name about its own output.
fn check_voids(skeleton: &BuildingSkeleton, violations: &mut Vec<Violation>) {
    for (index, mass) in skeleton.masses.iter().enumerate() {
        let id = MassId(index);
        let outline: Vec<Vec2> = mass.outline.iter().map(|node| node.position).collect();
        let voids: Vec<Vec<Vec2>> = mass
            .voids
            .iter()
            .map(|loop_nodes| plan_of(loop_nodes))
            .collect();
        for (void, held) in voids.iter().enumerate() {
            let meeting = self_meeting(held);
            if let Some(meeting) = meeting {
                violations.push(Violation::VoidNotSimple {
                    mass: id,
                    void,
                    meeting,
                });
            }
            // **The area comparison is exact for a simple loop and for nothing else, so it is asked
            // where the loop is one.** `crate::clip` is a sum of triangle intersections and a loop
            // that meets itself has no triangulation for it to be exact about: a symmetric bowtie
            // gives `8.0000` against `12.0000` and the same bowtie stood on its side gives `8.0000`
            // against `8.0000`, so the subtraction reported the clipper's own disagreement and
            // nothing about the outline — **and it reported it for one of two identical shapes, which
            // is what a fault with no name does.** The predicate is the one above rather than a second
            // simplicity test, so the two readings cannot come apart: a loop that meets itself is
            // reported by [`Violation::VoidNotSimple`], which names the shape, and the subtraction is
            // not asked of it.
            //
            // **The one loop this leaves to the subtraction is a pinch at a point its author wrote
            // twice**, which `self_meeting` answers `None` for because it builds and is deliberately
            // silent. **It is declined rather than subtracted, and that is a correction to what this
            // comment used to say**: the two terms were measured "equal (`9.0000` against `9.0000`
            // for two `3 x 3` squares sharing a corner)" and that was true of the subtraction and
            // false of the clipper — the loop encloses `18.0000`, [`crate::clip`] read `9.0000` for
            // it, both terms were halved, and the check was silent by cancellation rather than by
            // being right. It now reads "cannot measure" and the check is not asked, which is the
            // same silence for the reason it should be.
            // **The one loop this leaves to the subtraction that is not an authored pinch is one carrying
            // an edge of no length**, and `crate::clip` now takes that edge out before it clips rather than
            // clipping around it — **measured rather than assumed**: a `4 x 4` void with one node written
            // down twice read `8.0000` against its own `16.0000` before that removal, and the subtraction
            // was then **wrong in the direction that hides faults**: it missed `VoidNotInside` at every leak
            // under `5 cm`, because the difference it read grew as the square of the leak where the truth
            // grows as four times it. The walk still names the edge itself
            // ([`Problem::OutlineEdgeDegenerate`](crate::expand::Problem::OutlineEdgeDegenerate)), so
            // nothing is silent because of this.
            // **A loop `crate::clip` cannot cover is declined, not subtracted.** `self_meeting` is the
            // gate above and `clip`'s own ear walk is the second gate, and they are not the same
            // reading: `self_meeting` is silent for a meeting at a point the author wrote down,
            // which is a needle that doubles back along its own line over the very node it repeats —
            // `64.0000 m²` read as `32.0000`, so the subtraction would be asked about `0.0000` for a
            // loop that had left the outline at every leak under `4 cm`. **`None` means this module
            // cannot measure, and a check that cannot measure does not report**, which loses nothing
            // because the expander refuses that loop by name and the refusal is already reported
            // through [`Violation::DidNotExpand`] carrying
            // [`Problem::MiterUnresolved`](crate::expand::Problem::MiterUnresolved): there is no
            // building here for a hole to have opened in.
            if meeting.is_none() {
                let (Some(enclosed), Some(own)) = (
                    crate::clip::shared_area(&outline, held),
                    crate::clip::shared_area(held, held),
                ) else {
                    continue;
                };
                if (enclosed - own).abs() > VOID_AREA_TOLERANCE {
                    violations.push(Violation::VoidNotInside { mass: id, void });
                }
            }
        }
        for first in 0..voids.len() {
            for second in (first + 1)..voids.len() {
                // **Declined for the reason [`Violation::VoidNotInside`] above is**, and read the same
                // way: a pair of loops `crate::clip` cannot cover shares no area this module can
                // stand behind, and reporting that as no overlap would be a guess.
                if matches!(
                    crate::clip::shared_area(&voids[first], &voids[second]),
                    Some(shared) if shared > VOID_AREA_TOLERANCE
                ) {
                    violations.push(Violation::VoidOverlaps {
                        mass: id,
                        first,
                        second,
                    });
                }
            }
        }
        check_void_walls(id, mass, &mut *violations);
    }
}

/// Returns where a loop meets itself at a point its author did not write down, and `None` when it
/// does not.
///
/// # Invariants
///
/// **A point two of the loop's own nodes stand at is a meeting this returns `None` for, and that is
/// the one judgement in the function.** Two edges of a closed loop may only share a point when that
/// point is an end of both — every other shared point is the loop meeting itself — and an end of both
/// means two of the loop's nodes were authored at one place. A loop pinched that way is refused by
/// nothing: two `3 × 3` squares sharing one corner expand to an edge-manifold solid with no boundary
/// loop, Euler `0`, and a volume of `24.1920 m³` against `(offset − loop) × height` to
/// `2 × 10⁻⁶`. Naming it would refuse a document that builds a sound courtyard, which is the one
/// thing a check here must not do.
///
/// **The reading is the first meeting in the loop's own order**, and the pairs are walked
/// `(left, right)` with `right ≥ left + 2` and `(0, n − 1)` left out — the same enumeration
/// [`expand::offset_loop`](crate::expand::offset_loop)'s own crossing sweep uses, so the pair this
/// names and the pair the walk refuses are the same pair of edges counted the same way.
///
/// **The touch is read before the crossing, and the reason is the two are one contact read two ways.**
/// A segment running *through* another's endpoint answers
/// [`expand::segments_cross_properly`](crate::expand::segments_cross_properly) whenever the angle is
/// not degenerate, and naming two edges for it would ask the reader to find which of the pair's four
/// ends is the point.
///
/// **`O(nodes²)` and it is read off every loop this module measures**, the outline of a mass and the
/// loop of a courtyard alike, which is why it is the predicate shared by
/// [`Violation::OutlineNotSimple`] and [`Violation::VoidNotSimple`] and why it is the fourth
/// quadratic here with [`Violation::UnattachedOverlap`] and [`Violation::OpeningOverlaps`], for the
/// same trade and the same reason: this is called by an author asking what is wrong and never by
/// `build`. A loop that meets itself needs no bounding-box rejection either, because the two that
/// would pay for it are the ones that get refused by the cheapest reading in the pair above.
///
/// **The tolerance is [`TOUCH_TOLERANCE`],** the millimetre the rest of this module's plan readings
/// use, and both of the tests it serves are read at it: a node that stands on an edge, and two nodes
/// that stand at one place. Reading them at one number is the point — a pinch its author wrote twice
/// has to be the same pinch for the touching test and the coincidence test, or a courtyard is refused
/// by a rounding artefact on one side of the comparison.
fn self_meeting(loop_points: &[Vec2]) -> Option<SelfMeeting> {
    let count = loop_points.len();
    for left in 0..count {
        for right in left + 2..count {
            if left == 0 && right == count - 1 {
                continue;
            }
            let (from_a, to_a) = (loop_points[left], loop_points[(left + 1) % count]);
            let (from_b, to_b) = (loop_points[right], loop_points[(right + 1) % count]);
            if let Some(node) = node_standing_on(from_b, to_b, from_a, to_a, right, count) {
                return Some(SelfMeeting::Touch { edge: left, node });
            }
            if let Some(node) = node_standing_on(from_a, to_a, from_b, to_b, left, count) {
                return Some(SelfMeeting::Touch { edge: right, node });
            }
            if crate::expand::segments_cross_properly(from_a, to_a, from_b, to_b)
                && interior_to_both(from_a, to_a, from_b, to_b)
            {
                return Some(SelfMeeting::Crossing { left, right });
            }
        }
    }
    None
}

/// Returns the index of a node standing in the interior of a segment, given that segment's own two
/// ends and the indices they answer to.
///
/// `None` when no end of the segment stands on it, and `None` for an end that is **also** one of the
/// other segment's own ends — that is the authored coincidence [`self_meeting`] is silent for, and it
/// is excluded here rather than by the caller so that the two readings cannot come apart.
fn node_standing_on(
    from: Vec2,
    to: Vec2,
    segment_from: Vec2,
    segment_to: Vec2,
    first: usize,
    count: usize,
) -> Option<usize> {
    for (node, end) in [first, (first + 1) % count].into_iter().zip([from, to]) {
        if distance_to_segment(end, segment_from, segment_to) > TOUCH_TOLERANCE {
            continue;
        }
        if !is_an_end_of(end, segment_from, segment_to) {
            return Some(node);
        }
    }
    None
}

/// Returns whether two segments can cross away from their ends, which is what a crossing is.
///
/// # Invariants
///
/// **A product of exactly zero is what makes the strict crossing test answer true where it should
/// not.** `segments_cross_properly` compares `(first > 0.0) != (second > 0.0)`, and a point lying
/// *on* the other segment's line gives `0.0`, which is "not above zero" and so straddles against a
/// positive product. Two edges that meet at one of their own ends therefore read as crossing, and
/// the meeting is then either the touch [`node_standing_on`] reports or the authored pinch this
/// check is silent for — never a crossing, which by definition is a contact in the interior of both.
///
/// **It costs four distance comparisons and no more**, so it is asked before the crossing test rather
/// than after: on a loop that is simple it is the whole cost, and on one that is not the answer is the
/// same either way.
fn interior_to_both(from_a: Vec2, to_a: Vec2, from_b: Vec2, to_b: Vec2) -> bool {
    !is_an_end_of(from_a, from_b, to_b)
        && !is_an_end_of(to_a, from_b, to_b)
        && !is_an_end_of(from_b, from_a, to_a)
        && !is_an_end_of(to_b, from_a, to_a)
}

/// Returns whether a plan point is one of a segment's two own ends, within [`TOUCH_TOLERANCE`].
fn is_an_end_of(point: Vec2, from: Vec2, to: Vec2) -> bool {
    distance(point, from) <= TOUCH_TOLERANCE || distance(point, to) <= TOUCH_TOLERANCE
}

/// Reports a courtyard whose wall has no room between it and the mass's own wall.
///
/// # Invariants
///
/// **It is the fifth courtyard fault and it is the only one about the ring.** A void loop inside
/// the outline is not enough: the courtyard's ring stands `mass.wall` *outside* that loop, so a
/// void authored within a wall's width of the outline has its wall standing in the mass's own wall.
/// Both solids would still read closed, edge-manifold, outward and of the right Euler characteristic
/// — two rings occupying the same material are two sound solids — so nothing else in this module can
/// see it, and the building would be drawn with two walls occupying one space.
///
/// `over` is the plan area of the courtyard's wall that falls outside the mass's inner loop, so it
/// answers in the units the author drew in and is `0.000000` for a courtyard that fits. It is a
/// **remainder rather than an intersection**, which is the whole difference between this check and a
/// misreading of it: a courtyard that fits is *entirely* inside the rooms, so the area the two loops
/// share is the courtyard's own area, and what has to be compared is that against what it encloses.
///
/// **Both loops come from the expander's own walk.** The mass's inner loop is the one
/// [`derive::floor_slab`](crate::derive::floor_slab) derives a plate from, and the void's offset is
/// the one the ring is built on, so this check cannot hold a different idea of either. A loop that
/// will not offset — which is a fault the mass's own expansion reports by name — skips the check
/// rather than reporting a second, vaguer one.
fn check_void_walls(id: MassId, mass: &Mass, violations: &mut Vec<Violation>) {
    if mass.voids.is_empty() {
        return;
    }
    let mut problems = Vec::new();
    let Some(inside) = crate::expand::offset_loop(
        &mass.outline,
        crate::mass::AuthoredLoop::Outline { mass: id },
        mass.wall,
        mass.plan_area(),
        crate::expand::OffsetSide::Inward,
        &mut problems,
    ) else {
        return;
    };
    for (void, loop_nodes) in mass.voids.iter().enumerate() {
        // **The void is normalised the way the expander normalises it**, because which side a walk
        // offsets to is read from the winding: the same loop reversed is the same courtyard.
        let mut walked = loop_nodes.clone();
        if crate::expand::loop_area(&plan_of(loop_nodes)) < 0.0 {
            walked.reverse();
        }
        let mut problems = Vec::new();
        let Some(outer) = crate::expand::offset_loop(
            &walked,
            crate::mass::AuthoredLoop::Void { mass: id, void },
            mass.wall,
            crate::expand::loop_area(&plan_of(&walked)),
            crate::expand::OffsetSide::Outward,
            &mut problems,
        ) else {
            continue;
        };
        // **"Fits" is the void's own measure, compared against the rooms** — the same form as
        // `VoidNotInside` above, which asks whether the loop is enclosed by the outline. A courtyard
        // that fits is enclosed by the mass's *inner* loop, so the two questions are one relation
        // read against two loops, and what is left over is exactly how much of the wall is outside.
        // **A courtyard it cannot measure is declined rather than reported at `0.0000`.** Both loops
        // here are the expander's own walks, so a loop `crate::clip` cannot cover is one this walk
        // has already refused — and the refusal is staged above, by `offset_loop` answering `None`
        // and the loop being skipped. The subtraction is staged the same way, for the same reason:
        // [`Violation::DidNotExpand`] carries the walk's own name for the loop, and a check that
        // reported a fault on a courtyard that cannot be built would be naming a second, vaguer one.
        let own = crate::clip::shared_area(&outer, &outer);
        let over = own.and_then(|own| {
            crate::clip::shared_area(&inside, &outer).map(|shared| (own - shared).max(0.0))
        });
        let Some(over) = over else {
            continue;
        };
        if over > VOID_AREA_TOLERANCE {
            violations.push(Violation::VoidReachesTheWall {
                mass: id,
                void,
                over,
            });
        }
    }
}

/// Returns a loop's plan positions, which is what every loop reading in this module takes.
///
/// `O(nodes)`, and it is a projection rather than a copy of the loop: the elevations stay where they
/// were authored and only the plan is handed to the walks that read plan.
fn plan_of(loop_nodes: &[PlanNode]) -> Vec<Vec2> {
    loop_nodes.iter().map(|node| node.position).collect()
}

/// Reports two masses whose plans share area with no attachment between them.
///
/// # Invariants
///
/// An attachment in **either** direction ends the question: a wing overlaps its host by
/// construction and the host's own outline is what the attachment's station splits, so a pair with
/// one is a named joint rather than a coincidence of coordinates. What is left is two shells whose
/// plans share area with nothing written down about how they meet, which is a pile of boxes and
/// not a building.
///
/// **A pair is not asked at all when either outline meets itself**, and that is the gate this check
/// never had. [`plans_share_area`] would answer `None` for such a pair — not because the two
/// masses are apart but because [`crate::clip`] cannot triangulate the outline, and **an answer
/// about a measurement is not an answer about the two masses**. The fault is already named beside
/// this call, by [`check_masses`], so skipping the pair costs the author nothing and stops a number
/// being read that could not have been.
///
/// **The pair that *is* asked gets all three answers, and the third is the point of this change.**
/// [`plans_share_area`] answers whether the two plans share area and `None` when this module cannot
/// compare them, and each is reported: [`Violation::UnattachedOverlap`] for the overlap,
/// [`Violation::OverlapNotMeasured`] for the refusal. The gate above cannot catch the second, because
/// the outline [`crate::clip`] declines is not the one [`self_meeting`] names — the authored pinch is
/// silent to both, and it was that silence which answered "no overlap" about two masses that really
/// do share `18.0000 m²`. **A reader that could not measure must not report that there is nothing
/// there**, so the refusal is a name rather than an absence.
///
/// The pairs are read in list order and each pair is reported once, with the lower index first, so
/// the list is the same whatever order the masses were found in.
///
/// `readings` is [`check_masses`]'s own reading, one entry per mass and in the same order, so the gate
/// and the payload's culprit cannot be a second pair of tests that drifts from the ones that report.
fn check_overlap(
    skeleton: &BuildingSkeleton,
    violations: &mut Vec<Violation>,
    readings: &[OutlineReading],
) {
    for first in 0..skeleton.masses.len() {
        for second in (first + 1)..skeleton.masses.len() {
            if attached_either_way(skeleton, first, second) {
                continue;
            }
            // **The gate, and the whole of it: an outline that meets itself has no triangulation, so
            // the shared area between it and anything is a number `crate::clip` cannot stand behind.**
            // [`Violation::OutlineNotSimple`] has already named the loop, so nothing is lost by not
            // asking — and everything is gained by not answering "no overlap" about two masses whose
            // plans were never compared.
            if readings[first].meeting.is_some() || readings[second].meeting.is_some() {
                continue;
            }
            match plans_share_area(
                &skeleton.masses[first].outline,
                &skeleton.masses[second].outline,
            ) {
                Some(true) => violations.push(Violation::UnattachedOverlap {
                    mass: MassId(first),
                    other: MassId(second),
                }),
                // The two plans were compared and share nothing, which is the one answer this check
                // has always given and the only one it is allowed to give silently.
                Some(false) => {}
                // **The refusal is named rather than inherited.** The pair was not compared, so the
                // overlap question is open rather than answered, and this is where that is said out
                // loud. The mass the name points at is **the one whose plan could not be compared**,
                // because that is the loop an author can do something about and the other is only
                // the neighbour it stands against; both unmeasurable is reported with the lower
                // index. The expander still names the loop itself through
                // [`Violation::DidNotExpand`], and [`check_masses`] names the loop itself through
                // [`Violation::OutlineNotMeasured`] — this is the pair's question and that is the loop's.
                None => {
                    let (mass, other) = if readings[first].measurable {
                        (second, first)
                    } else {
                        (first, second)
                    };
                    violations.push(Violation::OverlapNotMeasured {
                        mass: MassId(mass),
                        other: MassId(other),
                    });
                }
            }
        }
    }
}

/// Returns whether either of two masses names the other as its host.
fn attached_either_way(skeleton: &BuildingSkeleton, first: usize, second: usize) -> bool {
    let names = |index: usize, host: MassId| {
        skeleton.masses[index].attachment.map(|one| one.host) == Some(host)
    };
    names(first, MassId(second)) || names(second, MassId(first))
}

/// Returns whether a mass's own plan has a triangulation to measure with.
///
/// **It asks [`crate::clip`] about the loop against itself, which is the one question that answers it**,
/// and it is asked once per mass, from [`OutlineReading::of`], rather than once per refused pair — the
/// payload of [`Violation::OverlapNotMeasured`] reads the answer [`check_masses`] already holds, so the
/// mass it names cannot be a different loop from the one that has no triangulation.
///
/// **A loop of fewer than three nodes is not asked**, and never reaches it: such a loop has no region
/// whatever, [`plans_share_area`] has already answered [`Some(false)`] for it, and
/// [`Violation::OutlineTooShort`] names it.
///
/// `O(nodes²)`, like every other quadratic in this module, and paid once per mass whose outline the
/// meeting reading has nothing to say about.
fn plan_is_measurable(outline: &[Vec2]) -> bool {
    crate::clip::shared_area(outline, outline).is_some()
}

/// Returns whether two closed plan polygons share an area, or `None` when this module cannot
/// compare them.
///
/// # Invariants
///
/// The test is the standard one for simple polygons: **a vertex of one strictly inside the other,
/// or a pair of edges crossing in both their interiors.** That is complete — two simple polygons
/// with a shared area have an intersection whose boundary is made of pieces of both, and a vertex
/// of that region is either a crossing or an original corner that fell inside the other polygon.
///
/// Three details are what make it answer the question this module means:
///
/// - "Strictly". A corner standing **on** the other's edge is a touch, not an overlap, and the
///   margin is [`TOUCH_TOLERANCE`]. Two masses whose walls are flush share a plane and no area,
///   which is a joint to author rather than an overlap to report.
/// - **No assumption of convexity.** The separating-axis test is the cheaper answer for two boxes
///   and it is wrong here, because a building outline may be an L and an L has no single direction
///   that separates it from anything.
/// - **`None` is this function saying it cannot compare the two plans, and it is said rather than
///   returned as `false`.** The two are not the same statement: one is a measurement and the other
///   is an absence of one, and a caller that cannot tell them apart reports "these two share
///   nothing" about masses it never looked at. Measured, a needle cut into the north edge of a mass
///   sharing `50.0000 m²` with its neighbour read `12.5000` before [`crate::clip`] learned to decline,
///   an outline carrying an edge of no length read `0.0000`, and a mass outline pinched at an
///   authored point — which the gate in [`check_overlap`] is silent about by design — read `false`
///   about a true `18.0000 m²`. What a caller does with `None` is its own decision, and the one here
///   is to name it ([`Violation::OverlapNotMeasured`]) rather than to inherit the silence.
///
/// **`None` is not reached by a pair this function can answer, and the two rejections before it are
/// both answers.** A loop of fewer than three nodes encloses no area whatever it stands beside, and
/// bounding boxes that miss cannot make polygons that meet: both are `Some(false)` because both are
/// facts about the two plans rather than about this module's ability to measure them.
fn plans_share_area(first: &[PlanNode], second: &[PlanNode]) -> Option<bool> {
    if first.len() < 3 || second.len() < 3 {
        return Some(false);
    }
    let one: Vec<Vec2> = first.iter().map(|node| node.position).collect();
    let other: Vec<Vec2> = second.iter().map(|node| node.position).collect();
    if !boxes_overlap(&one, &other) {
        return Some(false);
    }
    // **The shared area, computed rather than inferred.** A pair of rectangles sharing a wall has collinear
    // edges and a corner on an edge, which containment and proper crossing both miss; clipping the two
    // polygons and measuring what overlaps misses nothing, and a flush pair answers exactly zero.
    crate::clip::shared_area(&one, &other).map(|shared| shared > AREA_TOLERANCE)
}
/// Returns whether the bounding boxes of two plan polygons share anything.
///
/// The one cheap rejection before any edge is read, and it is sound for the convex test it
/// replaces here: boxes that miss cannot make polygons that meet.
fn boxes_overlap(first: &[Vec2], second: &[Vec2]) -> bool {
    let (low, high) = extremes(first);
    let (other_low, other_high) = extremes(second);
    (0..2).all(|axis| low[axis] <= other_high[axis] && other_low[axis] <= high[axis])
}

/// Returns the lowest and highest corner of a plan polygon's bounding box.
fn extremes(polygon: &[Vec2]) -> ([f32; 2], [f32; 2]) {
    let mut low = [f32::INFINITY; 2];
    let mut high = [f32::NEG_INFINITY; 2];
    for point in polygon {
        for axis in 0..2 {
            low[axis] = low[axis].min(point[axis]);
            high[axis] = high[axis].max(point[axis]);
        }
    }
    (low, high)
}

/// Returns how far a plan point is from a plan segment.
fn distance_to_segment(point: Vec2, from: Vec2, to: Vec2) -> f32 {
    let edge = crate::math::difference(to, from);
    let length_squared = crate::math::dot(edge, edge);
    let amount = if length_squared > f32::EPSILON {
        crate::math::clamp(
            crate::math::dot(crate::math::difference(point, from), edge) / length_squared,
            0.0,
            1.0,
        )
    } else {
        0.0
    };
    distance(point, crate::math::lerp(from, to, amount))
}

/// Checks the roofs: a host that exists, an eave loop with length, planes that span, crests in
/// range.
///
/// # Invariants
///
/// Nothing here is staged against the other checks. A plane with one row still has a row whose
/// crest index can be read, and reading it is the more useful report: the author who wrote two
/// faults is told both, which is the contract of this module. What is staged is only the reads a
/// value cannot support, and here there are none — [`crate::roof::Roof::crest_point`] answers an
/// index off its list with `None` rather than by indexing.
fn check_roofs(skeleton: &BuildingSkeleton, violations: &mut Vec<Violation>) {
    for (index, roof) in skeleton.roofs.iter().enumerate() {
        let id = RoofId(index);
        if skeleton.mass(roof.host).is_none() {
            violations.push(Violation::RoofHostMissing {
                roof: id,
                host: roof.host,
            });
        }
        if roof.eaves.len() < 3 {
            violations.push(Violation::RoofEaveTooShort {
                roof: id,
                nodes: roof.eaves.len(),
            });
        }
        for (plane_index, plane) in roof.planes.iter().enumerate() {
            if plane.rows.len() < 2 {
                violations.push(Violation::RoofPlaneTooShort {
                    roof: id,
                    plane: plane_index,
                });
            }
            if let Some(fault) = roof.covering.and_then(|covering| covering.fault()) {
                violations.push(Violation::CoveringFault { roof: id, fault });
            }
            match roof.profile_fault(plane_index) {
                Some(crate::roof::ProfileFault::OutOfOrder { step }) => {
                    violations.push(Violation::ProfileOutOfOrder {
                        roof: id,
                        plane: plane_index,
                        step,
                    });
                }
                Some(crate::roof::ProfileFault::AboveTheCrest { step }) => {
                    violations.push(Violation::ProfileReachesTheCrest {
                        roof: id,
                        plane: plane_index,
                        step,
                    });
                }
                None => {}
            }
            for (row_index, row) in plane.rows.iter().enumerate() {
                if row.crest >= roof.crest.len() {
                    violations.push(Violation::RoofCrestMissing {
                        roof: id,
                        plane: plane_index,
                        row: row_index,
                        crest: row.crest,
                    });
                }
            }
        }
    }
}

/// Checks that a solid is closed, edge-manifold, outward, and of the Euler characteristic its holes
/// account for.
///
/// # Invariants
///
/// **The Euler characteristic is `2 − 2 × the holes through the solid`, not `2`.** A closed shell with
/// no hole in it is a sphere, and that is what this check said until walls could be cut through: an
/// opening is a handle through the material, so each one takes the characteristic down by two, and a
/// mass with three openings is a genus-3 surface reading `−4`. Reporting `2` there was the check
/// outliving its assumption — measured, on the openings slice's acceptance wall, where the geometry
/// had just been proved right (`loops 0`, manifold, volume exact) and this was the one violation left.
///
/// **An opening is that handle unless it stands on the wall's own floor**, which makes it a doorway
/// and a notch in the wall's section rather than a tunnel through it: it leaves a lintel, the ring is
/// still a ring, and the characteristic does not move. [`is_doorway`] is the document's own reading of
/// that, and the volume model's base notch is what makes it true — there is no material under a
/// doorway's sill for a tunnel to be cut from.
///
/// **A hole is counted from the document and only when it resolves**, and only for **the ring it was
/// cut into**. A mass's ring reads its own outline's openings and a courtyard's ring reads its void
/// loop's, which is what makes `2 − 2(1 + h)` a statement about one solid: the two rings are two
/// solids and each carries its own handles. An opening whose slot the grid no longer holds builds
/// nothing at all (that mass does not splice, and `DidNotExpand` says so), so counting it here would
/// name a shape the mesh does not have.
fn check_solid(
    skeleton: &BuildingSkeleton,
    built: &BuildingMesh,
    solid: SolidId,
    violations: &mut Vec<Violation>,
) {
    let faces = built.faces_of(solid);
    let loops = built.boundary_loops_of(&faces);
    if loops != 0 {
        violations.push(Violation::SolidOpen { solid, loops });
    }
    if !built.is_manifold_of(&faces) {
        violations.push(Violation::SolidNonManifold { solid });
    }
    let volume = built.signed_volume_of(&faces);
    if volume <= 0.0 {
        violations.push(Violation::SolidInsideOut { solid, volume });
    }
    let holes = match solid {
        SolidId::Mass(mass) => handles(skeleton, mass, None),
        // **A courtyard's ring is cut by whatever the document cut into its wall, and by nothing
        // else.** It is a ring by construction like the mass's own — which is why `ring` below counts
        // it — and a door in its wall is a handle through the ring's own thickness exactly as a
        // window in the mass's wall is a handle through the mass's.
        SolidId::Void { mass, void } => handles(skeleton, mass, Some(void)),
        // A part, a wall, a roof, a slab, a member and a roll are cut by nothing: a hole is authored
        // against a mass's own wall, a template's masses are its own private document, and a member's
        // box and a roll's prism are solid.
        SolidId::Roof(_)
        | SolidId::Part(_)
        | SolidId::Wall(_)
        | SolidId::Slab(_)
        | SolidId::Member(_)
        | SolidId::Roll(_) => 0,
    };
    // **A ring is one handle before any opening is cut.** The volume model makes a mass a closed tube: its
    // inner and outer surfaces meet at the wall top and again at the base ring, so the solid is a torus and a torus is
    // `2 - 2g = 0`. The shell it replaced was a box, because its floor disc capped the inner surface and left exactly
    // one join at the wall top. Measured on the shed: `-6` for a mass with three openings, which is `2 - 2(1 + 3)`.
    // A part's solid is a template's own mass, expanded the same way, so it is a ring too. **A courtyard's ring is a
    // ring the same way and for the same reason**: measured `0` on a 4 x 4 courtyard in a 0.3 m wall, and `-2` on the
    // same courtyard with one window cut through its wall, which is `2 - 2(1 + 1)` — the number this expectation has
    // to follow rather than assume, because a hole through a ring is what makes it not-a-ring. A roof, a wall and a
    // slab are boxes and are cut by nothing.
    // **A slab with a hole through it is a ring too**, and it is the one slab that is: the prism over an
    // annulus is a torus exactly as a mass's wall ring is, and a hole in a plate is the one place the
    // word "ring" reaches a slab. Every other slab is a box and is cut by nothing.
    let holed = matches!(solid, SolidId::Slab(id) if skeleton.slab(id).is_some_and(|slab| slab.void.is_some()));
    let ring = i64::from(
        matches!(
            solid,
            SolidId::Mass(_) | SolidId::Part(_) | SolidId::Void { .. }
        ) || holed,
    );
    let expected = 2 - 2 * (ring + holes as i64);
    let euler = built.euler_characteristic_of(&faces);
    if euler != expected {
        violations.push(Violation::SolidNotASphere { solid, euler });
    }
}

/// Counts the handles one ring carries: the openings cut into its own wall that resolve, less the
/// doorways, which are notches in its section rather than tunnels through it.
///
/// # Invariants
///
/// **The ring is named by its loop and not by its mass**, and the openings are read out of the
/// document's own grouping keyed by [`FaceRef`], so a window in the mass's outer wall is never counted
/// against a courtyard standing inside it. Both are the same facts [`crate::expand::resolve_holes`]
/// reads when it cuts, from the same place: a document's openings are one list, and a fault that was
/// reported twice under two ring identities would be a fault an author has to triage twice.
///
/// **Only the loop the walk cuts from counts, and that is the same one sentence as
/// [`OpeningOnAFarLoop`](Violation::OpeningOnAFarLoop).** A hole named on a ring's far loop is not
/// counted here because no cut is made from it, and counting one that the expander refused would put
/// a handle on a solid that has none — the false `SolidNotASphere` a document gets for reporting a
/// fault it was never built from.
///
/// `O(openings)` over the one group that names this ring, and each hole's own resolution is
/// `O(log faces)` through [`BuildingSkeleton::aperture`].
fn handles(skeleton: &BuildingSkeleton, mass: MassId, void: Option<usize>) -> usize {
    skeleton
        .openings
        .by_face()
        .iter()
        .filter(|(face, _)| {
            face.mass == mass && face.void == void && face.loop_side == LoopSide::Authored
        })
        .flat_map(|(_, indices)| indices.iter())
        .filter(|index| match skeleton.aperture(**index) {
            None => false,
            Some((face, extents)) => !is_doorway(skeleton, face, &extents),
        })
        .count()
}

/// Returns whether an opening stands on the wall's own floor, which is a doorway.
///
/// # Invariants
///
/// **A doorway is a notch in the wall's section and not a tunnel through it.** It leaves a lintel
/// above its head, so the wall's ring is still a ring and the solid's characteristic does not move —
/// where the ring's base annulus is notched across the opening, there is no material under the sill to
/// make a tunnel of it. This is the document-level form of the question the expander answers on its
/// welded indices, and the two agree because both lines are straight between the same two nodes.
///
/// **Both of the face's own nodes must stand at the sill**, which is the whole test: a ground line
/// that reaches the sill at one jamb and not the other crosses it, and the expander refuses that
/// before an opening is ever cut. An opening the expander refused does not reach here, so a sill
/// below the ground line is not a case this has to answer. The nodes are the **face's own loop's**,
/// which for a courtyard is its void loop rather than the mass's outline.
fn is_doorway(skeleton: &BuildingSkeleton, face: FaceRef, extents: &Extents) -> bool {
    let Some(nodes) = skeleton.face_loop(face) else {
        return false;
    };
    let count = nodes.len();
    if count < 3 || face.edge >= count {
        return false;
    }
    nodes[face.edge].elevation == extents.low
        && nodes[(face.edge + 1) % count].elevation == extents.low
}

/// Which of a solid's surfaces a coverage reading is taken over.
///
/// **The bucket is a roof's skin, and that is the whole of it.** A mass's plate covers an authored
/// loop too — its top stands over the outline — but a plate is [`Slab`](crate::slab::Slab)'s own
/// solid and its outline is a level polygon the expander derived from the same loop, so a disagreement
/// here would be the expander checked against itself rather than a reading across the two halves of
/// the crate. The shell's soffit is gone with the shell, and with it the mesh half of the mass's own
/// coverage fault; what replaced it, [`Violation::PlateCoverage`], is the **document half's**: it
/// compares the plate's authored polygon against its host's authored outline, so it is answered by
/// [`document`] and never reaches this gathering pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Coverage {
    /// A roof's skin, measured against the roof's eave loop.
    Skin(RoofId),
}

/// Checks that a roof's skin tiles its eave loop.
///
/// # Invariants
///
/// **One pass gathers every roof's skin, and the reading is a net area over a gathered set of
/// faces.** The
/// buckets are keyed by the surface rather than by the solid, so the faces of a roof's four planes
/// land in one bucket however they were expanded, and a solid contributes `O(faces of that solid)`
/// between the gathering and the arithmetic.
///
/// **The tolerance is a width times the loop's own length** ([`COVERAGE_TOLERANCE`]), so the same
/// millimetre is held at every size; the missing area is allowed to differ from the authored one by
/// [`COVERAGE_TOLERANCE`] × the loop's perimeter.
///
/// **Staged on the surface being there at all.** A roof that expanded no skin has no coverage to
/// read, and the reason it has no skin is a [`Problem`] the expansion already reported — a missing
/// crest, an unresolvable station — rather than a gap in the roof this check could name.
fn check_coverage(
    skeleton: &BuildingSkeleton,
    built: &BuildingMesh,
    violations: &mut Vec<Violation>,
) {
    let mut measured: BTreeMap<Coverage, Vec<usize>> = BTreeMap::new();
    for (index, face) in built.faces.iter().enumerate() {
        let surface = match face.element {
            Element::Roof(id, RoofSurface::Skin { .. }) => Coverage::Skin(id),
            _ => continue,
        };
        measured.entry(surface).or_default().push(index);
    }
    for (surface, faces) in measured {
        let covered = built.plan_area_of(&faces).abs();
        match surface {
            Coverage::Skin(id) => {
                let Some(roof) = skeleton.roof(id) else {
                    continue;
                };
                let eave = roof.plan_area().abs();
                // **The boundary is what tells a covering from a hole plus a skirt**, and it is the
                // only thing that can: the two net to the same area (a review built exactly that roof
                // and got an empty violation list). A covering skin's boundary lies on the eave loop;
                // a notch's inner vertices and a skirt's outer ones do not.
                let loops = built.boundary_loops_of(&faces);
                if loops != 1 {
                    violations.push(Violation::RoofSkinNotADisc { roof: id, loops });
                }
                let off = built
                    .boundary_edges_of(&faces)
                    .iter()
                    .flat_map(|(from, to)| [*from, *to])
                    .collect::<BTreeSet<u32>>()
                    .iter()
                    .filter(|index| !on_eave_loop(built.points[**index as usize], roof))
                    .count();
                if off > 0 {
                    violations.push(Violation::RoofSkinLeavesTheEave { roof: id, off });
                }
                if (covered - eave).abs() > COVERAGE_TOLERANCE * roof.eave_length() {
                    violations.push(Violation::RoofDoesNotCover {
                        roof: id,
                        skin: covered,
                        eave,
                    });
                }
            }
        }
    }
}

/// Returns whether a point stands on the eave loop, in plan, within [`COVERAGE_TOLERANCE`].
///
/// # Invariants
///
/// The distance is to the loop's **segments** and not to its nodes, because a covering skin's
/// boundary vertices are the row stations the author wrote, and a station lands mid-edge as often as
/// it lands on a corner. `O(nodes)` per point over a loop of four, and the points are the skin's
/// boundary vertices rather than all of its corners.
fn on_eave_loop(point: Vec3, roof: &Roof) -> bool {
    let count = roof.eaves.len();
    if count < 2 {
        return false;
    }
    let at = [point[0], point[2]];
    (0..count).any(|index| {
        let from = roof.eaves[index].position;
        let to = roof.eaves[(index + 1) % count].position;
        distance_to_segment(at, from, to) < COVERAGE_TOLERANCE
    })
}

/// Checks every face's rhythm: that it is ordered, that it spans the face it claims, and that it has
/// bays at all.
///
/// # Invariants
///
/// **A grid is measured against its face's own extents**, which is what makes "outside the face" a
/// statement about the document rather than about a convention: the levels must lie in `[the lowest
/// ground under the edge, the wall top]` and the divisions in `[0, the edge's length]`, both within
/// [`GRID_TOLERANCE`]. It is `O(faces with a rhythm)` and reads two nodes per face.
///
/// Staged: a rhythm on a face whose mass is not in the document is reported as that and nothing else,
/// because there is nothing to measure its stops against.
/// Returns the two wall tops a face's edge stands at, or `None` when the face is not in the document.
///
/// # Invariants
///
/// **The two ends are what a grid has to be a rectangle between**, which is a fact about the *edge* and not
/// about the mass: a crowned mass has level edges, and a level mass has none. A court's own ring takes the
/// crown its wall reads at the court's nodes, so its top is answered the same way and by the same reading
/// the ring is built from.
fn edge_crowns(skeleton: &BuildingSkeleton, face: crate::grid::FaceRef) -> Option<(f32, f32)> {
    let mass = skeleton.mass(face.mass)?;
    let loop_: &[crate::mass::PlanNode] = match face.void {
        None => &mass.outline,
        Some(void) => mass.voids.get(void)?,
    };
    let count = loop_.len();
    if count < 2 || face.edge >= count {
        return None;
    }
    let at = |index: usize| loop_[index % count];
    let (first, second) = (at(face.edge), at(face.edge + 1));
    let (low, high) = match face.void {
        None if mass.crowns.len() == count => {
            (mass.crowns[face.edge], mass.crowns[(face.edge + 1) % count])
        }
        None => (mass.top, mass.top),
        // A court's ring is crowned by the mass's own crown surface read at the court's nodes, which is
        // the same array `courtyard::ring` builds from.
        Some(void) => {
            let crowns = crate::courtyard::court_crowns(mass, void);
            match crowns.len() {
                length if length == count => (crowns[face.edge], crowns[(face.edge + 1) % count]),
                _ => (mass.top, mass.top),
            }
        }
    };
    let _ = (first, second);
    Some((low, high))
}

fn check_grids(skeleton: &BuildingSkeleton, violations: &mut Vec<Violation>) {
    for face in skeleton.grids.faces() {
        let Some(grid) = skeleton.grid(face) else {
            continue;
        };
        // **A crowned edge is a trapezoid and a rhythm divides a rectangle.** The levels a grid holds
        // are read against one wall top, and a crowned edge's top is a line with two different ends,
        // so a bay's sill and head would be two numbers about no single face. Reported by name rather
        // than cut into bays a part cannot stand in.
        //
        // **The question is the edge's own two ends and not its mass's crown vector.** A crowned mass has
        // edges that are not crowned — every wall between two corners at the same height, which is what a
        // court's four faces are — and refusing a grid on those refused the one face a 一颗印 is all
        // openings on.
        if let Some((low, high)) = edge_crowns(skeleton, face) {
            if (high - low).abs() > GRID_TOLERANCE {
                violations.push(Violation::GridOnACrownedFace { face });
                continue;
            }
        }
        let (Some(length), Some((base, top))) =
            (skeleton.face_length(face), skeleton.face_extent(face))
        else {
            violations.push(Violation::GridFaceMissing { face });
            continue;
        };
        if grid.levels.len() < 2 || grid.divisions.len() < 2 {
            violations.push(Violation::GridTooShort { face });
            continue;
        }
        if !increasing(&grid.levels) || !increasing(&grid.divisions) {
            violations.push(Violation::GridNotOrdered { face });
            continue;
        }
        let levels_span = (grid.levels[0] - base).abs() <= GRID_TOLERANCE
            && (grid.levels[grid.levels.len() - 1] - top).abs() <= GRID_TOLERANCE;
        let divisions_span = grid.divisions[0].abs() <= GRID_TOLERANCE
            && (grid.divisions[grid.divisions.len() - 1] - length).abs() <= GRID_TOLERANCE;
        if !levels_span || !divisions_span {
            violations.push(Violation::GridOutsideFace { face });
        }
    }
}

/// Checks every opening: that it names a bay, that its insets leave one inside that bay, that the
/// hole stays inside the wall face, and that no two holes in one wall share area.
///
/// # Invariants
///
/// **Three stages, and each stops where the next one could not read.** An opening whose slot is off
/// the grid has no bay to measure, and one whose insets eat its bay has no rectangle to place on the
/// wall, so the later checks skip them rather than guess — the module's staging rule, applied to the
/// three openings faults one after another. **A hole that does reach the wall's boundary is still
/// gathered for the overlap comparison**, because it removes wall and its position is still a
/// position; staging skips what cannot be read, not what is wrong.
///
/// **The rectangle is read the way [`BuildingSkeleton::aperture`] reads it — the document's own `bay`
/// and then [`Opening::extents`](crate::openings::Opening::extents)**, which are the two reads that
/// method composes. The expander cuts from `apertures_of` and this measures from here, so the two
/// cannot disagree about where a hole is: a second arithmetic written out again in this file would be
/// a second answer, and would drift from the first the moment the cut's rule changed. The bay is read
/// through the document for the same reason — a slot is a slot, and re-reading the grid here would be
/// a second way to be wrong about it.
///
/// `O(openings)` for the three, plus the pair scan [`check_shared_wall`] does within one wall.
fn check_openings(skeleton: &BuildingSkeleton, violations: &mut Vec<Violation>) {
    // The holes that have a rectangle, in the document's own order, which is what makes the pair
    // scan report the lower index first without comparing anything.
    let mut cut: Vec<(usize, Extents)> = Vec::new();
    for (index, opening) in skeleton.openings.iter().enumerate() {
        // **Staged first, and before the bay is asked for.** A hole on a wall's far loop is not a hole
        // in a thinner wall, it is not a hole this crate can cut: the ring's walk reads the loop the
        // author drew and cuts through both faces at once, so this rectangle has nowhere to be measured
        // *and* nothing would be cut if it were. Reporting it as anything else would send an author
        // looking for a dimension that is not the fault.
        if opening.face.loop_side != LoopSide::Authored {
            violations.push(Violation::OpeningOnAFarLoop {
                opening: index,
                face: opening.face,
            });
            continue;
        }
        let Some(bay) = skeleton.bay(opening.face, opening.row, opening.column) else {
            violations.push(Violation::OpeningBayMissing {
                opening: index,
                face: opening.face,
            });
            continue;
        };
        let Some(extents) = opening.extents(bay) else {
            violations.push(Violation::OpeningExceedsBay { opening: index });
            continue;
        };
        if reaches_the_wall_edge(skeleton, opening.face, &extents) {
            violations.push(Violation::OpeningOutsideWall { opening: index });
        }
        cut.push((index, extents));
    }
    check_shared_wall(skeleton, &cut, violations);
}

/// Returns whether a hole stands within [`GRID_TOLERANCE`] of the boundary of its wall face.
///
/// # Invariants
///
/// **All four edges of the strip, and a single verdict.** The strip runs `0..face_length` across and
/// the ground line to the wall top up (see [`Violation::OpeningOutsideWall`]), so a hole that reaches
/// any one of its four boundaries is one fault and not four: the fix is the opening, and an author
/// told four times would look for four mistakes.
///
/// **The four comparisons are strict, each with the tolerance outside**, so a hole that *touches* the
/// boundary fires and one that stands a millimetre clear does not — touching is the case the phase's
/// cut cannot express at all. The margin is read in metres on every side, which is why the same
/// [`GRID_TOLERANCE`] this module's grid checks use applies here: it is the authored-number rounding,
/// not a geometric nicety.
///
/// **Staged, and the skip is a skip.** A wall whose mass is not in the document, or whose edge has no
/// two ends to stand between, has no strip to measure against and answers `false` — not a fault, and
/// not a guess in the other direction either. Such a face is reported by the grid checks that can
/// read the reference, and by [`Violation::OpeningBayMissing`] for an opening that names it.
///
/// **The face's strip is the face's own loop's**, so a courtyard's wall is measured against the void
/// loop its bays were laid out on and not against the mass's outline, which is a different loop at a
/// different place entirely.
fn reaches_the_wall_edge(skeleton: &BuildingSkeleton, face: FaceRef, extents: &Extents) -> bool {
    let Some(mass) = skeleton.mass(face.mass) else {
        return false;
    };
    let Some(length) = skeleton.face_length(face) else {
        return false;
    };
    let Some(line) = ground_line(skeleton, face) else {
        return false;
    };
    if length <= GRID_TOLERANCE {
        return false;
    }
    // The extreme of a straight line over an interval is at one of its ends, so the lowest ground
    // under the hole is under one of its two jambs rather than under its middle.
    let floor = ground_at(line, length, extents.from).max(ground_at(line, length, extents.to));
    !(extents.from > GRID_TOLERANCE
        && extents.to < length - GRID_TOLERANCE
        && extents.high < mass.top - GRID_TOLERANCE
        && extents.low > floor + GRID_TOLERANCE)
}

/// Returns the elevation one wall face stands on at the two ends of its edge: the ground line.
///
/// # Invariants
///
/// **It is one expression for both kinds of wall, and a courtyard's is the mass's.** A courtyard's
/// ring stands on the same plate the mass's own ring stands on — the courtyard's floor *is* the
/// plate's top — so the underside of that plate is the same line under both, and a hole in either
/// wall is measured against it the same way. Reading a courtyard's line off its void loop's own
/// elevations instead would make a door in a courtyard wall illegal while the identical door in the
/// mass's wall is legal, for a difference that is in nothing but which loop the grid was laid out on.
///
/// The line is the plate's **underside**, one slab below the elevation the ring stands at, and that
/// is also what makes a doorway legal in both: a door's sill stands on the ring's own base, one slab
/// above this line, so the check has a whole slab of clearance where the expander has none. The
/// expander is the stricter of the pair in both cases — `resolve_one` measures against the loop's own
/// nodes — so this stays the loose half of a pair the mesh half already decides, rather than becoming
/// a second and disagreeing arithmetic of its own.
///
/// `None` when the face has no loop with two ends, which is the staged case [`reaches_the_wall_edge`]
/// skips.
fn ground_line(skeleton: &BuildingSkeleton, face: FaceRef) -> Option<(f32, f32)> {
    let mass = skeleton.mass(face.mass)?;
    let nodes = skeleton.face_loop(face)?;
    let count = nodes.len();
    if count < 3 || face.edge >= count {
        return None;
    }
    let from = nodes[face.edge].elevation - mass.slab;
    let to = nodes[(face.edge + 1) % count].elevation - mass.slab;
    Some((from, to))
}

/// Returns the ground line's elevation at one arc-length station along the edge, which is `O(1)` and
/// a linear interpolation because the line between two nodes is straight.
fn ground_at(line: (f32, f32), length: f32, station: f32) -> f32 {
    let amount = crate::math::clamp(station / length, 0.0, 1.0);
    line.0 + (line.1 - line.0) * amount
}

/// Reports every pair of holes in one wall that share area.
///
/// # Invariants
///
/// **The walls are the loop, and the document is not.** [`Openings::by_face`] is the lookup written
/// for this walk — one pass that groups by wall — so a facade with openings on two of its forty walls
/// pays for those two rather than comparing the whole document per wall. Within a wall the pairs are
/// read out of the group, which the same lookup fills in the document's own order, so the lower index
/// comes first without a comparison of index against index.
///
/// `O(k^2)` in the holes of one wall, `O(openings log openings)` outside it. The quadratic is the
/// price of a pairwise area test and is paid only here; it is the trade [`document`] makes and states
/// for the mass overlap check too, and it is never paid by `build`.
fn check_shared_wall(
    skeleton: &BuildingSkeleton,
    cut: &[(usize, Extents)],
    violations: &mut Vec<Violation>,
) {
    for (_, indices) in skeleton.openings.by_face() {
        for first in 0..indices.len() {
            for second in (first + 1)..indices.len() {
                let (Some((index, left)), Some((other, right))) =
                    (hole_of(cut, indices[first]), hole_of(cut, indices[second]))
                else {
                    continue;
                };
                if left.overlaps(&right) {
                    violations.push(Violation::OpeningOverlaps {
                        opening: index,
                        other,
                    });
                }
            }
        }
    }
}

/// Returns one hole's rectangle by the opening's index, or `None` when it has none — a slot off the
/// grid or insets that ate the bay, neither of which has an area to share.
///
/// `O(log openings)`, over a list that is in index order because the caller walked the document in
/// order.
fn hole_of(cut: &[(usize, Extents)], index: usize) -> Option<(usize, Extents)> {
    let at = cut.binary_search_by_key(&index, |(held, _)| *held).ok()?;
    Some(cut[at])
}

/// Checks every placement: that its template is there, that its bay is there, and that what it holds
/// fits.
///
/// # Invariants
///
/// **The containment reading is in the bay's own coordinates**, which is what makes it `O(1)` and what
/// makes it independent of the frame the part will be built in: [`Placement::reach`] has already moved
/// the template's box to the bay's corner, so this compares four numbers against the bay's two extents
/// and reports how far past the worst one stands.
///
/// Staged: a placement whose template is missing is reported as that and nothing else, because there
/// is no box to compare; a template with no mass answers `None` from `reach` and is not this check's
/// business at all.
fn check_parts(skeleton: &BuildingSkeleton, violations: &mut Vec<Violation>) {
    for (index, placement) in skeleton.parts.placements().iter().enumerate() {
        let Some(template) = skeleton.parts.template(placement.template) else {
            violations.push(Violation::PlacementTemplateMissing { placement: index });
            continue;
        };
        // **What the part stands on decides what is measured.** A bay is a rectangle and is measured against
        // its width and height; a wall run is a line and is measured against its length — and, because a part
        // hung on a wall may reach past it, against the **pin** alone rather than the part's box.
        let (bay, run) = match placement.target {
            Target::Bay { face, row, column } => match skeleton.bay(face, row, column) {
                Some(bay) => (Some(bay), None),
                None => {
                    violations.push(Violation::PlacementBayMissing {
                        placement: index,
                        face,
                    });
                    continue;
                }
            },
            Target::Wall { wall } => match skeleton.walls.get(wall.0) {
                Some(run) => (None, Some((wall, run))),
                None => {
                    violations.push(Violation::PlacementWallMissing {
                        placement: index,
                        wall,
                    });
                    continue;
                }
            },
        };
        // A mounted part is resolved through its joint before anything is measured: a socket the
        // template does not name and an anchor naming an opening in another wall are faults of the
        // document, and neither has a box to compare until it is fixed.
        if let Some(joint) = &placement.joint {
            let Some(bay) = bay else {
                // Every anchor names a feature of a bay, so a mount on a wall run has nothing to resolve
                // against — reported by name rather than silently treated as a free placement.
                violations.push(Violation::PlacementMountOnAWall { placement: index });
                continue;
            };
            if template.socket(&joint.socket).is_none() {
                violations.push(Violation::PlacementSocketMissing {
                    placement: index,
                    socket: joint.socket.clone(),
                });
                continue;
            }
            let Target::Bay { face, .. } = placement.target else {
                continue;
            };
            if skeleton.anchor(face, bay, joint.anchor).is_none() {
                violations.push(Violation::PlacementAnchorMissing { placement: index });
                continue;
            }
        }
        // **A courtyard's bay is measured exactly as a mass's is.** It used to be reported here
        // before anything else was asked of it, because the expander declined to build the frame;
        // the frame is now read off the ring's own offset side and the only reading left to make is
        // the one every other bay makes.
        let Some(at) = skeleton.pin_of(placement) else {
            violations.push(Violation::PlacementAnchorMissing { placement: index });
            continue;
        };
        if let Some((wall, run)) = run {
            // The **pin**, not the box: a gate leaf is hung at a wall's end and closes the gap beyond it.
            let past = (-at.along).max(at.along - run.length()).max(-at.up);
            if past > GRID_TOLERANCE {
                violations.push(Violation::PlacementOffItsWall {
                    placement: index,
                    wall,
                    past,
                });
            }
            continue;
        }
        let Some(bay) = bay else {
            continue;
        };
        let Some(reach) = placement.reach(template, at) else {
            continue;
        };
        let past = (-reach.x.0)
            .max(reach.x.1 - bay.width())
            .max(-reach.y.0)
            .max(reach.y.1 - bay.height());
        if past > GRID_TOLERANCE {
            violations.push(Violation::PlacementOutsideBay {
                placement: index,
                past,
            });
        }
    }
}

/// Checks every wall: that it has a run, a thickness, and a top above the ground it crosses.
///
/// # Invariants
///
/// It mirrors the expander's own three refusals rather than deferring to them, so an author hears about
/// a wall that cannot be built before a mesh is asked for — the same pairing the openings' checks have
/// with their cutting. `O(walls)`, one comparison each.
fn check_walls(skeleton: &BuildingSkeleton, violations: &mut Vec<Violation>) {
    for (index, wall) in skeleton.walls.iter().enumerate() {
        let id = crate::wall::WallId(index);
        if wall.length() <= GRID_TOLERANCE {
            violations.push(Violation::WallTooShort {
                wall: id,
                length: wall.length(),
            });
            continue;
        }
        if wall.thickness <= GRID_TOLERANCE {
            violations.push(Violation::WallNotThick {
                wall: id,
                thickness: wall.thickness,
            });
            continue;
        }
        if wall.top <= wall.highest_ground() + GRID_TOLERANCE {
            violations.push(Violation::WallTooLow {
                wall: id,
                top: wall.top,
                highest: wall.highest_ground(),
            });
        }
    }
}

/// Checks every slab: that its outline is a polygon, its two elevations are the right way round, that
/// the mass it names is one the document holds, and a plate reaches the mass it stands under.
///
/// # Invariants
///
/// It mirrors the expander's own two refusals rather than deferring to them, so an author hears about a
/// slab that cannot be built before a mesh is asked for — the same pairing the walls' checks have.
/// The host is **the one check here that reads no geometry at all**, and it is reported first for the
/// reason [`check_roofs`] reports a roof's host first: a reference that names nothing is a fault in its
/// own right and it does not stage out anything below it, since every other question about the slab is
/// about the slab alone. `O(slabs)`, one comparison each, plus `O(nodes)` for a plate's host outline.
fn check_slabs(skeleton: &BuildingSkeleton, violations: &mut Vec<Violation>) {
    for (index, slab) in skeleton.slabs.iter().enumerate() {
        let id = crate::slab::SlabId(index);
        // **Read before the outline, and not staged by it.** The role is the whole of what is reported
        // and it is already on the slab, so this is one lookup per hosted slab in whatever role it was
        // built in; an unhoused slab is a step, a plinth or a canopy and is skipped by the `Some` itself.
        if let Some(host) = slab.host {
            if skeleton.mass(host).is_none() {
                violations.push(Violation::SlabHostMissing {
                    slab: id,
                    host,
                    role: slab.role,
                });
            }
        }
        if slab.outline.len() < 3 {
            violations.push(Violation::SlabNotAPolygon {
                slab: id,
                nodes: slab.outline.len(),
            });
            // **Staged rather than measured on**: a loop of two nodes encloses nothing, so its plan
            // area is `0.0` whatever the mass it stands under is, and a shortfall of the whole
            // outline would be the answer to a question the line above has already answered.
            continue;
        }
        if slab.thickness() <= GRID_TOLERANCE {
            violations.push(Violation::SlabNotThick {
                slab: id,
                thickness: slab.thickness(),
            });
        }
        check_plate_coverage(skeleton, slab, id, violations);
    }
}

/// Reads one hosted slab against the mass it stands under, when that slab is the mass's plate.
///
/// # Invariants
///
/// **It is staged four times and each stage has a reading of its own already made elsewhere**,
/// which is what keeps the fault this names about the plate an author drew:
///
/// - **A slab that is not a plate is not this fault.** Its [`SlabRole`](crate::slab::SlabRole) is
///   `Floor` or `Roof`, and it covers less than the mass's floor on purpose: a storey is inset by the
///   walls it meets and a roof hangs over the eave. The role is read first because it is the only one
///   of the four stages that needs no other value to answer.
/// - **No host is not this fault.** An unhoused plate is a step or a canopy standing on its own,
///   which is what [`Slab::host`](crate::slab::Slab::host) documents it to be.
/// - **A host the document does not hold is not this fault either.** There is no outline to measure
///   against, and the crate answers a reference that names nothing from its own reader — the mass
///   half's [`Violation::MassHostMissing`] is about a mass's host, and a slab's is a different
///   question this check leaves alone.
/// - **A host with fewer than three outline nodes has no plan area to compare**, and
///   [`Violation::OutlineTooShort`] has already said so.
///
/// **The floor a ground plate has to reach is the host's inner loop**, because the wall ring owns the
/// annulus outside it and stands on the ground line across that annulus — the same loop
/// [`derive::floor_slab`](crate::derive::floor_slab) seats a first floor on, read through the one walk
/// [`derive::inner_loop`](crate::derive::inner_loop) both of them use.
///
/// **The areas are compared as magnitudes**, for the reason [`Violation::RoofDoesNotCover`] says it
/// compares a skin's area that way: coverage asks how much of the loop is covered, and the sign that
/// would say otherwise is the winding's business, reported by its own check.
fn check_plate_coverage(
    skeleton: &BuildingSkeleton,
    slab: &crate::slab::Slab,
    id: crate::slab::SlabId,
    violations: &mut Vec<Violation>,
) {
    let Some(host) = slab.host else {
        return;
    };
    let Some(mass) = skeleton.mass(host) else {
        return;
    };
    if mass.outline.len() < 3 {
        return;
    }
    // **A role selects the plate, and a role is authored.** A floor and a flat roof are hosted slabs
    // that cover less than the floor by design — the one is inset by the walls it meets and the other
    // sits over the eave — so the elevation rule this replaced had to carry a tolerance wide enough to
    // admit the elevations a plate really is authored at, and a floor a millimetre above the ground
    // came through it. Nothing here reads a level: what a slab *is* is on the slab.
    if slab.role != crate::slab::SlabRole::Plate {
        return;
    }
    // **A ground plate is the mass's floor, and a mass's floor is its inner loop** — the wall ring owns
    // the annulus between the outline and that loop and stands on the ground line across it, so the
    // room is what the plate has to reach and nothing beyond. A wall the miter refuses has no inside to
    // measure against, and its expander is already reporting that, so the outline is what such a mass
    // is held to.
    let (required, perimeter) = match crate::derive::inner_loop(mass, host) {
        Some(inside) => (
            crate::expand::loop_area(&inside).abs(),
            plan_length(&inside),
        ),
        None => (mass.plan_area().abs(), outline_length(mass)),
    };
    let plate = slab.plan_area().abs();
    let short = required - plate;
    if short > COVERAGE_TOLERANCE * perimeter {
        violations.push(Violation::PlateCoverage {
            mass: host,
            slab: id,
            plate,
            outline: required,
            short,
        });
    }
}

/// Returns the total plan length of a mass's closed outline.
///
/// # Invariants
///
/// **It is the outline's own polyline and not its bounding box**, which is what makes the tolerance
/// [`COVERAGE_TOLERANCE`] converts mean the same millimetre here as it does on a roof: a gap of `w`
/// metres along an edge `L` long is an area of `w·L`, and a box would turn a millimetre of slip on
/// an L-shaped outline into an allowance several times the area the slip costs. `O(nodes)`, once per
/// plate that reaches the check.
fn outline_length(mass: &Mass) -> f32 {
    plan_length(
        &mass
            .outline
            .iter()
            .map(|node| node.position)
            .collect::<Vec<_>>(),
    )
}

/// Returns the length of a closed plan loop, which is the one thing both the outline and the inner
/// loop a plate is measured over are asked for.
///
/// `O(nodes)`, and the same walk either loop's tolerance is.
fn plan_length(loop_points: &[Vec2]) -> f32 {
    let count = loop_points.len();
    (0..count)
        .map(|index| distance(loop_points[index], loop_points[(index + 1) % count]))
        .sum()
}

/// Checks that no facet has no area.
///
/// # Invariants
///
/// **One pass over the faces, `O(F)`, and one cross product each.** The area is read from the
/// welded corners rather than from any quantity the mesh already carries: `normal` is a
/// *normalized* normal and normalizing divides the area away, which is exactly the number this
/// needs.
fn check_degenerate_faces(built: &BuildingMesh, violations: &mut Vec<Violation>) {
    for (index, face) in built.faces.iter().enumerate() {
        let area = area_of(built, face);
        if area < DEGENERATE_AREA && corners_coincide(built, face) {
            violations.push(Violation::FaceDegenerate { face: index, area });
        }
    }
}

/// Returns whether two of a face's corners stand at one point.
///
/// # Invariants
///
/// This is the discriminator between the two classes of a facet of no area, and it reads positions
/// because the areas are both zero: a **coincidence** is a point two elements each authored at one
/// place, and it is exact — the square's collapsed ridge stood two crest points at *the same*
/// coordinates — so the tolerance only has to clear `f32` rounding at building scale. Three distinct
/// collinear corners are the structural case a split host needs, and they measure above it.
fn corners_coincide(built: &BuildingMesh, face: &crate::mesh::BuildingFace) -> bool {
    let [a, b, c] = built.triangle(face);
    [(a, b), (b, c), (c, a)].iter().any(|(from, to)| {
        let delta = subtract(*to, *from);
        dot3(delta, delta).sqrt() < COINCIDENT_TOLERANCE
    })
}

/// Returns the area a face covers in space, from the cross product of two of its edges.
fn area_of(built: &BuildingMesh, face: &crate::mesh::BuildingFace) -> f32 {
    let [a, b, c] = built.triangle(face);
    let normal = crate::math::cross3(subtract(b, a), subtract(c, a));
    0.5 * (dot3(normal, normal)).sqrt()
}

/// Checks that every element's facets lie in one plane.
///
/// # Invariants
///
/// The grouping key is the face's [`Element`], which is the authored surface: one wall face, one
/// roof plane. That is the surface the contract makes planar, and grouping by it is what turns
/// "one plane" from a statement about a triangle — always true — into a statement about the thing
/// an author wrote down.
///
/// The plane is the group's own **least squares** plane: the direction its covariance's smallest
/// eigenvalue stands in, through the mean of its distinct vertices. A genuinely planar group
/// therefore has a departure of zero to within rounding, which is the floor
/// [`PLANARITY_TOLERANCE`] has to stay above.
///
/// The fit is a least squares fit and not a mean of the group's triangle normals, and the reason is
/// a property of the measurement rather than a preference: on the reference cube with one corner
/// lifted `0.25` m the fitted plane reads `0.0626`, and the mean of the group's normals reads about
/// as much again — both are statements about their own plane.
///
/// **What the reading is, stated exactly: the distance from the group's farthest vertex to the plane
/// that best fits the group, which is not the distance that vertex stands off the plane its
/// neighbours lie in.** With four corners, one of them `0.25` m off, the best plane tilts to share
/// the error, so the reading is a fraction of the displacement and the fraction belongs to the
/// group's shape rather than to the fault. Measured: a sloped quad of four corners with one corner
/// lifted `0.2500` m along the plane's normal reads `0.0833` — exactly a third — while the same
/// group with its corners exactly coplanar reads nothing at all over tolerance. The cube's wall top,
/// a `2 × 2` corner whose vertices all satisfy `x + z = 2`, is worse still at `0.0626`, because its
/// design matrix is near singular and the plane it finds is one of a family of equally good ones.
///
/// The check is a detector and that is all it claims: a corner a factor of three off still reads
/// eighty-three times over [`PLANARITY_TOLERANCE`], and a genuinely planar group reads zero to within
/// rounding. **Reporting the displacement itself needs a different instrument** — three vertices
/// picked and the triangle they name — and that is a change to what this check means, for whoever
/// wants it, rather than a repair to make quietly here.
fn check_planarity(built: &BuildingMesh, violations: &mut Vec<Violation>) {
    let mut groups: HashMap<Element, Vec<usize>> = HashMap::new();
    for (index, face) in built.faces.iter().enumerate() {
        groups.entry(face.element).or_default().push(index);
    }
    for faces in groups.into_values() {
        let Some(departure) = departure_of(built, &faces) else {
            continue;
        };
        if departure > PLANARITY_TOLERANCE {
            violations.push(Violation::FaceNotPlanar {
                face: faces[0],
                departure,
            });
        }
    }
}

/// Returns how far an element's farthest vertex stands off the element's own fitted plane, or
/// `None` when its vertices do not span a plane at all.
///
/// # Invariants
///
/// The vertices are the group's **distinct welded indices**, taken once each. A vertex at the
/// corner of a fan belongs to several of the group's triangles and weighting it by how many would
/// weight it by the shape of the triangulation rather than by the surface the author wrote, and
/// the surface — not the cut of it — is what has to be planar.
fn departure_of(built: &BuildingMesh, faces: &[usize]) -> Option<f32> {
    let mut vertices: BTreeSet<u32> = BTreeSet::new();
    for index in faces {
        for corner in built.faces[*index].indices {
            vertices.insert(corner);
        }
    }
    if vertices.len() < 3 {
        return None;
    }
    let count = vertices.len() as f32;
    let mean = [
        axis_mean(&vertices, built, 0) / count,
        axis_mean(&vertices, built, 1) / count,
        axis_mean(&vertices, built, 2) / count,
    ];
    let mut covariance = [[0.0f32; 3]; 3];
    for corner in &vertices {
        let offset = subtract(built.points[*corner as usize], mean);
        for row in 0..3 {
            for column in 0..3 {
                covariance[row][column] += offset[row] * offset[column];
            }
        }
    }
    let (unit, eigenvalues) = smallest_direction(&covariance);
    // Rank one is a line rather than a plane, so there is nothing for a vertex to stand off from.
    // Rank two is the ordinary case: a plane's own covariance is singular, and the zero direction
    // is exactly the normal being looked for.
    if eigenvalues[1] <= eigenvalues[2] * 1.0e-6 {
        return None;
    }
    let mut departure: f32 = 0.0;
    for corner in vertices {
        let offset = subtract(built.points[corner as usize], mean);
        departure = departure.max(dot3(offset, unit).abs());
    }
    Some(departure)
}

/// Returns the mean of one axis over a set of points.
fn axis_mean(vertices: &BTreeSet<u32>, built: &BuildingMesh, axis: usize) -> f32 {
    vertices
        .iter()
        .map(|corner| built.points[*corner as usize][axis])
        .sum::<f32>()
}

/// Returns the unit direction of a symmetric matrix's smallest eigenvalue, and its eigenvalues in
/// ascending order.
///
/// # Invariants
///
/// Cyclic Jacobi: each step zeroes the largest off-diagonal entry with a rotation in the plane it
/// lives in. It is the classic way of reading a plane's normal out of a point set without a closed
/// form, it converges in a handful of steps at this size, and it is **bounded anyway** — a run that
/// has not converged returns the best reading it has, and the caller's tolerance is in metres, not
/// in eigenvalues.
///
/// The rotations are accumulated into the columns of an identity, which becomes the matrix of
/// eigenvectors. That accumulator is not an optimisation: the steps destroy the matrix they
/// diagonalize, so after one step the entry that was rotated holds a zero rather than the direction
/// that eigenvalue stands in, and reading the eigenvector out of it gives the null vector.
///
/// **One angle is read per step and applied to both matrices.** Reading the angle off the
/// accumulator instead divides by its off-diagonal, which is zero — the accumulator starts as the
/// identity — and the angle comes out as a `NaN` that `normalized3` turns into a plausible-looking
/// `[0, 1, 0]` rather than into a failure.
///
/// **The step is `Jᵀ A J` and the accumulator is `Q J`, never `J A J` or `J Jᵀ`.** Those two wrong
/// spellings look right — a rotation applied to both sides of a symmetric matrix is still a
/// similarity in appearance — and they are invisible for the reason that matters: the caller zeroes
/// the entry the step was after, so a step that does not zero it is corrected by the next line, and
/// what is left wrong is the **accumulator**, which no line corrects. `J Jᵀ` is the identity, so
/// every step returned the accumulator to the identity and the fitted "normal" was a raw coordinate
/// axis: the departure a genuinely planar group reported was its extent along that axis, not its
/// distance off a plane. Measured on this tip before the fix, on the hip starter's plane 0 — a
/// trapezoid between two parallel lines, four vertices exactly coplanar — the fit reported
/// `departure = 0.8000002`, which is the `0.8` m the crest stands above the eave and nothing about
/// planarity, and it reported the same for the soffit of the very solid the census calls clean.
/// Three of the roof's four planes came out "not planar" on geometry that is planar to the digit.
fn smallest_direction(matrix: &[[f32; 3]; 3]) -> (Vec3, [f32; 3]) {
    let mut value = *matrix;
    let mut vectors = [[0.0f32; 3]; 3];
    for (index, column) in vectors.iter_mut().enumerate() {
        column[index] = 1.0;
    }
    let scale = value
        .iter()
        .flatten()
        .fold(0.0f32, |largest, entry| largest.max(entry.abs()));
    for _ in 0..16 {
        let (row, column) = largest_off_diagonal(&value);
        if value[row][column].abs() <= scale * 1.0e-9 {
            break;
        }
        let (cosine, sine) = rotation_of(&value, row, column);
        turn(&mut value, row, column, cosine, sine);
        // The rotation zeroes these analytically; the remainder is rounding, and leaving it would
        // let the next sweep find an entry that is not there.
        value[row][column] = 0.0;
        value[column][row] = 0.0;
        // `Q` takes the columns half alone: the direction for an eigenvalue is a **column** of it,
        // so the rows half belongs to the matrix being diagonalized and to nothing else.
        turn_columns(&mut vectors, row, column, cosine, sine);
    }
    let mut eigenvalues = [value[0][0], value[1][1], value[2][2]];
    let mut smallest = 0;
    for index in 1..3 {
        if eigenvalues[index] < eigenvalues[smallest] {
            smallest = index;
        }
    }
    eigenvalues.sort_by(f32::total_cmp);
    (
        crate::math::normalized3([
            vectors[0][smallest],
            vectors[1][smallest],
            vectors[2][smallest],
        ]),
        eigenvalues,
    )
}

/// Returns the Jacobi rotation that zeroes one off-diagonal entry of a symmetric matrix, as its
/// cosine and sine.
///
/// # Invariants
///
/// It is the root of the quadratic with the smaller magnitude, so the step stays small when the two
/// diagonals are close — which is the ordinary case here, since the covariance of a plane is
/// singular and its two non-zero eigenvalues are often near each other.
fn rotation_of(value: &[[f32; 3]; 3], row: usize, column: usize) -> (f32, f32) {
    let difference = value[row][row] - value[column][column];
    let off = value[row][column];
    let shifted = difference / (2.0 * off);
    let sign = if shifted >= 0.0 { 1.0 } else { -1.0 };
    let tangent = shifted - sign * (shifted * shifted + 1.0).sqrt();
    let cosine = 1.0 / (tangent * tangent + 1.0).sqrt();
    (cosine, tangent * cosine)
}

/// Right-multiplies a matrix by the rotation in the plane `row`, `column`: its columns `row` and
/// `column` are turned into one another.
///
/// # Invariants
///
/// This is the right factor, `J`: `(M J)[i][row] = c·M[i][row] - s·M[i][column]`, which is the form
/// [`rotation_of`] solves its angle for. It is also the **whole** of what the eigenvector
/// accumulator takes, because `Q` accumulates columns and `Q` takes no left factor at all.
///
/// `O(n)` in the matrix's side and no more: one pass over the two columns being mixed, each entry
/// read once and written once.
fn turn_columns(value: &mut [[f32; 3]; 3], row: usize, column: usize, cosine: f32, sine: f32) {
    for entry in value.iter_mut() {
        let along_row = entry[row];
        let along_column = entry[column];
        entry[row] = cosine * along_row - sine * along_column;
        entry[column] = sine * along_row + cosine * along_column;
    }
}

/// Turns the columns `row` and `column` of a matrix by `J`, and its rows by `Jᵀ`.
///
/// # Invariants
///
/// The two halves are **transposes of one another and not the same rotation twice**: a Jacobi step
/// is `A ← Jᵀ A J`, so the columns take `J` and the rows take `Jᵀ`. Applying `J` on the left as
/// well is what makes the step a conjugation that leaves the entry it was chasing where it was,
/// and the caller would not see it — it zeroes that entry by hand on the next line.
///
/// A round trip through both halves is therefore `Jᵀ Q J`, which for the identity accumulator is
/// the identity, and that is exactly how the fitted normal came to be a coordinate axis before the
/// accumulator was given the columns half alone.
fn turn(value: &mut [[f32; 3]; 3], row: usize, column: usize, cosine: f32, sine: f32) {
    turn_columns(value, row, column, cosine, sine);
    // Both rows being turned are read before either is written, which is the same arithmetic the
    // per-index walk produced and is only equal to it because the two rows are never the same one:
    // the caller reads its row and column from `largest_off_diagonal`, which returns row < column.
    let along_row = value[row];
    let along_column = value[column];
    value[row] = array::from_fn(|index| cosine * along_row[index] - sine * along_column[index]);
    value[column] = array::from_fn(|index| sine * along_row[index] + cosine * along_column[index]);
}

/// Returns the position of the largest off-diagonal entry of a symmetric matrix.
fn largest_off_diagonal(value: &[[f32; 3]; 3]) -> (usize, usize) {
    let mut found = (0usize, 1usize);
    let mut largest: f32 = 0.0;
    for (row, entries) in value.iter().enumerate() {
        for (column, entry) in entries.iter().enumerate().skip(row + 1) {
            if entry.abs() > largest {
                largest = entry.abs();
                found = (row, column);
            }
        }
    }
    found
}

/// Returns the total order two runs of this module agree on.
///
/// # Invariants
///
/// The key is the variant's rank first, so the answer reads as "the document's faults, then the
/// mesh's", and within a variant the thing it is about: the mass, then the plane, then the row, so
/// two edits of one building are diffed by hand without sorting the lines first. Every key is
/// unique within its rank, which is what makes the order total.
fn order_key(violation: &Violation) -> (u8, u64, u64, u64, u64) {
    match violation {
        Violation::OutlineReversed { mass, area } => (0, mass.0 as u64, ordered(*area), 0, 0),
        // **`OutlineNotSimple` shares rank `0` and takes its third, fourth and fifth slots**, for the
        // reason `VoidNotSimple` took slot three of rank `6`: rank `0` is the rank this module already
        // reserves for "a statement about a mass's own outline", and a new rank for it would sort a
        // shape fault after every grid, wall and part fault, which is not where it belongs. Its third
        // slot is `0` where `OutlineReversed`'s is [`ordered`] of a plan area, and the only `f32` bit
        // pattern [`ordered`] folds to `0` is `0xFFFFFFFF` — a negative `NaN` — which `plan_area` cannot
        // return for a loop of finite corners, so the two cannot collide. **The shape sorts before the
        // winding because it is the better question**: a loop that meets itself makes its own signed area
        // a statement about the clipper rather than about the outline, so an author is served by the
        // shape first and the winding second.
        Violation::OutlineNotSimple { mass, meeting } => {
            (0, mass.0 as u64, 0, 0, meeting_rank(meeting))
        }
        // **Rank `0`'s fourth slot, beside `OutlineNotSimple` rather than after every grid and wall
        // fault**, for the same reason that one took its own slots: a statement about a mass's own
        // outline belongs with the other two, and it is about the loop rather than about a shape or a
        // winding. The fourth slot is the whole of the choice — `0` is the outline that meets itself
        // and is named for its shape, `1` is the outline nothing else can name because it has no
        // triangulation. **They cannot both be reported for one mass**, because they are the two
        // alternatives of one [`OutlineReading`], so the slot only has to order them; and it puts the
        // shape first for the reason [`OutlineNotSimple`]'s own third slot does — a loop that meets
        // itself has no area to be measured against, so the shape is the better question.
        Violation::OutlineNotMeasured { mass } => (0, mass.0 as u64, 0, 1, 0),
        Violation::OutlineTooShort { mass, nodes } => (1, mass.0 as u64, *nodes as u64, 0, 0),
        Violation::ZeroHeight { mass, top, highest } => {
            (2, mass.0 as u64, ordered(*top), ordered(*highest), 0)
        }
        // Ranks `46`–`48`, at the end for the same reason the profile's are: a crown arrived after the
        // mass's own block closed, and a crown fault is a statement about a mass that already exists.
        Violation::CrownCountMismatch {
            mass,
            crowns,
            nodes,
        } => (46, mass.0 as u64, *crowns as u64, *nodes as u64, 0),
        Violation::CrownBelowBase {
            mass,
            node,
            top,
            base,
        } => (
            47,
            mass.0 as u64,
            *node as u64,
            ordered(*top),
            ordered(*base),
        ),
        Violation::UnattachedOverlap { mass, other } => (3, mass.0 as u64, other.0 as u64, 0, 0),
        // **Rank `3` and its fourth slot, beside `UnattachedOverlap` and not at the end.** The two are
        // one question asked of one pair of masses — whether two unattached plans share area — and the
        // fourth slot is what tells the answer from the refusal to answer: `0` is the pair measured
        // and overlapping, `1` is the pair not measured at all. They cannot both be reported for one
        // pair, so the slot only has to order them; putting this one at the end would sort a statement
        // about two masses' plans after every part, wall and plate fault, which is not where a
        // statement about the document's own geometry belongs.
        Violation::OverlapNotMeasured { mass, other } => (3, mass.0 as u64, other.0 as u64, 1, 0),
        // The void faults share a rank: each is a statement about one mass's own courtyards, and a document has
        // whichever of them it has. The rank's fourth slot says which part of the courtyard is wrong: `0` is the
        // loop's own fit, `2` is the ring's own, and `3` is the loop's own shape — so it sorts after both, because
        // a loop that meets itself is a fault about what the loop *is* and the other two are about where it stands.
        Violation::VoidNotInside { mass, void } => (6, mass.0 as u64, *void as u64, 0, 0),
        // **The fourth slot's `1` is retired and is not to be reused.** It was `VoidNotConvex`, the refusal of a
        // void loop with a reflex corner, which was lifted once the outward walk was measured across one; what the
        // walk still cannot do is refused by `offset_loop`'s own names and reaches the author through `DidNotExpand`.
        // The ranks are private to this function, so a hole costs nothing, and closing it would put every arm above
        // it in the diff of a one-variant deletion.
        //
        // `VoidNotSimple` took `3` rather than a rank of its own at the end, because rank `6` is the one this
        // module already reserves for "a statement about one mass's own courtyards" and this is exactly that: a
        // loop that is not simple is about the loop, next to the two faults that are about where it stands. Ranks
        // `41`–`43` were taken at the end instead because their variants are about a slab's host and an opening's
        // bay, which are not a courtyard's, and because those three had no block of their own to sit in.
        Violation::VoidNotSimple {
            mass,
            void,
            meeting,
        } => (6, mass.0 as u64, *void as u64, 3, meeting_rank(meeting)),
        Violation::VoidOverlaps {
            mass,
            first,
            second,
        } => (6, mass.0 as u64, *first as u64, *second as u64, 0),
        Violation::VoidReachesTheWall { mass, void, over } => {
            (6, mass.0 as u64, *void as u64, 2, ordered(*over))
        }
        Violation::MassHostMissing { mass, host } => (4, mass.0 as u64, host.0 as u64, 0, 0),
        Violation::RoofHostMissing { roof, host } => (5, roof.0 as u64, host.0 as u64, 0, 0),
        Violation::RoofEaveTooShort { roof, nodes } => (6, roof.0 as u64, *nodes as u64, 0, 0),
        Violation::RoofPlaneTooShort { roof, plane } => (7, roof.0 as u64, *plane as u64, 0, 0),
        // Rank `44` and `45`, taken at the end because the profile arrived after the roof's own block
        // was closed. They sort after every roof fault because a profile is a statement about a
        // plane that already exists: a plane with no rows and a profile is two faults, and the one
        // about the rows is the one to fix first.
        Violation::ProfileOutOfOrder { roof, plane, step } => {
            (44, roof.0 as u64, *plane as u64, *step as u64, 0)
        }
        Violation::ProfileReachesTheCrest { roof, plane, step } => {
            (45, roof.0 as u64, *plane as u64, *step as u64, 0)
        }
        Violation::CoveringFault { roof, fault } => {
            (50, roof.0 as u64, covering_rank(*fault), 0, 0)
        }
        Violation::RoofCrestMissing {
            roof,
            plane,
            row,
            crest,
        } => (8, roof.0 as u64, *plane as u64, *row as u64, *crest as u64),
        // One of these per validation, so the count is the only thing that has to order it; the
        // problems themselves travel in a deterministic order of their own, from the expander.
        Violation::DidNotExpand { problems } => (9, problems.len() as u64, 0, 0, 0),
        Violation::SolidOpen { solid, loops } => (10, solid_rank(solid), *loops as u64, 0, 0),
        Violation::SolidNonManifold { solid } => (11, solid_rank(solid), 0, 0, 0),
        Violation::SolidInsideOut { solid, volume } => {
            (12, solid_rank(solid), ordered(*volume), 0, 0)
        }
        Violation::SolidNotASphere { solid, euler } => {
            (13, solid_rank(solid), ordered_integer(*euler), 0, 0)
        }
        // **Rank `14` is retired and is not to be reused.** It was `MassDoesNotCover`, the only
        // consumer of a coverage bucket nothing but the shell's soffit could fill. The ranks are
        // private to this function and no reader outside it can see one, so a hole costs nothing —
        // and renumbering the twenty-six arms above `14` to close it would put every one of them in
        // the diff of a one-variant deletion, which is how a change of this size becomes hard to
        // read. A retired rank is a name a report may already have quoted.
        Violation::RoofDoesNotCover { roof, skin, eave } => {
            (15, roof.0 as u64, ordered(*skin), ordered(*eave), 0)
        }
        Violation::FaceNotPlanar { face, departure } => {
            (16, *face as u64, ordered(*departure), 0, 0)
        }
        Violation::RoofSkinLeavesTheEave { roof, off } => (19, roof.0 as u64, *off as u64, 0, 0),
        Violation::GridFaceMissing { face } => (
            20,
            face.mass.0 as u64,
            face.edge as u64,
            void_rank(*face),
            0,
        ),
        Violation::GridTooShort { face } => (
            21,
            face.mass.0 as u64,
            face.edge as u64,
            void_rank(*face),
            0,
        ),
        Violation::GridNotOrdered { face } => (
            22,
            face.mass.0 as u64,
            face.edge as u64,
            void_rank(*face),
            0,
        ),
        Violation::GridOutsideFace { face } => (
            23,
            face.mass.0 as u64,
            face.edge as u64,
            void_rank(*face),
            0,
        ),
        Violation::GridOnACrownedFace { face } => (
            49,
            face.mass.0 as u64,
            face.edge as u64,
            void_rank(*face),
            0,
        ),
        Violation::PlacementTemplateMissing { placement } => (28, *placement as u64, 0, 0, 0),
        Violation::PlacementBayMissing { placement, face } => (
            29,
            *placement as u64,
            face.mass.0 as u64,
            face.edge as u64,
            void_rank(*face),
        ),
        Violation::PlacementOutsideBay { placement, past } => {
            (30, *placement as u64, ordered(*past), 0, 0)
        }
        Violation::PlacementSocketMissing { placement, .. } => (31, *placement as u64, 0, 0, 0),
        Violation::PlacementAnchorMissing { placement } => (32, *placement as u64, 0, 0, 0),
        Violation::WallTooShort { wall, .. } => (33, wall.0 as u64, 0, 0, 0),
        Violation::WallNotThick { wall, .. } => (34, wall.0 as u64, 0, 0, 0),
        Violation::WallTooLow { wall, .. } => (35, wall.0 as u64, 0, 0, 0),
        Violation::SlabNotThick { slab, .. } => (36, slab.0 as u64, 0, 0, 0),
        Violation::SlabNotAPolygon { slab, .. } => (37, slab.0 as u64, 0, 0, 0),
        // The three faults a part hung on a wall can have, after the bay ones because they are the rarer
        // target and an author meets a bay's faults first.
        Violation::PlacementWallMissing { placement, wall } => {
            (38, *placement as u64, wall.0 as u64, 0, 0)
        }
        Violation::PlacementOffItsWall {
            placement,
            wall,
            past,
        } => (39, *placement as u64, wall.0 as u64, ordered(*past), 0),
        Violation::PlacementMountOnAWall { placement } => (40, *placement as u64, 0, 0, 0),
        // **Rank `41`, at the end, because the block is full.** Every rank from `0` to `13` is a mass, void or
        // roof authoring fault that predates the plate becoming a solid of its own, and `15` onward is the mesh
        // half and then the feature checks; the only hole in `0..=40` is `14`, which is retired above and must
        // not be reopened. So a plate's coverage cannot sit beside the roof's at `15` without renumbering
        // thirty arms, and the slice that retired `14` declined to renumber for a reason that applies here too:
        // an order that is already total and already printed by every report keeps the meaning a report may
        // already have read a number with. The cost is stated rather than hidden — a document's own plate no
        // longer sorts with the rest of that document's faults but last, after the placements'.
        Violation::PlateCoverage {
            mass,
            slab,
            plate,
            outline,
            ..
        } => (
            41,
            mass.0 as u64,
            slab.0 as u64,
            ordered(*plate),
            ordered(*outline),
        ),
        // **Rank `42`, beside `PlateCoverage` and for the reason that one took `41`.** The two are the
        // slab's own host faults — one where the host is there and the plate is short of its floor, one
        // where the host is not there at all — so they read together at the end of the list instead of
        // being split by the ranks the other elements' host faults occupy: `4` is a mass's and `5` a
        // roof's, this one is a slab's, and seating it beside them would renumber the forty arms above
        // them for a list no reader outside this function can see. The role is **carried but not sorted
        // by**, and that is the same fact twice over: a slab has exactly one host, so `slab` alone makes
        // the key unique within this rank, and the role is in the payload for the reader rather than for
        // the order — the order's job is to make two runs diff, not to restate the variant.
        Violation::SlabHostMissing { slab, host, .. } => (42, slab.0 as u64, host.0 as u64, 0, 0),
        // **Rank `43`, and it is the opening's rank beside the four above it for the reason they took
        // `24`–`27`.** Those four are the ways a hole fails against the wall it names, and this is the
        // fifth way: it fails against the *loop* it names, before a bay is asked for. It sits beside
        // them rather than at the end with the slab faults because a reader who has been told "a hole
        // names a bay" reads `24` first and this is the same sentence with one more word in it — but
        // the ranks cannot be renumbered for tidiness, so it takes the next free one and says so.
        Violation::OpeningOnAFarLoop { opening, face } => (
            43,
            *opening as u64,
            face.mass.0 as u64,
            face.edge as u64,
            void_rank(*face),
        ),
        // The four opening faults in the order an author meets them: the slot is not there, the bay
        // cannot hold the hole, the hole leaves the wall, and two holes in one wall fight.
        Violation::OpeningBayMissing { opening, face } => (
            24,
            *opening as u64,
            face.mass.0 as u64,
            face.edge as u64,
            void_rank(*face),
        ),
        Violation::OpeningExceedsBay { opening } => (25, *opening as u64, 0, 0, 0),
        Violation::OpeningOutsideWall { opening } => (26, *opening as u64, 0, 0, 0),
        Violation::OpeningOverlaps { opening, other } => (27, *opening as u64, *other as u64, 0, 0),
        Violation::RoofSkinNotADisc { roof, loops } => (18, roof.0 as u64, *loops as u64, 0, 0),
        Violation::FaceDegenerate { face, area } => (17, *face as u64, ordered(*area), 0, 0),
    }
}

/// Returns a number that sorts a face the way [`FaceRef`]'s own order does, a mass's own walls first
/// and the loop each wall is read from innermost to outermost.
///
/// # Invariants
///
/// A courtyard's faces sort after every one of the mass's own, which is [`Option`]'s own order and the
/// order the key would be incomplete without: two faces on the same mass and the same edge are two
/// different pieces of masonry once one of them is a courtyard's, and an order that could not tell them
/// apart would not be total.
///
/// **The loop side is the low bit and the void the rest, so the order every existing face already had
/// is the order it keeps.** A mass's own faces were `0` and courtyard `v` was `v + 1`; shifting those
/// up by one bit keeps every comparison between them and puts the far face immediately after its own
/// near one, which is the reading a report wants — the two faces of one piece of masonry, adjacent —
/// rather than all of a courtyard's near faces before all of its far ones.
fn void_rank(face: FaceRef) -> u64 {
    let loop_of = match face.void {
        Some(void) => void as u64 + 1,
        None => 0,
    };
    (loop_of << 1) | face.loop_side.rank()
}

/// Returns a number that sorts a void loop's own self-meeting.
///
/// # Invariants
///
/// **The two indices are halves rather than two slots, because the key is five wide and every arm
/// already fills it.** A loop has far fewer than `2^16` nodes, which is the same packing
/// [`void_rank`] and [`solid_rank`] already use where two indices had to share one slot.
///
/// **A crossing and a touch fold to the same number, and that is not a collision.** The key is read
/// to be total within a rank, and the rank's first three slots already name one mass and one void;
/// [`self_meeting`] returns the first meeting a loop has and stops, so a loop contributes one
/// violation and its key is unique whatever the reading says.
fn meeting_rank(meeting: &SelfMeeting) -> u64 {
    let (left, right) = match meeting {
        SelfMeeting::Crossing { left, right } => (*left, *right),
        SelfMeeting::Touch { edge, node } => (*edge, *node),
    };
    ((left as u64) << 16) | right as u64
}

/// Returns a number that sorts a solid the way the enum's own order does, masses first.
///
/// # Invariants
///
/// The five kinds take five ranges that cannot meet — masses below `2^32`, roofs in the next one,
/// parts above — so the order is total over any document and two runs diff. A part's rank is its
/// placement's index, which is the index its own faces name it by.
///
/// **A courtyard's rank carries both of its indices**: the mass in the high half of the range and
/// the void in the low, so the courtyards of one mass read in the order their author wrote them and
/// every one of them sorts after every plain box.
fn solid_rank(solid: &SolidId) -> u64 {
    match solid {
        SolidId::Mass(mass) => mass.0 as u64,
        SolidId::Roof(roof) => (1 << 32) | roof.0 as u64,
        SolidId::Part(placement) => (2 << 32) | *placement as u64,
        SolidId::Wall(wall) => (3 << 32) | wall.0 as u64,
        SolidId::Slab(slab) => (4 << 32) | slab.0 as u64,
        SolidId::Void { mass, void } => (5 << 32) | ((mass.0 as u64) << 16) | *void as u64,
        SolidId::Member(member) => (6 << 32) | member.0 as u64,
        SolidId::Roll(roll) => (7 << 32) | ((roll.roof.0 as u64) << 20) | roll.index as u64,
    }
}

/// Returns a rank for one covering fault, so two coverings' faults sort by which number is wrong.
fn covering_rank(fault: crate::covering::CoveringFault) -> u64 {
    match fault {
        crate::covering::CoveringFault::PeriodNotALength { .. } => 0,
        crate::covering::CoveringFault::FillOutOfRange { .. } => 1,
        crate::covering::CoveringFault::HeightNotARise { .. } => 2,
    }
}

/// Returns a `u64` that sorts like the `f32` it came from.
///
/// # Invariants
///
/// It is the bit pattern with the sign moved to the top, which is exactly what
/// [`f32::total_cmp`] orders by — so this makes an `f32` field usable in a `sort_by_key` without
/// the borrow a closure over two violations would need, and without `f32`'s own `Ord` (which it
/// does not have, because `NaN` compares with nothing).
fn ordered(value: f32) -> u64 {
    let bits = value.to_bits();
    let mask = (((bits as i32) >> 31) as u32) | 0x8000_0000;
    (bits ^ mask) as u64
}

/// Returns a `u64` that sorts like the `i64` it came from, negatives first.
fn ordered_integer(value: i64) -> u64 {
    (value as u64) ^ (1 << 63)
}
