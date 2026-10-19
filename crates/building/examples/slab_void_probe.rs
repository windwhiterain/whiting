//! The holed slab's acceptance: a floor with a way through it, measured against its own arithmetic.
//!
//! A slab with a hole is a prism over a **ring**: its two faces are annuli and the hole's wall is a face
//! of its own that looks into the hole. What must hold is that the solid closes (a torus, euler `0`),
//! that its volume is its own area times its thickness, and that the hole's wall faces the way it must —
//! into the void, where the material is not.
//!
//! Run with `cargo run -p whiting-building --example slab_void_probe`.

use whiting_building::slab::SlabId;
use whiting_building::slab::SlabSurface;
use whiting_building::{Building, BuildingSkeleton, Slab, SolidId};

fn main() {
    // A 6 x 4 floor with a 1.2 x 1.2 stairwell through it, 0.3 m thick.
    let plate = vec![[0.0, 0.0], [6.0, 0.0], [6.0, 4.0], [0.0, 4.0]];
    let hole = vec![[2.4, 1.4], [3.6, 1.4], [3.6, 2.6], [2.4, 2.6]];
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    skeleton
        .slabs
        .push(Slab::floor(plate.clone(), 2.7, 3.0).with_void(hole.clone()));
    let built = Building::new(skeleton.clone())
        .build()
        .expect("a holed floor expands");
    let violations = Building::new(skeleton.clone()).validate();
    println!("# a 6 x 4 floor with a 1.2 x 1.2 stairwell through it, 0.3 m thick");
    println!("  the document reports {} violation(s)", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }
    {
        let faces: Vec<usize> = (0..built.faces.len()).collect();
        let boundary = built.boundary_edges_of(&faces);
        println!("  debug: {} boundary edge(s)", boundary.len());
        for (from, to) in boundary.iter().take(8) {
            println!(
                "    {:.3},{:.3},{:.3} -> {:.3},{:.3},{:.3}",
                built.points[*from as usize][0],
                built.points[*from as usize][1],
                built.points[*from as usize][2],
                built.points[*to as usize][0],
                built.points[*to as usize][1],
                built.points[*to as usize][2]
            );
        }
    }
    assert_eq!(
        violations.len(),
        0,
        "a holed floor is a document like any other"
    );

    let solid = SolidId::Slab(SlabId(0));
    let faces = built.faces_of(solid);
    let euler = built.solid_euler_characteristic(solid);
    let volume = built.solid_signed_volume(solid);
    let expected = (6.0 * 4.0 - 1.2 * 1.2) * 0.3;
    println!(
        "  {} faces, loops {}, euler {euler} (a torus, not a box), manifold {}, volume {volume:.6} m³ \
         against {expected:.6}",
        faces.len(),
        built.solid_boundary_loops(solid),
        built.solid_is_manifold(solid)
    );
    assert_eq!(euler, 0, "a slab with one hole is a torus");
    assert!(built.solid_is_manifold(solid), "the hole is still closed");
    assert!(
        (volume - expected).abs() < 1.0e-4,
        "the volume is the ring's area times the thickness: {volume} against {expected}"
    );

    // **The hole's wall faces into the hole.** A face's normal at the hole's four sides points at the
    // hole's own centre, which is the side the material is not on.
    println!("\n# the hole's wall looks into the hole");
    let centre = [3.0_f32, 1.7];
    let mut inward = 0;
    let mut outward = 0;
    for face in &built.faces {
        if !matches!(
            face.element,
            whiting_building::Element::Slab(_, SlabSurface::VoidEdge { .. })
        ) {
            continue;
        }
        let normal = built.normal(face);
        let corners = built.triangle(face);
        let middle = [
            (corners[0][0] + corners[1][0] + corners[2][0]) / 3.0,
            (corners[0][2] + corners[1][2] + corners[2][2]) / 3.0,
        ];
        let toward = [centre[0] - middle[0], centre[1] - middle[1]];
        let dot = normal[0] * toward[0] + normal[2] * toward[1];
        if dot > 0.0 {
            inward += 1;
        } else {
            outward += 1;
        }
    }
    println!("  {inward} side face(s) look into the hole and {outward} look away");
    assert!(outward == 0, "every side of the hole must face the hole");

    println!("\n# a hole that cannot be filled is refused by name");
    let mut broken = BuildingSkeleton::new(Vec::new(), Vec::new());
    // An L-shaped loop: `ring_fill` bridges convex loops, and this one is not convex.
    let star = vec![
        [2.4, 1.4],
        [3.6, 1.4],
        [3.6, 2.0],
        [2.8, 2.0],
        [2.8, 2.6],
        [2.4, 2.6],
    ];
    broken
        .slabs
        .push(Slab::floor(plate, 2.7, 3.0).with_void(star));
    let problems = Building::new(broken)
        .build()
        .err()
        .map(|error| error.problems);
    let names: Vec<String> = problems
        .unwrap_or_default()
        .iter()
        .map(|problem| {
            format!("{problem:?}")
                .split(' ')
                .next()
                .unwrap_or("")
                .to_owned()
        })
        .collect();
    println!("  a non-convex hole: {names:?}");
    assert!(
        names.iter().any(|name| name == "SlabVoidNotFilled"),
        "a hole that cannot be filled must be refused"
    );
}
