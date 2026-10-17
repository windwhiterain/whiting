//! One bamboo, drawn: its wood as a single mesh and its leaves as one blade instanced once per leaf.
//!
//! # What this scene is for
//!
//! `vegetation` measures itself in [`plant_probe`](../../../vegetation/examples/plant_probe.rs): a plant
//! walks, a field counts, a blade is a solid. **Those are numbers about geometry, and geometry can satisfy
//! all of them and still look like nothing** — a leaf standing in the air, a crown that is a hedgehog of
//! triangles, a culm whose normals face inward. This is the scene that shows the plant reaching a frame,
//! and it lives here rather than in an example because a gallery scene is this tree's one picture format:
//! [`render`] draws it, `--census` reports what it holds, `--locate` answers what a pixel is.
//!
//! ```text
//! cargo run -p whiting-device-rasterize --example render -- bamboo out/bamboo
//! ```
//!
//! **It is in [`crate::Gallery::build`] and not in [`crate::Gallery::names`].** Those are two lists and the
//! second is not a shorter way of writing the first: `names` is the set the tree's suites hold themselves
//! to, and `scene`'s `clustered_lights` carries a table of expected pass counts per scene.
//! [`crate::sweep_picture`] is unlisted for the same reason, and this one is rendered by name on request.
//!
//! # The claim the frame makes
//!
//! **A leaf field is a distribution and not a generator**: one blade, `n` rigid placements, one mesh.
//! The document therefore carries three meshes — the wood, the blade, the ground — and the lighting bucket
//! carries `n + 2` placements of which the `n` leaves are one **instanced draw**: the emitter groups by
//! shader, mesh and images, so every leaf of every twig lands in one pass over one instance buffer. That is
//! a claim about a call site rather than about a table, which is why it is here and not in a probe, and
//! `plant_instancing_probe` measures the one part of it that could be quietly wrong — the transform.
//!
//! # What is not built here
//!
//! **No second mesh for a second node.** The plant is one object standing on a ground plane, so a material
//! per branch would be ten materials and ten passes over one tree. What differs between a culm and a twig
//! is geometry, which is already in the points. And the identity a pixel carries is the node's own, written
//! per corner; the leaf template is the one mesh whose ids cannot be baked, because it is one mesh drawn by
//! many rows — see [`crate::vegetation_mesh`].
//!
//! [`render`]: ../../../device-rasterize/examples/render.rs

use definition::Value;
use scene::standard::Stage;
use vegetation::decorator::{Seat, Template, placed};
use vegetation::derive::{Species, bamboo};
use vegetation::envelope::{Envelope, trim};
use vegetation::expand::{Detail, Expansion, expand};
use vegetation::field::Field;

use crate::building_identity::{building_extras, not_authored_extras, outputs_field, stage_set};
use crate::geometry::plane_facing;
use crate::kit::{
    camera_placed, flux_for_irradiance, instance, instances, light, light_with_radius, mesh,
    mesh_material, offset_by, post_processes, screen_material, surface,
};
use crate::shaders::post_shader;
use crate::vegetation_instancing::standing;
use crate::vegetation_mesh::{LEAF_BASE_ID, node_ids, template_points, wood_points};

/// The name the still scene is built and rendered under.
pub const SCENE: &str = "bamboo";

/// The name the same plant is built and rendered under **with a wind on it**, so a picture can show what a
/// field does beside the plant it does it to.
pub const WINDY: &str = "bamboo_wind";
/// The name the same plant is drawn under **fitted to a crown**, so a picture can show what the envelope took.
pub const CROWNED: &str = "bamboo_crown";
/// The name the same plant is drawn under **with a knot at the base of every bough**.
pub const KNOTTED: &str = "bamboo_knots";

/// The seed the plant's leaf field is derived from.
///
/// **The number is the scene's, not the crate's.** [`vegetation::derive::bamboo`] takes a seed for the leaf
/// field's own phase and jitter and uses it for nothing else, so this one number is the whole difference
/// between this plant's leaves and any other drawing of the same tree.
const SEED: u64 = 7;

/// The frame, which is **taller than it is wide**: the subject is 2.2 m up and about 2.7 m across, so a
/// landscape frame either crops the crown or stands so far back that a bamboo becomes a line.
const WIDTH: i64 = 1200;
const HEIGHT: i64 = 1400;

/// Where the camera stands, and what it looks at.
///
/// The eye is **five metres out and half a metre above the middle of the culm**, which is the whole of the
/// framing: the crown reaches 1.35 m either side of a culm that stands 2.2 m tall, and the frame is
/// narrower than it is tall, so the half width has to clear 1.35 m at the crown or the frame crops the one
/// part of a bamboo that is a bamboo. Standing at half the culm's height rather than above its crown puts
/// the crown against the sky and the floor's horizon behind the plant, so the two metres of culm are read
/// as height instead of as a plan seen from a roof.
const EYE: [f64; 3] = [3.40, 1.55, 3.80];
const TARGET: [f64; 3] = [0.16, 1.10, 0.05];
const FIELD_OF_VIEW: f64 = 0.66;
const NEAR: f64 = 0.05;
const FAR: f64 = 24.0;

/// The ground the plant stands on, and how far it reaches.
///
/// Six metres, which is further than the frame's own near corner, so the floor does not end inside the
/// picture: a floor edge at 3 m under a camera 5.4 m back is a horizon line half way up the frame, and the
/// whole of this scene is one plant and the ground it stands on.
const FLOOR_EXTENT: f64 = 6.0;
const FLOOR_TINT: [f64; 3] = [0.40, 0.38, 0.33];

/// The key, which is the light that makes a culm read as a round thing rather than a stripe.
///
/// It stands **above and to the lens's right**, so the culm has a lit side and a shadowed side at once and
/// every twig's shadow falls across the floor to the left of its caster. The reach is past the far corner
/// of the floor, so nothing in frame falls out of the cluster grid at its boundary.
const KEY_POSITION: [f64; 3] = [2.6, 3.9, 2.0];
const KEY_COLOUR: [f64; 3] = [1.0, 0.96, 0.86];
const KEY_IRRADIANCE: f64 = 3.0;
const KEY_REACH: f64 = 7.0;
const KEY_EXTENT: f64 = 3.0;

/// The fill, which names no reach and so is **global**: it is a second light on the shadowed side, it is
/// culled by nothing, and no shadow of it falls on the ground.
const FILL_POSITION: [f64; 3] = [-2.6, 1.5, -1.8];
const FILL_COLOUR: [f64; 3] = [0.44, 0.54, 0.76];

/// The two greens of a bamboo, which are not the same green.
///
/// A culm is waxy and pale and a leaf is dark and matte, and one tint for both makes a bamboo look like a
/// tree drawn in felt pen: the leaf is the thing this scene exists to draw, so it is the one that carries
/// the colour.
const CULM_TINT: [f64; 3] = [0.46, 0.50, 0.24];
const LEAF_TINT: [f64; 3] = [0.20, 0.38, 0.15];

/// The sky this scene draws where nothing else is, written once so the haze and the horizon agree.
const SKY: &str = r#"
fn sky_at(ray: vec3<f32>) -> vec3<f32> {
    let above = pow(clamp(ray.y, 0.0, 1.0), 0.60);
    let horizon = vec3<f32>(0.62, 0.68, 0.72);
    let zenith = vec3<f32>(0.16, 0.28, 0.50);
    return mix(horizon, zenith, above);
}
"#;

/// What the post stage does to the frame.
///
/// The air is an exponential over the geometry only, for the reason the shed's grade gives: the sky is
/// already the far field, and a haze that reached it as well would put two different greys on one horizon.
const GRADE: &str = r#"
    if depth >= 1.0 {
        color = sky_at(ray_through(in.uv));
    }
    if depth < 1.0 {
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp(1.0 - exp(-along_ray * 0.012), 0.0, 0.45);
        color = mix(color, sky_at(vec3<f32>(ray_through(in.uv).x, 0.02, ray_through(in.uv).z)), air);
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
"#;

/// The plant this scene draws, expanded at the crate's own default dial.
///
/// The dial is [`Detail::DEFAULT`] and not a number written here, because the point of the picture is the
/// plant and not a resolution: a scene that picked its own dial would be a second author of the geometry.
fn expansion(species: &Species) -> Expansion {
    expand(&species.plant, Detail::DEFAULT, &species.fields)
}

/// A knot: a short tube that bulges, seated on a bough's base so the culm shows where a limb leaves it.
fn knot() -> Template {
    let spine = shape::Curve::new(vec![
        shape::Jet::new([0.0, 0.0, 0.0], [0.012, 0.0, 0.0]),
        shape::Jet::new([0.014, 0.0, 0.0], [0.010, 0.0, 0.0]),
        shape::Jet::new([0.026, 0.0, 0.0], [0.004, 0.0, 0.0]),
    ])
    .expect("the knot's knots are distinct");
    let mut sweep = shape::Sweep::new(spine, shape::Section::circle());
    sweep.scale = shape::Scalar::new(&[(0.0, 0.010), (0.5, 0.026), (1.0, 0.004)])
        .expect("the knot's law is a law");
    Template {
        sweep,
        stations: 7,
        around: 12,
    }
}

/// One knot at the base of every bough: the seats a scene places.
fn knots(species: &Species) -> Vec<Seat> {
    species
        .plant
        .live()
        .filter(|(handle, _)| handle.depth() == 1)
        .map(|(handle, node)| {
            Seat::new(handle, 0, 0.1, node.attachment().slot)
                .sunk(0.012)
                .scaled(1.8)
        })
        .collect()
}

/// Resolves an `authored_id` this scene's frame writes into the handle that names the node.
///
/// # Invariants
///
/// - **The table is built by the same walk the mesh's ids were numbered by**, so a pixel and a handle are
///   the same number by construction rather than by two orderings that have not diverged yet.
/// - **An id this plant does not have answers `None`**, and so does the reserved zero, because *"no such
///   node"* and *"not an authored element"* are different answers.
pub fn handle_of(id: u32) -> Option<String> {
    let species = bamboo(SEED);
    node_ids(&species.plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Returns the sidecar table a frame of this scene is read beside: every id and the handle it names.
///
/// One line per node, in id order, because the table is read beside a picture and a reader looking for
/// *"which twig is that"* wants the ordered list rather than a lookup they have to write.
pub fn sidecar() -> String {
    let species = bamboo(SEED);
    let mut ids: Vec<(u32, String)> = node_ids(&species.plant)
        .into_iter()
        .map(|(handle, id)| (id, handle.to_string()))
        .collect();
    ids.sort_unstable();
    let mut table = String::from(
        "authored_id  handle                 a leaf of this node is drawn by the blade mesh with this offset\n",
    );
    for (id, handle) in ids {
        table.push_str(&format!(
            "{id:>10}  {handle:<22} {}\n",
            id as f64 - f64::from(LEAF_BASE_ID)
        ));
    }
    table
}

/// Builds the bamboo scene: one camera, a key and a fill, a floor, the plant's wood and its blade.
pub fn bamboo_picture() -> Value {
    plant_scene(&bamboo(SEED), &[])
}

/// The same plant under a lateral wind, drawn beside it: a field bends spines, and a picture is the only place
/// the bend is visible as a shape rather than as a number.
pub fn windy_picture() -> Value {
    plant_scene(
        &bamboo(SEED).with_fields(vec![Field::light([1.0, 0.0, 0.0], 0.12)]),
        &[],
    )
}

/// The same plant fitted to a crown: `trim` writes length nudges until each tip is on the envelope, so the frame
/// shows one tree with shorter boughs rather than a different tree.
pub fn crowned_picture() -> Value {
    let mut species = bamboo(SEED);
    let crown = Envelope::radial(&[(0.0, 0.55), (0.5, 0.45), (1.0, 0.35)]);
    trim(&mut species.plant, &crown);
    plant_scene(&species, &[])
}

/// Builds one plant scene: the camera, the lights, the floor, the wood and the blade, under whatever fields the
/// species carries.
/// The same plant with a knot at the base of every bough: a decorator is a template and three numbers, and the
/// frame shows the three numbers on the bark rather than in a table.
pub fn knots_picture() -> Value {
    let species = bamboo(SEED);
    let seats = knots(&species);
    plant_scene(&species, &seats)
}

fn plant_scene(species: &Species, seats: &[Seat]) -> Value {
    let species = species.clone();
    let decorators = placed(&species.plant, seats);
    let expanded = expansion(&species);
    let ids = node_ids(&species.plant);

    // **Two point schemas, and they are the two identities**: every authored mesh carries `authored_id`,
    // the ground carries none and writes the reserved zero from a constant.
    let extras = building_extras();
    let point = extras.point_schema();
    let ground = not_authored_extras();

    let mut materials = vec![
        mesh_material(
            "culm-cast",
            "plant_shadowing",
            surface(CULM_TINT, 0.0, 0.72, [0.0; 3]),
        ),
        mesh_material(
            "culm-depth",
            "plant_depthing",
            surface(CULM_TINT, 0.0, 0.72, [0.0; 3]),
        ),
        mesh_material(
            "culm-shade",
            "plant_lighting",
            surface(CULM_TINT, 0.0, 0.72, [0.0; 3]),
        ),
        // The blade's two caster rows share one material: a caster carries no id, so a material per group
        // would be six rows that differ only in a field no stage of theirs reads.
        mesh_material(
            "leaf-cast",
            "plant_shadowing",
            surface(LEAF_TINT, 0.0, 0.62, [0.0; 3]),
        ),
        mesh_material(
            "leaf-depth",
            "plant_depthing",
            surface(LEAF_TINT, 0.0, 0.62, [0.0; 3]),
        ),
    ];
    let leaf_body = surface(LEAF_TINT, 0.0, 0.62, [0.0; 3]);
    // **One material per leaf group, and the only thing that differs is the offset.** The blade is one mesh
    // drawn by every leaf, so a leaf's own corners carry the blade's base id and the node it hangs from is
    // its row's offset: `authored_offset = node - LEAF_BASE_ID`. The groups still draw in **one** pass,
    // because the emitter groups by shader, mesh and images and not by material.
    for (group, leaves) in expanded.leaves.iter().enumerate() {
        let node = ids[&leaves.node];
        materials.push(mesh_material(
            &format!("leaf-shade-{group}"),
            "plant_lighting",
            offset_by(leaf_body.clone(), f64::from(node - LEAF_BASE_ID)),
        ));
    }
    // A knot is drawn by every seat of its template, so its corners carry the blade's base id and each seat's row
    // carries the node it sits on: one mesh, one material per seat, and the seats still draw in one pass.
    if !decorators.is_empty() {
        for (stage, shader) in [("cast", "plant_shadowing"), ("depth", "plant_depthing")] {
            materials.push(mesh_material(
                &format!("knot-{stage}"),
                shader,
                surface(CULM_TINT, 0.0, 0.72, [0.0; 3]),
            ));
        }
        for (index, instance) in decorators.iter().enumerate() {
            let node = ids[&instance.node];
            materials.push(mesh_material(
                &format!("knot-shade-{index}"),
                "plant_lighting",
                offset_by(
                    surface(CULM_TINT, 0.0, 0.72, [0.0; 3]),
                    f64::from(node - LEAF_BASE_ID),
                ),
            ));
        }
    }
    let earth = surface(FLOOR_TINT, 0.0, 0.94, [0.0; 3]);
    for (stage, shader) in [
        ("cast", "ground_shadowing"),
        ("depth", "ground_depthing"),
        ("shade", "ground_lighting"),
    ] {
        materials.push(mesh_material(
            &format!("floor-{stage}"),
            shader,
            earth.clone(),
        ));
    }

    // **One leaf placement per leaf instance, and the label says which group and which rank**: a label
    // names one instance in the *whole* scene, so the same leaf is drawn in three passes under three
    // labels.
    let placed = |stage: &str| {
        let mesh = |label: &str, material: String| {
            instance(
                format!("{label}-{stage}"),
                label,
                [0.0; 3],
                [0.0; 3],
                [1.0; 3],
                &material,
            )
        };
        // **Two placements of its own and no more**: the wood is baked in world coordinates and the floor
        // is a plane. The blade has no placement of its own — every leaf places it, and a placement at the
        // origin would be a leaf growing out of the ground.
        let mut placements = vec![
            mesh("culm", format!("culm-{stage}")),
            mesh("floor", format!("floor-{stage}")),
        ];
        for (group, leaves) in expanded.leaves.iter().enumerate() {
            let material = if stage == "shade" {
                format!("leaf-shade-{group}")
            } else {
                format!("leaf-{stage}")
            };
            for leaf in &leaves.leaves {
                let (translation, rotation, scale) = standing(leaf);
                placements.push(instance(
                    format!("leaf-{stage}-{group}-{}", leaf.index),
                    "leaf",
                    translation,
                    rotation,
                    scale,
                    &material,
                ));
            }
        }
        // **One placement per decorator, and its frame is the seat's**: a knot has no orientation of its own to
        // choose, which is why the same conversion a blade uses converts it.
        for (index, decorator) in decorators.iter().enumerate() {
            let material = if stage == "shade" {
                format!("knot-shade-{index}")
            } else {
                format!("knot-{stage}")
            };
            let (translation, rotation, scale) = crate::vegetation_instancing::seated(decorator);
            placements.push(instance(
                format!("knot-{stage}-{index}"),
                "knot",
                translation,
                rotation,
                scale,
                &material,
            ));
        }
        placements
    };

    Value::structure([
        ("name".to_owned(), Value::String(SCENE.to_owned())),
        (
            "frame".to_owned(),
            Value::structure([
                ("width".to_owned(), Value::Int(WIDTH)),
                ("height".to_owned(), Value::Int(HEIGHT)),
            ]),
        ),
        (
            "cameras".to_owned(),
            Value::List(vec![camera_placed(EYE, TARGET, FIELD_OF_VIEW, NEAR, FAR)]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                light_with_radius(
                    "key",
                    KEY_POSITION,
                    KEY_COLOUR,
                    flux_for_irradiance(KEY_IRRADIANCE, KEY_REACH),
                    KEY_EXTENT,
                    KEY_REACH,
                ),
                light(
                    "fill",
                    FILL_POSITION,
                    FILL_COLOUR,
                    flux_for_irradiance(0.7, 3.0),
                    KEY_EXTENT,
                ),
            ]),
        ),
        (
            "shaders".to_owned(),
            Value::List({
                let mut shaders = stage_set("plant", "", "", &point, &extras);
                shaders.extend(stage_set("ground", "", "", &ground.point_schema(), &ground));
                shaders.push(post_shader(GRADE, SKY, None));
                shaders
            }),
        ),
        // The one image this scene writes beside the frame: which node each pixel shows.
        ("outputs".to_owned(), outputs_field()),
        ("meshes".to_owned(), {
            let mut meshes = vec![
                mesh(
                    "culm",
                    "Triangles",
                    point.clone(),
                    wood_points(&expanded.wood, &ids),
                ),
                mesh(
                    "leaf",
                    "Triangles",
                    point.clone(),
                    template_points(&species.templates[0]),
                ),
            ];
            // **A decorator mesh only when a seat asks for one**: a scene with no seats is the same document
            // it was before decorators existed, down to the frame it writes.
            if !decorators.is_empty() {
                meshes.push(mesh(
                    "knot",
                    "Triangles",
                    point,
                    crate::vegetation_mesh::decorator_points(&knot()),
                ));
            } else {
                // The knot mesh would be unused; `point` stays moved exactly once either way.
                let _ = &point;
            }
            meshes.push(mesh(
                "floor",
                "Triangles",
                ground.point_schema(),
                plane_facing(FLOOR_EXTENT, 0.0, true),
            ));
            Value::List(meshes)
        }),
        ("mesh_materials".to_owned(), Value::List(materials)),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        (
            "instances".to_owned(),
            instances(vec![
                (Stage::Shadowing, placed("cast")),
                (Stage::Depthing, placed("depth")),
                (Stage::Lighting, placed("shade")),
            ]),
        ),
    ])
}
