//! Reports what the landscape's massif is, before anything draws it.
//!
//! This is the cheap half of the authoring loop. A massif that will not validate and will not
//! expand costs no gpu work at all, and the three things this reports are the three a render
//! answers badly.
//!
//! - **Every violation**, named. A rib that crosses a line or a stretch of line that nothing
//!   faces builds a mesh that overlaps itself or has a hole in it, and both are perfectly ordinary
//!   pictures of a mountain.
//! - **The plan and the skyline**, drawn. The plan says whether the skeleton is the shape it was
//!   meant to be; the skyline says whether that shape reads as a mountain. A cone passes both — it
//!   is symmetric and it has a silhouette — so neither is a check on beauty.
//! - **The facets' aspect.** A block face has a shape. A belt zipped between two boundaries that
//!   descend at different rates gives triangles many times longer than they are wide, and a
//!   massif of those reads as folded paper however good its silhouette is. This is the number that
//!   says which one was built.
//!
//! ```text
//! cargo run -p whiting-gallery --example alpine_probe
//! ```
//!
//! It writes `out/alpine-plan.svg` and `out/alpine-skyline.svg`.

use std::collections::BTreeMap;

use mountain::tessellation::BeltRef;
use mountain::{FacetMesh, Massif, ValleyId, derive};
use whiting_gallery::landscape;

fn main() {
    std::fs::create_dir_all("out").ok();
    write_basins();
    let massif = landscape::massif();
    println!(
        "{} ridges, {} valleys, {} footprint nodes, step {}",
        massif.skeleton.ridges.len(),
        massif.skeleton.valleys.len(),
        massif.skeleton.footprint.len(),
        massif.tessellation.step,
    );
    println!(
        "ridge lines: {} connected piece(s), so they are {}",
        massif.skeleton.ridge_components(),
        match massif.skeleton.ridge_components() {
            0 | 1 => "a tree or a network",
            _ => "a scatter, which is neither",
        },
    );

    let violations = massif.validate();
    println!("violations: {}", violations.len());
    for violation in violations.iter().take(24) {
        println!("  {violation:?}");
    }
    if violations.len() > 24 {
        println!("  ... and {} more", violations.len() - 24);
    }

    match massif.build() {
        Ok(mesh) => {
            let (low, high) = mesh.bounds();
            println!(
                "mesh: {} faces, {} points, shortest edge {:.3}",
                mesh.faces.len(),
                mesh.points.len(),
                mesh.shortest_edge(),
            );
            println!(
                "plan area {:.1} against footprint {:.1}, difference {:.3}",
                mesh.plan_area(),
                massif.footprint_area(),
                mesh.plan_area() - massif.footprint_area(),
            );
            println!(
                "surface: {} boundary loop(s), euler {}, manifold {}",
                mesh.boundary_loops(),
                mesh.euler_characteristic(),
                mesh.is_manifold(),
            );
            println!(
                "bounds x [{:.1}, {:.1}], y [{:.1}, {:.1}], z [{:.1}, {:.1}]",
                low[0], high[0], low[1], high[1], low[2], high[2],
            );
            for valley in 0..massif.skeleton.valleys.len() {
                let faces = derive::faces(&massif.skeleton, ValleyId(valley));
                println!("  valley {valley} faces {faces:?}");
            }
            report_slivers(&mesh);
            report_creases(&mesh);
            write_skyline(&mesh, 300.0);
        }
        Err(failure) => println!("build failed: {failure}"),
    }
    write_plan(&massif);

    write_basins();
}

/// Prints every basin's walls, its rib count and the ceiling its floor was written under.
///
/// # Invariants
///
/// **A ceiling and not a verdict.** `validate` already refuses a floor that is not below the rock
/// either side of it, but a refusal names one number that is wrong and a hierarchy is authored as a
/// table of dozens of them. The ceiling is what an author needs: the highest the floor may stand at
/// each rib and still be a valley rather than a second ridge, against **both** walls, with the
/// tighter of the two printed because that is the one that binds.
fn write_basins() {
    let massif = landscape::massif();
    println!("\nbasins — the lower wall at each rib, and the floor actually written under it:");
    for (index, valley) in massif.skeleton.valleys.iter().enumerate() {
        let ribs = valley.nodes.len();
        let mut ceiling = Vec::with_capacity(ribs);
        for node in &valley.nodes {
            let wall = [node.left, node.right]
                .iter()
                .filter_map(|station| massif.skeleton.station_point(*station))
                .map(|point| point[1])
                .fold(f32::INFINITY, f32::min);
            ceiling.push(wall);
        }
        let floors: Vec<String> = valley
            .nodes
            .iter()
            .zip(ceiling)
            .map(|(node, wall)| format!("{:.0}/{}", node.elevation, wall))
            .collect();
        println!("  valley {index} ({} ribs): {}", ribs, floors.join("  "));
    }
}

/// Reports where the ranges stand in the frame, seen from the eye.
///
/// Reports where the two ranges stand in the frame, seen from the eye.
///
/// # Invariants
///
/// **Azimuth and not a position**, because the question is whether a range fills the frame or sits in
/// it, and a point's distance from the eye says nothing about that while its angle off the view axis
/// says exactly it.
///
/// **The spread across the frame is the number that matters for a range**, and it is the reason the
/// spine was authored as a line rather than a disc: a blob subtends one narrow slice of azimuth
/// wherever its nearest point is, and a range that runs across the view puts its two ends many
/// degrees apart. A report that gave only the summit would say nothing about whether the frame holds
/// the whole of it.
fn write_framing() {
    let half = landscape::field_of_view() * 0.5 * 180.0 / std::f64::consts::PI;
    println!("framing from the eye, half frame {half:.1} degrees off the axis:");
    for (name, summit, nearest) in landscape::framing() {
        let verdict = if nearest > half {
            "past the edge"
        } else {
            "inside"
        };
        println!(
            "  {name}: summit at {summit:.1} degrees, nearest point at {nearest:.1} — {verdict}"
        );
    }
}

/// Reports the faces that are long and thin, and where they are.
///
/// # Invariants
///
/// A belt is zipped between two boundaries that do not agree on how fast they are being walked,
/// so its cells come out with whatever aspect the two rates make. The number that says whether
/// that reads as a **block face** or as a sliver is the ratio of a face's longest edge to its
/// area's square root, so that is what is reported — and the face's own belt and centre, because
/// "there are slivers" is not a finding and "the slivers are all in the last segment of the
/// valleys" is.
fn report_slivers(mesh: &FacetMesh) {
    let mut worst: Vec<(f64, usize, [f64; 3], BeltRef, String)> = Vec::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        let [a, b, c] = mesh.triangle(face);
        let longest = [(a, b), (b, c), (c, a)]
            .into_iter()
            .map(|(p, q)| {
                ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
            })
            .fold(0.0f32, f32::max) as f64;
        let area = 0.5
            * ((b[0] - a[0]) as f64 * (c[1] - a[1]) as f64
                - (b[2] - a[2]) as f64 * (c[0] - a[0]) as f64)
                .abs();
        if area <= 1.0e-6 {
            continue;
        }
        let ratio = longest / area.sqrt();
        let centre = [
            (a[0] as f64 + b[0] as f64 + c[0] as f64) / 3.0,
            (a[1] as f64 + b[1] as f64 + c[1] as f64) / 3.0,
            (a[2] as f64 + b[2] as f64 + c[2] as f64) / 3.0,
        ];
        worst.push((
            ratio,
            index,
            centre,
            face.attributes.belt,
            format!("{:?}", face.attributes.belt),
        ));
    }
    worst.sort_by(|left, right| right.0.total_cmp(&left.0));
    let ratios: Vec<f64> = worst.iter().map(|entry| entry.0).collect();
    let median = ratios[ratios.len() / 2];
    let over: Vec<&(f64, usize, [f64; 3], BeltRef, String)> =
        worst.iter().filter(|entry| entry.0 > 6.0).collect();
    println!(
        "faces: {} | aspect (longest edge over sqrt area): median {median:.1}, \
         worst {:.1}, {} of {} over 6",
        ratios.len(),
        ratios[0],
        over.len(),
        ratios.len()
    );
    for entry in over.iter().take(8) {
        println!(
            "  sliver aspect {:.0} at [{:.0}, {:.0}, {:.0}] belt {:?}",
            entry.0, entry.2[0], entry.2[1], entry.2[2], entry.4
        );
    }
}

/// Reports where the shading normal jumps across a mesh edge, and whether the scene asked for it.
///
/// # Invariants
///
/// **It reports two groupings side by side**, because the number that matters is not "are there
/// creases" — a crest line is a crease and wants to be — but **how many creases there are beyond
/// the ones the scene named**. A belt is one strip of one flank, so grouping by it creases at
/// every rib as well as at every crest; a flank is the whole face, so grouping by it creases at
/// the crest and the valley floor and nowhere else. Both are counted here over the same edges, so
/// the two numbers are comparable rather than two separate opinions.
///
/// An edge's normal jump is read **at one shared vertex, on both of its faces** — the pair the
/// rasteriser interpolates towards each other across that edge. Reading it at each face's own end
/// of the edge instead measures the surface's curvature between two different vertices and calls
/// every curve a crease, which is a measurement of the mountain rather than of the mesh.
fn report_creases(mesh: &FacetMesh) {
    let flank =
        mountain::CornerNormals::of_grouped(mesh, whiting_gallery::identity::across_the_flank);
    let belt =
        mountain::CornerNormals::of_grouped(mesh, whiting_gallery::identity::across_the_belt);

    // Every edge, with the faces on it. The key is the edge's own two endpoints **in order**, so a
    // face that walks the edge the other way — which every two do, or the mesh would not be
    // manifold — lands on the same key.
    let mut edges: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        for corner in 0..3 {
            let (here, next) = (face.indices[corner], face.indices[(corner + 1) % 3]);
            edges
                .entry((here.min(next), here.max(next)))
                .or_default()
                .push(index);
        }
    }
    let corner_at = |face: usize, vertex: u32| -> usize {
        (0..3)
            .find(|corner| mesh.faces[face].indices[*corner] == vertex)
            .expect("an edge endpoint is one of the face's own corners")
    };

    let angle = |one: [f32; 3], other: [f32; 3]| -> f64 {
        let dot = one[0] * other[0] + one[1] * other[1] + one[2] * other[2];
        f64::from(dot.clamp(-1.0, 1.0).acos()) * 180.0 / std::f64::consts::PI
    };
    let mut rows: Vec<(f64, f64, String)> = Vec::new();
    for ((low, _), faces) in edges.iter().filter(|(_, on)| on.len() == 2) {
        let [one, two] = faces.as_slice() else {
            continue;
        };
        rows.push((
            angle(
                flank.at(*one, corner_at(*one, *low)),
                flank.at(*two, corner_at(*two, *low)),
            ),
            angle(
                belt.at(*one, corner_at(*one, *low)),
                belt.at(*two, corner_at(*two, *low)),
            ),
            format!("{:?}", mesh.faces[*one].attributes.belt),
        ));
    }

    let edges = rows.len();
    for (name, jumps) in [
        (
            "across a flank",
            rows.iter().map(|row| row.0).collect::<Vec<_>>(),
        ),
        (
            "across a belt",
            rows.iter().map(|row| row.1).collect::<Vec<_>>(),
        ),
    ] {
        let creased: Vec<f64> = jumps.iter().copied().filter(|jump| *jump > 0.0).collect();
        let total = creased.iter().sum::<f64>();
        let worst = creased.iter().copied().fold(0.0, f64::max);
        println!(
            "creases {name}: {} of {edges} interior edges, worst {worst:.1} degrees, \
             total {total:.0} degrees",
            creased.len()
        );
    }
    let mut worst: Vec<&(f64, f64, String)> = rows.iter().filter(|row| row.1 > 0.0).collect();
    worst.sort_by(|left, right| right.1.total_cmp(&left.1));
    println!("the largest jumps the belt grouping leaves behind:");
    for row in worst.iter().take(6) {
        println!(
            "  belt jump {:.1} degrees, flank {:.1} degrees, on {}",
            row.1, row.0, row.2
        );
    }
}

/// Writes the plan view: the rim, every crest coloured by its own elevation, every valley floor,
/// and every rib.
///
/// # Invariants
///
/// The ribs are drawn because they are the correspondence made visible. A rib that crosses
/// another rib or a line is a mesh that overlaps itself, and in the plan it is two lines crossing
/// — which is a thing an author can see, where the violation list is a thing they have to read.
fn write_plan(massif: &Massif) {
    let mut svg = String::from(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='-620 -620 1240 1240' width='900' \
         height='900' style='background:#101418'>\n",
    );
    let summit = massif.skeleton.ridges[0].nodes[0].elevation.max(1.0);

    let rim: Vec<String> = massif
        .skeleton
        .footprint
        .iter()
        .map(|node| format!("{:.1},{:.1}", node.position[0], node.position[1]))
        .collect();
    svg.push_str(&format!(
        "<polygon points='{}' fill='#1c2228' stroke='#5a6570' stroke-width='2'/>\n",
        rim.join(" ")
    ));

    for valley in 0..massif.skeleton.valleys.len() {
        let nodes = &massif.skeleton.valleys[valley].nodes;
        let path: Vec<String> = nodes
            .iter()
            .map(|node| format!("{:.1},{:.1}", node.position[0], node.position[1]))
            .collect();
        svg.push_str(&format!(
            "<polyline points='{}' fill='none' stroke='#2f6f5e' stroke-width='3' \
             stroke-dasharray='14 9'/>\n",
            path.join(" ")
        ));
        // The ribs: from each valley node to the two stations it faces.
        for node in nodes {
            for side in [mountain::Side::Left, mountain::Side::Right] {
                let Some(point) = massif.skeleton.station_position(node.station(side)) else {
                    continue;
                };
                svg.push_str(&format!(
                    "<line x1='{:.1}' y1='{:.1}' x2='{:.1}' y2='{:.1}' stroke='#c8863a' \
                     stroke-width='0.8' opacity='0.55'/>\n",
                    node.position[0], node.position[1], point[0], point[1]
                ));
            }
        }
    }

    for ridge in 0..massif.skeleton.ridges.len() {
        let nodes = &massif.skeleton.ridges[ridge].nodes;
        for pair in nodes.windows(2) {
            svg.push_str(&format!(
                "<line x1='{:.1}' y1='{:.1}' x2='{:.1}' y2='{:.1}' stroke='{}' \
                 stroke-width='3.5'/>\n",
                pair[0].position[0],
                pair[0].position[1],
                pair[1].position[0],
                pair[1].position[1],
                shade(pair[0].elevation / summit)
            ));
        }
    }

    svg.push_str(&format!(
        "<circle cx='{:.1}' cy='{:.1}' r='9' fill='#fff'/>\n",
        massif.skeleton.ridges[0].nodes[0].position[0],
        massif.skeleton.ridges[0].nodes[0].position[1]
    ));
    for ridge in 0..massif.skeleton.ridges.len() {
        let node = massif.skeleton.ridges[ridge].nodes.last().unwrap();
        svg.push_str(&format!(
            "<circle cx='{:.1}' cy='{:.1}' r='5' fill='{}'/>\n",
            node.position[0],
            node.position[1],
            shade(node.elevation / summit)
        ));
    }
    svg.push_str("</svg>\n");
    std::fs::write("out/alpine-plan.svg", svg).expect("the plan is written");
    println!("wrote out/alpine-plan.svg");
}

/// Writes the skyline, seen from a bearing.
///
/// # Invariants
///
/// It is the **upper envelope** of the whole mesh along the view's horizontal axis, not a
/// projection of any one crest. A cone has a perfectly good envelope and is still a cone; what
/// the drawing shows is how many times the envelope rises and falls, which is the number that
/// separates a mountain's silhouette from a tent pole's.
fn write_skyline(mesh: &FacetMesh, bearing_degrees: f64) {
    let bearing = bearing_degrees.to_radians();
    let right = [-bearing.sin(), bearing.cos()];
    let (low, high) = mesh.bounds();
    let bins = 420usize;
    let span = (high[0] - low[0]).abs().max((high[2] - low[2]).abs()) as f64 * 0.62;
    let mut top = vec![f32::MIN; bins];
    for point in &mesh.points {
        let along = point[0] as f64 * right[0] + point[2] as f64 * right[1];
        let bin = (((along + span) / (2.0 * span)) * bins as f64) as isize;
        if (0..bins as isize).contains(&bin) {
            top[bin as usize] = top[bin as usize].max(point[1]);
        }
    }
    let Some(first) = top.iter().position(|value| *value != f32::MIN) else {
        return;
    };
    let mut runs = 0;
    let mut rising = true;
    for pair in top[first..].windows(2) {
        if pair[1] > pair[0] + 1.0 && !rising {
            runs += 1;
            rising = true;
        }
        if pair[1] < pair[0] - 1.0 && rising {
            rising = false;
        }
    }
    println!(
        "skyline from {bearing_degrees} degrees: {} rise(s) across {} bins",
        runs, bins
    );

    let width: f64 = 1100.0;
    let height: f64 = 460.0;
    let peak = high[1].max(1.0) as f64;
    let points: Vec<String> = top
        .iter()
        .enumerate()
        .filter(|(_, value)| **value != f32::MIN)
        .map(|(bin, value)| {
            format!(
                "{:.1},{:.1}",
                bin as f64 * width / bins as f64,
                height - (*value as f64 / peak) * height
            )
        })
        .collect();
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {width:.0} {height:.0}' \
         width='{width:.0}' height='{height:.0}' style='background:#101418'>\n\
         <polyline points='{}' fill='none' stroke='#dfe6ee' stroke-width='2.5'/>\n\
         </svg>\n",
        points.join(" ")
    );
    std::fs::write("out/alpine-skyline.svg", svg).expect("the skyline is written");
    println!("wrote out/alpine-skyline.svg  ({runs} rise(s))");
}

/// Returns a colour for an elevation fraction: blue at the foot, white at the summit.
fn shade(fraction: f32) -> String {
    let low = [0.36, 0.46, 0.62];
    let high = [1.0, 0.98, 0.92];
    let t = fraction.clamp(0.0, 1.0);
    let mixed: Vec<String> = (0..3)
        .map(|channel| {
            let value = low[channel] + (high[channel] - low[channel]) * t;
            format!("{value:.2}")
        })
        .collect();
    format!("rgb({})", mixed.join(","))
}
