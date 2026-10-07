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

use mountain::tessellation::BeltRef;
use mountain::{FacetMesh, Massif, ValleyId, derive};
use whiting_gallery::landscape;

fn main() {
    std::fs::create_dir_all("out").ok();
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
            write_skyline(&mesh, 300.0);
        }
        Err(failure) => println!("build failed: {failure}"),
    }
    write_plan(&massif);
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
