//! Reports what a building expands to, and whether the checks that read it can fire.
//!
//! ```text
//! cargo run -p whiting-building --example expand_probe
//! ```
//!
//! This is a report and not a test: it prints measured facts — face counts, volumes, boundary
//! loops, euler characteristics — about hand-authored buildings, and then breaks them ways to show
//! the checks are not reading zero because they never fire. A check that has only ever seen a good
//! building is indistinguishable from a check that does nothing.

use std::collections::{BTreeMap, BTreeSet};

use whiting_building::Building;
use whiting_building::expand::build;
use whiting_building::grid::{FaceRef, Rhythm};
use whiting_building::mass::{Mass, MassId, PlanNode};
use whiting_building::mesh::{BuildingMesh, Element, MassSurface, RoofSurface, SolidId};
use whiting_building::openings::{Inset, Opening, RevealSide};
use whiting_building::ops;
use whiting_building::roof::{CrestPoint, EaveStation, Roof, RoofId, RoofPlane, RoofRow};
use whiting_building::skeleton::BuildingSkeleton;

const WIDTH: f32 = 6.0;
const DEPTH: f32 = 10.0;
const EAVE: f32 = 2.8;
const RIDGE: f32 = 5.0;
const THICKNESS: f32 = 0.3;

/// The house's mass: a 6 by 10 rectangle, its outline wound so that [`Mass::plan_area`] is
/// positive — which, in the frame `mountain::math` defines (`x` east, `y` up, `z` south), puts the
/// enclosed ground on the walker's **right** and every outer face away from it.
fn house_mass() -> Mass {
    Mass::new(
        vec![
            PlanNode::new([0.0, 0.0], 0.0),
            PlanNode::new([WIDTH, 0.0], 0.0),
            PlanNode::new([WIDTH, DEPTH], 0.0),
            PlanNode::new([0.0, DEPTH], 0.0),
        ],
        3.0,
    )
    .with_wall(0.3)
    .with_slab(0.4)
}

/// The house's eave loop: the four corners, level, at the top of the wall.
fn house_eaves() -> Vec<PlanNode> {
    vec![
        PlanNode::new([0.0, 0.0], EAVE),
        PlanNode::new([WIDTH, 0.0], EAVE),
        PlanNode::new([WIDTH, DEPTH], EAVE),
        PlanNode::new([0.0, DEPTH], EAVE),
    ]
}

/// Returns the station an arc length along the eave loop names.
///
/// The loop is `2 * (WIDTH + DEPTH)` long, so every length here divides it exactly and its station
/// resolves to exactly one loop node; a station a hair the other side of a corner would sample a
/// point a hair off it, and this report is about exact numbers.
///
/// **The four corners stand at `0`, `WIDTH`, `WIDTH + DEPTH` and `2 * WIDTH + DEPTH` of arc
/// length**, and only those four lengths name a corner: the loop is a rectangle walked in plan, so
/// the corner spacing follows the sides (`WIDTH`, `DEPTH`, `WIDTH`, `DEPTH`) rather than the order
/// the sides are drawn in. A plane that names anything else lands **mid-edge**, and the skin then
/// skips the corner it stepped over: the roof still closes, every closure predicate is green, and
/// the volume is short by the strip it never covered. `WIDTH + 2 * DEPTH` is the one that reads as
/// a corner for a square and lands a third of the way along the fourth side of anything else.
fn station(along: f32) -> EaveStation {
    EaveStation::new(along / (2.0 * (WIDTH + DEPTH)))
}

/// A hip roof: two trapezoids down the long sides, two triangles over the short ends.
///
/// **The planes are chained** — each plane's first row is the plane before it`'s last row, naming
/// the same crest point and the same eave station — which is how the skin is closed without two
/// constructions agreeing by luck. The run turns over at every hip, and the walk does not care: a
/// skin facet is wound so that it looks up, whichever way its two rows were written.
fn house_roof() -> Roof {
    let crest = vec![
        CrestPoint::new([WIDTH / 2.0, 2.5], RIDGE),
        CrestPoint::new([WIDTH / 2.0, DEPTH - 2.5], RIDGE),
    ];
    let row = |crest: usize, along: f32| RoofRow::new(crest, station(along));
    let planes = vec![
        // The near trapezoid, off the front two corners.
        RoofPlane::new(vec![row(0, 0.0), row(1, WIDTH)]),
        // The right end triangle, off the front and back right corners.
        RoofPlane::new(vec![row(1, WIDTH), row(1, WIDTH + DEPTH)]),
        // The far trapezoid, run the other way so that it shares the ridge with the near one.
        RoofPlane::new(vec![row(1, WIDTH + DEPTH), row(0, 2.0 * WIDTH + DEPTH)]),
        // The left end triangle, closing onto the first plane's first row.
        RoofPlane::new(vec![row(0, 2.0 * WIDTH + DEPTH), row(0, 0.0)]),
    ];
    Roof::new(MassId(0), house_eaves(), THICKNESS)
        .with_crest(crest)
        .with_planes(planes)
}

/// Returns the distinct solids a mesh holds, in the order its faces first name them.
fn solids(mesh: &BuildingMesh) -> Vec<SolidId> {
    let mut found: Vec<SolidId> = Vec::new();
    for face in &mesh.faces {
        let solid = face.element.solid();
        if !found.contains(&solid) {
            found.push(solid);
        }
    }
    found
}

/// Prints the measurements every solid of a mesh is read by.
fn report(name: &str, mesh: &BuildingMesh) {
    let (low, high) = mesh.bounds();
    println!("{name}");
    println!(
        "  {} points, {} faces, bounds x [{:.1}, {:.1}] y [{:.2}, {:.2}] z [{:.1}, {:.1}]",
        mesh.points.len(),
        mesh.faces.len(),
        low[0],
        high[0],
        low[1],
        high[1],
        low[2],
        high[2],
    );
    for solid in solids(mesh) {
        println!(
            "  {solid:?}: {} faces, loops {}, euler {}, manifold {}, volume {:.3}, plan area {:.3}",
            mesh.faces_of(solid).len(),
            mesh.solid_boundary_loops(solid),
            mesh.solid_euler_characteristic(solid),
            mesh.solid_is_manifold(solid),
            mesh.solid_signed_volume(solid),
            mesh.solid_plan_area(solid),
        );
    }
    let mut counts: Vec<(Element, usize)> = Vec::new();
    for face in &mesh.faces {
        match counts
            .iter_mut()
            .find(|(element, _)| *element == face.element)
        {
            Some((_, count)) => *count += 1,
            None => counts.push((face.element, 1)),
        }
    }
    for (element, count) in counts {
        println!("    {element:?}: {count} face(s)");
    }
}

/// Prints the problems a skeleton reports instead of a mesh.
fn report_problems(name: &str, skeleton: &BuildingSkeleton) {
    println!("{name}");
    match build(skeleton) {
        Ok(mesh) => println!("  built anyway: {} faces", mesh.faces.len()),
        Err(failure) => {
            println!("  {} problem(s), no mesh", failure.problems.len());
            for problem in &failure.problems {
                println!("    {problem:?}");
            }
        }
    }
}

/// Returns a copy of a mesh with one solid's winding reversed.
fn with_reversed_solid(mesh: &BuildingMesh, solid: SolidId) -> BuildingMesh {
    let mut flipped = mesh.clone();
    for face in &mut flipped.faces {
        if face.element.solid() == solid {
            face.indices.swap(1, 2);
        }
    }
    flipped
}

/// Returns a mass whose wall is thicker than half its own outline.
fn over_walled() -> Mass {
    Mass::new(
        vec![
            PlanNode::new([0.0, 0.0], 0.0),
            PlanNode::new([2.0, 0.0], 0.0),
            PlanNode::new([2.0, 2.0], 0.0),
            PlanNode::new([0.0, 2.0], 0.0),
        ],
        3.0,
    )
    .with_wall(1.5)
    .with_slab(0.4)
}

/// Returns a mass with a corner sharper than a miter may reach.
fn spiky() -> Mass {
    Mass::new(
        vec![
            PlanNode::new([0.0, 0.0], 0.0),
            PlanNode::new([10.0, 0.0], 0.0),
            PlanNode::new([2.0, 0.1], 0.0),
            PlanNode::new([10.0, 4.0], 0.0),
        ],
        3.0,
    )
    .with_wall(0.1)
    .with_slab(0.4)
}

/// Returns an L-shaped mass, whose offset loop is concave and so has to be clipped rather than
/// fanned.
fn l_shaped() -> Mass {
    Mass::new(
        vec![
            PlanNode::new([0.0, 0.0], 0.0),
            PlanNode::new([6.0, 0.0], 0.0),
            PlanNode::new([6.0, 4.0], 0.0),
            PlanNode::new([3.0, 4.0], 0.0),
            PlanNode::new([3.0, 8.0], 0.0),
            PlanNode::new([0.0, 8.0], 0.0),
        ],
        3.0,
    )
    .with_wall(0.3)
    .with_slab(0.4)
}

/// Returns an axis-aligned mass over the plan rectangle two corners name.
///
/// The corners are given in the order `low`, then `+x`, then `+z`, which is the winding
/// [`Mass::plan_area`] reads as positive.
fn box_mass(low: [f32; 2], high: [f32; 2]) -> Mass {
    Mass::new(
        vec![
            PlanNode::new(low, 0.0),
            PlanNode::new([high[0], low[1]], 0.0),
            PlanNode::new(high, 0.0),
            PlanNode::new([low[0], high[1]], 0.0),
        ],
        3.0,
    )
    .with_wall(0.3)
    .with_slab(0.4)
}

/// Returns a skeleton whose host outline `ops::attach_mass` split at `station`.
///
/// **The host that leaves is not the host that was written.** The op inserts a node standing
/// exactly between the two the station fell between, and that node is *collinear* with both of
/// them — the split is an interpolation along one edge, so it cannot be anything else. No author
/// typed it and nothing about the outline turns at it.
fn with_wing(host: Mass, station: f32, wing: Mass) -> BuildingSkeleton {
    let mut skeleton = BuildingSkeleton::new(vec![host], Vec::new());
    ops::attach_mass(&mut skeleton, MassId(0), station, wing).expect("the host outline splits");
    skeleton
}

/// Returns a copy of the house with a rhythm on every face: three bays across, two storeys up.
///
/// **The grid is authored on all four faces and the openings only on one of them**, because the
/// faces with no opening are the control: they must expand to exactly what they expanded to before
/// openings existed, which is what the un-cut numbers in each section are compared against.
fn gridded_house() -> BuildingSkeleton {
    let mut skeleton = BuildingSkeleton::new(vec![house_mass()], Vec::new());
    let columns = Rhythm::Even { count: 3 };
    let rows = Rhythm::Even { count: 2 };
    for face in skeleton.faces_of(MassId(0)) {
        ops::split_face(&mut skeleton, face, &columns, &rows).expect("the face is there");
    }
    skeleton
}

/// Adds one opening and returns its index, or `None` when the verb refuses it.
fn add(
    skeleton: &mut BuildingSkeleton,
    face: FaceRef,
    row: usize,
    column: usize,
    inset: Inset,
) -> usize {
    ops::add_opening(skeleton, Opening::new(face, row, column, inset))
        .expect("the slot is in the grid")
}

/// Prints what a set of openings takes out of one mass, and how the solid reads afterwards.
///
/// The number this exists for is `taken`: a wall with a hole holds less material by exactly
/// `hole area × wall thickness`, and that is a check no predicate about closure can make — a solid
/// built inside out, or one with a hole in the wrong place, still closes and still has Euler 2.
fn report_cut(name: &str, skeleton: &BuildingSkeleton, uncut: f32, holes: &[usize]) {
    println!("{name}");
    let mass = SolidId::Mass(MassId(0));
    let analytic: f32 = holes
        .iter()
        .map(|index| skeleton.aperture(*index).expect("just authored").1)
        .map(|extents| extents.area() * skeleton.masses[0].wall)
        .sum();
    for (index, extents) in holes.iter().map(|index| {
        let (_, extents) = skeleton.aperture(*index).expect("just authored");
        (*index, extents)
    }) {
        println!(
            "  opening {index}: hole [{:.4}, {:.4}] x [{:.4}, {:.4}], {:.4} m² of wall",
            extents.from,
            extents.to,
            extents.low,
            extents.high,
            extents.area()
        );
    }
    match build(skeleton) {
        Ok(mesh) => {
            let cut = mesh.solid_signed_volume(mass);
            let reveals = mesh
                .faces
                .iter()
                .filter(|face| matches!(face.element, Element::Mass(_, MassSurface::Reveal { .. })))
                .count();
            let per_hole = mesh.faces.iter().fold(
                BTreeMap::<usize, BTreeSet<RevealSide>>::new(),
                |mut sides, face| {
                    if let Element::Mass(_, MassSurface::Reveal { opening, side, .. }) =
                        face.element
                    {
                        sides.entry(opening).or_default().insert(side);
                    }
                    sides
                },
            );
            println!(
                "  the mass: {} faces, loops {}, euler {}, manifold {}, volume {cut:.4}",
                mesh.faces_of(mass).len(),
                mesh.solid_boundary_loops(mass),
                mesh.solid_euler_characteristic(mass),
                mesh.solid_is_manifold(mass),
            );
            let wall_faces = mesh
                .faces
                .iter()
                .filter(|face| {
                    face.element == Element::Mass(MassId(0), MassSurface::OuterWall { edge: 0 })
                })
                .count();
            println!("  the first wall's outer face is {wall_faces} triangles");
            // A through-opening is a **tunnel**, which is a handle on the solid's own surface, so the
            // Euler characteristic is two less per hole than the un-cut shell's. The number printed
            // here is that expectation read back, and the wall's own edges are why: the reveal joins
            // two parts of one closed surface, and a surface with one handle is a torus.
            println!(
                "  euler {} against 2 - 2 x {} hole(s) = {}, manifold {}, plan area {:.4}",
                mesh.solid_euler_characteristic(mass),
                holes.len(),
                2 - 2 * holes.len() as i64,
                mesh.solid_is_manifold(mass),
                mesh.solid_plan_area(mass),
            );
            println!(
                "  un-cut {uncut:.4}, cut {cut:.4}, taken {:.4}",
                uncut - cut
            );
            println!(
                "  the holes' own areas say {analytic:.4}, and the difference is {:.6}",
                (uncut - cut) - analytic
            );
            println!(
                "  reveal faces {reveals} for {} hole(s), 4 quads = 8 triangles each",
                holes.len()
            );
            for (opening, sides) in &per_hole {
                println!(
                    "    opening {opening} owns {} sides {sides:?}, one quad each",
                    sides.len()
                );
            }
        }
        Err(failure) => {
            println!("  {} problem(s), no mesh", failure.problems.len());
            for problem in &failure.problems {
                println!("    {problem:?}");
            }
        }
    }
}

/// Returns a mass whose front edge stands on ground that rises a metre over its length.
///
/// **A hole can only cross a ground line where there is one to cross.** On the level house every
/// sill is at or above the ground by its own inset, so this is the only way to author the refusal:
/// the far end of a bay stands higher than the near end, and a window whose sill is below the far
/// end's ground is below the ground it stands on.
fn ramp_mass() -> Mass {
    Mass::new(
        vec![
            PlanNode::new([0.0, 0.0], 0.0),
            PlanNode::new([WIDTH, 0.0], 0.0),
            PlanNode::new([WIDTH, DEPTH], 2.0),
            PlanNode::new([0.0, DEPTH], 2.0),
        ],
        3.0,
    )
    .with_wall(0.3)
    .with_slab(0.4)
}

/// Returns the ramp, with a rhythm on its rising face.
fn gridded_ramp() -> (BuildingSkeleton, FaceRef) {
    let mut skeleton = BuildingSkeleton::new(vec![ramp_mass()], Vec::new());
    let face = FaceRef::new(MassId(0), 1);
    ops::split_face(
        &mut skeleton,
        face,
        &Rhythm::Even { count: 3 },
        &Rhythm::Even { count: 2 },
    )
    .expect("the face is there");
    (skeleton, face)
}

/// Returns a copy of the L-shaped mass with a rhythm on its first face and one window in it.
///
/// **A cut on a concave outline is where the two discs have to hold together.** The cut stations are
/// collinear nodes of the floor's loop and of the soffit's, and the ear clip has to walk a polygon
/// that is no longer four corners — with a reflex node in it — and still use every edge of the loop
/// once, which is what closes the floor against the inner wall and the soffit against the outer one.
fn gridded_wing() -> (BuildingSkeleton, FaceRef) {
    let mut skeleton = BuildingSkeleton::new(vec![l_shaped()], Vec::new());
    let face = FaceRef::new(MassId(0), 0);
    ops::split_face(
        &mut skeleton,
        face,
        &Rhythm::Even { count: 3 },
        &Rhythm::Even { count: 2 },
    )
    .expect("the face is there");
    (skeleton, face)
}

/// Prints what one solid of a mesh is closed with, and whether the host's base annulus was cut whole.
///
/// The annulus is the second half of the measurement: the ring carries one quad per outline node on
/// each of its four surfaces, and the base strip is one of them, so an `n`-node outline must close
/// with `2 x n` base facets — **degenerate ones included**. A base carrying fewer has lost a facet and
/// the two ring edges around the node that lost it are each used once. `nodes` is the host's own node
/// count and is printed for the host alone: the wing was never split, and its facets are not what
/// this section is about.
///
/// **This reads the base strip and not one of the shell's two discs**, because a mass has no discs: a
/// floor a person walks on is the plate's own top, a [`Slab`](whiting_building::slab::Slab) solid of
/// its own, and the base annulus is the ring's closing surface — the one the collinear node
/// `ops::attach_mass` inserts lands in, which is what the reading was for.
fn report_solid(mesh: &BuildingMesh, solid: SolidId, nodes: Option<usize>) {
    println!(
        "  {solid:?}: {} faces, loops {}, euler {}, manifold {}, volume {:.3}",
        mesh.faces_of(solid).len(),
        mesh.solid_boundary_loops(solid),
        mesh.solid_euler_characteristic(solid),
        mesh.solid_is_manifold(solid),
        mesh.solid_signed_volume(solid),
    );
    let (SolidId::Mass(id), Some(nodes)) = (solid, nodes) else {
        return;
    };
    let count = mesh
        .faces
        .iter()
        .filter(|face| face.element == Element::Mass(id, MassSurface::WallBase))
        .count();
    println!(
        "    the base annulus: {count} facet(s), and {nodes} node(s) ask for {}",
        2 * nodes
    );
}

/// Prints one host that `ops::attach_mass` split, and whatever the build says about it.
fn report_split_host(name: &str, skeleton: &BuildingSkeleton) {
    println!("{name}");
    let nodes = skeleton.masses[0].outline.len();
    let station = skeleton.masses[1]
        .attachment
        .map_or(0.0, |attachment| attachment.station);
    println!("  host outline: {nodes} node(s), wing attached at station {station:.6}");
    match build(skeleton) {
        Ok(mesh) => {
            for solid in solids(&mesh) {
                let split = (solid == SolidId::Mass(MassId(0))).then_some(nodes);
                report_solid(&mesh, solid, split);
            }
            // The crate's own validator, read over the whole document: four predicates of this
            // probe's own making agreeing with the instrument the phase ships is what separates
            // "closed" from "closed by luck", because the validator was written before this case
            // existed and knows nothing about it.
            let violations = Building::new(skeleton.clone()).validate();
            println!(
                "  {} violation(s) from the crate's own validator",
                violations.len()
            );
            for violation in violations {
                println!("    {violation:?}");
            }
        }
        Err(failure) => {
            println!("  {} problem(s), no mesh", failure.problems.len());
            for problem in &failure.problems {
                println!("    {problem:?}");
            }
        }
    }
}

fn main() {
    let house = BuildingSkeleton::new(vec![house_mass()], vec![house_roof()]);
    let mesh = build(&house).expect("the hand-authored house expands");
    report("the hand-authored house: one mass, one hip roof", &mesh);

    let ell = BuildingSkeleton::new(vec![l_shaped()], Vec::new());
    report(
        "an L-shaped mass, whose offset loop is concave",
        &build(&ell).expect("the L-shaped mass expands"),
    );

    println!();
    println!("hosts split by ops::attach_mass, which inserts a collinear node");
    // The rectangle's closed outline is 20 long and its first side 6, so arc length 3 stands half
    // way along that side.
    report_split_host(
        "  a convex host, split half way along its first side",
        &with_wing(
            box_mass([0.0, 0.0], [6.0, 4.0]),
            3.0 / 20.0,
            box_mass([2.0, -2.0], [4.0, 0.0]),
        ),
    );
    // The L's closed outline is 28 long and its first side 6, so the same arc length 3 lands half
    // way along it again; its one concave node stands at index 3, *after* the node the split
    // inserted at index 1.
    report_split_host(
        "  the same L, split on its first side, its concave node behind the split",
        &with_wing(l_shaped(), 3.0 / 28.0, box_mass([2.0, -2.0], [4.0, 0.0])),
    );
    // The same L walked from its concave node, so the concave node stands at index 0 and the split
    // at index 1, directly after it. The ear clip scans the surviving nodes in list order, so a
    // concave node ahead of the split node is what stops the clip from reaching a real corner
    // first: the split node is then chosen as an ear with no turn at all, between the two nodes it
    // stands between.
    report_split_host(
        "  the same L from its concave node, split on the edge that starts there",
        &with_wing(
            Mass::new(
                vec![
                    PlanNode::new([3.0, 4.0], 0.0),
                    PlanNode::new([3.0, 8.0], 0.0),
                    PlanNode::new([0.0, 8.0], 0.0),
                    PlanNode::new([0.0, 0.0], 0.0),
                    PlanNode::new([6.0, 0.0], 0.0),
                    PlanNode::new([6.0, 4.0], 0.0),
                ],
                3.0,
            )
            .with_wall(0.3)
            .with_slab(0.4),
            2.0 / 28.0,
            box_mass([3.5, 5.0], [5.5, 7.0]),
        ),
    );

    println!();
    println!("walls cut by openings, and the material the holes take out of them");

    // Every case below is the same house with a rhythm on all four faces, so the un-cut volume is
    // one number and each hole's share of it is `hole area x wall thickness`.
    let uncut = build(&gridded_house())
        .expect("the mass expands without openings")
        .solid_signed_volume(SolidId::Mass(MassId(0)));
    let face = FaceRef::new(MassId(0), 0);
    println!(
        "the un-cut mass is {uncut:.4}, and its wall is {} thick",
        gridded_house().masses[0].wall
    );

    let mut windowed = gridded_house();
    let window = add(&mut windowed, face, 0, 1, Inset::new(0.5, 0.5, 0.4, 0.3));
    report_cut(
        "a window, in a middle bay of the ground storey",
        &windowed,
        uncut,
        &[window],
    );

    let mut doored = gridded_house();
    // The sill inset is zero: a door's sill is the elevation the mass was authored at, and the
    // margin under it is the slab's own thickness, one side, and the floor's own edge, the other.
    // The two jambs stand further in than the wall's own thickness because the miter takes that much
    // off each end of the inner edge: a jamb at the corner would put its reveal on a corner the
    // neighbouring wall still claims whole, and that is refused below.
    let door = add(&mut doored, face, 0, 0, Inset::new(0.6, 0.6, 0.0, 0.2));
    report_cut(
        "a door, whose sill stands on the ground the mass was authored at",
        &doored,
        uncut,
        &[door],
    );

    let mut stacked = gridded_house();
    let below = add(&mut stacked, face, 0, 1, Inset::new(0.5, 0.5, 0.4, 0.3));
    let above = add(&mut stacked, face, 1, 1, Inset::new(0.5, 0.5, 0.4, 0.4));
    report_cut(
        "two windows stacked in one bay: the storey line between them is the mullion",
        &stacked,
        uncut,
        &[below, above],
    );

    // The same cut on a concave outline, where the floor and the soffit are no longer four corners:
    // the cut stations are collinear nodes of both loops and the clip has to carry them.
    let (mut wing, wing_face) = gridded_wing();
    let wing_volume = build(&gridded_wing().0)
        .expect("the L expands without openings")
        .solid_signed_volume(SolidId::Mass(MassId(0)));
    let glazed = add(&mut wing, wing_face, 0, 1, Inset::new(0.5, 0.5, 0.4, 0.3));
    report_cut(
        "the same window on the L-shaped outline, whose floor is no longer a quadrilateral",
        &wing,
        wing_volume,
        &[glazed],
    );

    println!();
    println!("what is refused, and no mesh for the mass that asked for it");
    // A hole past its own bay is refused by the verb, so the document is written past it on purpose:
    // the question here is what the expander says when it finds one.
    let mut past_bay = gridded_house();
    past_bay
        .openings
        .push(Opening::new(face, 0, 0, Inset::new(0.0, 3.0, 0.3, 0.3)));
    report_problems(
        "  a hole that reaches past its own bay, written past the verb that refuses it",
        &past_bay,
    );

    let (mut ramp, ramp_face) = gridded_ramp();
    add(&mut ramp, ramp_face, 0, 0, Inset::new(0.5, 0.5, 0.5, 0.2));
    report_problems(
        "  a window whose sill is under the ground line at its far jamb, on a rising wall",
        &ramp,
    );

    let mut overlapped = gridded_house();
    add(&mut overlapped, face, 0, 1, Inset::new(0.5, 0.5, 0.4, 0.3));
    add(&mut overlapped, face, 0, 1, Inset::new(0.8, 0.8, 0.5, 0.4));
    report_problems("  two holes in one bay, one inside the other", &overlapped);

    let mut orphaned = gridded_house();
    let lone = add(&mut orphaned, face, 1, 1, Inset::new(0.5, 0.5, 0.4, 0.3));
    ops::clear_face(&mut orphaned, face);
    report_problems(
        "  an opening whose bay a rhythm cleared after the hole was authored",
        &orphaned,
    );
    println!("    (the opening it names was index {lone})");

    // The ground line passes one sill's elevation inside a band that carries no hole at all, so the
    // gap under that sill there would be a quad folded on itself: a bow tie, which closes over
    // nothing and reads as no fault of any kind. Both windows are legal where they stand — each sill
    // is above the ground under its own two jambs — and it is the stretch between them that fails.
    let (mut folded, folded_face) = gridded_ramp();
    add(
        &mut folded,
        folded_face,
        0,
        0,
        Inset::new(0.5, 0.5, 0.7, 0.2),
    );
    add(
        &mut folded,
        folded_face,
        0,
        1,
        Inset::new(0.5, 0.5, 1.3, 0.1),
    );
    report_problems(
        "  two windows, the lower sill crossing the ground line in the stretch between them",
        &folded,
    );

    // A window on the rising wall that clears its own ground: the gap under it has a **sloped bottom
    // edge** from one jamb to the other, and is still planar, because the whole wall lies in one
    // vertical plane.
    let (mut rising, rising_face) = gridded_ramp();
    let high = add(
        &mut rising,
        rising_face,
        1,
        2,
        Inset::new(0.5, 0.5, 0.5, 0.3),
    );
    let ramp_volume = build(&gridded_ramp().0)
        .expect("the ramp expands without openings")
        .solid_signed_volume(SolidId::Mass(MassId(0)));
    report_cut(
        "a window on the rising wall, its gap's bottom edge sloped from jamb to jamb",
        &rising,
        ramp_volume,
        &[high],
    );

    println!();
    println!("positive controls");

    let mass = SolidId::Mass(MassId(0));
    let inside_out = with_reversed_solid(&mesh, mass);
    println!(
        "  one solid wound the other way: its volume is {:.3}, against {:.3} as built",
        inside_out.solid_signed_volume(mass),
        mesh.solid_signed_volume(mass),
    );

    let mut without_one = mesh.faces_of(mass);
    let dropped = without_one.pop().expect("the mass has faces to drop");
    println!(
        "  one face dropped from that solid (face {dropped}): loops {}, euler {}, volume {:.3}",
        mesh.boundary_loops_of(&without_one),
        mesh.euler_characteristic_of(&without_one),
        mesh.signed_volume_of(&without_one),
    );

    let roof = SolidId::Roof(RoofId(0));
    let mut without_one = mesh.faces_of(roof);
    without_one.pop();
    println!(
        "  one face dropped from the roof instead: loops {}, euler {}",
        mesh.boundary_loops_of(&without_one),
        mesh.euler_characteristic_of(&without_one),
    );

    report_problems(
        "  a wall wider than half its own outline",
        &BuildingSkeleton::new(vec![over_walled()], Vec::new()),
    );
    report_problems(
        "  an outline corner sharper than a join may reach",
        &BuildingSkeleton::new(vec![spiky()], Vec::new()),
    );

    let mut nameless = house_roof().with_planes(vec![RoofPlane::new(vec![
        RoofRow::new(7, station(0.0)),
        RoofRow::new(1, station(WIDTH)),
    ])]);
    nameless.planes[0].rows[1].eave = station(WIDTH);
    report_problems(
        "  a roof row naming a crest point that is not there",
        &BuildingSkeleton::new(vec![house_mass()], vec![nameless]),
    );

    report_problems(
        "  a mass and a roof that both fail, so the report is a list",
        &BuildingSkeleton::new(
            vec![spiky(), over_walled()],
            vec![house_roof().with_planes(vec![RoofPlane::new(vec![
                RoofRow::new(0, station(0.0)),
                RoofRow::new(9, station(WIDTH)),
            ])])],
        ),
    );

    println!();
    println!("the fascia rule on a shed roof");
    let shed = BuildingSkeleton::new(
        vec![house_mass()],
        vec![
            Roof::new(MassId(0), house_eaves(), THICKNESS)
                .with_crest(vec![
                    CrestPoint::new([0.0, 0.0], RIDGE),
                    CrestPoint::new([WIDTH, 0.0], RIDGE),
                ])
                .with_planes(vec![RoofPlane::new(vec![
                    RoofRow::new(0, station(0.0)),
                    RoofRow::new(1, station(DEPTH)),
                ])]),
        ],
    );
    let shed_mesh = build(&shed).expect("the shed roof expands");
    let shed_roof = SolidId::Roof(RoofId(0));
    let of_surface = |wanted: fn(&Element) -> bool| -> Vec<usize> {
        shed_mesh
            .faces
            .iter()
            .enumerate()
            .filter(|(_, face)| wanted(&face.element))
            .map(|(index, _)| index)
            .collect::<Vec<_>>()
    };
    let skin = of_surface(|element| matches!(element, Element::Roof(_, RoofSurface::Skin { .. })));
    let fascia =
        of_surface(|element| matches!(element, Element::Roof(_, RoofSurface::Fascia { .. })));
    // The boundary count is read from the skin itself, because one boundary edge is what *causes*
    // one fascia band: counting the bands and calling the number the boundary would make the two
    // agree by construction rather than by measurement, which is the same "closed by luck" question
    // this probe exists to keep asking.
    println!(
        "  one plane whose skin has {} boundary edges, and {} fascia face(s): loops {}, euler {}, manifold {}, volume {:.3}",
        shed_mesh.boundary_edges_of(&skin).len(),
        fascia.len(),
        shed_mesh.solid_boundary_loops(shed_roof),
        shed_mesh.solid_euler_characteristic(shed_roof),
        shed_mesh.solid_is_manifold(shed_roof),
        shed_mesh.solid_signed_volume(shed_roof),
    );
}
