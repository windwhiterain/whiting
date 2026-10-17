//! The slab slice's acceptance: a flat roof as a plate, against arithmetic that can be written down.
//!
//! A slab's volume is `plan area × thickness`, so the mesh's signed volume has a closed form to compare
//! with — and the caps of a rectangular plate triangulate to the same 12 faces a box has. What the probe
//! also checks is that the *building under the plate* is unchanged by the choice of roof: the mass of a
//! flat-roofed starter and of a hipped one over the same starter are the same mass, because a roof is
//! above the wall top and a plate is a plate.
//!
//! Run with `cargo run -p whiting-building --example slab_probe`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::site::seat;
use whiting_building::slab::{Slab, SlabId};
use whiting_building::{Building, BuildingSkeleton, MassId, SolidId};

/// The starter this probe puts a plate on.
const STARTER: Starter = Starter {
    width: 7.0,
    depth: 11.0,
    height: 3.6,
    wall: 0.35,
    slab: 0.45,
    overhang: 0.7,
    sink: 0.15,
    rise: 2.1,
    roof_thickness: 0.28,
    roof: RoofStyle::Flat,
    seed: 3,
};

fn main() {
    let building = starter(&STARTER);
    let skeleton = building.skeleton.clone();
    println!("# a 7 x 11 building with a flat roof");
    println!(
        "  the document holds {} mass(es), {} roof(s) and {} slab(s) — a plate is not a roof",
        skeleton.masses.len(),
        skeleton.roofs.len(),
        skeleton.slabs.len()
    );
    let slab = &skeleton.slabs[0];
    let eave_width = STARTER.width + 2.0 * STARTER.overhang;
    let eave_depth = STARTER.depth + 2.0 * STARTER.overhang;
    println!(
        "  its plan is {:.4} x {:.4} = {:.6} m² (w + 2 x overhang by hand), {:.4} thick, so {:.6} m³ \
         by hand against {:.6} from the slab's own area",
        eave_width,
        eave_depth,
        eave_width * eave_depth,
        slab.thickness(),
        eave_width * eave_depth * slab.thickness(),
        slab.volume()
    );
    println!(
        "  its underside is at {:.4}, which is the wall top {:.4} less the sink {:.4}",
        slab.bottom, skeleton.masses[0].top, STARTER.sink
    );

    let built = building.build().expect("the flat-roofed building expands");
    let solid = SolidId::Slab(SlabId(0));
    println!("\n# the plate is a closed solid with the volume on the paper");
    println!(
        "  {} faces, loops {}, euler {}, manifold {}, volume {:.6} against {:.6} written",
        built.faces_of(solid).len(),
        built.solid_boundary_loops(solid),
        built.solid_euler_characteristic(solid),
        built.solid_is_manifold(solid),
        built.solid_signed_volume(solid),
        eave_width * eave_depth * slab.thickness()
    );
    println!(
        "  and the shell under it reports 0 violations: {}",
        Building::new(skeleton.clone()).validate().len()
    );

    println!("\n# the building under the plate does not depend on the roof style");
    let hipped = starter(&Starter {
        roof: RoofStyle::Hip,
        ..STARTER
    });
    let hipped_built = hipped.build().expect("the hipped building expands");
    println!(
        "  mass volume {:.6} flat-roofed against {:.6} hipped, and {} against {} faces",
        built.solid_signed_volume(SolidId::Mass(MassId(0))),
        hipped_built.solid_signed_volume(SolidId::Mass(MassId(0))),
        built.faces_of(SolidId::Mass(MassId(0))).len(),
        hipped_built.faces_of(SolidId::Mass(MassId(0))).len()
    );
    println!(
        "  the hipped one has {} roof(s) and {} slab(s); the flat one the other way round",
        hipped.skeleton.roofs.len(),
        hipped.skeleton.slabs.len()
    );

    println!("\n# seating moves the plate with the building it covers");
    let mut sloped = skeleton.clone();
    let report = seat(&mut sloped, |plan| 0.3 * plan[0], 0.0);
    let seated = Building::new(sloped.clone())
        .build()
        .expect("the seated building expands");
    let (_mass_low, mass_high) = seated.bounds_of(SolidId::Mass(MassId(0)));
    let (slab_low, slab_high) = seated.bounds_of(SolidId::Slab(SlabId(0)));
    println!(
        "  {} mass seated and {} slab moved; the plate now runs y {:.4} .. {:.4} and the wall top {:.4}",
        report.len(),
        report.slabs,
        slab_low[1],
        slab_high[1],
        mass_high[1]
    );
    println!(
        "  its thickness is still {:.4}: {:.4}",
        slab.thickness(),
        slab_high[1] - slab_low[1]
    );
    println!(
        "  and the seated document has {} violation(s)",
        Building::new(sloped).validate().len()
    );

    println!("\n# a plate with no host stays where its author put it");
    let mut loose = BuildingSkeleton::default();
    loose.slabs.push(Slab::new(
        vec![[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]],
        5.0,
        5.4,
    ));
    let loose_report = seat(&mut loose, |plan| 0.3 * plan[0], 0.0);
    println!(
        "  masses seated {}, slabs moved {}, and the plate is still at {:.4} .. {:.4}",
        loose_report.len(),
        loose_report.slabs,
        loose.slabs[0].bottom,
        loose.slabs[0].top
    );

    println!("\n# the two ways a slab cannot be a prism");
    let mut broken = BuildingSkeleton::default();
    broken.slabs.push(Slab::new(
        vec![[0.0, 0.0], [2.0, 0.0], [2.0, 1.0]],
        1.0,
        1.0,
    ));
    broken
        .slabs
        .push(Slab::new(vec![[0.0, 0.0], [1.0, 1.0]], 0.0, 1.0));
    let violations = Building::new(broken.clone()).validate();
    for violation in &violations {
        println!("  {violation:?}");
    }
    println!("  and the build refuses them rather than drawing a plate of no thickness:");
    for index in 0..broken.slabs.len() {
        let mut one = BuildingSkeleton::default();
        one.slabs.push(broken.slabs[index].clone());
        println!("    {:?}", Building::new(one).build().err());
    }

    println!("\n# identity names the plate");
    let identity = whiting_building::identity::Identity::of(&built, 1);
    for row in identity.rows() {
        if row.path.starts_with("slab") {
            println!("  {}", row.path);
        }
    }
}
