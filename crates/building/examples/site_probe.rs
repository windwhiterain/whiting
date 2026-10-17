//! The site slice's acceptance: what a document holds as a **place**, and whether the reading is the
//! one an author would write down.
//!
//! The claim this slice makes is that a site is not a new document — the skeleton already holds several
//! buildings and the walls that enclose them — but that looking at it as one was missing. So the probe
//! builds a small place, seats it on a slope, and checks the report against arithmetic done by hand.
//!
//! Run with `cargo run -p whiting-building --example site_probe`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::site::{report, seat};
use whiting_building::wall::Wall;
use whiting_building::{Attachment, Building, BuildingSkeleton, MassId, PlanNode};

/// Returns one building's mass, named, standing at `(x, z)` with its outline's own ground.
fn shed(name: &str, at: [f32; 2], width: f32, depth: f32) -> whiting_building::Mass {
    let half = [width * 0.5, depth * 0.5];
    let outline = [
        [-half[0], -half[1]],
        [half[0], -half[1]],
        [half[0], half[1]],
        [-half[0], half[1]],
    ]
    .into_iter()
    .map(|corner| PlanNode::new([at[0] + corner[0], at[1] + corner[1]], 0.0))
    .collect();
    whiting_building::Mass::new(outline, 2.8)
        .with_wall(0.25)
        .with_slab(0.3)
        .named(name)
}

fn main() {
    let mut site = BuildingSkeleton::default();
    // Three buildings: two sheds twelve metres apart and a store **attached to the first shed** as a
    // wing, whose bounding box overlaps the shed's by 4.5 m². The overlap is legal because the
    // document says the store lands on the shed; a wing that named no host would be UnattachedOverlap.
    site.masses.push(shed("shed", [0.0, 0.0], 5.0, 8.0));
    site.masses.push(shed("shed east", [12.0, 0.0], 5.0, 8.0));
    site.masses.push(
        shed("store", [3.0, 0.0], 4.0, 3.0).with_attachment(Attachment {
            host: MassId(0),
            station: 2.0,
        }),
    );
    // A courtyard around the first shed: four sides and a gate in the south one.
    let node = |x: f32, z: f32| PlanNode::new([x, z], 0.0);
    site.walls = vec![
        Wall::new(node(-4.5, -5.0), node(4.5, -5.0), 0.3, 1.8),
        Wall::new(node(4.5, -5.0), node(4.5, 6.0), 0.3, 1.8),
        Wall::new(node(4.5, 6.0), node(1.0, 6.0), 0.3, 1.8),
        Wall::new(node(-1.0, 6.0), node(-4.5, 6.0), 0.3, 1.8),
        Wall::new(node(-4.5, 6.0), node(-4.5, -5.0), 0.3, 1.8),
    ];

    println!("# the place before it has a ground");
    let before = report(&site);
    println!(
        "  {} buildings, {} walls, {:.4} m of wall, footprint {:.4} m²",
        before.buildings.len(),
        before.walls,
        before.wall_length,
        before.footprint()
    );
    for building in &before.buildings {
        println!(
            "  {:?}: footprint {:.4} m², top {:.4}, lowest ground {:.4}, {} roofs, {} openings, {} \
             parts",
            building.name,
            building.footprint,
            building.top,
            building.lowest,
            building.roofs,
            building.openings,
            building.parts
        );
    }
    println!(
        "  overlaps: {:?} (the store's box against the shed's is 1.5 × 3 = 4.5 m² by hand)",
        before.overlaps
    );

    println!("\n# the same place, seated on the plane z = 0.3 x");
    let mut seated = site.clone();
    let moved = seat(&mut seated, |plan| 0.3 * plan[0], 0.0);
    let after = report(&seated);
    println!(
        "  {} masses seated and {} walls given their ground",
        moved.len(),
        moved.walls
    );
    println!(
        "  ground now runs from {:.4} to {:.4} (relief {:.4}), and by hand the sheds stand on {:.4} and \
         {:.4}",
        after.ground.unwrap().0,
        after.ground.unwrap().1,
        after.relief(),
        0.3 * 2.5,
        0.3 * 14.5
    );
    for building in &after.buildings {
        println!(
            "  {:?}: top {:.4}, its own lowest ground {:.4}",
            building.name, building.top, building.lowest
        );
    }

    println!("\n# and the place still builds and validates");
    let built = Building::new(seated.clone())
        .build()
        .expect("the site expands");
    println!(
        "  {} points, {} faces; violations {}",
        built.points.len(),
        built.faces.len(),
        Building::new(seated).validate().len()
    );

    println!("\n# a building found by name, and a name that is not a handle");
    let named = after
        .building(MassId(1))
        .expect("there is a second building");
    println!("  mass 1 is {:?}", named.name);
    let twins = {
        let mut same = site.clone();
        same.masses[2] = shed("shed", [3.0, 0.0], 4.0, 3.0);
        let reported = report(&same);
        reported
            .buildings
            .iter()
            .filter(|building| building.name.as_deref() == Some("shed"))
            .count()
    };
    println!("  two buildings may share a name: {twins} of them do here");

    println!("\n# a site with one building and no walls is still a site");
    let lone = {
        let mut one = BuildingSkeleton::default();
        one.masses.push(shed("lone", [0.0, 0.0], 5.0, 8.0));
        report(&one)
    };
    println!(
        "  {} building, {} walls, relief {:.4}",
        lone.buildings.len(),
        lone.walls,
        lone.relief()
    );

    println!("\n# and a derived house is a site too, without being told");
    let house = starter(&Starter {
        width: 7.0,
        depth: 11.0,
        roof: RoofStyle::Hip,
        ..Starter::default()
    });
    let house_report = report(&house.skeleton);
    println!(
        "  the starter: {} building(s), footprint {:.4} m², {} roofs, ground {:?}",
        house_report.buildings.len(),
        house_report.footprint(),
        house_report.buildings[0].roofs,
        house_report.ground
    );
}
