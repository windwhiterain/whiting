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
use document::kit::{instance, mesh, mesh_material, surface};
use vegetation::derive::{Species, clump};
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

/// One instanced draw: which mesh, where, and how big.
type Instance = (String, [f64; 3], [f64; 3], [f64; 3]);

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

    /// Whether this extra draws nothing at all, which is what lets a scene stay byte-identical without plants.
    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
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
        for (mesh, translation, rotation, scale) in &self.wood {
            placed.push(instance(
                format!("{mesh}-{stage}"),
                mesh,
                *translation,
                *rotation,
                *scale,
                &format!("stem-{stage}"),
            ));
        }
        for (index, (mesh, translation, rotation, scale)) in self.leaves.iter().enumerate() {
            placed.push(instance(
                format!("{mesh}-{stage}-{index}"),
                mesh,
                *translation,
                *rotation,
                *scale,
                &format!("leaf-{stage}"),
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

/// Returns a settlement's plants: bamboo clumps at `seats`, seated on `ground`.
///
/// # Invariants
///
/// **One species and one expansion carry every clump**: the plant is grown once and the same culms are placed at each
/// seat, which is what makes four clumps four instances of one plant rather than four plants. A settlement's clumps are
/// meant to look like one another, and a scene that grew them separately would pay four times the expansion for a
/// difference nobody asked for.
/// How much larger a planted clump is than the species it was grown from: a settlement`s bamboo is a clump a person
/// walks past, and the species` own scale is a plant in a study.
const SCALE: f32 = 3.4;

/// How much larger a blade is drawn than the species makes it, so that a clump twenty metres away still shows foliage.
const LEAF: f32 = 1.8;

pub(crate) fn bamboo_clumps(seats: &[[f32; 2]], ground: &dyn Fn([f32; 2]) -> f32) -> Extra {
    let species: Species = clump(7);
    let expansion = expand(&species.plant, Detail::DEFAULT, &species.fields);
    let ids = node_ids(&species.plant);
    let mut extra = Extra::none();

    // The wood: one mesh for every clump, because every culm's loft is packed by the placement that stands it.
    let placements: Vec<Placement> = seats
        .iter()
        .enumerate()
        .map(|(index, at)| {
            let yaw = index as f32 * 1.7;
            standing([at[0], ground(*at), at[1]], yaw, SCALE)
        })
        .collect();
    extra.meshes.push((
        "plants-wood".to_owned(),
        wood_points_of(
            placements
                .iter()
                .map(|placement| (expansion.wood.as_slice(), &ids, placement)),
        ),
    ));
    for placement in &placements {
        let (translation, rotation, scale) = standing_at(placement, 1.0);
        extra
            .wood
            .push(("plants-wood".to_owned(), translation, rotation, scale));
    }

    // The blades: one template per its own index, and one instance per leaf.
    let mut labels: Vec<Option<String>> = vec![None; species.templates.len()];
    for group in &expansion.leaves {
        let template = group.template;
        let label = match &labels[template] {
            Some(label) => label.clone(),
            None => {
                let label = format!("plants-leaf-{template}");
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
                // **A settlement`s blades are drawn larger than a study`s**: a clump seen from the lane is a hundred
                // pixels high, and a blade at the species` own scale is under one. The placement is the plant`s; only what
                // is drawn of a leaf is scaled.
                let (translation, rotation, scale) = standing_at(&world, leaf.scale * LEAF);
                extra
                    .leaves
                    .push((label.clone(), translation, rotation, scale));
            }
        }
    }

    // **One stem and one leaf material, drawn by the plain shaders**, which take their colour from the material rather
    // than from the vertex: a settlement does not name its plants' nodes, so a blade has no node to be shaded from.
    let stem = || surface([0.42, 0.44, 0.24], 0.0, 0.86, [0.0; 3]);
    let blade = || surface([0.24, 0.38, 0.18], 0.0, 0.82, [0.0; 3]);
    for (label, body) in [("stem", stem as fn() -> Value), ("leaf", blade)] {
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
