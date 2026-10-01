//! A night field of paper lanterns.
//!
//! # Why this scene exists
//!
//! Every other scene in the gallery is lit and nothing in it emits. This one is the scene the
//! emission stage was built for: the lanterns carry their own colour instead of reflecting a
//! light, so what they contribute is added once rather than once per light, and the field is
//! lit by **two** lights precisely so that a term added once per light would be visibly wrong.
//!
//! The ground is dark water. A night field is mostly what the water does with the few things
//! that are brighter than it, so the surface body is a reflection of the lanterns and the
//! sky rather than a shading of the mud under them.

use definition::Value;

use crate::geometry::{cuboid, hash01, heightfield};
use crate::kit::{
    camera_placed, instance, instances, light, mesh, mesh_material, post_processes,
    screen_material, shaded_point_schema, surface, surface_reflecting,
};
use crate::shaders::{emission_shader, mesh_shaders, post_shader};

/// How far the field reaches, in world units.
const EXTENT: f64 = 52.0;

/// How many lanterns stand along each axis of the field.
const ROWS: usize = 6;

/// How far the water reaches, which is much further than the lanterns stand.
///
/// # Why the water is larger than the field
///
/// A sheet of water that ends is a model of a place rather than a place: its edge is a line of
/// contrast across the frame and the eye finds it at once. The water runs past the last
/// lantern far enough that its edge is outside the frame and beyond the range the reflection
/// reaches, so what the frame shows is water going dark towards a horizon rather than a raft.
const WATER_EXTENT: f64 = 300.0;

/// The height a lantern is hung at, above the water.
const HANG: f64 = 6.0;

/// The colour a lantern is lit with, which is what the emission stage adds.
const LANTERN: [f64; 3] = [1.0, 0.62, 0.30];

/// Builds the night field.
pub fn lanterns() -> Value {
    let (materials, instances) = build();

    Value::structure([
        ("name".to_owned(), Value::String("lanterns".to_owned())),
        ("time".to_owned(), Value::Float(0.0)),
        (
            "cameras".to_owned(),
            // Above and behind the near rank, looking down the field, and **placed rather than
            // fitted**: the water is much larger than the lanterns and fit to the water the
            // field would be a speck. A night field is a recession rather than a subject, and
            // what the frame has to show is the water between the camera and the far lights.
            Value::List(vec![camera_placed(
                [-16.0, 10.5, 40.0],
                [1.0, 3.0, -6.0],
                0.88,
                1.0,
                320.0,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                // The moon, dim and high behind the field, so the water has a sky to reflect
                // and the poles have an edge.
                light("moon", [-70.0, 120.0, 90.0], [0.24, 0.30, 0.50], 1.5, 300.0),
                // Two lanterns are also lights. A lantern's own stage adds its colour to
                // itself and lights nothing else — an emission is not a light — so what makes
                // the water glow under the field is these, and they are the reason the field
                // reads as a place rather than as a row of bright squares.
                light("near", [-20.0, 5.0, 16.0], [1.0, 0.55, 0.24], 2.4, 60.0),
                light("far", [10.0, 5.0, -22.0], [1.0, 0.58, 0.26], 2.0, 60.0),
            ]),
        ),
        ("shaders".to_owned(), Value::List(build_shaders())),
        ("meshes".to_owned(), Value::List(build_meshes())),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        ("mesh_materials".to_owned(), Value::List(materials)),
        ("instances".to_owned(), instances),
    ])
}

/// Builds the materials and the instances the field is drawn from.
fn build() -> (Vec<Value>, Value) {
    let mut materials = Vec::new();
    let mut shadowing = Vec::new();
    let mut depthing = Vec::new();
    let mut lighting = Vec::new();
    let mut emission = Vec::new();

    // The water, drawn by every stage that has something to do with it: it casts nothing, but
    // it is what the lanterns are seen against and what the depth buffer holds.
    let water = |label: &str, shader: &str| {
        mesh_material(
            label,
            shader,
            surface_reflecting([0.040, 0.050, 0.070], 0.0, 0.06, [0.0; 3], 1.0),
        )
    };
    materials.push(water("water-cast", "shadowing"));
    materials.push(water("water-depth", "depthing"));
    materials.push(water("water-shade", "lighting"));
    for (bucket, label) in [
        (&mut shadowing, "water-cast"),
        (&mut depthing, "water-depth"),
        (&mut lighting, "water-shade"),
    ] {
        bucket.push(instance(
            &format!("water-{label}"),
            "water",
            [0.0; 3],
            [0.0; 3],
            [1.0; 3],
            label,
        ));
    }

    // A pole is dark wood and emits nothing, so it has three stages and no fourth.
    materials.push(mesh_material(
        "pole-cast",
        "shadowing",
        surface([1.0, 1.0, 1.0], 0.0, 0.9, [0.0; 3]),
    ));
    materials.push(mesh_material(
        "pole-depth",
        "depthing",
        surface([1.0, 1.0, 1.0], 0.0, 0.9, [0.0; 3]),
    ));
    materials.push(mesh_material(
        "pole-shade",
        "lighting",
        surface([0.055, 0.045, 0.035], 0.0, 0.9, [0.0; 3]),
    ));

    // The lantern: the one material in the gallery with a fourth stage, and the reason this
    // scene is here. `emissive` is its own colour, which the emission body scales.
    materials.push(mesh_material(
        "lantern-cast",
        "shadowing",
        surface([1.0, 1.0, 1.0], 0.0, 0.8, LANTERN),
    ));
    materials.push(mesh_material(
        "lantern-depth",
        "depthing",
        surface([1.0, 1.0, 1.0], 0.0, 0.8, LANTERN),
    ));
    materials.push(mesh_material(
        "lantern-shade",
        "lighting",
        surface([0.62, 0.34, 0.20], 0.0, 0.8, LANTERN),
    ));
    materials.push(mesh_material(
        "lantern-glow",
        "emission_lantern",
        surface([1.0, 1.0, 1.0], 0.0, 0.8, LANTERN),
    ));

    for row in 0..ROWS {
        for column in 0..ROWS {
            let index = (row * ROWS + column) as i64;
            // A deterministic scatter, so an unchanged scene compiles to an unchanged field.
            let x = (row as f64 / (ROWS - 1) as f64 - 0.5) * 2.0 * EXTENT * 0.8
                + (hash01(index, 3, 11) - 0.5) * 14.0;
            let z = (column as f64 / (ROWS - 1) as f64 - 0.5) * 2.0 * EXTENT * 0.8
                + (hash01(index, 5, 17) - 0.5) * 14.0;
            // The hang varies a little, so the field is a row of lights at slightly different
            // heights rather than a lattice.
            let hang = HANG + (hash01(index, 7, 23) - 0.5) * 1.4;

            let pole = format!("pole-{row}-{column}");
            shadowing.push(instance(
                &format!("{pole}-cast"),
                "pole",
                [x, 0.0, z],
                [0.0; 3],
                [1.0; 3],
                "pole-cast",
            ));
            depthing.push(instance(
                &format!("{pole}-depth"),
                "pole",
                [x, 0.0, z],
                [0.0; 3],
                [1.0; 3],
                "pole-depth",
            ));
            lighting.push(instance(
                &format!("{pole}-shade"),
                "pole",
                [x, 0.0, z],
                [0.0; 3],
                [1.0; 3],
                "pole-shade",
            ));

            let lantern = format!("lantern-{row}-{column}");
            for (bucket, suffix, material) in [
                (&mut shadowing, "cast", "lantern-cast"),
                (&mut depthing, "depth", "lantern-depth"),
                (&mut lighting, "shade", "lantern-shade"),
                (&mut emission, "glow", "lantern-glow"),
            ] {
                bucket.push(instance(
                    &format!("{lantern}-{suffix}"),
                    "lantern",
                    [x, hang, z],
                    [0.0, hash01(index, 13, 29) * std::f64::consts::TAU, 0.0],
                    [1.0; 3],
                    material,
                ));
            }
        }
    }

    (
        materials,
        instances(vec![
            ("shadowing", shadowing),
            ("depthing", depthing),
            ("lighting", lighting),
            ("emission", emission),
        ]),
    )
}

/// Builds the shaders the field declares.
fn build_shaders() -> Vec<Value> {
    mesh_shaders(WATER_SURFACE, "")
        .into_iter()
        .chain(std::iter::once(emission_shader(
            "emission_lantern",
            GLOW,
            "",
        )))
        .chain(std::iter::once(post_shader(NIGHT, "")))
        .collect()
}

/// Builds the meshes the field is drawn from.
fn build_meshes() -> Vec<Value> {
    vec![
        mesh(
            "water",
            "Triangles",
            shaded_point_schema(),
            heightfield(96, WATER_EXTENT, |_, _| 0.0),
        ),
        mesh(
            "pole",
            "Triangles",
            shaded_point_schema(),
            cuboid(0.30, HANG, 0.30),
        ),
        mesh(
            "lantern",
            "Triangles",
            shaded_point_schema(),
            cuboid(1.5, 1.9, 1.5),
        ),
    ]
}

/// What the lighting stage does to the water.
///
/// # Invariants
///
/// The lanterns are not in this body. Nothing here emits: what the water does is reflect what
/// is above it, and the reflection is shading rather than emission, so it belongs to the
/// lighting stage and is counted once per light like every other lit term. The lanterns add
/// themselves in their own stage, once.
const WATER_SURFACE: &str = r#"
    let eye = uniforms.projection.position;
    let view = normalize(in.world_position - eye);
    let grazing = pow(1.0 - abs(dot(normal, view)), 4.0);
    // The water is dark and smooth, so what it shows is the sky and the lanterns, not a
    // colour of its own. The sky is the ambient the stage already computed, so a reflection is
    // the same shading seen edge on rather than a second light.
    //
    // It fades out with distance, which is what puts a horizon in the frame rather than an
    // edge: past the last light there is nothing above the water for it to show, and water
    // with nothing above it is black.
    let reach = 1.0 - clamp(length(in.world_position - eye) / 240.0, 0.0, 1.0);
    color = mix(color, color * 2.4 + vec3<f32>(0.02, 0.03, 0.06), clamp(grazing, 0.0, 0.85) * reach);
"#;

/// What the emission stage adds for a lantern.
///
/// # Invariants
///
/// `emitted` starts as the material's own `emissive`, which is the colour of the glass. A
/// lantern is not evenly bright: the paper is brightest where it is seen square on and falls
/// off towards its edges, and the whole field breathes a little. Both belong here rather than
/// in the lighting stage, because neither is lit — a lantern is as bright under three lights
/// as under one.
const GLOW: &str = r#"
    let towards = normalize(uniforms.projection.position - in.world_position);
    // The face of the lantern that points at the eye is the one whose paper is lit through.
    let facing = clamp(abs(dot(normalize(in.normal), towards)), 0.0, 1.0);
    // A slow breath over the field, offset by where the lantern is, so the field swells in a
    // wave rather than pulsing as one.
    let phase = in.world_position.x * 0.05 + in.world_position.z * 0.03 + uniforms.time * 0.9;
    let breath = 0.86 + 0.14 * sin(phase);
    emitted = emitted * 0.55 * breath * mix(0.45, 1.0, facing);
"#;

/// What the post stage does to the frame.
///
/// # Invariants
///
/// There is no tone curve here, and that is a decision rather than an omission. The frame is
/// mostly very dark with a few bright lanterns in it, and a curve that compresses the lanterns
/// also lifts the water: a Reinhard curve and a gamma turn a night into a grey fog. What keeps
/// a lantern from clipping is the emission stage writing a colour dim enough not to, which is
/// where the brightness of a lantern belongs.
const NIGHT: &str = r#"
    if depth >= 1.0 {
        let ray = ray_through(in.uv);
        let above = pow(clamp(ray.y, 0.0, 1.0), 0.6);
        color = mix(vec3<f32>(0.035, 0.045, 0.070), vec3<f32>(0.005, 0.008, 0.016), above);
    }
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    let vignette = clamp(1.0 - dot(centred, centred) * 1.4, 0.0, 1.0);
    color = color * vignette;
"#;
