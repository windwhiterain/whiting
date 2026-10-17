//! The floor slice's acceptance: an intermediate floor inside a mass's own walls, against arithmetic.
//!
//! The plate's plan is the mass's **inner** loop, so its area has a closed form — `(w - 2·wall)(d -
//! 2·wall)` — and the mesh's signed volume has one too. What the probe checks beyond it is that the inset is
//! the *same* inset the walls were built on, which is the whole reason the derivation is the crate's rather
//! than an author's: two insets would be two opinions about where the inside is.
//!
//! Run with `cargo run -p whiting-building --example floor_probe`.

use whiting_building::derive::{RoofStyle, Starter, floor_slab, starter};
use whiting_building::site::seat;
use whiting_building::slab::SlabId;
use whiting_building::{Building, MassId, SolidId};

const PLAN: Starter = Starter {
    width: 7.0,
    depth: 11.0,
    height: 3.6,
    wall: 0.35,
    slab: 0.45,
    overhang: 0.7,
    sink: 0.15,
    rise: 2.1,
    roof_thickness: 0.28,
    roof: RoofStyle::Hip,
    seed: 3,
};

const FLOOR_AT: f32 = 2.0;
const FLOOR_THICK: f32 = 0.2;

fn main() {
    let mut building = starter(&PLAN);
    let mass = building.skeleton.masses[0].clone();
    let floor = floor_slab(&mass, MassId(0), FLOOR_AT, FLOOR_THICK)
        .expect("the elevation is inside the building");
    building.skeleton.slabs.push(floor.clone());

    let inner_width = PLAN.width - 2.0 * PLAN.wall;
    let inner_depth = PLAN.depth - 2.0 * PLAN.wall;
    let inner_area = inner_width * inner_depth;
    println!(
        "# a {:.0} x {:.0} building with an intermediate floor",
        PLAN.width, PLAN.depth
    );
    println!(
        "  the floor's plan is {:.4} x {:.4} = {:.6} m² (w - 2 x wall by hand) against {:.6} from the \
         slab's own loop",
        inner_width,
        inner_depth,
        inner_area,
        floor.plan_area()
    );
    println!(
        "  it stands from y {:.4} to {:.4}, {:.4} thick, hosted by mass {} (which is seated with it)",
        floor.bottom,
        floor.top,
        floor.thickness(),
        floor.host.map(|id| id.0).unwrap_or(9)
    );

    let built = Building::new(building.skeleton.clone())
        .build()
        .expect("the building with a floor expands");
    let solid = SolidId::Slab(SlabId(0));
    println!("\n# the plate is closed and weighs what the paper says");
    println!(
        "  {} faces, loops {}, euler {}, manifold {}, volume {:.6} against {:.6} written",
        built.faces_of(solid).len(),
        built.solid_boundary_loops(solid),
        built.solid_euler_characteristic(solid),
        built.solid_is_manifold(solid),
        built.solid_signed_volume(solid),
        inner_area * FLOOR_THICK
    );

    println!("\n# the inset is the walls' own inset, on every side");
    let (mass_low, mass_high) = built.bounds_of(SolidId::Mass(MassId(0)));
    let (floor_low, floor_high) = built.bounds_of(solid);
    println!(
        "  in plan the floor is inset by x {:.4}/{:.4} and z {:.4}/{:.4}, against the wall's {:.4}",
        floor_low[0] - mass_low[0],
        mass_high[0] - floor_high[0],
        floor_low[2] - mass_low[2],
        mass_high[2] - floor_high[2],
        PLAN.wall
    );
    println!(
        "  and the building with a floor reports {} violation(s)",
        Building::new(building.skeleton.clone()).validate().len()
    );

    println!("\n# seating moves the floor with the building");
    let mut sloped = building.skeleton.clone();
    let report = seat(&mut sloped, |plan| 0.3 * plan[0], 0.0);
    let seated = Building::new(sloped.clone())
        .build()
        .expect("the seated building expands");
    let (seated_low, seated_high) = seated.bounds_of(solid);
    println!(
        "  {} mass and {} slab moved; the floor now runs y {:.4} .. {:.4}, which is the flat one plus \
         {:.4}",
        report.len(),
        report.slabs,
        seated_low[1],
        seated_high[1],
        report.masses[0].delta
    );
    println!(
        "  and the seated document has {} violation(s)",
        Building::new(sloped).validate().len()
    );

    println!("\n# a floor that is not inside the building is refused, not drawn");
    for (label, elevation, thickness) in [
        ("at the ground", 0.0, FLOOR_THICK),
        ("below the ground", -0.5, FLOOR_THICK),
        ("at the wall top", PLAN.height, FLOOR_THICK),
        ("above the wall top", PLAN.height + 0.5, FLOOR_THICK),
        ("of no thickness", FLOOR_AT, 0.0),
    ] {
        println!(
            "  {label}: {:?}",
            floor_slab(&mass, MassId(0), elevation, thickness).map(|slab| slab.top)
        );
    }

    println!("\n# a floor leaves a storey above it and one below");
    println!(
        "  {:.4} m below the floor's underside and {:.4} m above its top, in a {:.4} m building",
        FLOOR_AT - FLOOR_THICK,
        PLAN.height - FLOOR_AT,
        PLAN.height
    );
}
