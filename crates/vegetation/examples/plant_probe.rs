//! Measures the plant `vegetation` P1 promises, and exits non-zero if a reading leaves its threshold.
//!
//! The plant is the crate's own species — `derive::bamboo` — and not a tree this file writes, because a
//! default is ordinary data the moment it exists: the rows below measure the numbers a caller would start
//! from, and the same rows would measure any other species. The design's P1 exit
//! (`.agents/proposals/agentic-vegetation-generation.md` V1, V2 and V6) asks for closed wood, joints on the
//! parent's skin, a taper that is the *law* rather than a cone, and leaves that are instances of one template.
//!
//! ```text
//! cargo run -p whiting-vegetation --example plant_probe
//! ```

use shape::math::{length3, span, subtract};
use shape::mesh::{boundary_loops, euler_characteristic, is_manifold, signed_volume};
use whiting_vegetation::derive::{Species, bamboo};
use whiting_vegetation::expand::{Detail, expand, wood};

fn main() {
    let mut checks: Vec<(&str, bool)> = Vec::new();
    checks.push(("a census", the_census()));
    checks.push(("every solid is closed", every_loft_is_a_closed_solid()));
    checks.push(("the taper is the law", the_taper_is_the_law_not_a_cone()));
    checks.push(("a joint is on the skin", a_joint_sits_on_its_parents_skin()));
    checks.push(("one resolution dial", the_dial_is_the_only_resolution()));
    checks.push(("leaves are groups", the_leaves_are_groups_of_placements()));
    checks.push(("a leaf is one transform", a_leaf_costs_one_transform()));
    checks.push((
        "a leaf's scale is local",
        a_leafs_scale_is_in_its_own_frame(),
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

/// The species every row measures: the crate's own bamboo, at one seed.
fn species() -> Species {
    bamboo(20_261_017)
}

/// The whole plant's numbers, and the one reading that says the parts add up.
fn the_census() -> bool {
    let species = species();
    let plant = &species.plant;
    let detail = Detail::DEFAULT;
    let solids = wood(plant, detail);
    let triangles: usize = solids
        .iter()
        .map(|solid| solid.loft.triangles().len())
        .sum();
    let vertices: usize = solids
        .iter()
        .map(|solid| solid.loft.positions().len())
        .sum();
    let nodes: usize = plant.walk().count();
    println!("\n# the plant");
    println!(
        "  {nodes} nodes · {} solids · {vertices} vertices · {triangles} triangles at {detail:?}",
        solids.len()
    );
    println!(
        "  {:<28} {:>6} {:>6} {:>8}",
        "handle", "rings", "around", "triangles"
    );
    for solid in &solids {
        println!(
            "  {:<28} {:>6} {:>6} {:>8}",
            solid.handle.to_string(),
            solid.loft.rings(),
            solid.loft.around(),
            solid.loft.triangles().len()
        );
    }
    println!("  one solid per node, in walk order, and the triangles are the walk's whole output");
    nodes == 10 && solids.len() == nodes && triangles > 0
}

/// Every node is a closed solid, whatever its taper does at its ends.
fn every_loft_is_a_closed_solid() -> bool {
    let species = species();
    let solids = wood(&species.plant, Detail::DEFAULT);
    let mut worst_volume = f32::MAX;
    let mut closed = 0;
    for solid in &solids {
        let surface = solid.loft.view();
        if boundary_loops(surface) == 0
            && is_manifold(surface)
            && euler_characteristic(surface) == 2
            && signed_volume(surface) > 0.0
        {
            closed += 1;
        }
        worst_volume = worst_volume.min(signed_volume(surface));
    }
    println!("\n# the wood");
    println!(
        "  {closed} of {} solids are closed at χ = 2 with a positive volume; the smallest volume is {worst_volume:.3e}",
        solids.len()
    );
    println!(
        "  the branches end at a radius of 8e-4 rather than a point, and a leaf's blade ends at a point; both close"
    );
    closed == solids.len() && worst_volume > 0.0
}

/// The culm's radius is the taper law at every ring, including the rise the law puts at a node.
fn the_taper_is_the_law_not_a_cone() -> bool {
    let species = species();
    let plant = &species.plant;
    let solids = wood(
        plant,
        Detail {
            stations: 33,
            around: 24,
        },
    );
    let culm = &solids[0];
    let taper = plant.root().taper();
    let mut worst = 0.0f32;
    let mut radii = Vec::new();
    for ring in 0..culm.loft.rings() {
        let (_, uv) = culm.loft.vertex(ring, 0);
        let mut centroid = [0.0f32; 3];
        for slot in 0..culm.loft.around() {
            let (point, _) = culm.loft.vertex(ring, slot);
            for axis in 0..3 {
                centroid[axis] += point[axis] / culm.loft.around() as f32;
            }
        }
        let mut radius = 0.0f32;
        for slot in 0..culm.loft.around() {
            let (point, _) = culm.loft.vertex(ring, slot);
            radius = radius.max(length3(subtract(point, centroid)));
        }
        worst = worst.max((radius - taper.at(uv[0])).abs());
        radii.push(radius);
    }
    let at = |t: f32| radii[(t * (radii.len() - 1) as f32).round() as usize];
    let rise = at(0.32) - at(0.28);
    let cone = 0.030 + (0.010 - 0.030) * 0.32;
    println!("\n# the culm's taper");
    println!(
        "  worst radius against the law {worst:.3e} over {} rings",
        radii.len()
    );
    println!(
        "  the law rises 0.006 from 0.28 to 0.32 and the solid rises {rise:.6}: the node is in the geometry, not in a note"
    );
    println!(
        "  a straight cone between the law's ends would read {cone:.6} at 0.32, where the law reads {:.6}",
        taper.at(0.32)
    );
    worst < 1.0e-6 && rise > 0.004 && (taper.at(0.32) - cone).abs() > 1.0e-3
}

/// Every child's first point is on its parent's skin, and sunk by the amount it asked for.
fn a_joint_sits_on_its_parents_skin() -> bool {
    let species = species();
    let plant = &species.plant;
    let mut worst = 0.0f32;
    let mut joints = 0;
    for (handle, node) in plant.walk() {
        if handle.is_root() {
            continue;
        }
        let parent_handle = handle.parent().expect("a node below the root has a parent");
        let parent = plant.get(&parent_handle).expect("the parent resolves");
        let parent_spine = plant
            .spine(&parent_handle)
            .expect("the parent's spine resolves");
        let attachment = node.attachment();
        let station = attachment.station.clamp(0.0, 1.0);
        let skin = parent.taper().at(station) - attachment.sink;
        let placement = plant.placement(&handle).expect("the child resolves");
        let radial = span(placement.origin, parent_spine.at(station));
        worst = worst.max((radial - skin).abs());
        joints += 1;
    }
    println!("\n# the joints");
    println!(
        "  {joints} joints · worst distance from the parent's spine against its taper less the sink: {worst:.3e}"
    );
    println!(
        "  so a bough starts on the bark and a branch starts on the bough's, without a pass that places either"
    );
    joints == 9 && worst < 1.0e-5
}

/// The dial is the only resolution: doubling it quadruples the triangles and moves nothing else.
fn the_dial_is_the_only_resolution() -> bool {
    let species = species();
    let plant = &species.plant;
    let coarse = wood(plant, Detail::DEFAULT);
    let fine = wood(plant, Detail::DEFAULT.scaled(2));
    let coarse_triangles: usize = coarse
        .iter()
        .map(|solid| solid.loft.triangles().len())
        .sum();
    let fine_triangles: usize = fine.iter().map(|solid| solid.loft.triangles().len()).sum();
    let ratio = fine_triangles as f32 / coarse_triangles as f32;
    let closed = fine.iter().all(|solid| {
        boundary_loops(solid.loft.view()) == 0 && euler_characteristic(solid.loft.view()) == 2
    });
    let same_places = fine
        .iter()
        .zip(coarse.iter())
        .all(|(fine, coarse)| fine.handle == coarse.handle);
    println!("\n# the resolution dial");
    println!(
        "  {coarse_triangles} triangles at {:?} · {fine_triangles} at {:?} · ratio {ratio:.3}",
        Detail::DEFAULT,
        Detail::DEFAULT.scaled(2)
    );
    println!(
        "  doubling both dials makes four times the cells, and the caps' share shrinks, so the ratio is near 4 and above it"
    );
    println!(
        "  every solid stays closed: {closed}; the same nodes in the same order: {same_places}, so a coarser plant is the same plant"
    );
    closed && same_places && (3.5..4.5).contains(&ratio)
}

/// A field is a distribution: one group per node it puts leaves on, and the group's count is its own law.
fn the_leaves_are_groups_of_placements() -> bool {
    let species = species();
    let plant = &species.plant;
    let expansion = expand(plant, Detail::DEFAULT, &species.fields);
    let leaves: usize = expansion
        .leaves
        .iter()
        .map(|group| group.leaves.len())
        .sum();
    let branches: usize = plant
        .walk()
        .filter(|(handle, _)| handle.depth() == 2)
        .count();
    let one_template = expansion.leaves.iter().all(|group| group.template == 0);
    let ordered = expansion
        .leaves
        .windows(2)
        .all(|pair| pair[0].node < pair[1].node);
    let counts_match_the_law = expansion.leaves.iter().all(|group| {
        let node = plant.get(&group.node).expect("the group's node resolves");
        let field = &species.fields[0];
        let band = (field.band.1 - field.band.0).abs();
        let expected = (field.density.at(0.5) * band * node.spine().length()).round() as usize;
        group.leaves.len() == expected
    });
    println!("\n# the leaves");
    println!(
        "  {} groups over {branches} branches · {leaves} leaves · one template: {one_template}",
        expansion.leaves.len()
    );
    for group in &expansion.leaves {
        println!(
            "  {:<28} {:>2} leaves · template {}",
            group.node.to_string(),
            group.leaves.len(),
            group.template
        );
    }
    println!("  a group is one node's leaves of one template, in walk order: {ordered}");
    println!("  and each group's count is its own density law, rounded: {counts_match_the_law}");
    expansion.leaves.len() == branches
        && leaves > 0
        && one_template
        && ordered
        && counts_match_the_law
}

/// A leaf is a rigid frame: `k` of them cost the template's triangles once and `k` transforms.
fn a_leaf_costs_one_transform() -> bool {
    let species = species();
    let template = &species.templates[0];
    let loft = template.loft();
    let expansion = expand(&species.plant, Detail::DEFAULT, &species.fields);
    let leaves: usize = expansion
        .leaves
        .iter()
        .map(|group| group.leaves.len())
        .sum();
    let surface = loft.view();
    let closed = boundary_loops(surface) == 0
        && is_manifold(surface)
        && euler_characteristic(surface) == 2
        && signed_volume(surface) > 0.0;
    println!("\n# what one leaf costs");
    println!(
        "  {leaves} leaves draw the template's {} triangles once and carry {leaves} transforms",
        loft.triangles().len()
    );
    println!(
        "  the template is a closed blade at χ = {}, volume {:.3e}, {} vertices, {} rings of {}",
        euler_characteristic(surface),
        signed_volume(surface),
        loft.positions().len(),
        loft.rings(),
        loft.around()
    );
    println!(
        "  baked instead, the same leaves would be {} triangles and {} vertices, which is the bill D7 refuses to pay",
        leaves * loft.triangles().len(),
        leaves * loft.positions().len()
    );
    closed && leaves > 0
}

/// A leaf's scale is applied in the template's own frame, so the anchor stays on its branch.
///
/// # Invariants
///
/// The base ring is the template's own origin — the blade's width law is zero there — so every leaf's base
/// lands exactly on its anchor at any scale. Scaling the *placed* point instead would move it by
/// `(1 − scale) · |origin|`, which grows with how far up the plant the leaf hangs; that is the control, and
/// it is the defect a picture cannot see because the leaf stays the right size while sitting in the wrong
/// place.
fn a_leafs_scale_is_in_its_own_frame() -> bool {
    let species = species();
    let template = &species.templates[0];
    let loft = template.loft();
    let base: Vec<[f32; 3]> = (0..loft.around())
        .map(|slot| loft.vertex(0, slot).0)
        .collect();
    let expansion = expand(&species.plant, Detail::DEFAULT, &species.fields);
    let mut worst_base = 0.0f32;
    let mut worst_wrong = 0.0f32;
    let mut leaves = 0;
    for group in &expansion.leaves {
        for leaf in &group.leaves {
            let scale = leaf.scale;
            for point in &base {
                let local = [point[0] * scale, point[1] * scale, point[2] * scale];
                worst_base =
                    worst_base.max(span(leaf.placement.place(local), leaf.placement.origin));
                let placed = leaf.placement.place(*point);
                let wrong = [placed[0] * scale, placed[1] * scale, placed[2] * scale];
                worst_wrong = worst_wrong.max(span(wrong, leaf.placement.origin));
            }
            leaves += 1;
        }
    }
    println!("\n# a leaf's scale, and where the anchor goes");
    println!(
        "  {leaves} leaves · the base ring against its anchor {worst_base:.3e} (it is the template's own origin)"
    );
    println!(
        "  the control — scaling the placed point instead — moves the same corner by up to {worst_wrong:.3e} m"
    );
    println!(
        "  so the order is measured rather than assumed, and a blade keeps its branch however small it is drawn"
    );
    worst_base < 1.0e-6 && worst_wrong > 1.0e-3 && leaves > 0
}
