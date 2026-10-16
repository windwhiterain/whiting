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

use crate::mass::{Mass, MassId, PlanNode};
use crate::math::{
    Vec2, Vec3, cross, cross3, difference, distance, dot, length, lift, normalized, subtract,
};
use crate::mesh::{BuildingFace, BuildingMesh, Element, MassSurface, RoofSurface};
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
        if let Err(found) = expand_mass(&mut mesh, MassId(index), mass) {
            problems.extend(found);
        }
    }
    for (index, roof) in skeleton.roofs.iter().enumerate() {
        if let Err(found) = expand_roof(&mut mesh, RoofId(index), roof) {
            problems.extend(found);
        }
    }
    if problems.is_empty() {
        Ok(mesh)
    } else {
        Err(BuildError { problems })
    }
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
fn expand_mass(mesh: &mut BuildingMesh, id: MassId, mass: &Mass) -> Result<(), Vec<Problem>> {
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
    for edge in 0..count {
        let next = (edge + 1) % count;
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
    }
    // The two discs are two authored planes one slab apart, so they are two elements: naming both
    // `Floor` put them in one shading group and one planarity group, and the merged validator
    // reported the group as a non-planar surface at `departure = slab/2`.
    let floor_top = Element::Mass(id, MassSurface::FloorTop);
    let slab_soffit = Element::Mass(id, MassSurface::SlabSoffit);
    let walked = polygon(&mut solid, &inner, &inside_floor, floor_top, true)
        && polygon(&mut solid, &outside, &outside_base, slab_soffit, false);
    if !walked {
        problems.push(Problem::FloorDegenerate { mass: id });
        return Err(problems);
    }
    solid.splice(mesh);
    Ok(())
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
