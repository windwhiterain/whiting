//! Expansion: walking the skeletons into closed planar-facet solids.
//!
//! # Contract of this module
//!
//! - **`build` is a direct walk.** Every face comes from an authored element: no boolean, no plane
//!   intersection search, no triangulation of a point cloud. Two faces meet along an edge because
//!   they were built from the same authored row or corner.
//! - **Points are welded by authored identity.** Two faces built from one authored corner share one
//!   point index; nothing compares coordinates to decide it.
//! - **A mass expands to one closed solid**: outer wall, inner wall, the ring of wall top, and the
//!   floor slab's underside. **A roof expands to one closed solid**: skin, soffit one thickness
//!   below, and a fascia band around the eave.
//! - **Two solids are two solids.** A mass and the roof that names it, or two masses joined by an
//!   authored attachment, are not welded to one another: at building level the faces therefore meet
//!   along 4-face edges, and "watertight" means *every solid closed and outward, every joint
//!   authored* — never manifoldness of the whole. See the proposal's D5.
//! - **Failure is a list**, not the first error found: a caller fixing one problem should not have
//!   to run again to discover the next.
//!
//! # The two rules that decide which way a face looks
//!
//! **A solid's faces are wound to point out of it, and the surfaces that have an outside say which
//! way theirs is.** Each face is laid out from the authored order and then oriented by one uniform
//! rule — *a mass's floor and a roof plane's skin are wound so that the up component of their normal
//! is not negative* — rather than by a rule per surface. Two faces that meet along an authored row
//! are built from the same points, so they meet; the orientation rule is what makes them meet **head
//! to head**, and it is why the rows of a roof plane may be written in either direction without the
//! surface folding onto itself. Planes whose runs disagree about a shared hip line still fold there,
//! and a fold is **reported rather than hidden**: the two faces meet along that edge in the same
//! direction, which is the one shape `boundary_edges_of` cannot cancel — an edge used twice in the
//! same direction is an edge with no opposite — so that solid reads open, as
//! [`validate`](crate::validate)'s `SolidOpen`. Nothing names the disagreement itself: two planes
//! have to agree by authoring, which is what "two adjacent planes share the row on their boundary"
//! buys, and the report is the closed/open reading rather than a diagnosis of the cause.
//!
//! **The inner loop is a miter offset, and a miter that will not build is a problem.** Each outline
//! edge is shifted inward along its own normal and neighbouring shifted edges are intersected, so a
//! corner keeps its angle and the wall keeps its thickness. Every way that can fail — a corner
//! whose two edges run back along each other, a miter further from its corner than a join may run,
//! an offset edge with no length left, one running backwards along the edge it was shifted from, two
//! offset edges crossing, a loop that has stopped enclosing the mass — is a wall in the wrong place
//! rather than a wall missing, so each is reported by name.
//!
//! # The two discs
//!
//! The floor the mass stands on is the inner loop's polygon at the outline's own elevations, and the
//! slab's soffit is the whole outline one slab below. They are one slab apart, so they are two
//! authored planes and two elements — [`MassSurface::FloorTop`] and [`MassSurface::SlabSoffit`].
//! Naming both `Floor` put them in one planarity group, and the merged validator reported that group
//! as a non-planar surface at `departure = slab/2`; measured on the first tip where the expander and
//! the validator ran together.
//!
//! # What the underside is not
//!
//! **The slab's soffit is a disc over the whole outline, not a ring reaching in to the inner loop.**
//! A ring reaching the inner loop at the floor level puts three faces on the edge where the inner
//! wall stops, the floor ends and the ring's inner edge runs, and a solid cannot be closed and
//! outward with three faces on an edge — the rectangle this probe measures came out at loops 1 and
//! Euler 1 that way, while every one of its faces still met a neighbour. The soffit reaches the
//! inner loop nowhere: the slab is a slab, `slab` below the outline's own elevations, which is what
//! [`Mass::slab`](crate::mass::Mass::slab) says it is.

use std::collections::{BTreeMap, HashMap};

use crate::courtyard;
use crate::grid::{FaceRef, GRID_TOLERANCE};
use crate::mass::{AuthoredLoop, Mass, MassId, PlanNode};
use crate::math::{
    Vec2, Vec3, cross, cross3, difference, distance, dot, length, lift, normalized, subtract,
};
use crate::mesh::{BuildingFace, BuildingMesh, Element, MassSurface, PartSurface, RoofSurface};
use crate::openings::{Extents, RevealSide};
use crate::parts::{PartFrame, Target};
use crate::roof::{EaveStation, Roof, RoofId, RoofRow};
use crate::skeleton::BuildingSkeleton;
use crate::slab::{Slab, SlabId, SlabSurface};
use crate::wall::{Wall, WallId, WallSurface};

/// Two outline edges whose turn is smaller than this, in sine, are treated as parallel.
///
/// A miter divides by that sine, so a corner whose edges are within a hundredth of a microdegree of
/// collinear turns the rounding in two unit directions into a corner somewhere else entirely.
/// Collinear is not itself a fault: a node in the middle of a straight run has one miter, the point
/// the corner would have had.
const ANGLE_EPSILON: f32 = 1.0e-6;

/// The shortest an authored edge may be, in metres.
///
/// A tenth of a millimetre is below any edge a building is authored with, and an edge that short has
/// no direction for a wall to be built along: its normal falls back to an arbitrary one, which puts
/// the wall in a visible place that no check can account for.
const LENGTH_EPSILON: f32 = 1.0e-4;

/// How far a miter may run from its own corner, as a multiple of the wall thickness.
///
/// The miter of a corner whose interior angle is `theta` reaches `wall / sin(theta / 2)` from the
/// corner, so this is the measure SVG calls the miter ratio — `miter length / stroke width` — and
/// `4` is SVG's default limit, reached at the roughly 14 degree corner where a join visibly
/// over-runs the edges it joins. A wall is not a stroke, but the failure is the same one: past that
/// corner the two shifted edges cross each other far from the corner the author drew.
const MITER_LIMIT: f32 = 4.0;

/// One reason a skeleton does not expand.
#[derive(Debug, Clone, PartialEq)]
pub enum Problem {
    /// A mass's void loop is not convex, so the ring built from it and its offset is not built at all.
    ///
    /// # Invariants
    ///
    /// **A refusal rather than a guess, and the restriction is stated rather than discovered.**
    /// A courtyard's ring is one quad per loop edge between two loops of the same length, which is
    /// the mass's own ring construction and holds for any loop whose offset builds — the one case
    /// the crate has no measurement for is the void loop with a reflex corner, because a mass
    /// whose *outline* has one is offset **inward** and a void is offset **outward**: the corner
    /// that fails in one is not the corner that fails in the other, and this crate has measured
    /// both wrong in the past (a ring fill twice wrong by area, and the inward offset closing a
    /// courtyard's mouth). Rather than ship an unmeasured ring, the loop is refused by name here and
    /// reported in the document by
    /// [`Violation::VoidNotConvex`](crate::validate::Violation::VoidNotConvex). What would lift
    /// this is a measurement of the outward walk across a reflex corner, which is
    /// [`offset_loop`](offset_loop) — the same walk, asked the other question.
    VoidNotConvex {
        /// The mass.
        mass: MassId,
        /// Which of its voids, in the order it authored them.
        void: usize,
    },
    /// A mass's outline has fewer than three nodes, so it has no plan area to stand on.
    OutlineTooShort {
        /// The mass.
        mass: MassId,
        /// How many nodes it has.
        nodes: usize,
    },
    /// A roof plane has fewer than two rows, so it spans nothing.
    PlaneTooShort {
        /// The roof.
        roof: RoofId,
        /// The plane.
        plane: usize,
    },
    /// A roof row names a crest point that does not exist.
    CrestMissing {
        /// The roof.
        roof: RoofId,
        /// The plane.
        plane: usize,
        /// The row.
        row: usize,
        /// The index it named.
        crest: usize,
    },
    /// A mass's wall or its floor slab has no thickness, so its inner loop lands on its own outline.
    MassThickness {
        /// The mass.
        mass: MassId,
        /// Its wall thickness.
        wall: f32,
        /// Its floor slab thickness.
        slab: f32,
    },
    /// A loop edge has no length, so it names no direction to build the second loop along.
    OutlineEdgeDegenerate {
        /// The loop being offset.
        loop_of: AuthoredLoop,
        /// The loop edge, counted from the loop's first node.
        edge: usize,
    },
    /// A node's two loop edges run back along each other, so there is no corner to mitre.
    MiterUnresolved {
        /// The loop being offset.
        loop_of: AuthoredLoop,
        /// The loop node, counted from the loop's first node.
        node: usize,
    },
    /// A miter reaches further from its corner than the join limit allows, so the corner it builds is
    /// not the corner that was authored.
    MiterTooLong {
        /// The loop being offset.
        loop_of: AuthoredLoop,
        /// The loop node, counted from the loop's first node.
        node: usize,
        /// How far it reached, from the corner.
        reach: f32,
    },
    /// An edge of the offset loop has no length left in it.
    OffsetEdgeVanished {
        /// The loop being offset.
        loop_of: AuthoredLoop,
        /// The offset edge, counted over the offset loop.
        edge: usize,
        /// The length it was left with.
        length: f32,
    },
    /// An offset edge runs backwards along the edge it was shifted from, which is what an
    /// offset wider than the corner it offsets does.
    OffsetEdgeFolded {
        /// The loop being offset.
        loop_of: AuthoredLoop,
        /// The offset edge, counted over the offset loop.
        edge: usize,
    },
    /// Two offset edges that are not neighbours meet, so the loop is not the one the author drew.
    OffsetSelfIntersecting {
        /// The loop being offset.
        loop_of: AuthoredLoop,
        /// The first offset edge, counted over the offset loop.
        left: usize,
        /// The second.
        right: usize,
    },
    /// The offset loop has stopped enclosing the mass: for an inward offset its area is not positive
    /// or is not smaller than the authored loop's, and for an outward one it is not larger.
    ///
    /// **The comparison follows the side rather than being fixed**, because the two offsets fail in
    /// opposite directions: a wall built inward has to leave room inside its outline, and a
    /// courtyard's ring built outward has to stay outside the courtyard's own loop. An area that no
    /// longer encloses the authored loop is a wall in a place the author did not draw.
    OffsetInverted {
        /// The loop being offset.
        loop_of: AuthoredLoop,
        /// The area the offset loop encloses.
        area: f32,
    },
    /// A roof has no planes, so it expands to no solid at all.
    RoofEmpty {
        /// The roof.
        roof: RoofId,
        /// How many planes it has.
        planes: usize,
    },
    /// A roof's slab has no thickness, so its soffit lands on its own skin.
    RoofThickness {
        /// The roof.
        roof: RoofId,
        /// Its thickness.
        thickness: f32,
    },
    /// Two consecutive rows of a plane name one crest point at one eave station, so the facet
    /// between them has no area.
    RowPairDegenerate {
        /// The roof.
        roof: RoofId,
        /// The plane.
        plane: usize,
        /// The first of the two rows.
        row: usize,
    },
    /// A roof row names an eave station that the eave loop cannot resolve to a point.
    EaveUnresolved {
        /// The roof.
        roof: RoofId,
        /// The plane.
        plane: usize,
        /// The row.
        row: usize,
        /// The station it named.
        station: f32,
    },
    /// An opening names a slot the wall's rhythm does not hold, or insets that leave no rectangle
    /// inside its bay, so there is no hole of its own to cut.
    ///
    /// # Invariants
    ///
    /// **An opening that cannot be cut stops the wall rather than being dropped.** The document says
    /// a wall has a hole in it; a mesh built without one contradicts the document silently, and a
    /// solid nobody asked for is still a closed solid that every later check reads as sound. A
    /// vanished slot is [`validate`](crate::validate)'s own report as well — it can name the opening
    /// and the grid it missed — and the two are different statements: this one says the wall that
    /// would have carried the hole was not built.
    ///
    /// **The wall is named by its loop and not by a mass and an edge**, because a mass holds two
    /// kinds of wall: its own outline and a courtyard's ring, and "edge 1" is two different pieces of
    /// masonry with two different answers. Every fault below carries the loop for the reason
    /// [`offset_loop`] carries it, and none of them widens into a catch-all: each names one way the
    /// cut cannot be made.
    OpeningUnresolved {
        /// The authored loop the opening names: a mass's outline, or one of its courtyards.
        loop_of: AuthoredLoop,
        /// The loop edge the opening names, counted from the loop's first node.
        edge: usize,
        /// The opening's index in the document's own list.
        opening: usize,
    },
    /// An opening's hole is not strictly inside the wall face it is cut into, so the strip the cut
    /// decomposes has no band for it.
    ///
    /// `past` is how far the hole leaves the strip, in metres, over every side that does: its two
    /// ends along the edge, its head, its sill against the ground line, and the mitred offset edge
    /// its four corners project onto. The greatest of those is what is reported, so a hole that is
    /// wrong in two ways names the worse one.
    OpeningOutsideWall {
        /// The authored loop the hole is cut into.
        loop_of: AuthoredLoop,
        /// The loop edge the hole is cut into.
        edge: usize,
        /// The opening's index in the document's own list.
        opening: usize,
        /// How far it reaches past the strip it has to stay inside, in metres.
        past: f32,
    },
    /// The ground line under one wall crosses the elevation of one of that wall's own holes, so the
    /// complement of the hole in that band is a quad folded on itself rather than a quad.
    GroundCrossesOpening {
        /// The authored loop the hole is cut into.
        loop_of: AuthoredLoop,
        /// The loop edge the hole is cut into.
        edge: usize,
        /// The opening whose elevation the ground line crosses.
        opening: usize,
    },
    /// Two openings in one wall overlap, so the complement of their two holes is not a set of
    /// bands.
    ///
    /// Touching is not overlapping: two openings that meet exactly along an edge remove no wall
    /// twice, and they are cut as two holes whose rims share their corner points.
    OpeningsOverlap {
        /// The authored loop both holes are cut into.
        loop_of: AuthoredLoop,
        /// The loop edge both are cut into.
        edge: usize,
        /// The earlier of the two openings, by the document's own order.
        left: usize,
        /// The later.
        right: usize,
    },
    /// A placement could not be resolved, so nothing stands in its bay.
    ///
    /// # Invariants
    ///
    /// **It refuses the build, like every other problem here** — this crate's shape is that problems
    /// come back as a list and the mesh is all-or-nothing, so a document with an orphaned placement is
    /// not buildable until its author fixes the slot. What is *not* allowed is silence: a lookup that
    /// answers `None` filters, and [`BuildingSkeleton::unresolved_placements`] is the companion that
    /// names what it dropped, which is why this variant exists at all. The validator reports the same
    /// fault in the document's own terms (`PlacementTemplateMissing`, `PlacementBayMissing`), so an
    /// author reading either sees it.
    PlacementUnresolved {
        /// The placement, by its index in the document's placements.
        placement: usize,
    },
    /// A placement stands in a bay of a **courtyard's** wall, and this crate does not build that.
    ///
    /// # Invariants
    ///
    /// **A courtyard's bay resolves like any other — the grid, the slot and the pin are all there —
    /// and the frame a part would be stood in is the one thing about it that is not built.** The part
    /// frame is read off the wall's **outer** face, which is the face an outside viewer sees, and
    /// for a courtyard that is the offset loop rather than the loop the face is named by; a frame
    /// resolved from the face's own loop would stand the part in the courtyard's open air, a
    /// centimetre of masonry away from where its bay says it is. That is a part in a visible wrong
    /// place, which is worse than no part, so the placement is refused by name rather than guessed at
    /// a face it does not own.
    ///
    /// It is one variant and not a flag on [`PlacementUnresolved`](Self::PlacementUnresolved)
    /// because the fix is different: that one says the slot has gone, and this says the slot is
    /// there and the expander declines to stand anything in it.
    PlacementOnACourtWall {
        /// The placement, by its index in the document's placements.
        placement: usize,
    },
    /// A wall's two ends coincide, so it has no direction and no box.
    WallTooShort {
        /// The wall.
        wall: WallId,
        /// The length that was measured.
        length: f32,
    },
    /// A wall's thickness is not a thickness.
    WallNotThick {
        /// The wall.
        wall: WallId,
        /// The thickness that was given.
        thickness: f32,
    },
    /// A wall's top is at or below the ground it stands on, so it is not a wall.
    WallTooLow {
        /// The wall.
        wall: WallId,
        /// The top it was given.
        top: f32,
        /// The highest ground under its ends.
        highest: f32,
    },
    /// A slab's two elevations are the wrong way round, so it is not a prism.
    SlabNotThick {
        /// The slab.
        slab: SlabId,
        /// The thickness that came out of its elevations.
        thickness: f32,
    },
    /// A slab's outline is not a polygon.
    SlabNotAPolygon {
        /// The slab.
        slab: SlabId,
        /// How many nodes it had.
        nodes: usize,
    },
}

/// Everything that stopped a build.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildError {
    /// The problems, in a deterministic order.
    pub problems: Vec<Problem>,
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} problem(s) stopped the build",
            self.problems.len()
        )
    }
}

impl std::error::Error for BuildError {}

/// Expands a building skeleton into its solids.
///
/// # Invariants
///
/// **The whole list or nothing.** A mass or a roof that reports a problem contributes no points and
/// no faces, so a mesh never carries half a wall, and the problems name every solid that failed
/// rather than the first. The order is masses then roofs, each in list order, so two runs of one
/// document produce the same report.
/// One expansion per template, reused by every placement that stands it.
///
/// # Invariants
///
/// **The cache is sound because a template's expansion depends on nothing but the template.** Its skeleton is
/// the template's own, its parts are its own, and no placement enters it — so one mesh per template index is
/// the same value a second expansion would compute, and [`build`] takes its meshes from here rather than from
/// a second call. `O(templates)` expansions for any number of placements.
///
/// **A failed template is held too**, so every placement that stands it reports the same problems rather than
/// each re-discovering them.
pub struct TemplateCache {
    held: Vec<Option<Result<BuildingMesh, Vec<Problem>>>>,
}

impl TemplateCache {
    /// Expands every template in the document, once each.
    pub fn of(skeleton: &BuildingSkeleton) -> Self {
        let mut held = Vec::with_capacity(skeleton.parts.template_count());
        for index in 0..skeleton.parts.template_count() {
            held.push(
                skeleton
                    .parts
                    .template(index)
                    .map(|template| template.mesh()),
            );
        }
        Self { held }
    }

    /// Returns how many templates this cache expanded **without** a problem.
    ///
    /// It is the number the slice exists for: a document with twelve placements of one shutter answers `1`.
    pub fn expanded(&self) -> usize {
        self.held
            .iter()
            .filter(|held| matches!(held, Some(Ok(_))))
            .count()
    }

    /// Returns one template's own mesh, or the problems that stopped it, or `None` for a template the
    /// document does not hold.
    pub fn mesh(&self, template: usize) -> Option<&Result<BuildingMesh, Vec<Problem>>> {
        self.held.get(template).and_then(|held| held.as_ref())
    }
}
pub fn build(skeleton: &BuildingSkeleton) -> Result<BuildingMesh, BuildError> {
    build_with(skeleton, &TemplateCache::of(skeleton))
}

/// Expands a skeleton, taking each template's own mesh from a cache the caller supplies.
///
/// # Invariants
///
/// - **A template's expansion is a function of the template alone**, which is what makes the cache sound:
///   the mesh it hands back is exactly what [`Template::mesh`] would compute, computed once.
/// - **`build` is this function with a cache it builds itself**, so there is one path and no way for a caller
///   to get a different geometry by choosing the other one.
/// - **A cache entry that failed is reported, not skipped**: a template that cannot expand is a fault of
///   every placement that stands it, and the problems are cloned into each of them rather than counted once.
pub fn build_with(
    skeleton: &BuildingSkeleton,
    cache: &TemplateCache,
) -> Result<BuildingMesh, BuildError> {
    let mut mesh = BuildingMesh::new();
    let mut problems = Vec::new();
    for (index, mass) in skeleton.masses.iter().enumerate() {
        if let Err(found) = expand_mass(skeleton, &mut mesh, MassId(index), mass) {
            problems.extend(found);
        }
    }
    for (index, roof) in skeleton.roofs.iter().enumerate() {
        if let Err(found) = expand_roof(&mut mesh, RoofId(index), roof) {
            problems.extend(found);
        }
    }
    for index in 0..skeleton.parts.len() {
        match expand_placement(skeleton, index, cache) {
            Ok(Some(expanded)) => splice_expansion(&mut mesh, expanded.0, expanded.1),
            Ok(None) => problems.push(Problem::PlacementUnresolved { placement: index }),
            Err(found) => problems.extend(found),
        }
    }
    for (index, wall) in skeleton.walls.iter().enumerate() {
        if let Err(found) = expand_wall(&mut mesh, WallId(index), wall) {
            problems.extend(found);
        }
    }
    for (index, slab) in skeleton.slabs.iter().enumerate() {
        if let Err(found) = expand_slab(&mut mesh, SlabId(index), slab) {
            problems.extend(found);
        }
    }
    if problems.is_empty() {
        Ok(mesh)
    } else {
        Err(BuildError { problems })
    }
}

/// Builds one slab as the level prism it is: two caps through the ear clipper, and a rectangle per edge.
///
/// # Invariants
///
/// **The caps are the polygon and the sides are rectangles**, so every face is planar by construction —
/// the property a slab exists to have. A side is wound up, along, down, back (`[bottom_i, top_i,
/// top_i+1, bottom_i+1]`), which puts its normal out of the material for a positively wound loop; a cap
/// takes the loop's own winding, with the clipper's `up` flag deciding which way it faces.
fn expand_slab(mesh: &mut BuildingMesh, id: SlabId, slab: &Slab) -> Result<(), Vec<Problem>> {
    if slab.outline.len() < 3 {
        return Err(vec![Problem::SlabNotAPolygon {
            slab: id,
            nodes: slab.outline.len(),
        }]);
    }
    if slab.thickness() <= 0.0 {
        return Err(vec![Problem::SlabNotThick {
            slab: id,
            thickness: slab.thickness(),
        }]);
    }
    let mut solid = Solid::default();
    let bottom: Vec<u32> = slab
        .outline
        .iter()
        .map(|at| solid.point(lift(*at, slab.bottom)))
        .collect();
    let top: Vec<u32> = slab
        .outline
        .iter()
        .map(|at| solid.point(lift(*at, slab.top)))
        .collect();
    polygon(
        &mut solid,
        &slab.outline,
        &top,
        Element::Slab(id, SlabSurface::Top),
        true,
    );
    polygon(
        &mut solid,
        &slab.outline,
        &bottom,
        Element::Slab(id, SlabSurface::Bottom),
        false,
    );
    for index in 0..slab.outline.len() {
        let next = (index + 1) % slab.outline.len();
        let element = Element::Slab(id, SlabSurface::Edge { index });
        solid.triangle([bottom[index], top[index], top[next]], element);
        solid.triangle([bottom[index], top[next], bottom[next]], element);
    }
    solid.splice(mesh);
    Ok(())
}

/// Builds one wall run as the box it is: six quads, twelve triangles, all wound outward.
///
/// # Invariants
///
/// The six windings are checked against the run's own frame rather than against a picture: the left face
/// is `+n`, the right `−n`, the bottom `−up`, the top `+up`, and the two ends `∓d`, where `d` is the
/// run's direction and `n` its left normal. Each is the right-handed cross product of that face's own
/// first two edges, so a run whose ends coincide is a `Problem` rather than a box with a face of no area.
///
/// **The bottom follows the ground and the top does not**, which is the only asymmetry in the shape: the
/// lower four points carry their own node elevations and the upper four carry `top`. Every face is
/// therefore a quad with two parallel edges — both ends across the thickness, both sides up — so the box
/// is planar wherever it is looked at from.
fn expand_wall(mesh: &mut BuildingMesh, id: WallId, wall: &Wall) -> Result<(), Vec<Problem>> {
    if wall.direction().is_none() || wall.normal().is_none() {
        return Err(vec![Problem::WallTooShort {
            wall: id,
            length: wall.length(),
        }]);
    }
    if wall.thickness <= 0.0 {
        return Err(vec![Problem::WallNotThick {
            wall: id,
            thickness: wall.thickness,
        }]);
    }
    if wall.top <= wall.highest_ground() {
        return Err(vec![Problem::WallTooLow {
            wall: id,
            top: wall.top,
            highest: wall.highest_ground(),
        }]);
    }
    let normal = wall.normal().expect("checked above");
    let half = wall.thickness * 0.5;
    let mut corner = |node: crate::mass::PlanNode, out: f32, up: f32| {
        let plan = [
            node.position[0] + normal[0] * half * out,
            node.position[1] + normal[1] * half * out,
        ];
        mesh.points.push(lift(plan, up));
        (mesh.points.len() - 1) as u32
    };
    let (start, finish) = (wall.from, wall.to);
    let from_low = corner(start, 1.0, start.elevation);
    let from_floor = corner(start, -1.0, start.elevation);
    let from_top_left = corner(start, 1.0, wall.top);
    let from_top_right = corner(start, -1.0, wall.top);
    let to_low = corner(finish, 1.0, finish.elevation);
    let to_floor = corner(finish, -1.0, finish.elevation);
    let to_top_left = corner(finish, 1.0, wall.top);
    let to_top_right = corner(finish, -1.0, wall.top);
    let element = |surface: WallSurface| Element::Wall(id, surface);
    for (surface, corners) in [
        (
            WallSurface::Side { left: true },
            [from_low, from_top_left, to_top_left, to_low],
        ),
        (
            WallSurface::Side { left: false },
            [from_floor, to_floor, to_top_right, from_top_right],
        ),
        (
            WallSurface::Bottom,
            [from_low, to_low, to_floor, from_floor],
        ),
        (
            WallSurface::Top,
            [from_top_left, from_top_right, to_top_right, to_top_left],
        ),
        (
            WallSurface::End { start: true },
            [from_low, from_floor, from_top_right, from_top_left],
        ),
        (
            WallSurface::End { start: false },
            [to_low, to_top_left, to_top_right, to_floor],
        ),
    ] {
        for indices in [
            [corners[0], corners[1], corners[2]],
            [corners[0], corners[2], corners[3]],
        ] {
            mesh.faces.push(BuildingFace {
                indices,
                element: element(surface),
            });
        }
    }
    Ok(())
}

/// Builds one placement's template in the frame its bay gives it, or `None` when it does not resolve.
///
/// # Invariants
///
/// **The template is built by the crate's own `build`, in its own coordinates, and then moved** — not
/// rebuilt in the wall's frame. A part is therefore as closed, as welded and as oriented as a mass or
/// a roof, because it went through the same code; what this function adds is one affine map and one
/// rewriting of provenance.
///
/// The map is the frame the placement's own documentation states: the template's `x` runs along the
/// edge, its `y` up, and its `z` **into the wall**, with `x × y = z`. The origin is the point on the
/// wall's **outer face** at the pin's arc length and elevation, so a part authored from its own outer
/// face lies in the wall's thickness rather than hanging outside it. The wall's frame comes from
/// [`WallFace::of`] — the same reading the wall itself is built from — so there is no second
/// derivation of what "along" or "inward" means for an edge.
///
/// A placement whose template holds no mass answers `None` from
/// [`BuildingSkeleton::placed`](crate::skeleton::BuildingSkeleton::placed) and is
/// [`Problem::PlacementUnresolved`], like one whose slot has gone.
/// Returns the frame a placement stands its template in, or `None` when the placement does not resolve.
///
/// # Invariants
///
/// **One frame, used by both paths**: `build` applies it to a template's own mesh, and a scene applies the
/// same frame to the same mesh, so an instanced part and a baked one are the same geometry rather than two
/// renderings that agree approximately. The frame is the wall's own `along` direction, `up`, and
/// `inward` across the wall's thickness, with the origin on the wall's **outer face** at the pin's arc
/// length and elevation — the three facts the parts documentation states, resolved once.
///
/// A placement whose wall cannot be offset is **the mass's fault**, and the problems are returned rather
/// than swallowed: a scene asking for a frame must hear about a wall the miter refused.
pub fn part_frame(
    skeleton: &BuildingSkeleton,
    index: usize,
) -> Result<Option<PartFrame>, Vec<Problem>> {
    let Some(placed) = skeleton.placed(index) else {
        return Ok(None);
    };
    let Some(placement) = skeleton.parts.placement(index) else {
        return Ok(None);
    };
    match placement.target {
        Target::Bay { face, .. } => {
            let Some(bay) = placed.bay else {
                return Ok(None);
            };
            // **A courtyard's bay resolves and its frame does not, so it is refused rather than
            // approximated.** The frame below is read from the wall's *outer* face, and for a
            // courtyard that is the offset loop rather than the loop the face is named by; a frame
            // taken from the named loop would stand the part in the courtyard's open air. That is a
            // part in a visible wrong place rather than no part, so the placement is named — see
            // [`Problem::PlacementOnACourtWall`] for why this is a variant of its own.
            if face.is_court() {
                return Err(vec![Problem::PlacementOnACourtWall { placement: index }]);
            }
            let Some(mass) = skeleton.mass(face.mass) else {
                return Ok(None);
            };
            let mut problems = Vec::new();
            let Some(inner) = offset_loop(
                &mass.outline,
                AuthoredLoop::Outline { mass: face.mass },
                mass.wall,
                mass.plan_area(),
                OffsetSide::Inward,
                &mut problems,
            ) else {
                return Err(problems);
            };
            let wall = WallFace::of(
                &mass.outline,
                &inner,
                mass.wall,
                mass.top,
                face.edge,
                OffsetSide::Inward,
            );
            let origin = lift(
                wall.outer_at(bay.from + placed.at.along),
                bay.low + placed.at.up,
            );
            let inside = inward(wall.along);
            Ok(Some(PartFrame::new(
                origin,
                [wall.along[0], 0.0, wall.along[1]],
                [0.0, 1.0, 0.0],
                [inside[0], 0.0, inside[1]],
            )))
        }
        // **A wall run is its own frame**: the run's direction is `along`, the ground **under the pin** is the
        // base a pin's `up` is measured from, and `inward` is the run's normal — so a part hung on a wall
        // spans the wall's thickness about the line the author drew, and needs no miter to resolve.
        //
        // The base is interpolated along the run rather than read at its start, because the run's own bottom
        // follows the ground between its two nodes: a gate hung on a wall that climbs a slope must stand on
        // the ground at the gate, not at the far end of the wall. Measured before it was understood — a gate
        // on a `0.25 x` slope hung 1.5 m under its own doorway when the base was the run's start.
        Target::Wall { wall } => {
            let Some(run) = skeleton.walls.get(wall.0) else {
                return Ok(None);
            };
            let Some(direction) = run.direction() else {
                return Ok(None);
            };
            let span = run.length().max(GRID_TOLERANCE);
            let climb = run.to.elevation - run.from.elevation;
            let base = run.from.elevation + climb * (placed.at.along / span).clamp(0.0, 1.0);
            let origin = lift(
                step(run.from.position, direction, placed.at.along),
                base + placed.at.up,
            );
            let inside = inward(direction);
            Ok(Some(PartFrame::new(
                origin,
                [direction[0], 0.0, direction[1]],
                [0.0, 1.0, 0.0],
                [inside[0], 0.0, inside[1]],
            )))
        }
    }
}

/// Builds one placement's template in the frame its bay gives it, or `None` when it does not resolve.
///
/// # Invariants
///
/// It is **the two pieces and nothing else**: the template's own mesh ([`Template::mesh`]) mapped by
/// [`part_frame`]. There is no second expansion of a template and no second frame somewhere in this file,
/// which is what lets a scene instance the same geometry without a chance of drifting from it.
fn expand_placement(
    skeleton: &BuildingSkeleton,
    index: usize,
    cache: &TemplateCache,
) -> Result<Option<(Vec<Vec3>, Vec<BuildingFace>)>, Vec<Problem>> {
    // The placement must resolve — its bay or its wall, its pin, its reach — before a frame can be read for
    // it, and the frame is what carries the template into the document's space.
    if skeleton.placed(index).is_none() {
        return Ok(None);
    }
    let Some(frame) = part_frame(skeleton, index)? else {
        return Ok(None);
    };
    // **The template's mesh comes from the cache, not from a second expansion.** A template's expansion is a
    // function of the template alone ([`Template::mesh`]), so expanding it once per document and standing the
    // result in every placement is the same geometry for a twelfth of the work — and because `build` uses the
    // cache too, the cached and the uncached readings cannot diverge: there is one path.
    let Some(held) = skeleton
        .parts
        .placement(index)
        .and_then(|placement| cache.mesh(placement.template))
    else {
        return Ok(None);
    };
    let held = match held {
        Ok(held) => held,
        Err(problems) => return Err(problems.clone()),
    };
    let points = held
        .points
        .iter()
        .map(|point| frame.apply(*point))
        .collect();
    let faces = held
        .faces
        .iter()
        .map(|face| BuildingFace {
            indices: face.indices,
            element: Element::Part {
                placement: index,
                surface: match face.element {
                    Element::Mass(id, surface) => PartSurface::Mass(id, surface),
                    Element::Roof(id, surface) => PartSurface::Roof(id, surface),
                    // A template holds masses, roofs, walls and slabs, and a nested part is not a thing
                    // this crate builds: this arm keeps the map total, and the part being spliced into
                    // keeps its own surface.
                    Element::Part { surface, .. } => surface,
                    Element::Wall(id, surface) => PartSurface::Wall(id, surface),
                    Element::Slab(id, surface) => PartSurface::Slab(id, surface),
                },
            },
        })
        .collect();
    Ok(Some((points, faces)))
}

/// One solid being built, in its own point numbering.
///
/// # Invariants
///
/// **Points are numbered from zero and spliced only once the solid is known to be whole.** A mass
/// whose miter fails on its last corner has points and faces that belong to no solid, and a mesh
/// carrying half a wall is worse than one carrying the problem instead. A roof's skin is the one
/// place that reads its own numbering back — the boundary walk needs the faces in a mesh — and it
/// reads them while the solid is still private.
///
/// **A courtyard's ring is a solid of this shape too**, and it is built in its own numbering and
/// handed back whole rather than written into the mass's mesh: the two share no point and no edge by
/// construction, and [`ring_walk`] is what builds each of them.
#[derive(Debug, Default)]
pub(crate) struct Solid {
    points: Vec<Vec3>,
    faces: Vec<BuildingFace>,
}

impl Solid {
    /// Adds a point and returns its index.
    fn point(&mut self, at: Vec3) -> u32 {
        self.points.push(at);
        (self.points.len() - 1) as u32
    }

    /// Adds a triangle, wound as it is given.
    fn triangle(&mut self, corners: [u32; 3], element: Element) {
        self.faces.push(BuildingFace {
            indices: corners,
            element,
        });
    }

    /// Adds a quad as two triangles, wound as it is given.
    fn quad(&mut self, corners: [u32; 4], element: Element) {
        self.triangle([corners[0], corners[1], corners[2]], element);
        self.triangle([corners[0], corners[2], corners[3]], element);
    }

    /// Adds a polygon as a fan of triangles from its first corner, wound as it is given.
    ///
    /// **The corner list is a boundary walk, so the fan keeps its winding.** A polygon that is not
    /// convex is still filled correctly here because the callers build one long strip: every point
    /// after the first two is a station on the far edge, in order, so each triangle of the fan is
    /// the band between two neighbouring points of the walk and none of them overlap. `O(n)` in the
    /// corners, which is what a strip of `k` bands costs whether it arrives as `k` quads or as one.
    fn polygon(&mut self, corners: &[u32], element: Element) {
        for index in 1..corners.len().saturating_sub(1) {
            self.triangle([corners[0], corners[index], corners[index + 1]], element);
        }
    }

    /// Returns whether a triangle of this solid's own points faces the way `up` names.
    ///
    /// The test is the up component of the right-handed normal, and **a facet with no area has no
    /// normal to read**: it keeps the order it was given, which is a fold against the facets beside
    /// it that were read from theirs, and a fold reads as a boundary rather than as closed. A
    /// surface built from one loop therefore takes its winding from the loop rather than from each
    /// facet — see [`polygon`], whose degenerate facets are why.
    fn looks(&self, corners: [u32; 3], up: bool) -> bool {
        let [a, b, c] = corners.map(|index| self.points[index as usize]);
        let normal = cross3(subtract(b, a), subtract(c, a))[1];
        if normal == 0.0 {
            true
        } else {
            (normal > 0.0) == up
        }
    }

    /// Adds a triangle wound so that its normal points the way `up` names.
    fn oriented_triangle(&mut self, corners: [u32; 3], element: Element, up: bool) {
        if self.looks(corners, up) {
            self.triangle(corners, element);
        } else {
            self.triangle([corners[0], corners[2], corners[1]], element);
        }
    }

    /// Adds a quad wound so that its normal points up, as two triangles.
    fn up_quad(&mut self, corners: [u32; 4], element: Element) {
        if self.looks([corners[0], corners[1], corners[2]], true) {
            self.quad(corners, element);
        } else {
            self.quad([corners[0], corners[3], corners[2], corners[1]], element);
        }
    }

    /// Appends this solid to a mesh, renumbering its points.
    fn splice(self, mesh: &mut BuildingMesh) {
        let base = mesh.points.len() as u32;
        mesh.points.extend(self.points);
        mesh.faces
            .extend(self.faces.into_iter().map(|face| BuildingFace {
                indices: face.indices.map(|index| index + base),
                element: face.element,
            }));
    }

    /// Returns this solid's own points and faces, for a caller that splices it as a whole.
    ///
    /// This is [`splice`](Self::splice)'s other shape: a courtyard's ring is a solid of its own and
    /// the mesh must not be able to hold half of it, so it is handed over whole and shifted by
    /// [`splice_expansion`] rather than added to face by face.
    pub(crate) fn into_parts(self) -> (Vec<Vec3>, Vec<BuildingFace>) {
        (self.points, self.faces)
    }
}

/// Returns `from` moved `amount` along `direction`.
fn step(from: Vec2, direction: Vec2, amount: f32) -> Vec2 {
    [
        from[0] + direction[0] * amount,
        from[1] + direction[1] * amount,
    ]
}

/// Returns the direction a wall is built along from an outline edge walked plan-first.
///
/// The outline is wound so that its [`Mass::plan_area`](crate::mass::Mass::plan_area) is positive,
/// which in the frame [`crate::math`] takes (`x` east, `z` south) puts the mass on the walker's
/// **right**, so the inward normal is the walk turned a quarter turn to the walker's right — the
/// map `(x, z) ↦ (-z, x)` in plan coordinates — and the wall moves into the mass, never out of it.
/// (The same convention read as "the interior is on the left" is only true when `z` is read as
/// north, which is not this frame's; see [`Mass`](crate::mass::Mass)'s own docs.)
fn inward(direction: Vec2) -> Vec2 {
    [-direction[1], direction[0]]
}

/// Which side of an authored loop a wall is built on.
///
/// # Invariants
///
/// **Two loops are walked by one mitre, and this is the only thing that differs between them.** A
/// mass's wall stands *inside* its outline and a courtyard's ring stands *outside* the void's loop,
/// which is the same offset with the normals turned around — so the walk is written once and the
/// two callers pass this, rather than being written twice and having to agree by luck. It also
/// decides the one check whose direction flips: an inward offset must enclose less than the loop it
/// came from and an outward one must enclose more, so the area test in [`offset_loop`] reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OffsetSide {
    /// Into the loop's own interior, where a mass's wall is built.
    Inward,
    /// Out of the loop's own interior, where a courtyard's ring is built.
    Outward,
}

impl OffsetSide {
    /// Returns the direction this side offsets a walked edge along.
    fn apply(self, inward: Vec2) -> Vec2 {
        match self {
            Self::Inward => inward,
            Self::Outward => [-inward[0], -inward[1]],
        }
    }
}

/// Expands one mass into the closed ring of its wall, plus the closed ring of every courtyard it
/// authored.
///
/// # Invariants
///
/// **The surfaces are all one edge in two, and the courtyards are not among them.** The mass's own
/// ring is the wall: the inner wall stops at the node's own elevation, which is where the plate's
/// top is, so the inner loop carries two faces at that level — the wall's and the base's — and the
/// loop below it does not exist. A courtyard is **another ring standing on the same plate**, built
/// the same way from its own two loops, and the two solids share nothing: not a vertex, not an
/// edge, not a face. That is what makes the courtyard free — the three-surface edge the shell model
/// could not close does not arise, because no face is shared in the first place.
///
/// **Both rings are built by the same walk, and the two differ only in which way they were offset.**
/// A mass's wall is built *inward* from its outline and a courtyard's *outward* from its void loop, so
/// [`Ring`] carries the side and everything downstream — the windings, which of the two faces is
/// called outer, which surfaces the faces name — reads it rather than assuming the mass's
/// arrangement. One walk, one set of windings, no second implementation to agree with.
///
/// **A void that cannot be drawn stops the mass, and the mass's own ring is withheld with it.** The
/// rings are built into private point lists and spliced only once every one of them is whole, which
/// is the whole-list-or-nothing rule this function already held for one solid; a document that
/// authored a courtyard gets the courtyard or the refusal, never a mass with a hole drawn as wall.
///
/// See [`ring_walk`] for the walk itself, [`cut_edge`] for the wall a hole is cut into, and
/// [`resolve_holes`] for what is refused before a single point is added.
fn expand_mass(
    skeleton: &BuildingSkeleton,
    mesh: &mut BuildingMesh,
    id: MassId,
    mass: &Mass,
) -> Result<(), Vec<Problem>> {
    let count = mass.outline.len();
    if count < 3 {
        return Err(vec![Problem::OutlineTooShort {
            mass: id,
            nodes: count,
        }]);
    }
    if mass.wall <= 0.0 || mass.slab <= 0.0 {
        return Err(vec![Problem::MassThickness {
            mass: id,
            wall: mass.wall,
            slab: mass.slab,
        }]);
    }
    let mut problems = Vec::new();
    let Some(inner) = offset_loop(
        &mass.outline,
        AuthoredLoop::Outline { mass: id },
        mass.wall,
        mass.plan_area(),
        OffsetSide::Inward,
        &mut problems,
    ) else {
        return Err(problems);
    };
    // **Every opening in the document is grouped once per mass, and each ring reads the one group
    // that names it.** A mass holds `1 + voids` rings and each may be cut, so grouping inside the
    // ring walk would walk the whole opening list that many times; grouping here is `O(openings)` and
    // the group is keyed by the exact face, which is what keeps a courtyard's hole out of the mass's
    // own wall.
    let grouped = skeleton.openings.by_face();
    let ring = Ring {
        loop_of: AuthoredLoop::Outline { mass: id },
        walked: &mass.outline,
        offset: &inner,
        wall: mass.wall,
        top: mass.top,
        side: OffsetSide::Inward,
    };
    let mut solid = Solid::default();
    ring_walk(&mut solid, &ring, skeleton, &grouped, &mut problems);
    // **A courtyard is built before anything is spliced, and refused with the mass.** The ring is
    // built in its own point numbering by the module that owns it, and only the ones that came back
    // whole are handed to the mesh — so a mass whose second courtyard cannot be drawn contributes
    // neither its first nor the wall around it.
    let mut courtyards = Vec::with_capacity(mass.voids.len());
    for void in 0..mass.voids.len() {
        match courtyard::ring(
            skeleton,
            &grouped,
            id,
            void,
            mass.wall,
            mass.top,
            &mut problems,
        ) {
            Ok(expanded) => courtyards.push(expanded),
            Err(found) => problems.extend(found),
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    solid.splice(mesh);
    for (points, faces) in courtyards {
        splice_expansion(mesh, points, faces);
    }
    Ok(())
}

/// One wall ring to be built: an authored loop, the loop `wall` metres away from it, and the side it
/// was offset to.
///
/// # Invariants
///
/// **A ring is the mass's own wall and a courtyard's wall, and the only fact that tells them apart is
/// which way the offset went.** Both are two loops of the same length standing on the same ground
/// line with the same top; a mass's is built inward from the outline and a courtyard's outward from
/// its void loop. Everything downstream reads [`side`](Self::authored_is_outer) rather than carrying
/// its own answer — which loop is the outward-facing wall face, and therefore which winding every
/// quad on it must be — because a walk that assumed the mass's arrangement would wind every face of
/// a courtyard backwards, and a courtyard with no hole in it is exactly the case that would have
/// looked right while it was wrong.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Ring<'a> {
    /// Which authored loop this ring is built from, which is what every fault about it names.
    pub(crate) loop_of: AuthoredLoop,
    /// The authored loop, normalised: wound so its plan area is positive.
    pub(crate) walked: &'a [PlanNode],
    /// The loop one wall's thickness away from it.
    pub(crate) offset: &'a [Vec2],
    /// The wall's thickness.
    pub(crate) wall: f32,
    /// The wall's top, which both loops stand at.
    pub(crate) top: f32,
    /// Which way the offset was taken from the authored loop.
    pub(crate) side: OffsetSide,
}

impl Ring<'_> {
    /// Returns whether the ring's **authored** loop is the face an outside viewer sees.
    ///
    /// A mass's wall is built inward, so its outline is the outer face and its offset the inner one. A
    /// courtyard's ring is built outward, so the void loop is the face that looks *into* the courtyard
    /// — the mass's **inner** face — and the offset loop is the one the rooms see. This is the single
    /// question every winding in [`cut_edge`] and [`ring_walk`] is derived from.
    fn authored_is_outer(&self) -> bool {
        matches!(self.side, OffsetSide::Inward)
    }

    /// Returns one edge of the ring read as the vertical strip its wall is built in.
    fn face(&self, edge: usize) -> WallFace {
        WallFace::of(
            self.walked,
            self.offset,
            self.wall,
            self.top,
            edge,
            self.side,
        )
    }

    /// Returns the four surfaces one edge of this ring is drawn on.
    fn surfaces(&self, edge: usize) -> RingSurfaces {
        RingSurfaces::of(self.loop_of, edge)
    }
}

/// The four surfaces one wall ring is drawn on, which is all a mass's own wall and a courtyard's
/// differ in once the loops they are between are the same two.
///
/// # Invariants
///
/// **They are named by the loop they sit on and not by which way they look**, because the way a face
/// looks follows from the side the ring was offset to and the loop does not.
/// [`Ring::authored_is_outer`] is what turns one into the other, in one place. A [`MassSurface`] is
/// itself mass-agnostic — the mass rides on the [`Element`] — so this carries four names and the walk
/// stamps the one mass every one of them belongs to.
#[derive(Debug, Clone, Copy)]
struct RingSurfaces {
    /// The wall face on the loop the author drew.
    authored: MassSurface,
    /// The wall face on the offset loop.
    offset: MassSurface,
    /// The strip of wall top between them.
    top: MassSurface,
    /// The annulus at the loops' own elevations.
    base: MassSurface,
}

impl RingSurfaces {
    /// Returns the surfaces one edge of a ring is drawn on.
    fn of(loop_of: AuthoredLoop, edge: usize) -> Self {
        match loop_of {
            AuthoredLoop::Outline { .. } => Self {
                authored: MassSurface::OuterWall { edge },
                offset: MassSurface::InnerWall { edge },
                top: MassSurface::WallTop { edge },
                base: MassSurface::WallBase,
            },
            AuthoredLoop::Void { void, .. } => Self {
                authored: MassSurface::VoidWall { void, edge },
                offset: MassSurface::VoidInner { void, edge },
                top: MassSurface::VoidTop { void, edge },
                base: MassSurface::VoidBase { void },
            },
        }
    }

    /// Returns the face one loop of the ring carries.
    fn of_loop(&self, loop_side: LoopSide) -> MassSurface {
        match loop_side {
            LoopSide::Authored => self.authored,
            LoopSide::Offset => self.offset,
        }
    }
}

/// Appends one expansion's own points and faces to the mesh, renumbering the faces.
///
/// # Invariants
///
/// **The expansion is welded before it is renumbered.** A template's mesh and a courtyard's ring are
/// both built in a numbering that starts at zero and is private to their own builder, so this is
/// the one place a shift is applied — once, at the end, and to the face indices only, which is why
/// two expansions can be spliced in either order and still weld at the points they share by
/// authored identity rather than by arithmetic.
pub(crate) fn splice_expansion(
    mesh: &mut BuildingMesh,
    points: Vec<Vec3>,
    faces: Vec<BuildingFace>,
) {
    let base = mesh.points.len() as u32;
    mesh.points.extend(points);
    mesh.faces
        .extend(faces.into_iter().map(|face| BuildingFace {
            indices: face.indices.map(|corner| corner + base),
            element: face.element,
        }));
}

/// Builds one wall ring into a solid: the two faces, the strip of wall top, and the annulus at the
/// loops' own elevations — with every hole the document cut into it already removed.
///
/// # Invariants
///
/// **One walk, two rings.** A mass's own wall and a courtyard's wall are the same shape between the
/// same two loops, and the only difference is which loop was authored and which way the offset went;
/// [`Ring`] carries that and every winding below is derived from it rather than from the mass's
/// arrangement. Nothing here reads a mass, a void, or a wall thickness of its own, so a third kind of
/// ring is a new [`Ring`] and not a new walk.
///
/// **The base is the wall top's strip, one plane down, over the two cut loops.** The cut loops carry
/// the stations an opening's reveal added, which is what lets the strip's edges pair with the walls'
/// bottoms; the raw node arrays do not have them, and a strip built from those leaves every reveal's
/// sill edge unpaired — measured on the shed, where each door's sill showed up in the open-edge dump.
/// The two cut loops have the same length because the stations come from the same cuts, and the notch
/// list belongs to the loop rather than to a face, so it is read once.
///
/// **The base's winding starts on the ring's outer face**, which is what makes the strip look down
/// for either ring: the two loops swap which one that is, and a strip wound from the authored loop
/// alone would be the underside of a courtyard's floor — a face with material on neither side of it.
///
/// **Every refusal lands in `problems` rather than in a return value**, so a ring with two bad holes
/// reports both and the caller decides what to do with the list; the whole-list-or-nothing rule is
/// the caller's, because it is the one that knows whether this ring is the only thing a mass has.
pub(crate) fn ring_walk(
    solid: &mut Solid,
    ring: &Ring<'_>,
    skeleton: &BuildingSkeleton,
    grouped: &BTreeMap<FaceRef, Vec<usize>>,
    problems: &mut Vec<Problem>,
) {
    let count = ring.walked.len();
    let mass = ring.loop_of.mass();
    let authored_is_outer = ring.authored_is_outer();
    let mut authored_base = Vec::with_capacity(count);
    let mut authored_top = Vec::with_capacity(count);
    let mut offset_base = Vec::with_capacity(count);
    let mut offset_top = Vec::with_capacity(count);
    for node in 0..count {
        // **The ring's bottom is one elevation, on both loops.** The plate is its own solid below it, so the wall
        // does not reach down to the plate's underside: a ring whose base sat a plate thickness lower on one face would make
        // its base strip a slanted band, and the band is a warped quad wherever an opening's stations land unevenly
        // (measured as `FaceNotPlanar { departure: 0.19 }` on the shed, which is half a plate). The two were a plate
        // thickness apart until 2026-10-27, and that difference is what warped every base strip quad.
        authored_base.push(solid.point(lift(
            ring.walked[node].position,
            ring.walked[node].elevation,
        )));
        authored_top.push(solid.point(lift(ring.walked[node].position, ring.top)));
        offset_base.push(solid.point(lift(ring.offset[node], ring.walked[node].elevation)));
        offset_top.push(solid.point(lift(ring.offset[node], ring.top)));
    }
    let faces: Vec<WallFace> = (0..count).map(|edge| ring.face(edge)).collect();
    let nodes: Vec<EdgeNode> = (0..count)
        .map(|node| EdgeNode {
            authored_base: authored_base[node],
            authored_top: authored_top[node],
            offset_base: offset_base[node],
            offset_top: offset_top[node],
        })
        .collect();
    let by_edge = resolve_holes(skeleton, grouped, ring, &faces, problems);
    if !problems.is_empty() {
        return;
    }
    // The stations each cut edge added to the ring's base loop, empty for a face with no hole, and
    // the bands of that loop where a doorway notches the base.
    let mut cuts: Vec<Vec<CutStation>> = vec![Vec::new(); count];
    let mut notches: Vec<Vec<bool>> = vec![Vec::new(); count];
    for edge in 0..count {
        let holes = &by_edge[edge];
        let next = (edge + 1) % count;
        let surfaces = ring.surfaces(edge);
        if holes.is_empty() {
            solid.quad(
                wound(
                    authored_is_outer,
                    [
                        authored_top[edge],
                        authored_top[next],
                        authored_base[next],
                        authored_base[edge],
                    ],
                ),
                Element::Mass(mass, surfaces.authored),
            );
            solid.quad(
                wound(
                    authored_is_outer,
                    [
                        authored_top[edge],
                        offset_top[edge],
                        offset_top[next],
                        authored_top[next],
                    ],
                ),
                Element::Mass(mass, surfaces.top),
            );
            solid.quad(
                wound(
                    authored_is_outer,
                    [
                        offset_top[next],
                        offset_top[edge],
                        offset_base[edge],
                        offset_base[next],
                    ],
                ),
                Element::Mass(mass, surfaces.offset),
            );
            continue;
        }
        match cut_edge(
            solid,
            &surfaces,
            ring,
            authored_is_outer,
            edge,
            &faces[edge],
            holes,
            nodes[edge],
            nodes[next],
            problems,
        ) {
            Ok((stations, bands)) => {
                cuts[edge] = stations;
                notches[edge] = bands;
            }
            Err(found) => problems.extend(found),
        }
    }
    if !problems.is_empty() {
        return;
    }
    let (walked_loop, _) = split_loop(&authored_base, &cuts, &notches, |station| station.authored);
    let (offset_loop, base_notched) =
        split_loop(&offset_base, &cuts, &notches, |station| station.offset);
    // **The annulus is wound from the ring's outer face across to its inner one**, and which loop that
    // is follows from the side the ring was offset to rather than from which loop was authored.
    let (outer, inner) = if authored_is_outer {
        (&walked_loop, &offset_loop)
    } else {
        (&offset_loop, &walked_loop)
    };
    for edge in 0..outer.len() {
        // **A doorway notches the base, so there is no strip across it.** The wall has no material at the
        // ground line over that span, and a strip drawn there would be a face with material on neither
        // side of it — one whose four edges are claimed by the two walls' bottoms and the two jambs'.
        if base_notched[edge] {
            continue;
        }
        let next = (edge + 1) % outer.len();
        solid.quad(
            [outer[edge], outer[next], inner[next], inner[edge]],
            Element::Mass(mass, ring.surfaces(0).base),
        );
    }
    if std::env::var("SHED_DEBUG").is_ok() {
        report_unclosed(solid, ring.loop_of, outer.len());
    }
}

/// Prints every edge of a ring that has no opposite, naming the face that drew it.
///
/// # Invariants
///
/// **It reads the welded indices and not the coordinates**, because that is the same question
/// [`BuildingMesh::boundary_edges_of`] asks and the same answer: a face in the wrong place still meets
/// its neighbours, so a closure dump taken before this slice found nothing on a courtyard built the
/// other way round. It is behind an environment variable because a probe is the instrument here, not
/// a log line.
fn report_unclosed(solid: &Solid, loop_of: AuthoredLoop, nodes: usize) {
    let mut used: std::collections::HashMap<(u32, u32), usize> = std::collections::HashMap::new();
    for face in &solid.faces {
        for edge in [
            (face.indices[0], face.indices[1]),
            (face.indices[1], face.indices[2]),
            (face.indices[2], face.indices[0]),
        ] {
            *used.entry(edge).or_insert(0) += 1;
        }
    }
    for (edge, count) in &used {
        if count > &used.get(&(edge.1, edge.0)).copied().unwrap_or(0) {
            let owner = solid.faces.iter().find(|face| {
                let held = face.indices;
                (0..3).any(|at| (held[at], held[(at + 1) % 3]) == *edge)
            });
            eprintln!(
                "OPEN {loop_of:?} owner {:?} {:?} -> {:?}",
                owner.map(|face| face.element),
                solid.points[edge.0 as usize],
                solid.points[edge.1 as usize]
            );
        }
    }
    let bases = solid
        .faces
        .iter()
        .filter(|face| match face.element {
            Element::Mass(_, MassSurface::WallBase)
            | Element::Mass(_, MassSurface::VoidBase { .. }) => true,
            _ => false,
        })
        .count();
    eprintln!("BASES {bases} emitted for a loop of {nodes} walls");
}

/// Returns one face's walk of the ring's base loop: that face's own bottom points with every cut
/// station written into it, and the spans over which the base is notched.
///
/// # Invariants
///
/// **A cut wall's own bottom edge is several edges, so the base it meets has to be several too.**
/// The wall over one band ends on the segment between two stations, and nothing else uses that
/// segment; a base walked node to node has one long edge there instead, and the segment is then a
/// boundary. The stations are therefore **collinear nodes of the loop**, which is the same state
/// [`ops::attach_mass`](crate::ops::attach_mass) leaves behind, and each carries the point of the
/// face it belongs to: the strip's edge and the wall's bottom edge are one edge by index, never by
/// two points standing at the same coordinates.
///
/// The stations arrive in the loop's own order because they are walked from the node they follow,
/// so one pass over the nodes reads both faces in the order the loop was authored in. **The
/// notch answers a span and not a point**, because a doorway notches the base between its two
/// jambs; a node opens the first band of the edge that starts at it, and the loop's closing span
/// belongs to the edge that ends at its last node.
fn split_loop(
    corners: &[u32],
    cuts: &[Vec<CutStation>],
    notches: &[Vec<bool>],
    station_of: impl Fn(&CutStation) -> u32,
) -> (Vec<u32>, Vec<bool>) {
    let mut welded = Vec::with_capacity(corners.len());
    let mut notched = Vec::with_capacity(corners.len());
    for (node, corner) in corners.iter().enumerate() {
        welded.push(*corner);
        notched.push(
            if node + 1 == corners.len() {
                notches[node].last().copied()
            } else {
                notches[node].first().copied()
            }
            .unwrap_or(false),
        );
        for (station, band) in cuts[node].iter().zip(notches[node].iter().skip(1)) {
            welded.push(station_of(station));
            notched.push(*band);
        }
    }
    (welded, notched)
}

/// Resolves every opening cut into one authored wall ring, and reports every way the document's
/// openings cannot be cut.
///
/// # Invariants
///
/// **The ring reads its own face and no other.** `grouped` is the whole document's openings keyed by
/// the exact [`FaceRef`], and a ring looks up the faces its own loop names — so a hole in a
/// courtyard's wall can never be cut into the mass's own wall two edges away, which is what a filter
/// on the mass alone would have allowed once a courtyard's faces became addressable.
///
/// **The two lookups are read as a pair, and both come from the document.** `aperture` answers the
/// rectangle a hole cuts and `None` for one that cannot be cut at all; `unresolved_openings` and
/// `apertures_of` are this crate's stated pair of companions, and this walk composes them from the
/// single definition both of them are built on, so there is one answer to "does this opening resolve"
/// rather than two that could drift.
///
/// **A refused ring carries none of its holes.** Nothing here writes to the solid, and the caller
/// splices only once the whole mass is built, so a wall that cannot be cut is absent rather than
/// half present.
fn resolve_holes(
    skeleton: &BuildingSkeleton,
    grouped: &BTreeMap<FaceRef, Vec<usize>>,
    ring: &Ring<'_>,
    faces: &[WallFace],
    problems: &mut Vec<Problem>,
) -> Vec<Vec<Hole>> {
    let mut by_edge: Vec<Vec<Hole>> = vec![Vec::new(); faces.len()];
    // **One pass over the document's own group for this ring**, rather than a lookup per edge: the
    // group is keyed by the exact face, so an opening that names an edge this loop does not have is
    // seen here instead of being missed by every edge's lookup — which is the difference between a
    // named refusal and a hole that was quietly never cut.
    for (face, indices) in grouped {
        if face.mass != ring.loop_of.mass() || face.void != ring.loop_of.void() {
            continue;
        }
        let Some(wall) = faces.get(face.edge) else {
            for index in indices {
                problems.push(Problem::OpeningUnresolved {
                    loop_of: ring.loop_of,
                    edge: face.edge,
                    opening: *index,
                });
            }
            continue;
        };
        for index in indices {
            // **One definition of "resolved"**, which is [`BuildingSkeleton::aperture`]: a slot the grid
            // no longer holds and insets that ate the bay are the same statement, and this walk needs both
            // halves of the answer — the rectangle to cut and the refusal to report.
            match skeleton.aperture(*index) {
                None => problems.push(Problem::OpeningUnresolved {
                    loop_of: ring.loop_of,
                    edge: face.edge,
                    opening: *index,
                }),
                Some((_, extents)) => {
                    match resolve_one(ring.loop_of, face.edge, *index, extents, wall) {
                        Ok(hole) => by_edge[face.edge].push(hole),
                        Err(problem) => problems.push(problem),
                    }
                }
            }
        }
    }
    // Two holes sharing wall remove it twice, and the complement of two overlapping rectangles is
    // not a set of bands at all. Touching is not sharing — `Extents::overlaps` says so and the
    // expander reads it rather than re-deriving it — so a divided facade opened bay by bay cuts.
    // Each unordered pair once, in the document's own order.
    for edge in 0..by_edge.len() {
        for left in 0..by_edge[edge].len() {
            for right in left + 1..by_edge[edge].len() {
                if by_edge[edge][left]
                    .outer
                    .overlaps(&by_edge[edge][right].outer)
                {
                    let (first, second) =
                        ordered(by_edge[edge][left].opening, by_edge[edge][right].opening);
                    problems.push(Problem::OpeningsOverlap {
                        loop_of: ring.loop_of,
                        edge,
                        left: first,
                        right: second,
                    });
                }
            }
        }
    }
    by_edge
}

/// Returns two indices in the order they were authored.
fn ordered(first: usize, second: usize) -> (usize, usize) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}

/// Resolves one hole against the wall it is cut into, or reports how it leaves the strip.
///
/// # Invariants
///
/// **A hole must be strictly inside the strip, on both faces, and the two tests are not the same
/// test.** Across, the strip is the edge's own length and the offset edge's own length; a mitre moves
/// both ends of the offset edge, so a hole that clears the authored strip can still project off the
/// other one, and it is measured where it lands rather than assumed to have travelled with it. Down,
/// the binding floor is the **offset** wall's own, because that is the line the ring's base annulus
/// meets: a sill below it is a hole under the building, and a sill standing on it is a door.
///
/// **A sill may stand exactly on the offset wall's own floor, and that is a door.** The band below
/// it is then empty rather than inverted, the reveal's inner edge lies in the base's own plane,
/// and the base's edge closes it — no second mechanism and no special case in the walk. What is
/// refused is a sill below the ground line at either of the hole's own jambs, which is the same
/// answer read at two places because the line is straight. `past` is the greatest of every way the
/// hole leaves the strip, so a hole wrong in two ways names the worse one.
fn resolve_one(
    loop_of: AuthoredLoop,
    edge: usize,
    opening: usize,
    extents: Extents,
    wall: &WallFace,
) -> Result<Hole, Problem> {
    let past_start = GRID_TOLERANCE - extents.from;
    let past_end = extents.to - (wall.length - GRID_TOLERANCE);
    let past_head = GRID_TOLERANCE - (wall.top - extents.high);
    // How far the sill is below the ground line at the worse of the hole's two jambs, which is the
    // whole of that test: a sill at or above the ground everywhere is legal, a sill exactly on it
    // is a door, and both ends are the whole test because the line is straight between them.
    let below_ground = wall
        .offset_floor_at(extents.from)
        .max(wall.offset_floor_at(extents.to))
        - extents.low;
    // The hole's two jambs, projected onto the offset edge's line and measured against the ends the
    // miter left it: the projection moves nothing along the edge, but the offset edge is shorter than
    // the authored one at both ends, and a jamb that lands at or past one of them would put its reveal
    // on a corner the neighbouring wall still claims whole.
    let past_offset = (GRID_TOLERANCE + wall.offset_from - extents.from)
        .max(extents.to + GRID_TOLERANCE - wall.offset_to);
    let past = past_start
        .max(past_end)
        .max(past_head)
        .max(below_ground)
        .max(past_offset);
    if past > 0.0 {
        return Err(Problem::OpeningOutsideWall {
            loop_of,
            edge,
            opening,
            past,
        });
    }
    Ok(Hole {
        opening,
        outer: extents,
    })
}

/// One edge of an authored loop read as the vertical strip its wall is built in.
///
/// # Invariants
///
/// **`u` is arc length along the authored edge, and it is also arc length along the offset edge's own
/// line — the two are the same number, while the offset edge itself is shorter at both ends.** The
/// miter moves the offset edge's *ends*: a right angle takes one wall's thickness off each, and a
/// sharper corner takes more, so the offset loop's edge runs from `offset_from` to `offset_to` rather
/// than from `0` to the authored edge's length. Projecting a point of one face onto the other edge's
/// line is therefore one dot product and moves nothing along the edge — the projection is the identity
/// in `u` — while *where the other edge is* is what the miter moved. Reading the offset rim as the
/// authored rim shifted by a constant would move the hole's jambs; reading it as the projection and
/// then refusing a hole that lands off the offset edge's own ends is what the geometry says.
///
/// The ground line is the whole of the wall's bottom, on both faces, and everything about a door
/// follows from that. The plate is a solid of its own beneath the ring, so the wall does not reach
/// down to its underside: a cut station whose floor stood a plate thickness below the authored
/// corners would draw the ring's base as a sawtooth across every opening, and the base strip would
/// be a warped plane rather than the ground. The two faces still read the ground line on two lines
/// of their own — see [`WallFace::offset_floor_at`] — which is a fact about the mitred corners and
/// not about the plate.
///
/// **The loops are named by the walk, not by which way they look.** `origin` is the authored loop and
/// `offset_origin` the one a wall's thickness away, whichever of the two is the ring's outward-facing
/// face; [`Ring::authored_is_outer`] is what says that, and nothing here assumes it.
#[derive(Debug, Clone, Copy)]
struct WallFace {
    /// Where the authored edge starts, in plan.
    origin: Vec2,
    /// The unit direction the authored edge runs.
    along: Vec2,
    /// Where the authored edge's line meets the offset edge's line: the authored edge shifted along
    /// whichever way this ring's offset went by the wall's thickness.
    offset_origin: Vec2,
    /// How long the authored edge is.
    length: f32,
    /// Where the offset edge starts along its own line, which the miter moved off the authored edge's
    /// start.
    offset_from: f32,
    /// Where the offset edge ends along its own line, which the miter moved off the authored edge's
    /// end.
    offset_to: f32,
    /// The ground line's own two elevations, at the authored edge's two ends.
    ground: [f32; 2],
    /// The wall top.
    top: f32,
}

impl WallFace {
    /// Reads one edge of an authored loop as the strip its wall is built in.
    ///
    /// **The side is the only difference between the two rings**: a mass's offset lies along the loop's
    /// inward normal and a courtyard's along the outward one, which is exactly
    /// [`OffsetSide::apply`] already computes for the offset walk, so the strip and the loop it is
    /// offset from cannot disagree about which way "into the wall" is.
    fn of(
        loop_nodes: &[PlanNode],
        offset: &[Vec2],
        wall: f32,
        top: f32,
        edge: usize,
        side: OffsetSide,
    ) -> Self {
        let count = loop_nodes.len();
        let next = (edge + 1) % count;
        let origin = loop_nodes[edge].position;
        let far = loop_nodes[next].position;
        let along = normalized(difference(far, origin));
        Self {
            origin,
            along,
            offset_origin: step(origin, side.apply(inward(along)), wall),
            length: distance(far, origin),
            offset_from: dot(difference(offset[edge], origin), along),
            offset_to: dot(difference(offset[next], origin), along),
            ground: [loop_nodes[edge].elevation, loop_nodes[next].elevation],
            top,
        }
    }

    /// Returns the ground line's elevation at an arc length along the **authored** edge.
    ///
    /// The line is straight between the edge's two nodes, which is what makes both ends the whole
    /// test: a linear function attains its extremes on an interval at the interval's ends.
    ///
    /// **This is one of the wall's own faces' floor at every station, the two corners included**,
    /// because the ring's base is one elevation over the whole loop: `ring_walk` puts the authored
    /// corners on this line, and a station that read anything else would draw the base annulus across
    /// an opening as a band from one level to another.
    fn ground_at(&self, along: f32) -> f32 {
        self.ground[0] + (self.ground[1] - self.ground[0]) * (along / self.length)
    }

    /// Returns the floor the offset wall stands on: the ground line itself, because that is where the
    /// ring's base is.
    ///
    /// **Read in the offset edge's own arc length, not the authored one's.** The offset wall's bottom
    /// edge runs between the two *offset* corners at the two *nodes'* elevations, so on a wall whose
    /// ground rises it is a different line from the authored wall's: its slope is the ground's rise
    /// spread over the offset edge's own length. Reading the same elevation at the same arc length for
    /// both faces would redraw the offset wall's floor along a line the un-cut wall does not have, and
    /// a ring would then hold a different amount of material depending on whether a window had been
    /// cut in it — a difference in volume with nothing in the geometry to account for it.
    fn offset_floor_at(&self, along: f32) -> f32 {
        let inside = along.clamp(self.offset_from, self.offset_to);
        let span = self.offset_to - self.offset_from;
        self.ground[0] + (self.ground[1] - self.ground[0]) * ((inside - self.offset_from) / span)
    }

    /// Returns the plan position of an arc length along the authored edge.
    fn outer_at(&self, along: f32) -> Vec2 {
        step(self.origin, self.along, along)
    }

    /// Returns the plan position of an arc length along the authored edge, on the offset edge.
    ///
    /// This is the **projection of the authored rim's corner onto the offset edge**, and it is the
    /// whole of what the offset face needs: one dot product, and exact because the two lines are
    /// parallel — the offset moves a line along its own normal, and a perpendicular projection puts
    /// the point straight back at the same arc length. The arc length handed in is the authored edge's,
    /// so one number describes a station on both faces of the wall, and that is what welds a hole's
    /// rim on one face to the reveal leaving it on the other.
    ///
    /// **The projection is onto a segment and not onto a line**, because the offset wall stops at the
    /// two mitred corners. The two ends of the walk's own station list are those corners' authored
    /// stations, which the projection puts a miter's reach beyond the offset edge's own ends; clamped
    /// they land on the corner itself, which is where the node loop already put the floor and the top
    /// and where the neighbouring wall's offset face ends. No hole is affected: a hole is refused
    /// unless it lands strictly inside that segment.
    fn offset_at(&self, along: f32) -> Vec2 {
        let corner = dot(difference(self.outer_at(along), self.origin), self.along);
        let inside = corner.clamp(self.offset_from, self.offset_to);
        step(self.offset_origin, self.along, inside)
    }
}

/// One opening resolved against the wall it is cut into.
///
/// # Invariants
///
/// **A hole is one rectangle, and the inner face's rim is where that rectangle projects rather than
/// a second copy of it.** The station table holds one column per break in arc length and one row per
/// elevation, for both faces at once, and the inner face's column is the projection — see
/// [`WallFace::inner_at`] — so there is no second `Extents` that could disagree with the first about
/// where a jamb lands. What the projection has to satisfy is checked once, where the hole is
/// resolved: a jamb that lands off the inner edge's own ends would leave the inner wall's bands
/// reaching past the corners that close them.
///
/// A door carries no flag: it is a hole whose sill stands on the inner wall's own floor, and the
/// band walk finds that by measuring rather than by being told.
#[derive(Debug, Clone, Copy)]
struct Hole {
    /// The opening's index in the document's list, which is what a reveal names.
    opening: usize,
    /// The rectangle it cuts in the outer face's own coordinates, which is the wall's own frame.
    outer: Extents,
}

/// Which of a ring's two loops a face is drawn on.
///
/// # Invariants
///
/// **The loop is the fact the walk winds by, and it is not the same as which way the face looks.** A
/// mass's authored loop *is* its outward-facing wall; a courtyard's authored loop is the face that
/// looks into the courtyard, and its outward-facing wall is the offset loop. Naming the two by their
/// loops is what lets one set of windings serve both: [`gap`] and [`corner_band`] ask for the loop and
/// derive the winding, and [`Ring::authored_is_outer`] is the one place the swap is decided — by
/// [`RingSurfaces::of_loop`], which turns a loop into the name the mesh carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopSide {
    /// The loop the author drew.
    Authored,
    /// The loop a wall's thickness away from it.
    Offset,
}

impl LoopSide {
    /// Returns the elevation this loop's own floor stands at an arc length along the authored edge.
    ///
    /// The two floors are different lines wherever the ground slopes, because the offset loop's span is
    /// shorter than the authored one and the miter moved its ends — see [`WallFace::offset_floor_at`]
    /// for why reading one number for both would make a ring hold a different amount of material
    /// depending on whether a window had been cut in it.
    fn floor_at(self, wall: &WallFace, along: f32) -> f32 {
        match self {
            Self::Authored => wall.ground_at(along),
            Self::Offset => wall.offset_floor_at(along),
        }
    }

    /// Returns the corners of one band, wound so the face looks out of the material.
    ///
    /// **The windings differ between the two loops and not between the two rings, because which loop
    /// carries the outward-facing wall has already been asked once** — `authored_is_outer` says which
    /// of these two orders the ring needs, and a courtyard's are both the other way round.
    fn corners(
        self,
        authored_is_outer: bool,
        near: &[u32],
        far: &[u32],
        low: usize,
        high: usize,
    ) -> [u32; 4] {
        wound(
            authored_is_outer,
            match self {
                Self::Authored => [near[high], far[high], far[low], near[low]],
                Self::Offset => [far[high], near[high], near[low], far[low]],
            },
        )
    }
}

/// Returns one quad's four corners, wound for whichever loop carries the ring's outward face.
///
/// # Invariants
///
/// **Every quad in a ring walk spans the wall's thickness, and which way it must wind is one fact
/// about the ring, not one fact per face.** A mass's authored loop carries the outward face, so the
/// walk's orders are written once for that case; a courtyard's authored loop is the inward one, so its
/// orders are the same lists **reversed**. That is the whole of it: reversing a quad's corners flips
/// its normal and flips nothing else, so one reversal per quad and one flag per ring is enough — and a
/// face that is right on a mass and inside-out on a courtyard would still close, still be manifold,
/// still be counted and still be of positive volume, so [`SolidInsideOut`](crate::validate::Violation::SolidInsideOut)
/// is the only thing in the crate that could ever have said so.
fn wound(authored_is_outer: bool, corners: [u32; 4]) -> [u32; 4] {
    if authored_is_outer {
        corners
    } else {
        [corners[3], corners[2], corners[1], corners[0]]
    }
}

/// One station of a cut wall: the points of both loops at every elevation the cut uses.
///
/// # Invariants
///
/// **Index `0` is the loop's own floor — the ground line, on both loops — and index `1` is the wall
/// top, and a hole's sill that stands on the ground is that floor point itself.** The three are
/// welded here rather than compared afterwards: a doorway's sill is the wall's own bottom edge, the
/// ring's base is notched across the opening rather than drawn under it, and a second point at the
/// same place would put that edge on the surface once and nowhere else. Every other elevation is a
/// point of its own, and the same index is what both the wall's bands and the two reveals that
/// border it read.
struct StationPoints {
    /// The authored loop's points: the ground line, the wall top, then every hole elevation in
    /// [`cuts`](../cuts).
    authored: Vec<u32>,
    /// The offset loop's, in the same order.
    offset: Vec<u32>,
}

impl StationPoints {
    /// Returns this station's own points on one loop.
    fn pick(&self, loop_side: LoopSide) -> &[u32] {
        match loop_side {
            LoopSide::Authored => &self.authored,
            LoopSide::Offset => &self.offset,
        }
    }

    /// Returns whether one end of a span stands on the wall's own floor at that row, on both loops.
    ///
    /// **The test is on the welded indices and not on the two elevations**, because the walk welds a
    /// sill that sits on a floor into that floor's own point: equal indices *are* equal elevations here,
    /// and comparing the numbers again would compare what the walk has already decided. One end is one
    /// reading; the span stands only when both of its ends do, which is what keeps the base's notch and
    /// the absence of a sill one decision rather than two.
    fn stands_on_floor(&self, row: usize) -> bool {
        self.authored[row] == self.authored[0] && self.offset[row] == self.offset[0]
    }
}

/// One station a cut added to an edge, as the two points the ring's base walk needs there.
///
/// # Invariants
///
/// **A station is one elevation on both loops of one edge, one point each**, and the base walk
/// reads each through the loop it is drawing. Both are the walls' own floor points, made where the
/// station is and never found by comparing coordinates afterwards — a base walk that rebuilt them
/// would put a second vertex at one elevation and the ring would open along its bottom.
#[derive(Debug, Clone, Copy)]
struct CutStation {
    /// The authored loop's own floor point there, which is one of the base annulus's nodes.
    authored: u32,
    /// The offset loop's own floor point there, which is the other.
    offset: u32,
}

/// One end of an edge: the four authored points of its node.
///
/// # Invariants
///
/// **A cut reuses these rather than making its own copies of them.** The first and last of an
/// edge's stations are its own two nodes, and the ring, the base and the top strip all meet there
/// through the points the node loop made — the neighbouring edge's wall claims the same corner
/// points, and a second point at the same coordinates would leave the ring's two ends meeting
/// nothing: a corner with no face across it, which reads as a boundary rather than as a corner.
#[derive(Debug, Clone, Copy)]
struct EdgeNode {
    /// The authored loop's floor point at this node.
    authored_base: u32,
    /// The authored loop's top point at this node.
    authored_top: u32,
    /// The offset loop's floor point at this node.
    offset_base: u32,
    /// The offset loop's top point at this node.
    offset_top: u32,
}

/// Cuts one wall into the complement of its holes, and returns the stations it split.
///
/// # Invariants
///
/// **The break points in `u` are the edge's own ends and every hole's two jambs, and the break
/// points in elevation are the wall top and every hole's sill and head — the whole wall's, not the
/// band's.** The first is the decomposition the holes ask for. The second is what makes the
/// decomposition close, and it is not a refinement of the first: a band beside a hole is one quad
/// from the ground line to the wall top while the hole's own band is two, so at the hole's jamb the
/// one quad's vertical edge runs the whole height and the two run its halves. The three are not
/// the same edges, every one of them is used once, and the wall reads open with no problem of any
/// kind. Splitting every band at every elevation the wall names makes both sides of each jamb walk
/// the same segments, so each is used exactly twice, and a band beside a hole comes out as several
/// stacked quads that share the band's two `u` values with the band next to them. The cost is
/// faces: a wall of `n` holes makes `O(n^2)` of them, which is the shape of the constraint and not
/// an accident of the walk.
///
/// **A hole's corners are one point each, and the reveal leaving it reads the same indices.** The
/// wall's bands and the two reveals that border each rim corner are built from the station's own
/// table, so nothing is compared by coordinate to decide that two faces meet.
///
/// **The ground line may not cross one of the wall's own elevations inside a band.** A band whose
/// floor rises past a hole's sill before the band ends would fold that quad on itself — a bow tie,
/// which no closure check reads and which a fan triangulation would fill inside out. Refused by
/// name rather than clamped, because a clamped wall is a wall in a place the author did not draw.
///
/// **The wall top, the floor and the base annulus follow the cut rather than being authored twice.**
/// Each band of the wall ends on its own top segment and its own floor segment, and each of those is
/// used by exactly one face of the ring — the top strip and the annulus — so the three are built band
/// by band over the same stations. That is what the stations returned here are for.
fn cut_edge(
    solid: &mut Solid,
    surfaces: &RingSurfaces,
    ring: &Ring<'_>,
    authored_is_outer: bool,
    edge: usize,
    wall: &WallFace,
    holes: &[Hole],
    near: EdgeNode,
    far: EdgeNode,
    problems: &mut Vec<Problem>,
) -> Result<(Vec<CutStation>, Vec<bool>), Vec<Problem>> {
    let mass = ring.loop_of.mass();
    let mut stations = vec![0.0, wall.length];
    let mut cuts = Vec::with_capacity(2 * holes.len());
    for hole in holes {
        stations.push(hole.outer.from);
        stations.push(hole.outer.to);
        cuts.push(hole.outer.low);
        cuts.push(hole.outer.high);
    }
    // Sorted and deduped by value rather than by tolerance: two holes that meet exactly along one
    // jamb are one station and one elevation, which is the divided facade's own case, and two that
    // miss each other by a rounding error leave a sliver band — thin, positive, and closed, which is
    // what any band between two breaks is.
    stations.sort_by(f32::total_cmp);
    stations.dedup();
    cuts.sort_by(f32::total_cmp);
    cuts.dedup();

    for loop_side in [LoopSide::Authored, LoopSide::Offset] {
        for band in 0..stations.len() - 1 {
            let low = loop_side
                .floor_at(wall, stations[band])
                .min(loop_side.floor_at(wall, stations[band + 1]));
            let high = loop_side
                .floor_at(wall, stations[band])
                .max(loop_side.floor_at(wall, stations[band + 1]));
            for hole in holes {
                for elevation in [hole.outer.low, hole.outer.high] {
                    if elevation > low && elevation < high {
                        problems.push(Problem::GroundCrossesOpening {
                            loop_of: ring.loop_of,
                            edge,
                            opening: hole.opening,
                        });
                        return Ok((Vec::new(), Vec::new()));
                    }
                }
            }
        }
    }

    let mut points: Vec<StationPoints> = Vec::with_capacity(stations.len());
    let mut added: Vec<CutStation> = Vec::with_capacity(stations.len());
    let mut notches: Vec<bool> = Vec::with_capacity(stations.len() - 1);
    let last = stations.len() - 1;
    for (index, station) in stations.iter().enumerate() {
        let mut authored = Vec::with_capacity(cuts.len() + 2);
        let mut offset = Vec::with_capacity(cuts.len() + 2);
        let node = match index {
            0 => Some(near),
            index if index == last => Some(far),
            _ => None,
        };
        match node {
            Some(node) => {
                authored.push(node.authored_base);
                offset.push(node.offset_base);
                authored.push(node.authored_top);
                offset.push(node.offset_top);
            }
            None => {
                authored.push(solid.point(lift(wall.outer_at(*station), wall.ground_at(*station))));
                offset.push(solid.point(lift(
                    wall.offset_at(*station),
                    wall.offset_floor_at(*station),
                )));
                authored.push(solid.point(lift(wall.outer_at(*station), wall.top)));
                offset.push(solid.point(lift(wall.offset_at(*station), wall.top)));
                // A hole's sill that stands on one of the two floors **is** that floor's point,
                // welded by the walk rather than found by comparing coordinates afterwards. The
                // comparison is exact because the band walk reads the same two numbers: a door's
                // sill is the wall's own bottom edge on both loops, and the base's notch there is
                // what the annulus's edge closes. A corner station carries no such row: its vertical
                // line is one edge, which is what the neighbouring wall still draws.
                for cut in &cuts {
                    authored.push(if *cut == wall.ground_at(*station) {
                        authored[0]
                    } else {
                        solid.point(lift(wall.outer_at(*station), *cut))
                    });
                    offset.push(if *cut == wall.offset_floor_at(*station) {
                        offset[0]
                    } else {
                        solid.point(lift(wall.offset_at(*station), *cut))
                    });
                }
            }
        }
        let base = authored[0];
        let other = offset[0];
        points.push(StationPoints { authored, offset });
        if index > 0 && index + 1 < stations.len() {
            added.push(CutStation {
                authored: base,
                offset: other,
            });
        }
    }

    for band in 0..stations.len() - 1 {
        let start = stations[band];
        let end = stations[band + 1];
        let covering: Vec<&Hole> = holes
            .iter()
            .filter(|hole| hole.outer.from <= start && hole.outer.to >= end)
            .collect();
        let levels = band_levels(wall, &cuts, &covering, start, end);
        // **A band whose opening has its sill on the ground is a doorway, and the ring's base is
        // notched across it.** The wall has no material at the ground line over that span, so there
        // is no base to draw there either, and the notch's four edges are the two walls' own bottom
        // edges and the two jambs' own bottom edges — each used once, by the face that owns it. The
        // test is on the welded indices for the reason [`gap`] gives: the walk welds a sill that
        // stands on a floor into that floor's own point, so equal indices *are* equal elevations.
        notches.push(covering.first().is_some_and(|hole| {
            let sill = cut_index(&cuts, hole.outer.low);
            points[band].stands_on_floor(sill) && points[band + 1].stands_on_floor(sill)
        }));
        for loop_side in [LoopSide::Authored, LoopSide::Offset] {
            let element = Element::Mass(mass, surfaces.of_loop(loop_side));
            let starts_at_corner = band == 0;
            let ends_at_corner = band + 2 == stations.len();
            if starts_at_corner || ends_at_corner {
                let (corner, other) = if starts_at_corner {
                    (band, band + 1)
                } else {
                    (band + 1, band)
                };
                let corners = corner_band(
                    loop_side,
                    authored_is_outer,
                    ends_at_corner,
                    points[corner].pick(loop_side),
                    levels.of(loop_side),
                    points[other].pick(loop_side),
                );
                solid.polygon(&corners, element);
            } else {
                let near = points[band].pick(loop_side);
                let far = points[band + 1].pick(loop_side);
                for (low, high) in levels.of(loop_side) {
                    gap(
                        solid,
                        loop_side,
                        authored_is_outer,
                        near,
                        far,
                        *low,
                        *high,
                        element,
                    );
                }
            }
        }
        solid.quad(
            wound(
                authored_is_outer,
                [
                    points[band].authored[1],
                    points[band].offset[1],
                    points[band + 1].offset[1],
                    points[band + 1].authored[1],
                ],
            ),
            Element::Mass(mass, surfaces.top),
        );
    }

    for hole in holes {
        let from = station_index(&stations, hole.outer.from);
        let to = station_index(&stations, hole.outer.to);
        let low = cut_index(&cuts, hole.outer.low);
        let high = cut_index(&cuts, hole.outer.high);
        // **A jamb is split at every cut of its wall, because the wall's own faces are.** The cut
        // elevations are global to the wall — that is what makes neighbouring bands share whole edges
        // — so a reveal emitted as one quad from its sill to its head draws that jamb as a single
        // segment while the wall beside it draws two, and the solid opens. Measured before it was
        // understood: a door in one bay with a window in the next read `loops 8, euler -12, manifold
        // false` while the volume was exactly right, the mismatch being the window's `[1.8, 2.7]` jamb
        // against the wall's `[1.8, 2.5]` and `[2.5, 2.7]`, split by a cut that belongs to a hole in
        // another bay. The head and the sill are split at every **station** the hole's own span holds, for
        // the mirror reason given where they are emitted: their edges run along the wall, and a station
        // inside the span is a place where the wall's own horizontal edge is already broken.
        //
        // Each quad is wound so its right-handed normal points out of the material and into the hole:
        // the left one faces along the edge towards `to`, the right one back towards `from`. Read in
        // the frame the wall is built in — `along`, `up`, and the inward normal — those are the two
        // orders below.
        for row in low..high {
            solid.quad(
                wound(
                    authored_is_outer,
                    [
                        points[from].authored[row],
                        points[from].authored[row + 1],
                        points[from].offset[row + 1],
                        points[from].offset[row],
                    ],
                ),
                Element::Mass(
                    mass,
                    MassSurface::Reveal {
                        loop_of: ring.loop_of,
                        edge,
                        opening: hole.opening,
                        side: RevealSide::Left,
                    },
                ),
            );
            solid.quad(
                wound(
                    authored_is_outer,
                    [
                        points[to].authored[row],
                        points[to].offset[row],
                        points[to].offset[row + 1],
                        points[to].authored[row + 1],
                    ],
                ),
                Element::Mass(
                    mass,
                    MassSurface::Reveal {
                        loop_of: ring.loop_of,
                        edge,
                        opening: hole.opening,
                        side: RevealSide::Right,
                    },
                ),
            );
        }
        // **A head or a sill is split at every station its own span holds, for the same reason a jamb is
        // split at every cut.** The wall's bands and both discs have a node at every station, so a hole
        // whose `u` span contains another hole's jamb station meets the wall along **two** horizontal edges
        // there and not one: emitted as a single quad, its edge is used once while the floor's or the
        // band's is used twice, and the solid reads open. Measured, and it is the closed form of the
        // failure: a doorway with one inset and a window above it with another gave `loops 4, euler -6`
        // while the same pair with equal insets closed.
        //
        // With no intermediate station — the ordinary case — this emits exactly the one quad it used to.
        let mut breaks = Vec::with_capacity(to - from + 1);
        breaks.push(from);
        breaks.extend((from + 1)..to);
        breaks.push(to);
        for pair in breaks.windows(2) {
            let (near, far) = (pair[0], pair[1]);
            solid.quad(
                wound(
                    authored_is_outer,
                    [
                        points[near].authored[high],
                        points[far].authored[high],
                        points[far].offset[high],
                        points[near].offset[high],
                    ],
                ),
                Element::Mass(
                    mass,
                    MassSurface::Reveal {
                        loop_of: ring.loop_of,
                        edge,
                        opening: hole.opening,
                        side: RevealSide::Head,
                    },
                ),
            );
            // **A sill that stands on the wall's own floor is not drawn**, because the ring's base
            // is notched across the opening rather than drawn under it: the two would name the same
            // four points, and one face of them is the whole of the ground there. The test is the
            // weld the station table records, at both ends of the span, which is what
            // `band_levels` reads when it declines to emit a below-sill band.
            if !points[near].stands_on_floor(low) || !points[far].stands_on_floor(low) {
                solid.quad(
                    wound(
                        authored_is_outer,
                        [
                            points[near].authored[low],
                            points[near].offset[low],
                            points[far].offset[low],
                            points[far].authored[low],
                        ],
                    ),
                    Element::Mass(
                        mass,
                        MassSurface::Reveal {
                            loop_of: ring.loop_of,
                            edge,
                            opening: hole.opening,
                            side: RevealSide::Sill,
                        },
                    ),
                );
            }
        }
    }
    Ok((added, notches))
}

/// Returns the index of a station in the walk's own table.
///
/// `O(log n)`, and exact: every station is one of the values the table was built from.
fn station_index(stations: &[f32], along: f32) -> usize {
    stations.partition_point(|station| *station < along)
}

/// Returns the index of an elevation in the walk's own cut list.
fn cut_at(cuts: &[f32], elevation: f32) -> usize {
    cuts.partition_point(|cut| *cut < elevation)
}

/// Returns the index an elevation is held at, counting the floor as `0` and the wall top as `1`.
fn cut_index(cuts: &[f32], elevation: f32) -> usize {
    2 + cut_at(cuts, elevation)
}

/// Returns the horizontal edges of one band's complement on both loops, as index pairs into the
/// station's own table.
///
/// # Invariants
///
/// **The complement is the gaps between the holes that span the band, and the gap under the lowest
/// one is the loop's own floor rather than a constant.** That gap's quad has a sloped bottom edge
/// and is still planar, because the whole wall lies in one vertical plane; the gaps above it are
/// between two constant elevations. A gap with no height at **either** end is **not emitted**: it is
/// the door's, and the base annulus's own edge closes its sill instead.
///
/// The emptiness test is made at both of the band's own ends and the gap stands if it has height at
/// **either** of them, which is what a ground line that rises needs and what keeps the two bands at
/// a station agreeing: both read the shared station's own floor, so a gap is a face on both sides of
/// a station or on neither. **The two loops' floors are read separately** and are different lines
/// wherever the ground slopes — see [`LoopSide::floor_at`].
fn band_levels(
    wall: &WallFace,
    cuts: &[f32],
    covering: &[&Hole],
    start: f32,
    end: f32,
) -> BandLevels {
    let mut pieces: Vec<(Option<f32>, f32)> = Vec::with_capacity(covering.len() + 1);
    let mut bottom: Option<f32> = None;
    for hole in covering {
        pieces.push((bottom, hole.outer.low));
        bottom = Some(hole.outer.high);
    }
    pieces.push((bottom, wall.top));

    let mut sides = [
        (LoopSide::Authored, Vec::new()),
        (LoopSide::Offset, Vec::new()),
    ];
    for (loop_side, emitted) in &mut sides {
        for (bottom, top) in &pieces {
            let floor = bottom.map_or_else(
                || {
                    loop_side
                        .floor_at(wall, start)
                        .min(loop_side.floor_at(wall, end))
                },
                |below| below,
            );
            let mut lower = *bottom;
            for cut in cuts
                .iter()
                .copied()
                .filter(|cut| *cut > floor && *cut < *top)
            {
                if let Some(pair) = piece(*loop_side, cuts, wall, lower, cut, start, end) {
                    emitted.push(pair);
                }
                lower = Some(cut);
            }
            if let Some(pair) = piece(*loop_side, cuts, wall, lower, *top, start, end) {
                emitted.push(pair);
            }
        }
    }
    BandLevels {
        authored: sides[0].1.clone(),
        offset: sides[1].1.clone(),
    }
}

/// One band's complement on both loops, as index pairs into the station's own table.
struct BandLevels {
    /// The authored loop's gaps, from the bottom up.
    authored: Vec<(usize, usize)>,
    /// The offset loop's, in the same order.
    offset: Vec<(usize, usize)>,
}

impl BandLevels {
    /// Returns one loop's own gaps.
    fn of(&self, loop_side: LoopSide) -> &[(usize, usize)] {
        match loop_side {
            LoopSide::Authored => &self.authored,
            LoopSide::Offset => &self.offset,
        }
    }
}

/// Returns the corners of one band that reaches a corner of the wall, as a polygon.
///
/// # Invariants
///
/// **A corner's vertical line is one edge, and the neighbouring wall still draws it as one.** So the
/// corner station of a cut wall carries its floor and its top and nothing between them, the band that
/// reaches it has one subdivided side and one side that is not subdivided — which no quad can have —
/// and the band is emitted as a single polygon fanned from the corner instead. **The fan is also
/// fewer faces than the quads it replaces**: `k` gaps over a corner side are `k + 1` triangles where
/// they were `2k`, and none of them puts a point on the corner line, which is the whole point.
///
/// The far side is walked through the **gaps' own boundaries**, never through each gap's two ends:
/// neighbouring gaps share the point between them, and a list that named both would put one point in
/// the walk twice and build a triangle with two corners in one place. The four orders are the four
/// ways round that walk — the outer face crosses the top from the near station and returns along the
/// bottom, and the inner face's own quad is wound the other way round.
fn corner_band(
    loop_side: LoopSide,
    authored_is_outer: bool,
    at_end: bool,
    corner: &[u32],
    gaps: &[(usize, usize)],
    other: &[u32],
) -> Vec<u32> {
    let Some((_, highest)) = gaps.last() else {
        return Vec::new();
    };
    let mut bounds: Vec<u32> = gaps.iter().map(|(low, _)| other[*low]).collect();
    bounds.push(other[*highest]);
    let mut corners = Vec::with_capacity(2 + bounds.len());
    match (loop_side, at_end) {
        (LoopSide::Authored, false) => {
            corners.extend([corner[0], corner[1]]);
            corners.extend(bounds.iter().rev().copied());
        }
        (LoopSide::Authored, true) => {
            corners.extend([corner[1], corner[0]]);
            corners.extend(bounds.iter().copied());
        }
        (LoopSide::Offset, false) => {
            corners.push(corner[0]);
            corners.extend(bounds.iter().copied());
            corners.push(corner[1]);
        }
        (LoopSide::Offset, true) => {
            corners.push(corner[1]);
            corners.extend(bounds.iter().rev().copied());
            corners.push(corner[0]);
        }
    }
    // **The walk is reversed for a ring whose authored loop faces inward**, which flips the fan's
    // normal and nothing else: a boundary walk is a closed list of the same corners either way round,
    // so the polygon is the same face and the other side of it.
    if !authored_is_outer {
        corners.reverse();
    }
    corners
}

/// Emits one gap of one band, as a quad or as the triangle it becomes where it stands on the floor.
///
/// # Invariants
///
/// **A gap with no height at one of its two ends is a triangle, and it is still a face.** A ground
/// line that rises makes that ordinary — the wall thins to nothing exactly where the ground reaches
/// the sill — and dropping that face would leave the edge its other end still draws unclaimed, which
/// reads as a boundary on a solid that is otherwise whole. Whether an end stands is read from the
/// **point indices**, which is the same answer as reading the elevations: the walk welds a sill that
/// sits on a floor to that floor's own point, so equal indices *are* equal elevations here.
///
/// A gap with no height at **either** end has no area at all and is not emitted: that is the door's,
/// and the base annulus's own edge closes its sill instead.
fn gap(
    solid: &mut Solid,
    loop_side: LoopSide,
    authored_is_outer: bool,
    near: &[u32],
    far: &[u32],
    low: usize,
    high: usize,
    element: Element,
) {
    let corners = loop_side.corners(authored_is_outer, near, far, low, high);
    match (near[high] == near[low], far[high] == far[low]) {
        (true, true) => {}
        (true, false) => solid.triangle([corners[0], corners[1], corners[2]], element),
        (false, true) => solid.triangle([corners[0], corners[2], corners[3]], element),
        (false, false) => solid.quad(corners, element),
    }
}

/// Returns the index pair of one gap, or `None` when it has no height at either of the band's ends.
///
/// `top` is resolved to the row the station's table holds it at, and **the wall top is not one of
/// the cuts**: it has a row of its own, so a gap that ends there names that row rather than a second
/// copy of the point.
fn piece(
    loop_side: LoopSide,
    cuts: &[f32],
    wall: &WallFace,
    bottom: Option<f32>,
    top: f32,
    start: f32,
    end: f32,
) -> Option<(usize, usize)> {
    let high = if top == wall.top {
        1
    } else {
        cut_index(cuts, top)
    };
    let (from, stands) = match bottom {
        None => (
            0,
            top > loop_side.floor_at(wall, start) || top > loop_side.floor_at(wall, end),
        ),
        Some(below) => (cut_index(cuts, below), top > below),
    };
    stands.then_some((from, high))
}

/// Returns the loop offset beside an authored one, or `None` after reporting every way it could not
/// be built.
///
/// # Invariants
///
/// **A corner is the intersection of its two shifted edges**, so the wall keeps its thickness and
/// the corner keeps its angle; nothing compares coordinates and nothing repairs a corner after the
/// fact. Every way the walk can go wrong is reported and named rather than clamped, because a
/// clamped corner is a wall in a place the author did not draw and no later check can see it: the
/// faces still meet, so the mesh is still closed, and the mistake reads as a building.
///
/// **One walk, two sides, and the loop is named in every fault.** A mass's wall is built inward
/// from its outline and a courtyard's ring is built outward from the void's loop; both are the same
/// mitred walk with the normals turned around, so there is one implementation and no second one to
/// drift from it. What a fault is *about* is the authored loop, not the mass, so `loop_of` rides
/// in all of them — "offset edge 2 has no length left" must not be answerable as a courtyard edge.
///
/// The checks read in this order: a loop edge with no length; a node whose two edges run back along
/// each other; a miter past the join limit; an offset edge with no length left; an offset edge
/// running backwards along the edge it was shifted from, which is what an offset wider than a
/// corner does; two offset edges meeting; and an area that no longer stands on the side it was
/// offset to.
pub(crate) fn offset_loop(
    outline: &[PlanNode],
    loop_of: AuthoredLoop,
    wall: f32,
    outline_area: f32,
    side: OffsetSide,
    problems: &mut Vec<Problem>,
) -> Option<Vec<Vec2>> {
    let count = outline.len();
    let mut directions = Vec::with_capacity(count);
    for edge in 0..count {
        let along = difference(outline[(edge + 1) % count].position, outline[edge].position);
        if length(along) <= LENGTH_EPSILON {
            problems.push(Problem::OutlineEdgeDegenerate { loop_of, edge });
            break;
        }
        directions.push(normalized(along));
    }
    if !problems.is_empty() {
        return None;
    }
    let normals: Vec<Vec2> = directions
        .iter()
        .copied()
        .map(|along| side.apply(inward(along)))
        .collect();

    let mut inner = Vec::with_capacity(count);
    for node in 0..count {
        let before = (node + count - 1) % count;
        let turn = cross(directions[before], directions[node]);
        if turn.abs() <= ANGLE_EPSILON {
            if dot(directions[before], directions[node]) <= 0.0 {
                problems.push(Problem::MiterUnresolved { loop_of, node });
                continue;
            }
            inner.push(step(outline[node].position, normals[node], wall));
            continue;
        }
        let corner = outline[node].position;
        let near = step(outline[before].position, normals[before], wall);
        let far = step(corner, normals[node], wall);
        let reach = cross(difference(far, near), directions[node]) / turn;
        let mitred = step(near, directions[before], reach);
        let distance_reached = distance(mitred, corner);
        if distance_reached > MITER_LIMIT * wall {
            problems.push(Problem::MiterTooLong {
                loop_of,
                node,
                reach: distance_reached,
            });
            continue;
        }
        inner.push(mitred);
    }
    if inner.len() != count {
        return None;
    }

    for edge in 0..count {
        let along = difference(inner[(edge + 1) % count], inner[edge]);
        let size = length(along);
        if size <= LENGTH_EPSILON {
            problems.push(Problem::OffsetEdgeVanished {
                loop_of,
                edge,
                length: size,
            });
            continue;
        }
        if dot(along, directions[edge]) <= 0.0 {
            problems.push(Problem::OffsetEdgeFolded { loop_of, edge });
        }
    }
    for left in 0..count {
        for right in left + 2..count {
            if left == 0 && right == count - 1 {
                continue;
            }
            if segments_cross_properly(
                inner[left],
                inner[(left + 1) % count],
                inner[right],
                inner[(right + 1) % count],
            ) {
                problems.push(Problem::OffsetSelfIntersecting {
                    loop_of,
                    left,
                    right,
                });
            }
        }
    }
    let area = loop_area(&inner);
    // **The side decides which way the area must have gone**, and both sides want a loop that is
    // still wound the way the authored loop was: an inward offset has to be smaller and an outward
    // one larger, so a single fixed comparison would refuse every outward ring the moment it built.
    let turned = match side {
        OffsetSide::Inward => area <= 0.0 || area >= outline_area,
        OffsetSide::Outward => area <= outline_area,
    };
    if turned {
        problems.push(Problem::OffsetInverted { loop_of, area });
    }
    if problems.is_empty() {
        Some(inner)
    } else {
        None
    }
}

/// Returns the signed plan area a closed loop encloses, positive when its interior is on the left.
///
/// # Invariants
///
/// **It is `O(nodes)` and reads the whole loop once**, which is what both callers need: the offset
/// walk compares the loop it built against the one it came from, and a void's loop is normalised to
/// this sign before anything is built from it, because which way the loop is walked is what says
/// which side the material is on.
pub(crate) fn loop_area(loop_points: &[Vec2]) -> f32 {
    let count = loop_points.len();
    if count < 3 {
        return 0.0;
    }
    let mut total = 0.0;
    for index in 0..count {
        total += cross(loop_points[index], loop_points[(index + 1) % count]);
    }
    total * 0.5
}

/// Returns whether two segments cross **properly**: each has the other's endpoints strictly on opposite sides.
///
/// # Invariants
///
/// **A touch is not a crossing and neither is a shared line.** The non-strict test above answers true for two
/// collinear segments — `0 <= 0` on both products — so the two halves of one face read as a crossing, and a
/// concave outline's offset loop, which is exactly right, was refused for some wall thicknesses and accepted for
/// others. This is the strict form the offset check asks for.
fn segments_cross_properly(from_a: Vec2, to_a: Vec2, from_b: Vec2, to_b: Vec2) -> bool {
    let along = difference(to_a, from_a);
    let first = cross(along, difference(from_b, from_a));
    let second = cross(along, difference(to_b, from_a));
    let other = difference(to_b, from_b);
    let third = cross(other, difference(from_a, from_b));
    let fourth = cross(other, difference(to_a, from_b));
    (first > 0.0) != (second > 0.0) && (third > 0.0) != (fourth > 0.0)
}

/// Returns whether a point is inside or on a triangle wound with its interior on the left.
///
/// The test includes the triangle's own edges, and that is the whole point of it: a node lying
/// exactly on a candidate ear's edge makes that ear's face pass through the node, which is a
/// vertex another face also uses, and the two surfaces then meet at a point one of them has no
/// vertex for. A strict test clips that ear, fills the loop with overlapping triangles, and every
/// check afterwards reads a closed solid.
fn inside_triangle(point: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    cross(difference(b, a), difference(point, a)) >= 0.0
        && cross(difference(c, b), difference(point, b)) >= 0.0
        && cross(difference(a, c), difference(point, c)) >= 0.0
}

/// Fills one of a solid's horizontal surfaces with a loop's polygon, and returns whether it could.
///
/// # Invariants
///
/// **The loop is clipped in the order it was authored**, so the triangles come out consistent with
/// one another and with the wall they meet, and a concave outline gets a surface that lies inside it:
/// an ear is a corner that turns the same way as the loop and whose triangle holds no other corner. A
/// fan from one node would be shorter, and would leave a concave mass's floor folded over itself —
/// which no closure check reports.
///
/// **Every ear a clip chooses emits a facet, one with no area included.** Each step consumes exactly
/// the two ring edges around the node it removes and creates exactly one diagonal, so the facets use
/// every edge of the loop once — which is what makes the surface closed against the ring it sits on,
/// for **any** order the clip happens to choose its ears in. Dropping the facet of a node with no
/// turn breaks that at the dropped node: the ring edges around it are then each used once, by the
/// wall alone, and the solid comes out with a boundary nobody reports. A node with no turn is what
/// [`ops::attach_mass`](crate::ops::attach_mass) leaves behind every time it splits a host outline
/// at a station between two nodes — measured 2026-10-16 on the probe's split-host section, where an
/// L written from its concave node expanded to `loops 2, euler 0, manifold false` with no problem
/// of any kind, and a facet carries no area, so its volume did not notice.
///
/// **The winding is the loop's, not the facet's.** Every facet of a planar polygon faces the way the
/// loop's own authored order faces it, so the surface is oriented once, here, from the loop's
/// signed area. A facet with no area has no normal to ask, and asking anyway is how the degenerate
/// facet of a down-facing soffit came out wound the other way round from every facet beside it: the
/// ring edges around its node were used twice in the **same** direction, which cancels to nothing,
/// and the solid kept one boundary loop after the facet was restored — measured on the same probe
/// case, `loops 1, euler 2, manifold true`.
///
/// A facet with no area carries no normal of its own, so a corner of one that no other facet of
/// the same element touches is answered by [`normalized3`](crate::math::normalized3)'s fallback of
/// up: right on the floor, inverted on the soffit, at one vertex of a disc nobody looks at from
/// below. Nothing measures it, and nothing else about the facet is unusual — it is in its element's
/// plane like every other, so the planarity check reads it as part of the plane it belongs to.
///
/// `up` is which way the surface faces, and the mass uses it twice over the same loop: the floor
/// looks up out of the room, the roof's soffit looks down out of the slab.
///
/// `O(n^3)` in the loop's nodes over the whole clip, which is what ear clipping costs; a wall ring is
/// tens of nodes and this is one surface of one mass.

fn polygon(
    solid: &mut Solid,
    loop_points: &[Vec2],
    corners: &[u32],
    element: Element,
    up: bool,
) -> bool {
    let as_authored = loop_area(loop_points) > 0.0;
    let wound = |corners: [u32; 3]| {
        if as_authored == up {
            corners
        } else {
            [corners[0], corners[2], corners[1]]
        }
    };
    let mut remaining: Vec<usize> = (0..loop_points.len()).collect();
    while remaining.len() > 3 {
        let Some(ear) = clip_ear(loop_points, &remaining) else {
            return false;
        };
        solid.triangle(
            wound([corners[ear.before], corners[ear.after], corners[ear.here]]),
            element,
        );
        remaining.retain(|node| *node != ear.here);
    }
    // The last three stand in the loop's own order, so they are wound the way the ear above it is
    // not: an ear's facet is the loop's order reversed, and the three that are left take the same
    // treatment so that the surface is wound one way round.
    solid.triangle(
        wound([
            corners[remaining[0]],
            corners[remaining[2]],
            corners[remaining[1]],
        ]),
        element,
    );
    true
}

/// One corner of a loop, as the three nodes a triangle on it would use.
struct Ear {
    before: usize,
    here: usize,
    after: usize,
}

/// Returns one ear of a loop, or `None` when the loop has none left.
///
/// # Invariants
///
/// The remaining loop keeps the winding it was authored with, which is what lets a surface's
/// triangles be wound the same way round the whole polygon. **A node in a straight run is an ear
/// and is clipped like any other**: its triangle has no area, but the two ring edges around it are
/// still the edges the walls beside it meet, and its facet is what uses each of them the second
/// time. (The justification that stood here — that the two nodes it stands between are already the
/// edge the surrounding faces use — was backwards: the surrounding faces use `(before, here)` and
/// `(here, after)`, and only this facet closes them.)
///
/// A node whose two edges run back along each other is not a straight run but a spike, and it never
/// reaches here: [`offset_loop`] refuses it by name before any corner is mitred.
fn clip_ear(loop_points: &[Vec2], remaining: &[usize]) -> Option<Ear> {
    let count = remaining.len();
    for step in 0..count {
        let ear = Ear {
            before: remaining[(step + count - 1) % count],
            here: remaining[step],
            after: remaining[(step + 1) % count],
        };
        // A negative turn is a corner that turns against the loop: the triangle on it would lie
        // outside, and clipping it would fold the surface over itself.
        let turn = cross(
            difference(loop_points[ear.here], loop_points[ear.before]),
            difference(loop_points[ear.after], loop_points[ear.here]),
        );
        if turn < 0.0 {
            continue;
        }
        let (a, b, c) = (
            loop_points[ear.before],
            loop_points[ear.here],
            loop_points[ear.after],
        );
        if remaining.iter().all(|other| {
            *other == ear.before
                || *other == ear.here
                || *other == ear.after
                || !inside_triangle(loop_points[*other], a, b, c)
        }) {
            return Some(ear);
        }
    }
    None
}

/// One roof being walked, with the two welds that hold its surfaces together.
struct RoofWalk<'a> {
    roof: &'a Roof,
    id: RoofId,
    problems: Vec<Problem>,
    /// Crest index to point, so a crest named by two planes is one point.
    crests: HashMap<usize, u32>,
    /// Eave station to point, so a station named by two planes is one point.
    eaves: HashMap<u32, u32>,
    /// The solid, whose points are the skin's first and their lowered copies after.
    solid: Solid,
}

impl RoofWalk<'_> {
    /// Returns the point a crest index names, welded to the one already added for that index.
    fn crest_point(&mut self, crest: usize) -> Option<u32> {
        if let Some(point) = self.crests.get(&crest) {
            return Some(*point);
        }
        let at = self.roof.crest_point(crest)?;
        let point = self.solid.point(at);
        self.crests.insert(crest, point);
        Some(point)
    }

    /// Returns the point an eave station names, welded to the one already added for that station.
    ///
    /// # Invariants
    ///
    /// **The key is normalised the way the reader normalises it**, because the two have to read one
    /// station as one place: [`Roof::eave_point`] resolves `t.rem_euclid(1.0)`, so a row naming
    /// `t = 1.0` and a row naming `t = 0.0` sample **one** point. Keyed on the raw bits they would
    /// weld into two vertices standing on the same corner, and the seam between them is one no
    /// closure predicate has to see: the two copies sit in different faces, each of which is
    /// internally closed. The key here and the reading in there are one operation written twice.
    fn eave_point(&mut self, station: EaveStation) -> Option<u32> {
        let key = station.t.rem_euclid(1.0).to_bits();
        if let Some(point) = self.eaves.get(&key) {
            return Some(*point);
        }
        let at = self.roof.eave_point(station)?;
        let point = self.solid.point(at);
        self.eaves.insert(key, point);
        Some(point)
    }

    /// Returns the corners of the skin facet between two rows, in the order the rows name them.
    ///
    /// A quad whose two rows name one crest point is a triangle, and one whose two rows name one
    /// eave station is a triangle the other way round; two rows naming both leave a facet with no
    /// area, which is reported rather than emitted as the zero-area triangle no closure check sees.
    /// Comparing the welded point indices rather than the crest indices is the same test: a point is
    /// welded by authored identity, so equal points are equal authors.
    fn skin_corners(
        &mut self,
        plane: usize,
        row: usize,
        from: &RoofRow,
        to: &RoofRow,
    ) -> Option<Vec<u32>> {
        let Some(crest) = self.crest_point(from.crest) else {
            self.problems.push(Problem::CrestMissing {
                roof: self.id,
                plane,
                row,
                crest: from.crest,
            });
            return None;
        };
        let Some(high) = self.crest_point(to.crest) else {
            self.problems.push(Problem::CrestMissing {
                roof: self.id,
                plane,
                row: row + 1,
                crest: to.crest,
            });
            return None;
        };
        let Some(front) = self.eave_point(from.eave) else {
            self.problems.push(Problem::EaveUnresolved {
                roof: self.id,
                plane,
                row,
                station: from.eave.t,
            });
            return None;
        };
        let Some(back) = self.eave_point(to.eave) else {
            self.problems.push(Problem::EaveUnresolved {
                roof: self.id,
                plane,
                row: row + 1,
                station: to.eave.t,
            });
            return None;
        };
        if crest == high {
            if front == back {
                self.problems.push(Problem::RowPairDegenerate {
                    roof: self.id,
                    plane,
                    row,
                });
                return None;
            }
            return Some(vec![crest, back, front]);
        }
        if front == back {
            return Some(vec![crest, high, front]);
        }
        Some(vec![crest, high, back, front])
    }
}

/// Expands one roof into one closed solid of skin, soffit and fascia.
fn expand_roof(mesh: &mut BuildingMesh, id: RoofId, roof: &Roof) -> Result<(), Vec<Problem>> {
    if roof.planes.is_empty() {
        return Err(vec![Problem::RoofEmpty {
            roof: id,
            planes: 0,
        }]);
    }
    let mut walk = RoofWalk {
        roof,
        id,
        problems: Vec::new(),
        crests: HashMap::new(),
        eaves: HashMap::new(),
        solid: Solid::default(),
    };
    if roof.thickness <= 0.0 {
        walk.problems.push(Problem::RoofThickness {
            roof: id,
            thickness: roof.thickness,
        });
    }
    for (plane, rows) in roof.planes.iter().enumerate() {
        if rows.rows.len() < 2 {
            walk.problems
                .push(Problem::PlaneTooShort { roof: id, plane });
            continue;
        }
        for row in 0..rows.rows.len() - 1 {
            let element = Element::Roof(id, RoofSurface::Skin { plane });
            match walk
                .skin_corners(plane, row, &rows.rows[row], &rows.rows[row + 1])
                .map(|corners| (corners, element))
            {
                Some((corners, element)) if corners.len() == 4 => {
                    walk.solid
                        .up_quad([corners[0], corners[1], corners[2], corners[3]], element);
                }
                Some((corners, element)) if corners.len() == 3 => {
                    walk.solid.oriented_triangle(
                        [corners[0], corners[1], corners[2]],
                        element,
                        true,
                    );
                }
                Some(_) => {}
                None => continue,
            }
        }
    }
    if !walk.problems.is_empty() {
        return Err(walk.problems);
    }

    let coordinates = walk.solid.points.clone();
    let skin = walk.solid.faces.clone();
    let skin_points = coordinates.len() as u32;
    let lowered: Vec<u32> = (0..skin_points)
        .map(|index| {
            let at = coordinates[index as usize];
            walk.solid.point([at[0], at[1] - roof.thickness, at[2]])
        })
        .collect();
    for face in &skin {
        if let Element::Roof(_, RoofSurface::Skin { plane }) = face.element {
            walk.solid.triangle(
                [
                    lowered[face.indices[2] as usize],
                    lowered[face.indices[1] as usize],
                    lowered[face.indices[0] as usize],
                ],
                Element::Roof(id, RoofSurface::Soffit { plane }),
            );
        }
    }

    let open = BuildingMesh {
        points: coordinates,
        faces: skin,
    };
    let boundary = open.boundary_edges_of(&(0..open.faces.len()).collect::<Vec<_>>());
    for (edge, (from, to)) in boundary.iter().copied().enumerate() {
        walk.solid.quad(
            [to, from, lowered[from as usize], lowered[to as usize]],
            Element::Roof(id, RoofSurface::Fascia { edge }),
        );
    }
    walk.solid.splice(mesh);
    Ok(())
}
