//! Reports what the authoring verbs and the starter produce, before anything draws them.
//!
//! Every number below is a measurement of the **document** — the skeleton an author edits — rather
//! than of a mesh; `census`, `expand_probe` and `validate_probe` are where a document is expanded and
//! checked. The claims being measured here are the ones the authoring modules make about themselves:
//!
//! - the outline is wound so the solid is on the walker's **right** — the winding whose signed plan
//!   area is positive, which is the condition under which `up × along` points away from the solid
//!   (`Mass::plan_area` states the rule and the frame it was measured in) — and the eave loop covers
//!   it and stands outside it;
//! - the crest sits at `top - sink + rise`, and every row of every plane names a crest point that
//!   exists and a station inside `[0, 1)`;
//! - **two neighbouring planes share one row — literally the same crest index and the same eave
//!   station** — which is the whole of the watertightness claim, and it is measured here rather
//!   than asserted;
//! - the roof's planes are planar and wound outward, which is the claim that fixes the domain of
//!   `hip_roof` and `gable_roof`;
//! - the ops refuse what would leave a dangling index, and nothing else;
//! - the same seed derives the same building, and a different seed may or may not land on another
//!   of the three published ridge insets.
//!
//! ```text
//! cargo run -p whiting-building --example authoring_probe
//! ```

use whiting_building::derive::{self, Starter};
use whiting_building::math::{Vec3, cross3, dot3, length3, subtract};
use whiting_building::ops;
use whiting_building::{Join, Mass, MassId, PlanNode, Roof, RoofRow};

fn main() {
    let starter = Starter::default();

    println!("== the starter mass ==");
    let building = derive::starter(&starter);
    let mass = &building.skeleton.masses[0];
    println!(
        "outline nodes {}, plan area {:.3} — {}",
        mass.outline.len(),
        mass.plan_area(),
        if mass.plan_area() > 0.0 {
            "positive, so the solid is on the walker's right and `up × along` is the outer face"
        } else {
            "negative, so every wall's outer face points inward and the building is not drawn"
        }
    );
    println!(
        "top {:.2}, wall {:.2}, slab {:.2}, host of roof 0: {:?}",
        mass.top, mass.wall, mass.slab, building.skeleton.roofs[0].host.0
    );

    println!();
    println!("== the hip ==");
    let hip = derive::hip_roof(
        MassId(0),
        mass,
        starter.overhang,
        starter.rise,
        starter.sink,
        starter.roof_thickness,
    );
    report_roof("hip", hip.as_ref(), mass, &starter);
    println!(
        "crest spacing {:.3} against the hip's own inset, over a short span of {:.3}",
        crest_spacing(hip.as_ref().expect("a rectangle is a quadrilateral")),
        short_span(hip.as_ref().expect("a rectangle is a quadrilateral"))
    );

    println!();
    println!("== the gable ==");
    let gable = derive::gable_roof(
        MassId(0),
        mass,
        starter.overhang,
        starter.rise,
        starter.sink,
        starter.roof_thickness,
    );
    report_roof("gable", gable.as_ref(), mass, &starter);

    println!();
    println!("== refusals ==");
    refusals(mass, &starter);

    println!();
    println!("== the verbs ==");
    verbs();

    println!();
    println!("== the seed discipline ==");
    seed();
}

/// Prints every measurement of one roof against the mass it sits on.
fn report_roof(label: &str, roof: Option<&Roof>, mass: &Mass, starter: &Starter) {
    let Some(roof) = roof else {
        println!("{label}: None, which is the refusal this shape was meant to get");
        return;
    };
    println!(
        "{label}: {} eave nodes, eave length {:.3}, eave plan area {:.3} against the mass's {:.3} ({:.3} more, {:.2}x)",
        roof.eaves.len(),
        roof.eave_length(),
        roof.plan_area(),
        mass.plan_area(),
        roof.plan_area() - mass.plan_area(),
        roof.plan_area() / mass.plan_area()
    );
    // The cover claim, measured at the one place it can fail: the mid-point of each wall, where the
    // eave stands exactly `overhang` outside it if the mitre did its job.
    println!(
        "{label}:   standing outside the walls by {}",
        overhang_measured(mass, roof, starter.overhang)
    );

    let eave_elevation = mass.top - starter.sink;
    println!(
        "{label}: {} crest points at {} against the wanted {:.3}",
        roof.crest.len(),
        elevations(roof),
        eave_elevation + starter.rise
    );

    let mut rows = 0;
    let mut sound = 0;
    for (index, plane) in roof.planes.iter().enumerate() {
        let counts: Vec<String> = plane
            .rows
            .iter()
            .map(|row| {
                let in_range = row.crest < roof.crest.len();
                let station = row.eave.t;
                let in_station = (0.0..1.0).contains(&station);
                if in_range && in_station {
                    sound += 1;
                }
                format!(
                    "crest {} {} eave {:.4} {}",
                    row.crest,
                    if in_range { "in range" } else { "OUT OF RANGE" },
                    station,
                    if in_station {
                        "in [0,1)"
                    } else {
                        "OUTSIDE [0,1)"
                    }
                )
            })
            .collect();
        rows += plane.rows.len();
        println!(
            "{label}:   plane {index}: {} rows — {}",
            plane.rows.len(),
            counts.join("; ")
        );
    }
    println!(
        "{label}: {rows} rows, {sound} of them naming a crest point and a station inside [0,1)"
    );

    // The watertightness claim, measured: one row, written twice, compared for identity.
    let mut shared = 0;
    let mut edges = 0;
    let mut pairs = 0;
    for first in 0..roof.planes.len() {
        for second in first + 1..roof.planes.len() {
            pairs += 1;
            let common = roof.planes[first]
                .rows
                .iter()
                .filter(|row| roof.planes[second].rows.contains(row))
                .count();
            let same_crest: Vec<usize> = roof.planes[first]
                .rows
                .iter()
                .map(|row| row.crest)
                .collect::<std::collections::BTreeSet<_>>()
                .intersection(
                    &roof.planes[second]
                        .rows
                        .iter()
                        .map(|row| row.crest)
                        .collect::<std::collections::BTreeSet<_>>(),
                )
                .copied()
                .collect();
            if common > 0 {
                shared += common;
                println!("{label}:   planes {first}/{second}: {common} row(s) literally identical");
            } else if same_crest.len() == 2 {
                edges += 1;
                println!(
                    "{label}:   planes {first}/{second}: no shared row, they meet along crest {:?}",
                    same_crest
                );
            } else {
                println!("{label}:   planes {first}/{second}: meet on nothing");
            }
        }
    }
    println!(
        "{label}: {pairs} plane pair(s), {shared} shared row(s), {edges} shared crest edge(s)"
    );

    // The two claims the expander will lean on: each quad planar, each normal out of the solid.
    let mut worst_planarity = 0.0f32;
    let mut least_up = f32::INFINITY;
    let mut least_out = f32::INFINITY;
    for plane in &roof.planes {
        for pair in plane.rows.windows(2) {
            let Some(face) = quad(roof, &pair[0], &pair[1]) else {
                continue;
            };
            let normal = cross3(subtract(face[1], face[0]), subtract(face[2], face[0]));
            let reach = length3(normal);
            if reach <= f32::EPSILON {
                continue;
            }
            // How far the fourth point stands off the plane of the first three, in metres.
            worst_planarity =
                worst_planarity.max((dot3(subtract(face[3], face[0]), normal) / reach).abs());
            let eave = [
                0.25 * (face[2][0] + face[3][0]),
                0.25 * (face[2][2] + face[3][2]),
            ];
            let crest = [
                0.5 * (face[0][0] + face[1][0]),
                0.5 * (face[0][2] + face[1][2]),
            ];
            let outward = normal[0] * (eave[0] - crest[0]) + normal[2] * (eave[1] - crest[1]);
            least_up = least_up.min(normal[1] / reach);
            least_out = least_out.min(outward / reach);
        }
    }
    println!(
        "{label}: worst deviation from planar {:.6} m, smallest normal y {:.3}, smallest horizontal outward {:.3}",
        worst_planarity, least_up, least_out
    );
}

/// Returns the four corners of the quad two rows span, in the winding this crate documents.
fn quad(roof: &Roof, first: &RoofRow, second: &RoofRow) -> Option<[Vec3; 4]> {
    Some([
        roof.crest_point(first.crest)?,
        roof.crest_point(second.crest)?,
        roof.eave_point(second.eave)?,
        roof.eave_point(first.eave)?,
    ])
}

/// Returns the two crest points' elevations as one string.
fn elevations(roof: &Roof) -> String {
    roof.crest
        .iter()
        .map(|point| format!("{:.3}", point.elevation))
        .collect::<Vec<_>>()
        .join(" and ")
}

/// Returns the plan distance between the two crest points.
fn crest_spacing(roof: &Roof) -> f32 {
    let (Some(first), Some(second)) = (roof.crest.first(), roof.crest.last()) else {
        return f32::NAN;
    };
    whiting_building::math::distance(first.position, second.position)
}

/// Returns the eave loop's extent across its long axis, which is the bounding box's short side.
fn short_span(roof: &Roof) -> f32 {
    let extent = extent(roof);
    extent[0].min(extent[1])
}

/// Returns the eave loop's plan bounding box as `[width, depth]`.
fn extent(roof: &Roof) -> [f32; 2] {
    let mut low = [f32::INFINITY; 2];
    let mut high = [f32::NEG_INFINITY; 2];
    for node in &roof.eaves {
        for axis in 0..2 {
            low[axis] = low[axis].min(node.position[axis]);
            high[axis] = high[axis].max(node.position[axis]);
        }
    }
    [high[0] - low[0], high[1] - low[1]]
}

/// Returns how far the eave loop stands outside the outline, measured on each edge's mid-point.
///
/// The distance is to the **nearest point of the eave loop's segments**, not to the edge with the
/// same index: the loop is read back the way a reader would read it, so an ordering fault in the
/// derivation could not hide behind an assumed correspondence.
fn overhang_measured(mass: &Mass, roof: &Roof, wanted: f32) -> String {
    let mut values = Vec::new();
    for index in 0..mass.outline.len() {
        let next = (index + 1) % mass.outline.len();
        let middle = [
            0.5 * (mass.outline[index].position[0] + mass.outline[next].position[0]),
            0.5 * (mass.outline[index].position[1] + mass.outline[next].position[1]),
        ];
        let mut closest = f32::INFINITY;
        for eave in 0..roof.eaves.len() {
            let following = (eave + 1) % roof.eaves.len();
            closest = closest.min(segment_distance(
                middle,
                roof.eaves[eave].position,
                roof.eaves[following].position,
            ));
        }
        values.push(format!("{closest:.3}"));
    }
    format!(
        "{} at the wall mid-points, asked {wanted}",
        values.join(", ")
    )
}

/// Returns the distance from a plan point to a plan segment.
fn segment_distance(point: [f32; 2], from: [f32; 2], to: [f32; 2]) -> f32 {
    let along = whiting_building::math::difference(to, from);
    let reach = whiting_building::math::length(along);
    let amount = if reach <= f32::EPSILON {
        0.0
    } else {
        (whiting_building::math::dot(whiting_building::math::difference(point, from), along)
            / (reach * reach))
            .clamp(0.0, 1.0)
    };
    whiting_building::math::distance(
        point,
        [from[0] + along[0] * amount, from[1] + along[1] * amount],
    )
}

/// Reports what the roofs and the ops refuse, and what they do instead.
fn refusals(mass: &Mass, starter: &Starter) {
    let triangle = Mass::new(
        vec![
            PlanNode::new([-3.0, -4.0], 0.0),
            PlanNode::new([3.0, -4.0], 0.0),
            PlanNode::new([0.0, 5.0], 0.0),
        ],
        starter.height,
    );
    println!(
        "a three-node outline: hip {}, gable {}",
        if derive::hip_roof(
            MassId(0),
            &triangle,
            starter.overhang,
            starter.rise,
            starter.sink,
            0.25
        )
        .is_some()
        {
            "Some"
        } else {
            "None"
        },
        if derive::gable_roof(
            MassId(0),
            &triangle,
            starter.overhang,
            starter.rise,
            starter.sink,
            0.25
        )
        .is_some()
        {
            "Some"
        } else {
            "None"
        }
    );

    // Four nodes, but the opposite sides are not parallel: the planes of a hip over it would not
    // be planar, so the derivation reports rather than guesses.
    let skewed = Mass::new(
        vec![
            PlanNode::new([-3.0, -4.0], 0.0),
            PlanNode::new([3.0, -3.0], 0.0),
            PlanNode::new([4.0, 5.0], 0.0),
            PlanNode::new([-1.0, 4.0], 0.0),
        ],
        starter.height,
    );
    println!(
        "a four-node loop whose long sides are not parallel: hip {} (the domain is narrower than four nodes)",
        if derive::hip_roof(
            MassId(0),
            &skewed,
            starter.overhang,
            starter.rise,
            starter.sink,
            0.25
        )
        .is_some()
        {
            "Some"
        } else {
            "None"
        }
    );

    let body = mass.clone();
    let roof = derive::hip_roof(
        MassId(0),
        &body,
        starter.overhang,
        starter.rise,
        starter.sink,
        0.25,
    )
    .expect("a rectangle is a quadrilateral");
    let mut skeleton = whiting_building::BuildingSkeleton::new(vec![body], vec![roof.clone()]);
    println!(
        "remove_mass of a mass a roof is hosted on: {}",
        if ops::remove_mass(&mut skeleton, MassId(0)).is_some() {
            "Some"
        } else {
            "None"
        }
    );
    let wing = Mass::new(
        vec![
            PlanNode::new([0.0, -0.5], 0.0),
            PlanNode::new([2.0, -0.5], 0.0),
            PlanNode::new([2.0, 0.5], 0.0),
            PlanNode::new([0.0, 0.5], 0.0),
        ],
        starter.height,
    );
    ops::attach_mass(&mut skeleton, MassId(0), 0.5, wing).expect("the body is there");
    println!(
        "remove_mass of a mass a wing attaches to: {}",
        if ops::remove_mass(&mut skeleton, MassId(0)).is_some() {
            "Some"
        } else {
            "None"
        }
    );
    ops::remove_roof(&mut skeleton, whiting_building::RoofId(0));
    ops::remove_mass(&mut skeleton, MassId(1)).expect("the wing names nothing");
    println!(
        "with the roof and the wing gone, remove_mass of the body: {}",
        match ops::remove_mass(&mut skeleton, MassId(0)) {
            Some(_) => "Some",
            None => "None",
        }
    );
    println!(
        "remove_mass of an id off the end: {}",
        if ops::remove_mass(&mut skeleton, MassId(9)).is_some() {
            "Some"
        } else {
            "None"
        }
    );
}

/// Reports what the split and the attach do to the document.
fn verbs() {
    let starter = Starter::default();
    let building = derive::starter(&starter);
    let mut host = building.skeleton.masses[0].clone();
    let before_nodes = host.outline.len();
    let before_length = perimeter(&host);
    let at = ops::split_outline(&mut host, 0.3).expect("four nodes is a loop with an edge");
    let added = &host.outline[at];
    println!(
        "split_outline at 0.3: index {at}, nodes {before_nodes} -> {}, at ({:.3}, {:.3}) at elevation {:.3}",
        host.outline.len(),
        added.position[0],
        added.position[1],
        added.elevation,
    );
    println!(
        "  the perimeter is still {:.3} against {:.3}, so the split added a node and moved nothing",
        perimeter(&host),
        before_length
    );
    println!(
        "split_outline at 0.5, which lands on the node the outline already had: index {}, nodes {}",
        ops::split_outline(&mut host, 0.5).expect("a loop with an edge"),
        host.outline.len()
    );
    println!(
        "split_outline on a one-node outline: {}",
        if ops::split_outline(
            &mut Mass::new(vec![PlanNode::new([0.0, 0.0], 0.0)], 3.0),
            0.5
        )
        .is_some()
        {
            "Some"
        } else {
            "None"
        }
    );

    let body = building.skeleton.masses[0].clone();
    let mut skeleton = building.skeleton.clone();
    let nodes_before = skeleton.masses[0].outline.len();
    let wing = Mass::new(
        vec![
            PlanNode::new([0.0, -0.5], 0.0),
            PlanNode::new([2.0, -0.5], 0.0),
            PlanNode::new([2.0, 0.5], 0.0),
            PlanNode::new([0.0, 0.5], 0.0),
        ],
        starter.height,
    );
    let id = ops::attach_mass(&mut skeleton, MassId(0), 0.25, wing).expect("the body is there");
    // What the attach split, done by hand on a copy of the same outline: the joint the attachment
    // names has to be a node the host now carries, which is the whole of the op's argument.
    let mut alone = body.clone();
    let at = ops::split_outline(&mut alone, 0.25).expect("four nodes");
    let joint = alone.outline[at];
    let carried = skeleton.masses[0]
        .outline
        .iter()
        .any(|node| node.position == joint.position && node.elevation == joint.elevation);
    println!(
        "attach_mass at 0.25: mass {}, host nodes {nodes_before} -> {}, attachment {:?}",
        id.0,
        skeleton.masses[0].outline.len(),
        skeleton.masses[id.0]
            .attachment
            .map(|attachment| match attachment.join {
                Join::Wing { station } => station,
                Join::Storey => 0.0,
            })
    );
    println!(
        "  the joint the attachment names is at ({:.3}, {:.3}), and the host carries that node: {carried}",
        joint.position[0], joint.position[1]
    );
    println!(
        "attach_mass to an id off the end: {}",
        if ops::attach_mass(&mut skeleton, MassId(7), 0.25, Mass::new(Vec::new(), 1.0)).is_some() {
            "Some"
        } else {
            "None"
        }
    );
    println!(
        "remove_roof of an id off the end: {}",
        if ops::remove_roof(&mut skeleton, whiting_building::RoofId(9)).is_some() {
            "Some"
        } else {
            "None"
        }
    );
}

/// Returns the closed loop's total plan length.
fn perimeter(mass: &Mass) -> f32 {
    let nodes = &mass.outline;
    (0..nodes.len())
        .map(|index| {
            whiting_building::math::distance(
                nodes[index].position,
                nodes[(index + 1) % nodes.len()].position,
            )
        })
        .sum()
}

/// Reports what the seed decides, and what it is guaranteed to decide.
fn seed() {
    let starter = Starter::default();
    let first = derive::starter(&starter);
    let second = derive::starter(&starter);
    println!(
        "the same seed twice: buildings identical {}, masses {} and {}, crest {:?}",
        first == second,
        first.skeleton.masses.len(),
        second.skeleton.masses.len(),
        first.skeleton.roofs[0]
            .crest
            .iter()
            .map(|point| [point.position[0], point.position[1]])
            .collect::<Vec<_>>(),
    );

    for other in [2u64, 3, 4, 5, 6, 7, 8] {
        let alternative = derive::starter(&Starter {
            seed: other,
            ..starter.clone()
        });
        let spacing = crest_spacing(&alternative.skeleton.roofs[0]);
        let span = short_span(&alternative.skeleton.roofs[0]);
        let extent = extent(&alternative.skeleton.roofs[0]);
        let long = extent[0].max(extent[1]);
        // The ridge is inset from each end, so the spacing says which of the three it was.
        let ratio = (long - spacing) / (2.0 * span);
        let chosen = [0.4f32, 0.5, 0.6]
            .iter()
            .find(|candidate| (**candidate - ratio).abs() < 1.0e-3);
        println!(
            "seed {other}: crest spacing {spacing:.3}, inset {ratio:.4} of the short span, so {}",
            match chosen {
                Some(value) => format!("the published {value}"),
                None => "none of the published three".to_string(),
            }
        );
    }

    // The fork rule itself, which is what keeps one wing's edit from re-rolling another's choice.
    let root = derive::ROOT_KEY;
    println!(
        "fork from the root at 0.25 is {} and at 0.5 is {}: {}",
        derive::fork(root, 0.25),
        derive::fork(root, 0.5),
        if derive::fork(root, 0.25) != derive::fork(root, 0.5) {
            "different keys, so different streams"
        } else {
            "the same key, which would be a bug"
        }
    );
    println!(
        "a wing of a wing at 0.5 on 0.25 is {} against {} for a root wing at 0.75: {}",
        derive::fork(derive::fork(root, 0.25), 0.5),
        derive::fork(root, 0.75),
        if derive::fork(derive::fork(root, 0.25), 0.5) != derive::fork(root, 0.75) {
            "the ancestry is part of the key, not the depth"
        } else {
            "the same key, which would be a bug"
        }
    );
}
