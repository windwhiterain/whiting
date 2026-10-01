//! A stage lit by one spot light: the one gallery scene whose subject is a cone.
//!
//! # What this scene is for
//!
//! Every other scene in the gallery is lit by lights that reach everywhere they can see. This
//! one is lit by a light that reaches a **cone**, and it exists because a spot light is the one
//! kind whose shading cannot be checked by any other scene: the falloff between the two angles,
//! the hard edge at the outer one, and a shadow volume whose shape is a perspective frustum
//! rather than a box are three things that either read correctly in one image or are not there
//! at all.
//!
//! So the scene is arranged to make all three legible at once, and it is deliberately the
//! smallest scene in the gallery:
//!
//! - A **flat floor** and nothing else to light, so the pool of light the cone cuts out of it
//!   is the whole of what the frame is about. A pool with a bright middle, a soft ring and a
//!   hard edge is the signature of the two angles, and it is the first thing to look for.
//! - **Three boxes of different heights**, all inside the pool, so the shadows they cast are
//!   long enough to be shapes rather than smudges, and all pointing away from the light. A
//!   shadow that is not cone shaped is a shadow drawn through a box, which is what a point
//!   light's volume is.
//! - A spot light **standing above and off to one side**, aimed down and slightly forward, so
//!   the pool is not a circle in the middle of the frame and the camera sees the cone obliquely
//!   rather than down its axis.
//!
//! The floor's own extent is much larger than the pool, because the darkness outside the pool is
//! half of what the picture is: a scene where the cone filled the frame could not tell a narrow
//! cone from a wide one.

use definition::Value;

use crate::geometry::{cuboid, plane_facing};
use crate::kit::{
    camera_placed, gallery_custom_schema, gallery_instance_schema, instance, mesh, mesh_material,
    post_processes, screen_material, shaded_point_schema, shader, spot, surface,
};
use crate::shaders::{depthing, lighting, post_shader, shadowing};
use scene::standard::Stage;

/// How far the floor reaches either side of the middle, in world units.
///
/// Wider than the pool of light the cone cuts out of it, because the darkness outside the pool
/// is half of what the picture is. It is not much wider, because the shadow page is fitted to
/// everything that is drawn: a floor reaching twice as far makes the page twice as deep for the
/// same cone, and every texel of it is spread thinner.
const FLOOR_EXTENT: f64 = 9.0;

/// Where the light stands, and the height of the floor it stands over.
const LAMP: [f64; 3] = [-1.6, 8.5, 4.2];

/// Builds the scene.
pub fn spotlight() -> Value {
    let point = shaded_point_schema();
    let inputs = crate::attributes::gallery_vertex_inputs(&point);
    let custom = gallery_custom_schema();

    let shaders = Value::List(vec![
        shader(
            Stage::Shadowing,
            "shadowing",
            &shadowing(&inputs),
            scene::standard::shadow_pass_schema(),
            point.clone(),
            gallery_instance_schema(),
            custom.clone(),
        ),
        shader(
            Stage::Depthing,
            "depthing",
            &depthing(&inputs),
            scene::standard::depth_pass_schema(),
            point.clone(),
            gallery_instance_schema(),
            custom.clone(),
        ),
        shader(
            Stage::Lighting,
            "lighting",
            &lighting(&inputs, "", ""),
            scene::standard::light_pass_schema(),
            point.clone(),
            gallery_instance_schema(),
            custom.clone(),
        ),
        post_shader(GRADE, ""),
    ]);

    // A material names one shader, and a shader is drawn in the one stage it belongs to, so
    // anything that casts a shadow is drawn three times. The tints of the four lighting
    // materials are the only thing telling the boxes apart in a frame lit by a single light,
    // and three greys under one cone are one grey.
    //
    // The two stages that only **place** a mesh have one material each for the whole scene, and
    // that is a requirement rather than a tidiness: a depthing pass clears the depth buffer it
    // draws into, so a scene with two of them leaves the depth of the first pass cleared away by
    // the second, and every object drawn in the lighting passes is then tested against a depth
    // that no longer holds it. One material is one pass is one clear.
    let materials = Value::List(vec![
        mesh_material("caster", "shadowing", surface([1.0; 3], 0.0, 1.0, [0.0; 3])),
        mesh_material(
            "occluder",
            "depthing",
            surface([1.0; 3], 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material(
            "floor",
            "lighting",
            surface([0.62, 0.60, 0.57], 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material(
            "tall",
            "lighting",
            surface([1.0, 0.1, 0.1], 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material(
            "wide",
            "lighting",
            surface([0.1, 1.0, 0.1], 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material(
            "thin",
            "lighting",
            surface([0.1, 0.1, 1.0], 0.0, 1.0, [0.0; 3]),
        ),
    ]);

    // One table of the boxes, read by all three stages, because an instance label names one
    // placement in the **whole** scene and the same placement is drawn once per stage. The
    // sizes are baked into the meshes and the placements into the instances, so a box's
    // dimensions and where it stands are two statements about two different meshes.
    let boxes: [(&str, f64, f64, f64, [f64; 3]); 3] = [
        ("tall", 1.3, 3.4, 1.3, [0.6, 0.0, -0.6]),
        ("wide", 2.2, 1.3, 2.2, [2.6, 0.0, -1.9]),
        ("thin", 0.7, 4.6, 0.7, [-1.1, 0.0, -2.4]),
    ];

    let mut meshes = vec![mesh(
        "floor",
        "Triangles",
        point,
        plane_facing(FLOOR_EXTENT, 0.0, true),
    )];
    for (label, width, height, depth, _) in boxes {
        meshes.push(mesh(
            label,
            "Triangles",
            shaded_point_schema(),
            cuboid(width, height, depth),
        ));
    }

    // Every drawn thing is a caster: the floor is what receives the shadows, and a receiver
    // that is not a caster is a receiver whose shadows are missing wherever something stands
    // between it and the light. The boxes cast onto the floor, so all three are drawn into the
    // page as well as onto it.
    //
    // **Every box in `boxes` is placed here, and the two tables are the same length.** A mesh
    // with no placement is a mesh nothing draws: it is built, it costs a vertex buffer, and it
    // is invisible, so a scene that declares three casters and places two has a shadowing pass
    // that renders one occluder and a floor, and a viewer cannot tell which of the three the
    // missing one is.
    let placements: Vec<(&str, [f64; 3])> = vec![
        ("floor", [0.0; 3]),
        ("tall", [0.6, 0.0, -0.6]),
        ("wide", [2.6, 0.0, -1.9]),
        ("thin", [-1.1, 0.0, -2.4]),
    ];
    assert_eq!(
        placements.len(),
        boxes.len() + 1,
        "every box is placed and the floor is the one extra"
    );
    // One list per stage, holding every placement in the scene. The two stages that only
    // **place** a mesh name one material for all of them, because a caster's row carries
    // nothing a shadow page or a depth buffer can use; the lighting stage has no such
    // question, because there every mesh is named after its own tinted material.
    let bucket = |stage: &str, shared: Option<&str>| -> Value {
        Value::List(
            placements
                .iter()
                .map(|(label, at)| {
                    instance(
                        // A label names one placement in the whole scene, so the three lists
                        // cannot share labels and each of them carries its own.
                        format!("{stage}-{label}"),
                        label,
                        *at,
                        [0.0; 3],
                        [1.0; 3],
                        shared.unwrap_or(label),
                    )
                })
                .collect(),
        )
    };

    Value::structure([
        ("name".to_owned(), Value::String("spotlight".to_owned())),
        (
            "cameras".to_owned(),
            // Above the floor and off to one side, looking down at about a third of a right
            // angle. The angle is the point: from directly above, the pool is a circle and the
            // shadows are radial, and a cone is round, so a cone cannot be told from a sphere
            // in that view. Seen obliquely the pool is an ellipse and the shadows stretch away
            // from the lamp, which is what a cone looks like in a room.
            //
            // The near plane is three units, not a hand's width, and that is about the **cluster
            // grid** rather than about the picture. Depth slices divide the range between the
            // near and far planes, and a camera a hand's width from its near plane puts the
            // whole floor into the last two percent of the range: the light is then culled per
            // slice and the pool of light has a staircase along its edge that belongs to no
            // lighting model at all.
            Value::List(vec![camera_placed(
                [7.4, 4.6, 9.2],
                [-0.4, 0.9, -1.0],
                0.92,
                3.0,
                30.0,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![spot(
                "lamp",
                LAMP,
                // Down and forward, not straight down: a lamp directly overhead throws every
                // shadow straight out from under its own object, and the frame is then a plan
                // view of the cone rather than a view of it.
                [0.18, -1.0, -0.36],
                [1.0, 0.93, 0.80],
                // Bright enough that the middle of the pool is not grey after the grade, and
                // no brighter: a cone whose core clips to white is a cone whose falloff cannot
                // be read, because the whole of the inner angle is one value.
                1.15,
                // A wide gap between the two angles, so the falloff is a ring rather than an
                // edge: about nine degrees at full strength and thirty two at nothing.
                0.16,
                0.56,
                // Wide enough that the cone, and not this number, is what bounds the page. The
                // page reaches the far end of the drawn scene, so the extent only has to be at
                // least as wide as the cone is there; a smaller one would narrow the page to the
                // middle of the light and leave the outer ring of the cone with no shadow at
                // all, which reads as a bug rather than as a choice.
                9.5,
            )]),
        ),
        ("shaders".to_owned(), shaders),
        ("meshes".to_owned(), Value::List(meshes)),
        ("mesh_materials".to_owned(), materials),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        (
            "instances".to_owned(),
            // One bucket per stage, keyed by the stage's own name, each holding every
            // placement in the scene under that stage's materials.
            Value::dictionary([
                ("shadowing".to_owned(), bucket("shadowing", Some("caster"))),
                ("depthing".to_owned(), bucket("depthing", Some("occluder"))),
                ("lighting".to_owned(), bucket("lighting", None)),
            ]),
        ),
    ])
}

/// What the post stage does to the frame.
const GRADE: &str = r#"
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
"#;
