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

use whiting_building::Building;
use whiting_building::expand::build;
use whiting_building::mass::{Mass, MassId, PlanNode};
use whiting_building::mesh::{BuildingMesh, Element, MassSurface, RoofSurface, SolidId};
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

/// Prints what one solid of a mesh is closed with, and whether the host's two discs were cut whole.
///
/// The disc count is the second half of the measurement: an `n`-node loop triangulates into
/// exactly `n - 2` facets, **degenerate ones included**, so a disc carrying fewer than that has
/// lost a facet and the two ring edges around the node that lost it are each used once. `nodes` is
/// the host's own node count and is printed for the host alone: the wing was never split, and its
/// facets are not what this section is about.
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
    for surface in [MassSurface::FloorTop, MassSurface::SlabSoffit] {
        let count = mesh
            .faces
            .iter()
            .filter(|face| face.element == Element::Mass(id, surface))
            .count();
        println!(
            "    {surface:?}: {count} facet(s), and {nodes} node(s) ask for {}",
            nodes - 2
        );
    }
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
