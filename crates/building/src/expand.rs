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

use std::collections::HashMap;

use crate::grid::GRID_TOLERANCE;
use crate::mass::{Mass, MassId, PlanNode};
use crate::math::{
    Vec2, Vec3, cross, cross3, difference, distance, dot, length, lift, normalized, subtract,
};
use crate::mesh::{BuildingFace, BuildingMesh, Element, MassSurface, PartSurface, RoofSurface};
use crate::openings::{Extents, RevealSide};
use crate::roof::{EaveStation, Roof, RoofId, RoofRow};
use crate::skeleton::BuildingSkeleton;

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
    /// An outline edge has no length, so it names no inward direction to build the wall along.
    OutlineEdgeDegenerate {
        /// The mass.
        mass: MassId,
        /// The outline edge, counted from the outline's first node.
        edge: usize,
    },
    /// A node's two outline edges run back along each other, so there is no corner to mitre.
    MiterUnresolved {
        /// The mass.
        mass: MassId,
        /// The outline node, counted from the outline's first node.
        node: usize,
    },
    /// A miter reaches further from its corner than the join limit allows, so the corner it builds is
    /// not the corner that was authored.
    MiterTooLong {
        /// The mass.
        mass: MassId,
        /// The outline node, counted from the outline's first node.
        node: usize,
        /// How far it reached, from the corner.
        reach: f32,
    },
    /// An edge of the offset loop has no length left in it.
    OffsetEdgeVanished {
        /// The mass.
        mass: MassId,
        /// The offset edge, counted over the offset loop.
        edge: usize,
        /// The length it was left with.
        length: f32,
    },
    /// An offset edge runs backwards along the outline edge it was shifted from, which is what an
    /// offset wider than the corner it offsets does.
    OffsetEdgeFolded {
        /// The mass.
        mass: MassId,
        /// The offset edge, counted over the offset loop.
        edge: usize,
    },
    /// Two offset edges that are not neighbours meet, so the loop is not the one the author drew.
    OffsetSelfIntersecting {
        /// The mass.
        mass: MassId,
        /// The first offset edge, counted over the offset loop.
        left: usize,
        /// The second.
        right: usize,
    },
    /// The offset loop has stopped enclosing the mass: its area is not positive, or it is not
    /// smaller than the outline's.
    OffsetInverted {
        /// The mass.
        mass: MassId,
        /// The area it encloses.
        area: f32,
    },
    /// The offset loop has no triangulation left, which a self-crossing outline leaves behind.
    FloorDegenerate {
        /// The mass.
        mass: MassId,
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
    /// **An opening that cannot be cut stops the mass rather than being dropped.** The document says
    /// a wall has a hole in it; a mesh built without one contradicts the document silently, and a
    /// solid nobody asked for is still a closed solid that every later check reads as sound. A
    /// vanished slot is [`validate`](crate::validate)'s own report as well — it can name the opening
    /// and the grid it missed — and the two are different statements: this one says the wall that
    /// would have carried the hole was not built.
    OpeningUnresolved {
        /// The mass.
        mass: MassId,
        /// The outline edge the opening names, counted from the outline's first node.
        edge: usize,
        /// The opening's index in the document's own list.
        opening: usize,
    },
    /// An opening's hole is not strictly inside the wall face it is cut into, so the strip the cut
    /// decomposes has no band for it.
    ///
    /// `past` is how far the hole leaves the strip, in metres, over every side that does: its two
    /// ends along the edge, its head, its sill against the ground line, and the mitred inner edge
    /// its four corners project onto. The greatest of those is what is reported, so a hole that is
    /// wrong in two ways names the worse one.
    OpeningOutsideWall {
        /// The mass.
        mass: MassId,
        /// The outline edge the hole is cut into.
        edge: usize,
        /// The opening's index in the document's own list.
        opening: usize,
        /// How far it reaches past the strip it has to stay inside, in metres.
        past: f32,
    },
    /// The ground line under one wall crosses the elevation of one of that wall's own holes, so the
    /// complement of the hole in that band is a quad folded on itself rather than a quad.
    GroundCrossesOpening {
        /// The mass.
        mass: MassId,
        /// The outline edge the hole is cut into.
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
        /// The mass.
        mass: MassId,
        /// The outline edge both holes are cut into.
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
pub fn build(skeleton: &BuildingSkeleton) -> Result<BuildingMesh, BuildError> {
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
        match expand_placement(skeleton, index) {
            Ok(Some((points, faces))) => {
                let base = mesh.points.len() as u32;
                mesh.points.extend(points);
                mesh.faces
                    .extend(faces.into_iter().map(|face| BuildingFace {
                        indices: face.indices.map(|corner| corner + base),
                        element: face.element,
                    }));
            }
            Ok(None) => problems.push(Problem::PlacementUnresolved { placement: index }),
            Err(found) => problems.extend(found),
        }
    }
    if problems.is_empty() {
        Ok(mesh)
    } else {
        Err(BuildError { problems })
    }
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
fn expand_placement(
    skeleton: &BuildingSkeleton,
    index: usize,
) -> Result<Option<(Vec<Vec3>, Vec<BuildingFace>)>, Vec<Problem>> {
    let Some((template, bay, _reach)) = skeleton.placed(index) else {
        return Ok(None);
    };
    let placement = match skeleton.parts.placement(index) {
        Some(placement) => *placement,
        None => return Ok(None),
    };
    let Some(mass) = skeleton.mass(placement.face.mass) else {
        return Ok(None);
    };
    let mut problems = Vec::new();
    let Some(inner) = offset_loop(
        &mass.outline,
        placement.face.mass,
        mass.wall,
        mass.plan_area(),
        &mut problems,
    ) else {
        return Err(problems);
    };
    let wall = WallFace::of(mass, placement.face.edge, &inner);
    let held = build(&template.skeleton()).map_err(|error| error.problems)?;
    let origin = lift(
        wall.outer_at(bay.from + placement.at.along),
        bay.low + placement.at.up,
    );
    let along = [wall.along[0], 0.0, wall.along[1]];
    let inside = inward(wall.along);
    let inward3 = [inside[0], 0.0, inside[1]];
    let up = [0.0, 1.0, 0.0];
    let points = held
        .points
        .iter()
        .map(|point| {
            let mut moved = origin;
            for (axis, direction) in [(0, along), (1, up), (2, inward3)] {
                for component in 0..3 {
                    moved[component] += point[axis] * direction[component];
                }
            }
            moved
        })
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
                    // A template holds masses and roofs and nothing else, so this arm is what a
                    // template that somehow held a part would fold into: the face belongs to the part
                    // it is spliced into, and its own surface is kept.
                    Element::Part { surface, .. } => surface,
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
#[derive(Debug, Default)]
struct Solid {
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

/// Expands one mass into one closed solid of five surfaces.
///
/// # Invariants
///
/// **Five surfaces, and every edge of them belongs to exactly two.** The inner wall stops at the
/// node's own elevation, which is where the floor is, so the inner loop carries two faces at that
/// level — the wall's and the floor's — and the loop below it does not exist: the underside is the
/// whole outline one slab below, a disc rather than a ring. A ring reaching in to the inner loop at
/// the floor level would put the wall's bottom, the floor's edge and the ring's inner edge on one
/// edge, and a solid cannot be closed and outward with three faces on an edge.
///
/// **A face with no opening is built exactly as it was before openings existed**, on the same four
/// authored node points and in the same order: the cut is a second walk, over the faces that have
/// holes, and it touches nothing else. That is what keeps every P1 measurement where it was, and
/// it is a property of where the branch is rather than of the branch itself.
///
/// See [`cut_edge`] for the wall a hole is cut into, and [`resolve_holes`] for what is refused
/// before a single point is added.
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
        id,
        mass.wall,
        mass.plan_area(),
        &mut problems,
    ) else {
        return Err(problems);
    };
    let outside: Vec<Vec2> = mass.outline.iter().map(|node| node.position).collect();

    let mut solid = Solid::default();
    let mut outside_base = Vec::with_capacity(count);
    let mut outside_top = Vec::with_capacity(count);
    let mut inside_floor = Vec::with_capacity(count);
    let mut inside_top = Vec::with_capacity(count);
    for node in 0..count {
        let plan = mass.outline[node].position;
        outside_base.push(solid.point(lift(plan, mass.outline[node].elevation - mass.slab)));
        outside_top.push(solid.point(lift(plan, mass.top)));
        inside_floor.push(solid.point(lift(inner[node], mass.outline[node].elevation)));
        inside_top.push(solid.point(lift(inner[node], mass.top)));
    }
    let faces: Vec<WallFace> = (0..count)
        .map(|edge| WallFace::of(mass, edge, &inner))
        .collect();
    let nodes: Vec<EdgeNode> = (0..count)
        .map(|node| EdgeNode {
            outer_base: outside_base[node],
            outer_top: outside_top[node],
            inner_floor: inside_floor[node],
            inner_top: inside_top[node],
        })
        .collect();
    let by_edge = resolve_holes(skeleton, id, &faces, &mut problems);
    if !problems.is_empty() {
        return Err(problems);
    }
    // The stations each cut edge added to the two discs' loops, empty for a face with no hole.
    let mut cuts: Vec<Vec<CutStation>> = vec![Vec::new(); count];
    for edge in 0..count {
        let holes = &by_edge[edge];
        let next = (edge + 1) % count;
        if holes.is_empty() {
            solid.quad(
                [
                    outside_top[edge],
                    outside_top[next],
                    outside_base[next],
                    outside_base[edge],
                ],
                Element::Mass(id, MassSurface::OuterWall { edge }),
            );
            solid.quad(
                [
                    outside_top[edge],
                    inside_top[edge],
                    inside_top[next],
                    outside_top[next],
                ],
                Element::Mass(id, MassSurface::WallTop { edge }),
            );
            solid.quad(
                [
                    inside_top[next],
                    inside_top[edge],
                    inside_floor[edge],
                    inside_floor[next],
                ],
                Element::Mass(id, MassSurface::InnerWall { edge }),
            );
            continue;
        }
        match cut_edge(
            &mut solid,
            id,
            edge,
            &faces[edge],
            holes,
            nodes[edge],
            nodes[next],
        ) {
            Ok(stations) => cuts[edge] = stations,
            Err(found) => problems.extend(found),
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    // The two discs are two authored planes one slab apart, so they are two elements: naming both
    // `Floor` put them in one shading group and one planarity group, and the merged validator
    // reported the group as a non-planar surface at `departure = slab/2`.
    let floor_top = Element::Mass(id, MassSurface::FloorTop);
    let slab_soffit = Element::Mass(id, MassSurface::SlabSoffit);
    let (inner_loop, floor_points) = split_loop(&inner, &inside_floor, &cuts, |station| {
        (station.inner, station.floor)
    });
    let (outside_loop, soffit_points) = split_loop(&outside, &outside_base, &cuts, |station| {
        (station.outer, station.soffit)
    });
    let walked = polygon(&mut solid, &inner_loop, &floor_points, floor_top, true)
        && polygon(
            &mut solid,
            &outside_loop,
            &soffit_points,
            slab_soffit,
            false,
        );
    if !walked {
        problems.push(Problem::FloorDegenerate { mass: id });
        return Err(problems);
    }
    solid.splice(mesh);
    Ok(())
}

/// Returns a disc's loop and its points, with the cut stations written into it.
///
/// # Invariants
///
/// **A cut wall's own bottom edge is several edges, so the disc it meets has to be several too.**
/// The wall over one band ends on the segment between two stations, and nothing else uses that
/// segment; a disc walked node to node has one long edge there instead, and the segment is then a
/// boundary. The stations are therefore **collinear nodes of the loop**, which is the same state
/// [`ops::attach_mass`](crate::ops::attach_mass) leaves behind, and which
/// [`polygon`] answers with a facet of no area rather than with a lost edge.
///
/// The stations arrive in the loop's own order because they are walked from the node they follow,
/// so one pass over the nodes reads both loops in the order the outline was authored in.
fn split_loop(
    loop_points: &[Vec2],
    corners: &[u32],
    cuts: &[Vec<CutStation>],
    station_of: impl Fn(&CutStation) -> (Vec2, u32),
) -> (Vec<Vec2>, Vec<u32>) {
    let mut points = Vec::with_capacity(loop_points.len());
    let mut welded = Vec::with_capacity(loop_points.len());
    for (node, point) in loop_points.iter().enumerate() {
        points.push(*point);
        welded.push(corners[node]);
        for station in &cuts[node] {
            let (position, index) = station_of(station);
            points.push(position);
            welded.push(index);
        }
    }
    (points, welded)
}

/// Resolves every opening of one mass against the walls it names, and reports every way the
/// document's openings cannot be cut.
///
/// # Invariants
///
/// **The two lookups are read as a pair, and both come from the document.** `apertures_of` answers
/// what can be cut and filters the rest away; `unresolved_openings` answers what could not be. An
/// expander reading only the first would build a wall with a hole silently missing and a document
/// that says the wall has one, so both are read and neither is re-derived here.
///
/// The faces to read are enumerated through [`Openings::by_face`](crate::openings::Openings::by_face),
/// which is `O(openings)` once, rather than by asking every face of the mass — that would put the
/// whole document into the cost of every wall. The keys arrive in `(mass, edge)` order and the
/// indices in the order they were authored, so two runs of one document report in one order.
///
/// **A refused mass carries none of its holes.** Nothing here writes to the solid, and
/// [`expand_mass`] splices only once the whole mass is built, so a wall that cannot be cut is
/// absent rather than half present.
fn resolve_holes(
    skeleton: &BuildingSkeleton,
    id: MassId,
    faces: &[WallFace],
    problems: &mut Vec<Problem>,
) -> Vec<Vec<Hole>> {
    let mut by_edge: Vec<Vec<Hole>> = vec![Vec::new(); faces.len()];
    for (face, _) in skeleton.openings.by_face() {
        if face.mass != id {
            continue;
        }
        for index in skeleton.unresolved_openings(face) {
            problems.push(Problem::OpeningUnresolved {
                mass: id,
                edge: face.edge,
                opening: index,
            });
        }
        for (index, extents) in skeleton.apertures_of(face) {
            let Some(wall) = faces.get(face.edge) else {
                problems.push(Problem::OpeningUnresolved {
                    mass: id,
                    edge: face.edge,
                    opening: index,
                });
                continue;
            };
            match resolve_one(id, face.edge, index, extents, wall) {
                Ok(hole) => by_edge[face.edge].push(hole),
                Err(problem) => problems.push(problem),
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
                        mass: id,
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
/// test.** Across, the strip is the edge's own length and the inner edge's own length; a mitre moves
/// both ends of the inner edge, so a hole that clears the outer strip can still project off the
/// inner one, and it is measured where it lands rather than assumed to have travelled with it. Down,
/// the two faces have **two different floors**: the outer wall stands on the ground line one slab
/// below, and the inner wall stands on the ground line itself, because that is where the floor is.
/// The binding floor is therefore the inner one, and the difference between the two is what makes a
/// door legal rather than a hole in the ground.
///
/// **A sill may stand exactly on the inner wall's own floor, and that is a door.** The band below
/// it is then empty rather than inverted, the reveal's inner edge lies in the floor's own plane,
/// and the floor's edge closes it — no second mechanism and no special case in the walk. What is
/// refused is a sill below the ground line at either of the hole's own jambs, which is the same
/// answer read at two places because the line is straight. `past` is the greatest of every way the
/// hole leaves the strip, so a hole wrong in two ways names the worse one.
fn resolve_one(
    id: MassId,
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
        .inner_floor_at(extents.from)
        .max(wall.inner_floor_at(extents.to))
        - extents.low;
    // The hole's two jambs, projected onto the inner edge's line and measured against the ends the
    // miter left it: the projection moves nothing along the edge, but the inner edge is shorter than
    // the outer one at both ends, and a jamb that lands at or past one of them would put its reveal
    // on a corner the neighbouring wall still claims whole.
    let past_inner = (GRID_TOLERANCE + wall.inner_from - extents.from)
        .max(extents.to + GRID_TOLERANCE - wall.inner_to);
    let past = past_start
        .max(past_end)
        .max(past_head)
        .max(below_ground)
        .max(past_inner);
    if past > 0.0 {
        return Err(Problem::OpeningOutsideWall {
            mass: id,
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

/// One outline edge read as the vertical strip its wall is built in.
///
/// # Invariants
///
/// **`u` is arc length along the outer edge, and it is also arc length along the inner edge's own
/// line — the two are the same number, while the inner edge itself is shorter at both ends.** The
/// miter moves the inner edge's *ends*: a right angle takes one wall's thickness off each, and a
/// sharper corner takes more, so the inner loop's edge runs from `inner_from` to `inner_to` rather
/// than from `0` to the outer edge's length. Projecting a point of the outer face onto the inner
/// edge's line is therefore one dot product and moves nothing along the edge — the projection is the
/// identity in `u` — while *where the inner edge is* is what the miter moved. Reading the inner rim
/// as the outer rim shifted by a constant would move the hole's jambs; reading it as the projection
/// and then refusing a hole that lands off the inner edge's own ends is what the geometry says.
///
/// The two floors are the other half of the reading: the outer wall starts one slab below the
/// ground, the inner wall starts at the ground, and everything about a door follows from that pair.
#[derive(Debug, Clone, Copy)]
struct WallFace {
    /// Where the outer edge starts, in plan.
    origin: Vec2,
    /// The unit direction the outer edge runs.
    along: Vec2,
    /// Where the outer edge's line meets the inner edge's line: the outer edge shifted along its own
    /// inward normal by the wall's thickness.
    inner_origin: Vec2,
    /// How long the outer edge is.
    length: f32,
    /// Where the inner edge starts along its own line, which the miter moved off the outer edge's
    /// start.
    inner_from: f32,
    /// Where the inner edge ends along its own line, which the miter moved off the outer edge's end.
    inner_to: f32,
    /// The ground line's own two elevations, at the outer edge's two ends.
    ground: [f32; 2],
    /// The wall top.
    top: f32,
    /// The floor slab's thickness, which is what the outer wall stands below the ground line by.
    slab: f32,
}

impl WallFace {
    /// Reads one outline edge as the strip its wall is built in.
    fn of(mass: &Mass, edge: usize, inner: &[Vec2]) -> Self {
        let count = mass.outline.len();
        let next = (edge + 1) % count;
        let origin = mass.outline[edge].position;
        let far = mass.outline[next].position;
        let along = normalized(difference(far, origin));
        Self {
            origin,
            along,
            inner_origin: step(origin, inward(along), mass.wall),
            length: distance(far, origin),
            inner_from: dot(difference(inner[edge], origin), along),
            inner_to: dot(difference(inner[next], origin), along),
            ground: [mass.outline[edge].elevation, mass.outline[next].elevation],
            top: mass.top,
            slab: mass.slab,
        }
    }

    /// Returns the ground line's elevation at an arc length along the **outer** edge.
    ///
    /// The line is straight between the edge's two nodes, which is what makes both ends the whole
    /// test: a linear function attains its extremes on an interval at the interval's ends.
    fn ground_at(&self, along: f32) -> f32 {
        self.ground[0] + (self.ground[1] - self.ground[0]) * (along / self.length)
    }

    /// Returns the floor the inner wall stands on: the ground line itself, because that is where the
    /// mass's floor is.
    ///
    /// **Read in the inner edge's own arc length, not the outer's.** The inner wall's bottom edge runs
    /// between the two *inner* corners at the two *nodes'* elevations, so on a wall whose ground rises
    /// it is a different line from the outer wall's: its slope is the ground's rise spread over the
    /// inner edge's own length. Reading the same elevation at the same arc length for both faces would
    /// redraw the inner wall's floor along a line the un-cut wall does not have, and a mass would then
    /// hold a different amount of material depending on whether a window had been cut in it — a
    /// difference in volume with nothing in the geometry to account for it.
    fn inner_floor_at(&self, along: f32) -> f32 {
        let inside = along.clamp(self.inner_from, self.inner_to);
        let span = self.inner_to - self.inner_from;
        self.ground[0] + (self.ground[1] - self.ground[0]) * ((inside - self.inner_from) / span)
    }

    /// Returns the floor the outer wall stands on: the ground line one slab below it.
    fn outer_floor_at(&self, along: f32) -> f32 {
        self.ground_at(along) - self.slab
    }

    /// Returns the plan position of an arc length along the outer edge.
    fn outer_at(&self, along: f32) -> Vec2 {
        step(self.origin, self.along, along)
    }

    /// Returns the plan position of an arc length along the outer edge, on the inner edge.
    ///
    /// This is the **projection of the outer rim's corner onto the inner edge**, and it is the whole
    /// of what the inner face needs: one dot product, and exact because the two lines are parallel
    /// — the offset moves a line along its own normal, and a perpendicular projection puts the point
    /// straight back at the same arc length. The arc length handed in is the outer's, so one number
    /// describes a station on both faces of the wall, and that is what welds a hole's outer rim to
    /// the reveal leaving it.
    ///
    /// **The projection is onto a segment and not onto a line**, because the inner wall stops at the
    /// two mitred corners. The two ends of the walk's own station list are those corners' outer
    /// stations, which the projection puts a mitre's reach beyond the inner edge's own ends; clamped
    /// they land on the corner itself, which is where the node loop already put the floor and the top
    /// and where the neighbouring wall's inner face ends. No hole is affected: a hole is refused
    /// unless it lands strictly inside that segment.
    fn inner_at(&self, along: f32) -> Vec2 {
        let corner = dot(difference(self.outer_at(along), self.origin), self.along);
        let inside = corner.clamp(self.inner_from, self.inner_to);
        step(self.inner_origin, self.along, inside)
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

/// Which of a wall's two faces a band is emitted on.
#[derive(Debug, Clone, Copy, PartialEq)]
enum FaceSide {
    /// Seen from outside, with the ground line one slab below it.
    Outer,
    /// Seen from inside, standing on the ground line itself.
    Inner,
}

impl FaceSide {
    /// Returns the elevation this face's own floor stands at.
    fn floor_at(self, wall: &WallFace, along: f32) -> f32 {
        match self {
            Self::Outer => wall.outer_floor_at(along),
            Self::Inner => wall.inner_floor_at(along),
        }
    }

    /// Returns this side's own points of one station, by the index the cut numbers them by.
    fn pick(self, station: &StationPoints) -> &Vec<u32> {
        match self {
            Self::Outer => &station.outer,
            Self::Inner => &station.inner,
        }
    }
}

/// One station of a cut wall: the points of both faces at every elevation the cut uses.
///
/// # Invariants
///
/// **Index `0` is the face's own floor and index `1` is the wall top, and a hole's sill that stands
/// on that floor is the floor point itself.** The two are welded here rather than compared
/// afterwards: a door's sill is the inner wall's own bottom edge, the floor's edge there closes it,
/// and a second point at the same place would put that edge on the surface once and nowhere else.
/// Every other elevation is a point of its own, and the same index is what both the wall's bands
/// and the two reveals that border it read.
struct StationPoints {
    /// The outer face's points: floor, top, then every hole elevation in [`cuts`](../cuts).
    outer: Vec<u32>,
    /// The inner face's, in the same order.
    inner: Vec<u32>,
}

/// One station a cut added to an edge, as the two loop nodes the discs need.
#[derive(Debug, Clone, Copy)]
struct CutStation {
    /// Its plan position on the outline's own edge.
    outer: Vec2,
    /// Its plan position on the inner loop's own edge.
    inner: Vec2,
    /// The outer wall's floor point there, which is the soffit's own node.
    soffit: u32,
    /// The inner wall's floor point there, which is the floor's own node.
    floor: u32,
}

/// One end of an edge: the four authored points of its node.
///
/// # Invariants
///
/// **A cut reuses these rather than making its own copies of them.** The first and last of an
/// edge's stations are its own two nodes, and the ring, the floor and the soffit all meet there
/// through the points the node loop made — the neighbouring edge's wall claims the same corner
/// points, and a second point at the same coordinates would leave the ring's two ends meeting
/// nothing: a corner with no face across it, which reads as a boundary rather than as a corner.
#[derive(Debug, Clone, Copy)]
struct EdgeNode {
    /// The outer wall's floor point at this node.
    outer_base: u32,
    /// The outer wall's top point at this node.
    outer_top: u32,
    /// The inner wall's floor point at this node.
    inner_floor: u32,
    /// The inner wall's top point at this node.
    inner_top: u32,
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
/// **The wall top, the floor and the soffit follow the cut rather than being authored twice.** Each
/// band of the wall ends on its own top segment and its own floor segment, and each of those is used
/// by exactly one face of the ring, the floor and the soffit, so the three are built band by band
/// over the same stations. That is what the stations returned here are for.
fn cut_edge(
    solid: &mut Solid,
    id: MassId,
    edge: usize,
    wall: &WallFace,
    holes: &[Hole],
    near: EdgeNode,
    far: EdgeNode,
) -> Result<Vec<CutStation>, Vec<Problem>> {
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

    for side in [FaceSide::Outer, FaceSide::Inner] {
        for band in 0..stations.len() - 1 {
            let low = side
                .floor_at(wall, stations[band])
                .min(side.floor_at(wall, stations[band + 1]));
            let high = side
                .floor_at(wall, stations[band])
                .max(side.floor_at(wall, stations[band + 1]));
            for hole in holes {
                for elevation in [hole.outer.low, hole.outer.high] {
                    if elevation > low && elevation < high {
                        return Err(vec![Problem::GroundCrossesOpening {
                            mass: id,
                            edge,
                            opening: hole.opening,
                        }]);
                    }
                }
            }
        }
    }

    let mut points: Vec<StationPoints> = Vec::with_capacity(stations.len());
    let mut added: Vec<CutStation> = Vec::with_capacity(stations.len());
    let last = stations.len() - 1;
    for (index, station) in stations.iter().enumerate() {
        let mut outer = Vec::with_capacity(cuts.len() + 2);
        let mut inner = Vec::with_capacity(cuts.len() + 2);
        let authored = match index {
            0 => Some(near),
            index if index == last => Some(far),
            _ => None,
        };
        match authored {
            Some(node) => {
                outer.push(node.outer_base);
                inner.push(node.inner_floor);
                outer.push(node.outer_top);
                inner.push(node.inner_top);
            }
            None => {
                outer.push(
                    solid.point(lift(wall.outer_at(*station), wall.outer_floor_at(*station))),
                );
                inner.push(
                    solid.point(lift(wall.inner_at(*station), wall.inner_floor_at(*station))),
                );
                outer.push(solid.point(lift(wall.outer_at(*station), wall.top)));
                inner.push(solid.point(lift(wall.inner_at(*station), wall.top)));
                // A hole's sill that stands on one of the two floors **is** that floor's point,
                // welded by the walk rather than found by comparing coordinates afterwards. The
                // comparison is exact because the band walk reads the same two numbers: a door's
                // sill is the inner wall's own bottom edge, and the floor's edge there is what
                // closes it. A corner station carries no such row: its vertical line is one edge,
                // which is what the neighbouring wall still draws.
                for cut in &cuts {
                    outer.push(if *cut == wall.outer_floor_at(*station) {
                        outer[0]
                    } else {
                        solid.point(lift(wall.outer_at(*station), *cut))
                    });
                    inner.push(if *cut == wall.inner_floor_at(*station) {
                        inner[0]
                    } else {
                        solid.point(lift(wall.inner_at(*station), *cut))
                    });
                }
            }
        }
        let soffit = outer[0];
        let floor = inner[0];
        points.push(StationPoints { outer, inner });
        if index > 0 && index + 1 < stations.len() {
            added.push(CutStation {
                outer: wall.outer_at(*station),
                inner: wall.inner_at(*station),
                soffit,
                floor,
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
        for (side, surface) in [
            (FaceSide::Outer, MassSurface::OuterWall { edge }),
            (FaceSide::Inner, MassSurface::InnerWall { edge }),
        ] {
            let element = Element::Mass(id, surface);
            let starts_at_corner = band == 0;
            let ends_at_corner = band + 2 == stations.len();
            if starts_at_corner || ends_at_corner {
                let (corner, other) = if starts_at_corner {
                    (band, band + 1)
                } else {
                    (band + 1, band)
                };
                let corners = corner_band(
                    side,
                    ends_at_corner,
                    side.pick(&points[corner]),
                    levels.of(side),
                    side.pick(&points[other]),
                );
                solid.polygon(&corners, element);
            } else {
                let near = side.pick(&points[band]);
                let far = side.pick(&points[band + 1]);
                for (low, high) in levels.of(side) {
                    gap(solid, side, near, far, *low, *high, element);
                }
            }
        }
        solid.quad(
            [
                points[band].outer[1],
                points[band].inner[1],
                points[band + 1].inner[1],
                points[band + 1].outer[1],
            ],
            Element::Mass(id, MassSurface::WallTop { edge }),
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
        // another bay. The head and the sill are single quads: their edges run along the wall rather
        // than up it, and no cut divides a horizontal edge.
        //
        // Each quad is wound so its right-handed normal points out of the material and into the hole:
        // the left one faces along the edge towards `to`, the right one back towards `from`. Read in
        // the frame the wall is built in — `along`, `up`, and the inward normal — those are the two
        // orders below.
        for row in low..high {
            solid.quad(
                [
                    points[from].outer[row],
                    points[from].outer[row + 1],
                    points[from].inner[row + 1],
                    points[from].inner[row],
                ],
                Element::Mass(
                    id,
                    MassSurface::Reveal {
                        edge,
                        opening: hole.opening,
                        side: RevealSide::Left,
                    },
                ),
            );
            solid.quad(
                [
                    points[to].outer[row],
                    points[to].inner[row],
                    points[to].inner[row + 1],
                    points[to].outer[row + 1],
                ],
                Element::Mass(
                    id,
                    MassSurface::Reveal {
                        edge,
                        opening: hole.opening,
                        side: RevealSide::Right,
                    },
                ),
            );
        }
        for (side, corners) in [
            (
                RevealSide::Head,
                [
                    points[from].outer[high],
                    points[to].outer[high],
                    points[to].inner[high],
                    points[from].inner[high],
                ],
            ),
            (
                RevealSide::Sill,
                [
                    points[from].outer[low],
                    points[from].inner[low],
                    points[to].inner[low],
                    points[to].outer[low],
                ],
            ),
        ] {
            solid.quad(
                corners,
                Element::Mass(
                    id,
                    MassSurface::Reveal {
                        edge,
                        opening: hole.opening,
                        side,
                    },
                ),
            );
        }
    }
    Ok(added)
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

/// Returns the horizontal edges of one band's complement on both faces, as index pairs into the
/// station's own table.
///
/// # Invariants
///
/// **The complement is the gaps between the holes that span the band, and the gap under the lowest
/// one is the face's own floor rather than a constant.** That gap's quad has a sloped bottom edge
/// and is still planar, because the whole wall lies in one vertical plane; the gaps above it are
/// between two constant elevations. A gap with no height at **either** end is **not emitted**: it is
/// the door's, and the floor's own edge closes its sill instead.
///
/// The emptiness test is made at both of the band's own ends and the gap stands if it has height at
/// **either** of them, which is what a ground line that rises needs and what keeps the two bands at
/// a station agreeing: both read the shared station's own floor, so a gap is a face on both sides of
/// a station or on neither.
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

    let mut sides = [(FaceSide::Outer, Vec::new()), (FaceSide::Inner, Vec::new())];
    for (side, emitted) in &mut sides {
        for (bottom, top) in &pieces {
            let floor = bottom.map_or_else(
                || side.floor_at(wall, start).min(side.floor_at(wall, end)),
                |below| below,
            );
            let mut lower = *bottom;
            for cut in cuts
                .iter()
                .copied()
                .filter(|cut| *cut > floor && *cut < *top)
            {
                if let Some(pair) = piece(*side, cuts, wall, lower, cut, start, end) {
                    emitted.push(pair);
                }
                lower = Some(cut);
            }
            if let Some(pair) = piece(*side, cuts, wall, lower, *top, start, end) {
                emitted.push(pair);
            }
        }
    }
    BandLevels {
        outer: sides[0].1.clone(),
        inner: sides[1].1.clone(),
    }
}

/// One band's complement on both faces, as index pairs into the station's own table.
struct BandLevels {
    /// The outer face's gaps, from the bottom up.
    outer: Vec<(usize, usize)>,
    /// The inner face's gaps, from the bottom up.
    inner: Vec<(usize, usize)>,
}

impl BandLevels {
    /// Returns one side's own gaps.
    fn of(&self, side: FaceSide) -> &[(usize, usize)] {
        match side {
            FaceSide::Outer => &self.outer,
            FaceSide::Inner => &self.inner,
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
    side: FaceSide,
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
    match (side, at_end) {
        (FaceSide::Outer, false) => {
            corners.extend([corner[0], corner[1]]);
            corners.extend(bounds.iter().rev().copied());
        }
        (FaceSide::Outer, true) => {
            corners.extend([corner[1], corner[0]]);
            corners.extend(bounds.iter().copied());
        }
        (FaceSide::Inner, false) => {
            corners.push(corner[0]);
            corners.extend(bounds.iter().copied());
            corners.push(corner[1]);
        }
        (FaceSide::Inner, true) => {
            corners.push(corner[1]);
            corners.extend(bounds.iter().rev().copied());
            corners.push(corner[0]);
        }
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
/// and the floor's own edge closes its sill instead.
fn gap(
    solid: &mut Solid,
    side: FaceSide,
    near: &[u32],
    far: &[u32],
    low: usize,
    high: usize,
    element: Element,
) {
    let corners = match side {
        FaceSide::Outer => [near[high], far[high], far[low], near[low]],
        FaceSide::Inner => [far[high], near[high], near[low], far[low]],
    };
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
    side: FaceSide,
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
            top > side.floor_at(wall, start) || top > side.floor_at(wall, end),
        ),
        Some(below) => (cut_index(cuts, below), top > below),
    };
    stands.then_some((from, high))
}

/// Returns the mass's inner loop, or `None` after reporting every way it could not be built.
///
/// # Invariants
///
/// **A corner is the intersection of its two shifted edges**, so the wall keeps its thickness and
/// the corner keeps its angle; nothing compares coordinates and nothing repairs a corner after the
/// fact. Every way the walk can go wrong is reported and named rather than clamped, because a
/// clamped corner is a wall in a place the author did not draw and no later check can see it: the
/// faces still meet, so the mesh is still closed, and the mistake reads as a building.
///
/// The checks read in this order: an outline edge with no length; a node whose two edges run back
/// along each other; a miter past the join limit; an offset edge with no length left; an offset
/// edge running backwards along the edge it was shifted from, which is what an offset wider than a
/// corner does; two offset edges meeting; and an area that is no longer a smaller positive one.
fn offset_loop(
    outline: &[PlanNode],
    id: MassId,
    wall: f32,
    outline_area: f32,
    problems: &mut Vec<Problem>,
) -> Option<Vec<Vec2>> {
    let count = outline.len();
    let mut directions = Vec::with_capacity(count);
    for edge in 0..count {
        let along = difference(outline[(edge + 1) % count].position, outline[edge].position);
        if length(along) <= LENGTH_EPSILON {
            problems.push(Problem::OutlineEdgeDegenerate { mass: id, edge });
            break;
        }
        directions.push(normalized(along));
    }
    if !problems.is_empty() {
        return None;
    }
    let normals: Vec<Vec2> = directions.iter().copied().map(inward).collect();

    let mut inner = Vec::with_capacity(count);
    for node in 0..count {
        let before = (node + count - 1) % count;
        let turn = cross(directions[before], directions[node]);
        if turn.abs() <= ANGLE_EPSILON {
            if dot(directions[before], directions[node]) <= 0.0 {
                problems.push(Problem::MiterUnresolved { mass: id, node });
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
                mass: id,
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
                mass: id,
                edge,
                length: size,
            });
            continue;
        }
        if dot(along, directions[edge]) <= 0.0 {
            problems.push(Problem::OffsetEdgeFolded { mass: id, edge });
        }
    }
    for left in 0..count {
        for right in left + 2..count {
            if left == 0 && right == count - 1 {
                continue;
            }
            if segments_cross(
                inner[left],
                inner[(left + 1) % count],
                inner[right],
                inner[(right + 1) % count],
            ) {
                problems.push(Problem::OffsetSelfIntersecting {
                    mass: id,
                    left,
                    right,
                });
            }
        }
    }
    let area = loop_area(&inner);
    if area <= 0.0 || area >= outline_area {
        problems.push(Problem::OffsetInverted { mass: id, area });
    }
    if problems.is_empty() {
        Some(inner)
    } else {
        None
    }
}

/// Returns the signed plan area a closed loop encloses, positive when its interior is on the left.
fn loop_area(loop_points: &[Vec2]) -> f32 {
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

/// Returns whether two edges of one loop meet or cross.
///
/// The test is inclusive because two edges of a simple loop that are not neighbours never touch, so
/// a touch is as much a defect as a crossing.
fn segments_cross(from_a: Vec2, to_a: Vec2, from_b: Vec2, to_b: Vec2) -> bool {
    let along = difference(to_a, from_a);
    let first = cross(along, difference(from_b, from_a));
    let second = cross(along, difference(to_b, from_a));
    let other = difference(to_b, from_b);
    let third = cross(other, difference(from_a, from_b));
    let fourth = cross(other, difference(to_a, from_b));
    first * second <= 0.0 && third * fourth <= 0.0
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
