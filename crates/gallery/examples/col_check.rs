//! Does a col stand at the height of the crest, or at the height of the plain?
//!
//! A junction is authored as one crest's **end** and another's start, so the col is the last node of
//! its parent — and the pass that takes the massif's foot down to the plain tapers the last fifth of
//! every line. If it does not know which ends are the massif's edge and which are interior, it levels
//! the cols with the tips, and every saddle in the range stands at the plain's height while the
//! silhouette between them keeps its own.
//!
//! This asks the built mesh, because the authored skeleton cannot answer it: the question is what the
//! one welded vertex at each junction ended up holding.
//!
//! ```text
//! cargo run -p whiting-gallery --example col_check
//! ```

use mountain::line::LineRef;
use whiting_gallery::landscape;

/// The plan point one end of an authored crest stands at.
fn end_at(crest: usize, far: bool) -> [f32; 2] {
    let massif = landscape::massif();
    let nodes = &massif.skeleton.ridges[crest].nodes;
    let node = if far { nodes.last() } else { Some(&nodes[0]) };
    let at = node.expect("every authored crest has both ends");
    at.position
}

fn main() {
    let massif = landscape::massif();
    let mesh = massif.build().expect("the authored massif expands");
    println!("--- what the cols are standing on ---");

    // A crest end is a junction when some other crest's end names a station on it, or when it is
    // authored `On` another line. Both readings are needed: naming is the authored fact, and being
    // named is what makes it shared.
    let count = massif.skeleton.ridges.len();
    let mut reported = 0;
    for crest in 0..count {
        let ridge = &massif.skeleton.ridges[crest];
        let nodes = &ridge.nodes;
        let last = nodes.len() - 1;
        let far = &nodes[last];
        let named = ridge.end.station().is_some();
        let names_other = (0..count).any(|other| {
            other != crest
                && massif.skeleton.ridges[other]
                    .start
                    .station()
                    .is_some_and(|station| {
                        station.line == LineRef::Ridge(mountain::RidgeId(crest)) && station.t > 0.5
                    })
        });
        if !(named || names_other) {
            continue;
        }
        // The mesh vertex the junction's own point resolves to.
        let index = mesh.points.iter().position(|point| {
            (point[0] - far.position[0]).abs() < 0.6 && (point[2] - far.position[1]).abs() < 0.6
        });
        let built = index.map(|at| mesh.points[at][1]);
        println!(
            "  crest {crest:>2} ends at [{:.0}, {:.0}] authored {:.0} -> mesh {}  ({})",
            far.position[0],
            far.position[1],
            far.elevation,
            built.map_or("not found".to_owned(), |e| format!("{e:.0}")),
            if named {
                "names another line"
            } else {
                "named by a child"
            },
        );
        reported += 1;
    }
    println!("{reported} junction ends of {count} crests");

    // And the same question of the summit, which is a start rather than an end.
    let first = &massif.skeleton.ridges[0].nodes[0];
    println!(
        "  crest 0 starts at [{:.0}, {:.0}] authored {:.0}",
        first.position[0], first.position[1], first.elevation,
    );
    let _ = end_at;
}
