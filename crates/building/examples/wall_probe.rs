//! The wall slice's acceptance: a courtyard with a gate, and the numbers a box makes possible.
//!
//! A wall is the one element whose volume can be written down on paper — `length × thickness × mean
//! height` — and this probe compares that number with the signed volume the expansion measures. It also
//! checks the two facts that make a courtyard work: that several wall **solids** may meet at a corner
//! without any of them being open, and that a **gate is a gap** between two runs rather than a hole
//! through one.
//!
//! Run with `cargo run -p whiting-building --example wall_probe`.

use whiting_building::site::seat;
use whiting_building::wall::{Wall, WallId, WallSurface};
use whiting_building::{Building, BuildingSkeleton, Element, PlanNode, SolidId};

const THICKNESS: f32 = 0.30;
const TOP: f32 = 2.00;

fn node(x: f32, z: f32) -> PlanNode {
    PlanNode::new([x, z], 0.0)
}

/// Returns a 10 × 6 yard enclosed by four sides, with a 2.4 m **gate gap** in its south side.
///
/// The south side is therefore two runs and the gap between them; every other side is one. Nothing here
/// knows what a gate is — a gate is the space where no wall was authored.
fn courtyard() -> BuildingSkeleton {
    BuildingSkeleton {
        walls: vec![
            // the north side, one run
            Wall::new(node(-5.0, -3.0), node(5.0, -3.0), THICKNESS, TOP),
            // the east side
            Wall::new(node(5.0, -3.0), node(5.0, 3.0), THICKNESS, TOP),
            // the south side, split around the gate
            Wall::new(node(5.0, 3.0), node(1.2, 3.0), THICKNESS, TOP),
            Wall::new(node(-1.2, 3.0), node(-5.0, 3.0), THICKNESS, TOP),
            // the west side
            Wall::new(node(-5.0, 3.0), node(-5.0, -3.0), THICKNESS, TOP),
        ],
        ..Default::default()
    }
}

fn main() {
    let skeleton = courtyard();
    let written: f32 = skeleton.walls.iter().map(|wall| wall.volume()).sum();
    let length: f32 = skeleton.walls.iter().map(|wall| wall.length()).sum();
    println!(
        "# a 10 x 6 yard: {} walls, {:.4} m of wall, {:.4} m³ written down",
        skeleton.walls.len(),
        length,
        written
    );

    let built = Building::new(skeleton.clone())
        .build()
        .expect("five boxes expand");
    println!("\n# each run is a box, and the volume says so");
    let mut measured = 0.0;
    for index in 0..skeleton.walls.len() {
        let solid = SolidId::Wall(WallId(index));
        let volume = built.solid_signed_volume(solid);
        measured += volume;
        println!(
            "  wall {index}: {:.4} m long, {} faces, loops {}, euler {}, manifold {}, volume {:.6} \
             against {:.6} written",
            skeleton.walls[index].length(),
            built.faces_of(solid).len(),
            built.solid_boundary_loops(solid),
            built.solid_euler_characteristic(solid),
            built.solid_is_manifold(solid),
            volume,
            skeleton.walls[index].volume()
        );
    }
    println!(
        "  total {measured:.6} against {written:.6}, off by {:.2e}",
        (measured - written).abs()
    );

    println!("\n# the gate is the gap between two runs, measured");
    let east_of_gate = skeleton.walls[2].to.position[0];
    let west_of_gate = skeleton.walls[3].from.position[0];
    println!(
        "  the south side runs to x {east_of_gate:.4} and starts again at x {west_of_gate:.4}: a gate \
         {:.4} m wide",
        east_of_gate - west_of_gate
    );

    println!("\n# four walls meet at each corner and every one of them stays closed");
    let corners = [
        ([-5.0_f32, -3.0], "north-west"),
        ([5.0, -3.0], "north-east"),
        ([5.0, 3.0], "south-east"),
        ([-5.0, 3.0], "south-west"),
    ];
    for (corner, name) in corners {
        let touching = skeleton
            .walls
            .iter()
            .filter(|wall| {
                (wall.from.position[0] - corner[0]).abs() < 1.0e-6
                    && (wall.from.position[1] - corner[1]).abs() < 1.0e-6
                    || (wall.to.position[0] - corner[0]).abs() < 1.0e-6
                        && (wall.to.position[1] - corner[1]).abs() < 1.0e-6
            })
            .count();
        println!("  the {name} corner is shared by {touching} runs, and they interpenetrate there");
    }
    let open = (0..skeleton.walls.len())
        .filter(|index| built.solid_boundary_loops(SolidId::Wall(WallId(*index))) != 0)
        .count();
    println!("  walls that are not closed: {open}");

    println!("\n# the ground under the walls, and a wall that follows it");
    let mut sloped = skeleton.clone();
    let placed = seat(&mut sloped, |plan| 0.3 * plan[0], 0.0);
    println!(
        "  seating wrote {} walls' grounds and left {} terraces alone (there were none)",
        placed.walls,
        placed.len()
    );
    let north = &sloped.walls[0];
    println!(
        "  the north run now stands on {:.4} at one end and {:.4} at the other, its top at {:.4}, and \
         its volume is {:.6} against {:.6} before",
        north.from.elevation,
        north.to.elevation,
        north.top,
        north.volume(),
        skeleton.walls[0].volume()
    );
    let sloped_built = Building::new(sloped.clone())
        .build()
        .expect("the seated walls expand");
    let (low, high) = sloped_built.bounds_of(SolidId::Wall(WallId(0)));
    println!(
        "  its box now runs from y {:.4} to {:.4}, and it is still closed: {}",
        low[1],
        high[1],
        sloped_built.solid_boundary_loops(SolidId::Wall(WallId(0))) == 0
    );

    println!("\n# the three ways a wall cannot be one");
    let mut broken = skeleton.clone();
    broken
        .walls
        .push(Wall::new(node(0.0, 0.0), node(0.0, 0.0), THICKNESS, TOP));
    broken
        .walls
        .push(Wall::new(node(0.0, 0.0), node(1.0, 0.0), 0.0, TOP));
    broken
        .walls
        .push(Wall::new(node(0.0, 0.0), node(1.0, 0.0), THICKNESS, 0.0));
    let violations = Building::new(broken.clone()).validate();
    for violation in &violations {
        println!("  {violation:?}");
    }
    println!("  and the build refuses them rather than drawing a box of no thickness:");
    for wall in broken.walls.iter().skip(skeleton.walls.len()) {
        let mut one = BuildingSkeleton::default();
        one.walls.push(*wall);
        println!("    {:?}", Building::new(one).build().err());
    }

    println!("\n# identity names the walls too");
    let identity = whiting_building::identity::Identity::of(&built, 1);
    for row in identity.rows() {
        if matches!(row.element, Element::Wall(_, _)) {
            println!("  {}", row.path);
        }
    }
    let sides = built
        .faces
        .iter()
        .filter(|face| matches!(face.element, Element::Wall(_, WallSurface::Side { .. })))
        .count();
    println!("  {sides} faces are the walls' long sides");
}
