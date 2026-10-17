//! Measures the tree's invariants, each with a control that can fail, and exits non-zero if one does.
//!
//! Nothing here is a test file: this is the probe that says whether `vegetation`'s tree does what its own
//! documentation claims. The rows are `.agents/proposals/agentic-vegetation-generation.md` D11 (the rng is
//! forked per node and local to its path) and R4.1 (the lines carry their correspondence: a child is
//! *placed* on its parent's surface, never searched for), plus the handle's two halves — that a path
//! reaches the node it names, and that a stale path says where it stopped.
//!
//! ```text
//! cargo run -p whiting-vegetation --example tree_probe
//! ```

use shape::math::{dot3, length3, subtract};
use shape::{Curve, Jet, Scalar};
use whiting_vegetation::handle::Handle;
use whiting_vegetation::node::{Attachment, Node};
use whiting_vegetation::plant::Plant;

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push((
        "a path reaches its node",
        a_path_reaches_the_node_it_names(),
    ));
    checks.push((
        "a stale path is named",
        a_stale_path_says_where_it_stopped(),
    ));
    checks.push((
        "rng is forked by path",
        the_rng_is_a_function_of_the_path_alone(),
    ));
    checks.push((
        "a child hangs on the skin",
        a_child_sits_on_its_parents_skin(),
    ));
    checks.push((
        "a spine is placed rigidly",
        a_placed_spine_keeps_its_shape(),
    ));

    println!("\n# the exit");
    for (name, passed) in &checks {
        println!("  {name:<26} {}", if *passed { "pass" } else { "FAIL" });
    }
    let failed = checks.iter().filter(|(_, passed)| !passed).count();
    println!("  {failed} of {} invariant(s) failed", checks.len());
    if failed > 0 {
        std::process::exit(1);
    }
}

/// A straight spine of `length` standing on its own origin.
fn straight(length: f32) -> Curve {
    Curve::new(vec![
        Jet::new([0.0, 0.0, 0.0], [0.0, length, 0.0]),
        Jet::new([0.0, length, 0.0], [0.0, length, 0.0]),
    ])
    .expect("a straight spine's knots are distinct")
}

/// A taper from `base` to half of it.
fn taper(base: f32) -> Scalar {
    Scalar::new(&[(0.0, base), (1.0, base * 0.5)]).expect("two knots are a law")
}

/// A plant with `boughs` boughs on its trunk, each carrying two twigs: 1 + 3 · `boughs` nodes.
fn plant_with(boughs: usize) -> Plant {
    let mut root = Node::root(straight(1.0), taper(0.2));
    for bough in 0..boughs {
        let station = 0.3 + 0.2 * bough as f32;
        let mut node = Node::attached(
            straight(0.6),
            taper(0.08),
            Attachment::new(station, 0.25 * bough as f32),
        );
        for twig in 0..2 {
            node.push(Node::attached(
                straight(0.3),
                taper(0.02),
                Attachment::new(0.5 + 0.2 * twig as f32, 0.1),
            ));
        }
        root.push(node);
    }
    Plant::new(root, 7)
}

/// A handle reaches the node the author would point at, and the walk names every node once.
fn a_path_reaches_the_node_it_names() -> bool {
    let plant = plant_with(2);
    let expected = [
        ("trunk", 1.0f32),
        ("trunk/bough 0", 0.6),
        ("trunk/bough 0/branch 0", 0.3),
        ("trunk/bough 0/branch 1", 0.3),
        ("trunk/bough 1", 0.6),
        ("trunk/bough 1/branch 0", 0.3),
        ("trunk/bough 1/branch 1", 0.3),
    ];
    let mut worst = 0.0f32;
    let mut found = 0;
    let mut handles = Vec::new();
    for (handle, node) in plant.walk() {
        let text = handle.to_string();
        if let Some((_, length)) = expected.iter().find(|(name, _)| *name == text) {
            found += 1;
            worst = worst.max((node.spine().length() - length).abs());
        }
        handles.push(text);
    }
    println!("\n# the walk names every node, and each name reaches its node");
    println!("  {} handles: {}", handles.len(), handles.join(", "));
    println!("  7 expected, {found} matched by name, worst spine length error {worst:.3e}");
    println!("  the root's name is the first generation's, and each step adds its own");
    handles.len() == expected.len() && found == expected.len() && worst < 1.0e-6
}

/// A handle whose child does not exist is reported with the last handle that did and the index that did not.
fn a_stale_path_says_where_it_stopped() -> bool {
    let plant = plant_with(2);
    let exists = plant.get(&Handle::root().child(1)).is_ok();
    let stale = plant.get(&Handle::root().child(1).child(2));
    let deep = plant.get(&Handle::root().child(0).child(0).child(0));
    let named = matches!(&stale, Err(orphan) if orphan.from == Handle::root().child(1) && orphan.index == 2);
    println!("\n# a path that does not resolve");
    println!("  trunk/bough 1 exists: {exists}");
    match (&stale, &deep) {
        (Err(orphan), Err(deeper)) => {
            println!("  trunk/bough 1/branch 2 → {orphan}");
            println!("  trunk/bough 0/branch 0/twig 0 → {deeper}");
        }
        _ => println!("  a stale path resolved, which is the defect this row exists for"),
    }
    println!(
        "  the report names the last handle that resolved, so a reader can see where the tree ends"
    );
    exists && named && deep.is_err()
}

/// A node's numbers depend on the plant's seed and the node's path, and on nothing else.
fn the_rng_is_a_function_of_the_path_alone() -> bool {
    let two = plant_with(2);
    let three = plant_with(3);
    let handle = Handle::root().child(0).child(1);
    let mut first = two.rng(&handle);
    let mut second = three.rng(&handle);
    let left: Vec<f32> = (0..8).map(|_| first.unit()).collect();
    let right: Vec<f32> = (0..8).map(|_| second.unit()).collect();
    let same = left == right;
    let elsewhere = {
        let mut rng = two.rng(&Handle::root().child(1).child(1));
        rng.unit()
    };
    let differs = elsewhere != left[0];
    let in_range = left.iter().all(|value| (0.0..1.0).contains(value));
    let mut totals = 0.0f64;
    let mut rng = two.rng(&Handle::root());
    let draws = 10_000;
    for _ in 0..draws {
        totals += f64::from(rng.unit());
    }
    let mean = totals / f64::from(draws);
    println!("\n# a node's numbers are its seed and its path");
    println!(
        "  8 draws from trunk/bough 0/branch 1, in a 2-bough and a 3-bough plant: same {same}"
    );
    println!(
        "  the first draw from a different path is {elsewhere:.6} against {:.6}: differ {differs}",
        left[0]
    );
    println!(
        "  all draws in [0, 1): {in_range} · mean of {draws} draws {mean:.6} (a uniform mean is 0.5)"
    );
    println!(
        "  inserting a bough elsewhere leaves this node's stream untouched, which is what makes an edit local"
    );
    same && differs && in_range && (mean - 0.5).abs() < 0.01
}

/// A child's origin is on its parent's surface, `sink` inside it, and its frame's `x` points outward.
fn a_child_sits_on_its_parents_skin() -> bool {
    let plant = plant_with(2);
    let mut worst_radial = 0.0f32;
    let mut worst_origin = 0.0f32;
    let mut worst_square = 0.0f32;
    let mut worst_tangent = 0.0f32;
    for (handle, node) in plant.walk() {
        if handle.is_root() {
            continue;
        }
        let parent_handle = handle.parent().expect("a node below the root has a parent");
        let parent = plant.get(&parent_handle).expect("the parent resolves");
        let attachment = node.attachment();
        let surface = plant
            .surface(&parent_handle, attachment)
            .expect("the parent's surface resolves");
        let parent_placement = plant
            .placement(&parent_handle)
            .expect("the parent's placement resolves");
        let station = attachment.station.clamp(0.0, 1.0);
        let spine_point = parent.spine().at(station);
        let radial = length3(subtract(surface.origin, spine_point));
        worst_radial = worst_radial.max((radial - parent.taper().at(station)).abs());
        let child = plant
            .placement(&handle)
            .expect("the child's placement resolves");
        // The surface frame is in the parent's own coordinates, so it reaches the world only through the
        // parent's placement — comparing it against the child's world origin directly is a mistake the
        // first run of this row made, and it read 5.2e-1.
        let expected = parent_placement.place(surface.origin);
        worst_origin = worst_origin.max(length3(subtract(child.origin, expected)));
        worst_square = worst_square.max((length3(surface.x) - 1.0).abs());
        worst_tangent =
            worst_tangent.max(dot3(surface.x, parent.spine().frame(station).tangent).abs());
    }
    println!("\n# where a child's origin is");
    println!(
        "  radius against the parent's taper law: worst {worst_radial:.3e} (a circle section's skin)"
    );
    println!(
        "  the child's world origin against the parent's surface frame: worst {worst_origin:.3e}"
    );
    println!(
        "  the frame's x is a unit vector to {worst_square:.3e} and square to the parent's tangent to {worst_tangent:.3e}"
    );
    println!("  a joint is three numbers and the parent's frame, so nothing here is searched for");
    worst_radial < 1.0e-6
        && worst_origin < 1.0e-6
        && worst_square < 1.0e-6
        && worst_tangent < 1.0e-6
}

/// A placed spine is the authored one, rigidly moved: its length, its start, and its first frame survive.
fn a_placed_spine_keeps_its_shape() -> bool {
    let plant = plant_with(2);
    let mut worst_length = 0.0f32;
    let mut worst_start = 0.0f32;
    let mut worst_normal = 0.0f32;
    for (handle, node) in plant.walk() {
        let placed = plant.spine(&handle).expect("every node's spine resolves");
        let placement = plant
            .placement(&handle)
            .expect("every node's placement resolves");
        worst_length = worst_length.max((placed.length() - node.spine().length()).abs());
        worst_start = worst_start.max(length3(subtract(placed.at(0.0), placement.origin)));
        let authored = placement.rotate(node.spine().frame(0.0).normal);
        worst_normal = worst_normal.max(1.0 - dot3(placed.frame(0.0).normal, authored));
    }
    println!("\n# a spine, placed");
    println!(
        "  worst length change {worst_length:.3e} · worst start against the placement's origin {worst_start:.3e}"
    );
    println!(
        "  worst turn of the initial normal {worst_normal:.3e}, which is the section's orientation carried down"
    );
    worst_length < 1.0e-5 && worst_start < 1.0e-6 && worst_normal < 1.0e-6
}
