//! Plants in a settlement's scene: the bridge between a vegetation's own world and a building's.
//!
//! # What the bridge is
//!
//! A building is expanded into a mesh by the `building` crate and its points carry a material id; a plant is expanded by
//! the `vegetation` crate into **wood** (lofts in the plant's own frame) and **leaf groups** (one blade mesh and one
//! placement per leaf). `vegetation_mesh` already knows how to pack either into a `PackedList` a scene's `mesh` takes,
//! and `vegetation_instancing` already knows how to turn a leaf's placement into an instance transform. Nothing here is
//! new geometry: the module is the twenty lines of glue that a *settlement* needs and a picture does not.
//!
//! # Why the plant's meshes use the plain schema
//!
//! A leaf's own mesh carries a per-corner id that a picture reads as **which node this pixel is** — that is how a leaf is
//! shaded from the branch it hangs on. A settlement does not name its plants' nodes, so these meshes are drawn with the
//! **plain** shaders and materials, exactly as the ground is, and the id a blade's corner carries is simply not declared.
//!
//! # Invariants
//!
//! **The wood is baked in world space and the leaves are instanced.** Wood is one solid per culm per placement and there
//! are few of them; a blade is one small quad and there are hundreds per plant, so a blade template is one mesh and every
//! leaf of every clump is an instance of it. Baking the wood costs points; instancing the leaves costs one mesh.

use definition::{PackedList, Value};
use document::kit::{instance, mesh_material, surface};
use vegetation::derive::{Species, clump, tree};
use vegetation::expand::{Detail, expand};
use vegetation::plant::Placement;

use crate::vegetation_instancing::standing_at;
use crate::vegetation_mesh::{node_ids, template_points, wood_points_of};

/// How a settlement's plants enter its scene: two meshes and the placement of every instance of them.
pub(crate) struct Extra {
    /// The plant's meshes, already packed.
    meshes: Vec<(String, PackedList)>,
    /// The plant's materials, one set per label.
    materials: Vec<Value>,
    /// The wood's own placements, as `(mesh, translation, rotation, scale)`.
    wood: Vec<Instance>,
    /// Every leaf, as the same tuple.
    leaves: Vec<Instance>,
}

/// One instanced draw: which mesh, which material, where, and how big.
///
/// # Invariants
///
/// **The material is named in the tuple and not derived from the mesh.** A leaf mesh is `clump-leaf-0` and its material is
/// `clump-leaf`, so a name built from the mesh asks for `clump-leaf-0-leaf-depth`, which no scene declares — measured the
/// first time two species stood in one scene.
type Instance = (String, String, [f64; 3], [f64; 3], [f64; 3]);

impl Extra {
    /// A scene with no plants.
    pub fn none() -> Self {
        Self {
            meshes: Vec::new(),
            materials: Vec::new(),
            wood: Vec::new(),
            leaves: Vec::new(),
        }
    }

    /// Absorbs another extra: what a settlement with two species has after growing both.
    ///
    /// # Invariants
    ///
    /// **The mesh labels are the prefix, so two species never collide**: a clump`s wood is `clump-wood` and a tree`s is
    /// `tree-wood`, and each instance names its own mesh`s material, so a merged extra needs no second bookkeeping.
    pub fn merge(&mut self, other: Extra) {
        self.meshes.extend(other.meshes);
        self.materials.extend(other.materials);
        self.wood.extend(other.wood);
        self.leaves.extend(other.leaves);
    }

    /// Returns the extra's meshes, as `(label, packed list)`.
    pub fn meshes(&self) -> Vec<(String, PackedList)> {
        self.meshes.clone()
    }

    /// Returns the extra's materials.
    pub fn materials(&self) -> Vec<Value> {
        self.materials.clone()
    }

    /// Returns every instance of the extra for one stage.
    pub fn instances(&self, stage: &str) -> Vec<Value> {
        let mut placed = Vec::with_capacity(self.wood.len() + self.leaves.len());
        for (mesh, material, translation, rotation, scale) in &self.wood {
            placed.push(instance(
                format!("{mesh}-{stage}"),
                mesh,
                *translation,
                *rotation,
                *scale,
                &format!("{material}-{stage}"),
            ));
        }
        for (index, (mesh, material, translation, rotation, scale)) in
            self.leaves.iter().enumerate()
        {
            placed.push(instance(
                format!("{mesh}-{stage}-{index}"),
                mesh,
                *translation,
                *rotation,
                *scale,
                &format!("{material}-{stage}"),
            ));
        }
        placed
    }
}

/// Returns a standing frame at `origin`, turned `yaw` about the vertical.
///
/// # Invariants
///
/// The frame is **right handed with `y` up**: `x` is the plant's outward direction, `y` its growth, `z = x × y`. A yaw
/// about `y` is what turns a clump towards the light it grew towards, and it is the only freedom a plant seated on level
/// ground has.
fn standing(origin: [f32; 3], yaw: f32, scale: f32) -> Placement {
    let (sine, cosine) = yaw.sin_cos();
    Placement {
        origin,
        x: [cosine * scale, 0.0, -sine * scale],
        y: [0.0, scale, 0.0],
        z: [sine * scale, 0.0, cosine * scale],
    }
}

/// Returns a settlement's **bamboo clumps**: the scrub a Dai hamlet keeps at its edges.
///
/// # Invariants
///
/// **One species and one expansion carry every clump**: the plant is grown once and the same culms are placed at each seat,
/// which is what makes six clumps six placements of one plant. A settlement's clumps are meant to look like one another,
/// and a scene that grew them separately would pay six times the expansion for a difference nobody asked for.
pub(crate) fn bamboo_clumps(seats: &[[f32; 2]], ground: &dyn Fn([f32; 2]) -> f32) -> Extra {
    // **`clump` and not `bamboo`**: `derive::bamboo` is a *culm* — a few blades on its upper nodes, which is what a grove
    // is made of and not what one plant is. `clump` puts six limbs at every node and dense leaves on them, and a settlement
    // needs the plant a person walks past.
    plants("clump", clump(7), seats, ground, 3.4, 1.8)
}

/// Returns a settlement's **trees**: `derive::tree` at a village's own scale.
///
/// # Why it is not used yet
///
/// The vegetation crate has five species: **bamboo** (a culm), **tuft**, **clump**, **shrub** and **sculpted** (which is a
/// bamboo with a hand's edits). Stretched to a tree's height a **shrub is a twig** — measured at `11.0`, where its limbs
/// are nine metres of bare wood with a shrub's leaves on them — and no dial makes a crown out of a species that has no
/// crown. A settlement's tree needs a species of its own: more generations, a bough that divides at the top, and a leaf
/// field dense enough to be a canopy. This entry point is the seam that species will use.
///
/// # Invariants
///
/// `derive::shrub` is three generations of self-similar limbs, each a fraction of its parent's radius — the same rule a
/// tree obeys, grown to a shrub's height. At `5.5` it is a tree of eight or nine metres with a crown the width of a house,
/// which is the size of the trees a 傣寨 or a 山村 keeps between its plots.
pub(crate) fn trees(seats: &[[f32; 2]], ground: &dyn Fn([f32; 2]) -> f32) -> Extra {
    plants("tree", tree(3), seats, ground, 1.9, 3.2)
}

/// Returns `seats` worth of one species, under a `prefix` that names its meshes and materials.
///
/// # Invariants
///
/// **The prefix is the mesh label**, so two species in one scene never collide and a merged extra needs no bookkeeping: a
/// clump's wood is `clump-wood` and a tree's is `tree-wood`, and each instance names the material of its own mesh.
fn plants(
    prefix: &str,
    species: Species,
    seats: &[[f32; 2]],
    ground: &dyn Fn([f32; 2]) -> f32,
    scale: f32,
    leaf_scale: f32,
) -> Extra {
    let expansion = expand(&species.plant, Detail::DEFAULT, &species.fields);
    let ids = node_ids(&species.plant);
    let mut extra = Extra::none();
    let wood_label = format!("{prefix}-wood");

    // The wood: one mesh for every plant, because every solid's loft is packed by the placement that stands it.
    let placements: Vec<Placement> = seats
        .iter()
        .enumerate()
        .map(|(index, at)| {
            let yaw = index as f32 * 1.7;
            standing([at[0], ground(*at), at[1]], yaw, scale)
        })
        .collect();
    extra.meshes.push((
        wood_label.clone(),
        wood_points_of(
            placements
                .iter()
                .map(|placement| (expansion.wood.as_slice(), &ids, placement)),
        ),
    ));
    // **The wood's instance is the identity, because its mesh is already in world space.** `wood_points_of` bakes each
    // placement into the points it packs, so a second translation by that same placement adds the seat twice: measured on
    // the hillside as a village's clumps drawn at twice their plan coordinate and forty metres above a plateau that stands
    // at twenty, and in the flat hamlet as only the clumps nearest the origin being visible at all.
    let stem_material = format!("{wood_label}-stem");
    let leaf_material = format!("{prefix}-leaf");
    extra.wood.push((
        wood_label.clone(),
        stem_material.clone(),
        [0.0; 3],
        [0.0; 3],
        [1.0; 3],
    ));

    // The blades: one template per its own index, and one instance per leaf.
    let mut labels: Vec<Option<String>> = vec![None; species.templates.len()];
    for group in &expansion.leaves {
        let template = group.template;
        let label = match &labels[template] {
            Some(label) => label.clone(),
            None => {
                let label = format!("{prefix}-leaf-{template}");
                extra
                    .meshes
                    .push((label.clone(), template_points(&species.templates[template])));
                labels[template] = Some(label.clone());
                label
            }
        };
        for leaf in &group.leaves {
            for placement in &placements {
                let world = placement.then(&leaf.placement);
                // **A settlement's blades are drawn larger than a study's**: a leaf seen from the lane is under a pixel at
                // the species' own scale. The placement is the plant's; only what is drawn of a leaf is scaled.
                let (translation, rotation, scale) = standing_at(&world, leaf.scale * leaf_scale);
                extra.leaves.push((
                    label.clone(),
                    leaf_material.clone(),
                    translation,
                    rotation,
                    scale,
                ));
            }
        }
    }

    // **One stem and one leaf material, drawn by the plain shaders**, which take their colour from the material rather
    // than from the vertex: a settlement does not name its plants' nodes, so a blade has no node to be shaded from.
    let stem = || surface([0.42, 0.44, 0.24], 0.0, 0.86, [0.0; 3]);
    let blade = || surface([0.24, 0.38, 0.18], 0.0, 0.82, [0.0; 3]);
    for (label, body) in [
        (stem_material, stem as fn() -> Value),
        (leaf_material, blade),
    ] {
        for (stage, shader) in [
            ("cast", "plain_shadowing"),
            ("depth", "plain_depthing"),
            ("shade", "plain_lighting"),
        ] {
            extra
                .materials
                .push(mesh_material(&format!("{label}-{stage}"), shader, body()));
        }
    }
    extra
}
