//! The census of a starter house: what this crate claims, measured rather than asserted.
//!
//! Prints the document's counts, every violation the validator reports, and — for each solid the
//! skeleton expands to — the four facts this phase's contract rests on: the boundary is closed
//! (zero loops), the surface is a sphere topologically (Euler characteristic 2), every edge is
//! shared by exactly two faces, and the signed volume is **positive**, which is what says the
//! faces point out of the building rather than into it.
//!
//! Run with `cargo run -p whiting-building --example census`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::{BuildingMesh, MassId, RoofId, SolidId, Violation};

fn main() {
    for (style, width, depth) in [
        (RoofStyle::Hip, 7.0, 11.0),
        (RoofStyle::Gable, 7.0, 11.0),
        (RoofStyle::Hip, 6.0, 6.0),
    ] {
        let building = starter(&Starter {
            width,
            depth,
            roof: style,
            ..Starter::default()
        });
        println!(
            "== {style:?} {width}x{depth}: masses {} roofs {}, crest {} , planes {}",
            building.skeleton.masses.len(),
            building.skeleton.roofs.len(),
            building.skeleton.roofs[0].crest.len(),
            building.skeleton.roofs[0].planes.len(),
        );
        println!(
            "   mass plan area {:.4}, highest outline {:.4}, wall top {:.4}, eave area {:.4}",
            building.skeleton.masses[0].plan_area(),
            building.skeleton.masses[0].highest(),
            building.skeleton.masses[0].top,
            building.skeleton.roofs[0].plan_area(),
        );

        let violations = building.validate();
        println!("   violations: {}", violations.len());
        for violation in &violations {
            // The element a departing face belongs to is the whole diagnosis: a group that spans
            // two authored planes is a naming fault, and one face off its own plane is geometry.
            match violation {
                Violation::FaceNotPlanar { face, departure } => {
                    let built = building.build().expect("the skeleton expands");
                    println!(
                        "     FaceNotPlanar {{ face: {face}, departure: {departure} }} on {:?}",
                        built.faces[*face].element
                    );
                }
                other => println!("     {other:?}"),
            }
        }

        match building.build() {
            Ok(mesh) => {
                println!("   points {} faces {}", mesh.points.len(), mesh.faces.len());
                report(&mesh, SolidId::Mass(MassId(0)));
                report(&mesh, SolidId::Roof(RoofId(0)));
            }
            Err(error) => println!("   did not expand: {:?}", error.problems),
        }
    }
}

/// Prints one solid's four facts and its two areas.
fn report(mesh: &BuildingMesh, solid: SolidId) {
    println!(
        "   {solid:?}: faces {} loops {} euler {} manifold {} volume {:.4} plan area {:.4}",
        mesh.faces_of(solid).len(),
        mesh.solid_boundary_loops(solid),
        mesh.solid_euler_characteristic(solid),
        mesh.solid_is_manifold(solid),
        mesh.solid_signed_volume(solid),
        mesh.solid_plan_area(solid),
    );
}
