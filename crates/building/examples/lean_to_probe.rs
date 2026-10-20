//! Measures how far a **lean-to's** faces stand off one plane, and prints the roof's own rows beside it.
//!
//! # Why this exists
//!
//! A drum tower's skirts are `derive::lean_to` calls, and `validate` refused twenty-eight of them with
//! `FaceNotPlanar { departure: 0.0102 .. 0.0202 }` — ten to twenty times `PLANARITY_TOLERANCE`. The four plan
//! points of a skirt are a rectangle at two heights, so the single plane's four vertices *are* coplanar by
//! construction, and reading the derivation did not say where the departure comes from. This probe says it:
//! every triangle of the roof's solid is measured against the plane through the roof's own crest points and
//! its own eave row, so a face that is off is named with its three corners.
//!
//! Run with `cargo run -p whiting-building --example lean_to_probe`.

use whiting_building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, derive};

/// The mass the shed hangs from, and the shed's own numbers — the tower's skirt, in the small.
const HALF_X: f32 = 3.0;
const HALF_Z: f32 = 2.3;
const TOP: f32 = 3.2;
const REACH: f32 = 1.5;
const DROP: f32 = 0.72;
const THICKNESS: f32 = 0.12;

fn main() {
    println!("# one lean-to on a rectangular wall, which is what a drum tower's skirt is");
    let building = document();
    let built = match building.build() {
        Ok(mesh) => mesh,
        Err(error) => {
            println!(
                "  the document does not expand: {} problem(s)",
                error.problems.len()
            );
            for problem in error.problems.iter().take(8) {
                println!("    {problem:?}");
            }
            return;
        }
    };
    let violations = building.validate();
    println!("  violations: {}", violations.len());
    for violation in violations.iter().take(8) {
        println!("    {violation:?}");
    }

    // **The plane the skin should lie in**: the eave edge (front-left to front-right, at `low`) and the crest
    // edge (back-left to back-right, at `high`) are two parallel lines, so one plane holds both.
    let low = TOP - DROP;
    let out = HALF_Z + REACH;
    let plane = Plane::through(
        [-HALF_X, low, -out],
        [HALF_X, low, -out],
        [-HALF_X, TOP, -HALF_Z],
    );
    println!(
        "\n  the plane: through ({:.1}, {low:.2}, {:.1}), ({HALF_X:.1}, {low:.2}, {:.1}), ({:.1}, {TOP:.2}, {:.1})",
        -HALF_X, -out, -out, -HALF_X, -HALF_Z
    );

    let roof = whiting_building::SolidId::Roof(whiting_building::RoofId(0));
    let faces = built.faces_of(roof);
    println!("  the roof's solid has {} triangle(s)", faces.len());
    let mut worst: f32 = 0.0;
    for index in &faces {
        let face = &built.faces[*index];
        let triangle = built.triangle(face);
        let mut departure: f32 = 0.0;
        for corner in triangle {
            departure = departure.max(plane.distance(corner));
        }
        worst = worst.max(departure);
        if departure > 1.0e-4 {
            println!(
                "    face {index}: off by {departure:.6}  corners {:.3?}",
                triangle.map(|point| [point[0], point[1], point[2]])
            );
        }
    }
    println!("  worst departure over the whole roof: {worst:.6}");

    println!("\n# the whole tower: four skirts at seven levels and a crown on the same mass");
    {
        let mut document = four_skirts(7);
        let violations = document.validate();
        println!("  skirts only: {} violation(s)", violations.len());
        // The crown, on the same host as the twenty-eight sheds.
        let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
        let top = 0.45 + 7.0 * 2.4;
        let mut mass = Mass::new(
            [
                PlanNode::new([-HALF_X, -HALF_Z], 0.45),
                PlanNode::new([HALF_X, -HALF_Z], 0.45),
                PlanNode::new([HALF_X, HALF_Z], 0.45),
                PlanNode::new([-HALF_X, HALF_Z], 0.45),
            ]
            .to_vec(),
            top,
        );
        mass.wall = 0.20;
        mass.slab = 0.28;
        let id = MassId(skeleton.masses.len());
        skeleton.masses.push(mass);
        let out_x = HALF_X + REACH;
        let out_z = HALF_Z + REACH;
        for level in 1..=7_usize {
            let line = 0.45 + level as f32 * 2.4;
            let plans: [[[f32; 2]; 4]; 4] = [
                [
                    [-HALF_X, -out_z],
                    [HALF_X, -out_z],
                    [HALF_X, -HALF_Z],
                    [-HALF_X, -HALF_Z],
                ],
                [
                    [out_x, -HALF_Z],
                    [out_x, HALF_Z],
                    [HALF_X, HALF_Z],
                    [HALF_X, -HALF_Z],
                ],
                [
                    [HALF_X, out_z],
                    [-HALF_X, out_z],
                    [-HALF_X, HALF_Z],
                    [HALF_X, HALF_Z],
                ],
                [
                    [-out_x, HALF_Z],
                    [-out_x, -HALF_Z],
                    [-HALF_X, -HALF_Z],
                    [-HALF_X, HALF_Z],
                ],
            ];
            for plan in plans {
                if let Some(shed) = derive::lean_to(id, plan, line - DROP, line, THICKNESS) {
                    skeleton.roofs.push(shed);
                }
            }
        }
        if let Some(hip) = derive::hip_roof(id, &skeleton.masses[id.0], 0.8, 1.8, 0.08, 0.18) {
            skeleton
                .roofs
                .push(hip.with_covering(whiting_building::Covering {
                    period: 0.28,
                    fill: 0.62,
                    height: 0.05,
                }));
        }
        let tower = Building::new(skeleton);
        let tower_violations = tower.validate();
        println!(
            "  skirts and a crown: {} violation(s)",
            tower_violations.len()
        );
        for violation in tower_violations.iter().take(12) {
            println!("    {violation:?}");
        }
        document = tower;
        let _ = document;
    }

    println!("\n# a hip over plans from square to long, and over rises from shallow to steep");
    for (deep, rise, overhang) in [
        (6.0_f32, 1.8_f32, 0.8_f32),
        (6.0, 2.6, 1.1),
        (4.6, 1.8, 0.8),
        (4.6, 2.6, 1.1),
        (2.0, 1.8, 0.8),
        (4.6, 0.9, 0.8),
    ] {
        let document = crown(deep, rise, overhang);
        let violations = document.validate();
        println!(
            "  {deep:.1} deep, rise {rise:.1}, overhang {overhang:.1}: {} violation(s)",
            violations.len()
        );
        for violation in violations.iter().take(2) {
            println!("    {violation:?}");
        }
    }

    println!("\n# four skirts a level, on one tall mass: the drum tower's own arrangement");
    for levels in [1_usize, 2, 7] {
        let document = four_skirts(levels);
        let violations = document.validate();
        println!("  {} level(s), {} violation(s)", levels, violations.len());
        for violation in violations.iter().take(3) {
            println!("    {violation:?}");
        }
    }

    println!("\n# a crest on the wall top, and a crest a storey below it");
    for tall in [false, true] {
        let document = two_skirts(tall);
        let violations = document.validate();
        println!(
            "  wall top {:.2}: {} violation(s)",
            if tall { 17.25 } else { TOP },
            violations.len()
        );
        for violation in violations.iter().take(4) {
            println!("    {violation:?}");
        }
    }
}

/// A plane through three points, as `n · x = d`.
struct Plane {
    normal: [f32; 3],
    d: f32,
}

impl Plane {
    fn through(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Self {
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let normal = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        let normal = [normal[0] / length, normal[1] / length, normal[2] / length];
        let d = normal[0] * a[0] + normal[1] * a[1] + normal[2] * a[2];
        Self { normal, d }
    }

    fn distance(&self, point: [f32; 3]) -> f32 {
        (self.normal[0] * point[0] + self.normal[1] * point[1] + self.normal[2] * point[2] - self.d)
            .abs()
    }
}

/// Returns a document with **one tall wall and two skirts**: one whose crest is the wall top, one whose
/// crest is a storey down. The tower')s skirts are the second kind, and the first kind is here as the
/// control.
fn two_skirts(tall: bool) -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let top = if tall { 17.25 } else { TOP };
    let mut mass = Mass::new(
        [
            PlanNode::new([-HALF_X, -HALF_Z], 0.0),
            PlanNode::new([HALF_X, -HALF_Z], 0.0),
            PlanNode::new([HALF_X, HALF_Z], 0.0),
            PlanNode::new([-HALF_X, HALF_Z], 0.0),
        ]
        .to_vec(),
        top,
    );
    mass.wall = 0.20;
    mass.slab = 0.28;
    let id = MassId(skeleton.masses.len());
    skeleton.masses.push(mass);
    let out = HALF_Z + REACH;
    let plan: [[f32; 2]; 4] = [
        [-HALF_X, -out],
        [HALF_X, -out],
        [HALF_X, -HALF_Z],
        [-HALF_X, -HALF_Z],
    ];
    // The crest on the wall top, and the crest one storey below it.
    for line in [top, top - 2.4] {
        if let Some(shed) = derive::lean_to(id, plan, line - DROP, line, THICKNESS) {
            skeleton
                .roofs
                .push(shed.with_covering(whiting_building::Covering {
                    period: 0.28,
                    fill: 0.62,
                    height: 0.05,
                }));
        }
    }
    Building::new(skeleton)
}

/// Returns a mass wearing one **hip**: the drum tower's crown, and the one piece the tower could not have.
fn crown(deep: f32, rise: f32, overhang: f32) -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let half_deep = deep * 0.5;
    let mut mass = Mass::new(
        [
            PlanNode::new([-HALF_X, -half_deep], 0.0),
            PlanNode::new([HALF_X, -half_deep], 0.0),
            PlanNode::new([HALF_X, half_deep], 0.0),
            PlanNode::new([-HALF_X, half_deep], 0.0),
        ]
        .to_vec(),
        TOP,
    );
    mass.wall = 0.20;
    mass.slab = 0.28;
    let id = MassId(skeleton.masses.len());
    skeleton.masses.push(mass);
    if let Some(hip) = derive::hip_roof(id, &skeleton.masses[id.0], overhang, rise, 0.08, 0.18) {
        skeleton
            .roofs
            .push(hip.with_covering(whiting_building::Covering {
                period: 0.28,
                fill: 0.62,
                height: 0.05,
            }));
    }
    Building::new(skeleton)
}

/// Returns a tower body with **four skirts at each of `levels` heights** — the drum tower')s own arrangement,
/// one mass and every skirt hosted by it.
fn four_skirts(levels: usize) -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let top = 0.45 + 7.0 * 2.4;
    let mut mass = Mass::new(
        [
            PlanNode::new([-HALF_X, -HALF_Z], 0.45),
            PlanNode::new([HALF_X, -HALF_Z], 0.45),
            PlanNode::new([HALF_X, HALF_Z], 0.45),
            PlanNode::new([-HALF_X, HALF_Z], 0.45),
        ]
        .to_vec(),
        top,
    );
    mass.wall = 0.20;
    mass.slab = 0.28;
    let id = MassId(skeleton.masses.len());
    skeleton.masses.push(mass);
    let out_x = HALF_X + REACH;
    let out_z = HALF_Z + REACH;
    for level in 1..=levels {
        let line = 0.45 + level as f32 * 2.4;
        let plans: [[[f32; 2]; 4]; 4] = [
            [
                [-HALF_X, -out_z],
                [HALF_X, -out_z],
                [HALF_X, -HALF_Z],
                [-HALF_X, -HALF_Z],
            ],
            [
                [out_x, -HALF_Z],
                [out_x, HALF_Z],
                [HALF_X, HALF_Z],
                [HALF_X, -HALF_Z],
            ],
            [
                [HALF_X, out_z],
                [-HALF_X, out_z],
                [-HALF_X, HALF_Z],
                [HALF_X, HALF_Z],
            ],
            [
                [-out_x, HALF_Z],
                [-out_x, -HALF_Z],
                [-HALF_X, -HALF_Z],
                [-HALF_X, HALF_Z],
            ],
        ];
        for plan in plans {
            if let Some(shed) = derive::lean_to(id, plan, line - DROP, line, THICKNESS) {
                skeleton
                    .roofs
                    .push(shed.with_covering(whiting_building::Covering {
                        period: 0.28,
                        fill: 0.62,
                        height: 0.05,
                    }));
            }
        }
    }
    Building::new(skeleton)
}

/// Returns the document: one mass and one lean-to hanging off its `-z` wall.
fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let mut mass = Mass::new(
        [
            PlanNode::new([-HALF_X, -HALF_Z], 0.0),
            PlanNode::new([HALF_X, -HALF_Z], 0.0),
            PlanNode::new([HALF_X, HALF_Z], 0.0),
            PlanNode::new([-HALF_X, HALF_Z], 0.0),
        ]
        .to_vec(),
        TOP,
    );
    mass.wall = 0.24;
    mass.slab = 0.30;
    let id = MassId(skeleton.masses.len());
    skeleton.masses.push(mass);
    let out = HALF_Z + REACH;
    let plan: [[f32; 2]; 4] = [
        [-HALF_X, -out],
        [HALF_X, -out],
        [HALF_X, -HALF_Z],
        [-HALF_X, -HALF_Z],
    ];
    let roof = derive::lean_to(id, plan, TOP - DROP, TOP, THICKNESS)
        .expect("a rectangle between two heights is a lean-to");
    skeleton.roofs.push(roof);
    Building::new(skeleton)
}
