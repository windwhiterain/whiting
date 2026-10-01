//! A stage lit by one directional light: the one gallery scene whose subject is a sun.
//!
//! # What this scene is for
//!
//! Every other scene in the gallery is lit by lights that **stand somewhere**, and so is the
//! one after this. A directional light is the only kind that has no position at all, and it is
//! shaded by a different path from the same shader: a stated input of the lighting pass rather
//! than a member of any cluster list, and a page fitted to the **scene** along the light's
//! direction rather than to a box around a place.
//!
//! Both of those can be wrong in a way that renders. A directional light handled as a point
//! light at some arbitrary origin would fall off with distance and shrink its page, and a
//! directional light given no page at all — the one silent failure this checkpoint can have —
//! draws a scene with **no shadow in it and nothing to say so**, because a fragment tested
//! against page minus one is lit unshadowed by design. So the scene is arranged so that a
//! reader can tell both apart in one image.
//!
//! - A **flat floor** and nothing else to light, because the picture is about what the shadows
//!   on it do.
//! - **Four boxes standing well apart across the whole floor**, so their shadows are the same
//!   length and the same width whether they are near the camera or at the far wall.
//! - The sun **low and to one side**, so every shadow is long — a short shadow is a smudge
//!   whose length and width nobody can compare.
//!
//! # What a reader should look at
//!
//! Four shadows, all pointing the same way and all of one length. Then the **gaps between the
//! shadow and the box**: a sun's shadow is displaced from its object by the same amount
//! everywhere, so the gap should be the same at the near end of a shadow as at the far end.
//! And then the **far** shadows against the **near** ones: same edge, same softness, same
//! offset. A far shadow that is blurrier, or shorter, or a slightly different angle to its
//! neighbours, is a page fitted to the light rather than to the scene.
//!
//! # One thing this scene cannot show, and why that is not a defect
//!
//! The obvious contrast — "a sun's shadows are parallel and a lamp's converge" — **does not
//! hold in this renderer**, and the scene should not be read as if it did. A point light's page
//! is `orthographic × look_at`, so it is already a parallel projection and its shadows are
//! already parallel. What actually differs between the two kinds is that a point light's page
//! is a box **centred on the light** and a sun's is a box **centred on the scene**, and that a
//! sun has no distance falloff. So the readable differences are the offset and the length
//! above, and the "converging lamp shadows" claim would be a test of something the renderer
//! never implemented.
//!
//! # Known defect, not a property of this scene
//!
//! The lighting pass is not depth tested, so every box is drawn additively over the floor with
//! no depth test and the floor shows through it — a transparent panel rather than a solid. That
//! is a renderer defect that predates this scene and is written up in
//! `.agents/proposals/lighting-pass-has-no-depth-test.md`; it is not a directional light and
//! not something this scene chose. It does not spoil what the scene is for, because the claim
//! being checked is about the shadows **on the floor**, and the floor is the one surface with
//! nothing drawn over it.

use definition::Value;

use crate::geometry::{cuboid, plane_facing};
use crate::kit::{
    camera_placed, directional, gallery_custom_schema, gallery_instance_schema, instance, mesh,
    mesh_material, post_processes, screen_material, shaded_point_schema, shader, surface,
};
use crate::shaders::{depthing, lighting, post_shader, shadowing};
use scene::standard::Stage;

/// How far the floor reaches either side of the middle, in world units.
///
/// The sun's page is **fitted to the drawn scene**, so a floor twice as wide as it needs to be
/// makes the page twice as wide for the same boxes, and every texel of it is spent on sand no
/// shadow falls on. The floor is only a little wider than the outermost boxes.
const FLOOR_EXTENT: f64 = 8.5;

/// Builds the scene.
pub fn sunlight() -> Value {
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

    // The tints are what tell the four boxes apart, and a floor under one sun has no falloff
    // to separate them by brightness. The floor is the one surface the sun does not make
    // brighter with distance, so a grey floor is a flat grey and the shadows are the only thing
    // on it worth looking at.
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
            surface([0.55, 0.53, 0.50], 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material(
            "near",
            "lighting",
            surface([1.0, 0.32, 0.24], 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material(
            "far",
            "lighting",
            surface([0.20, 0.44, 0.90], 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material(
            "left",
            "lighting",
            surface([0.92, 0.80, 0.26], 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material(
            "right",
            "lighting",
            surface([0.36, 0.80, 0.42], 0.0, 1.0, [0.0; 3]),
        ),
    ]);

    // One table of the boxes, read by all three stages, because an instance label names one
    // placement in the **whole** scene and the same placement is drawn once per stage.
    //
    // The four are spread along a line running away from the camera and across it, and their
    // heights differ so the shadows are four different lengths from the same sun. Equal
    // heights would make four equal shadows, and one correct shadow is indistinguishable from
    // one that happens to be right.
    let boxes: [(&str, f64, f64, f64, [f64; 3]); 4] = [
        ("near", 1.4, 2.6, 1.4, [-0.4, 0.0, 1.6]),
        ("far", 1.6, 3.9, 1.6, [0.6, 0.0, -3.4]),
        ("left", 1.2, 1.7, 1.2, [-3.6, 0.0, -1.1]),
        ("right", 1.3, 3.1, 1.3, [3.3, 0.0, -0.4]),
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

    // Every drawn thing is a caster, floor included: the floor is what receives the shadows,
    // and a receiver that is not a caster is a receiver whose shadows are missing wherever
    // something stands between it and the sun.
    //
    // **Every box in `boxes` is placed here, and the two tables are the same length.** A mesh
    // with no placement is a mesh nothing draws: it is built, it costs a vertex buffer, and it
    // is invisible, so a scene declaring four casters and placing three has a shadowing pass
    // that renders two occluders and a floor, and a viewer cannot tell which of the four the
    // missing one is.
    let placements: Vec<(&str, [f64; 3])> = std::iter::once(("floor", [0.0; 3]))
        .chain(boxes.iter().map(|(label, _, _, _, at)| (*label, *at)))
        .collect();
    assert_eq!(
        placements.len(),
        boxes.len() + 1,
        "every box is placed and the floor is the one extra"
    );
    // One list per stage, holding every placement in the scene. The two stages that only
    // **place** a mesh name one material for all of them; the lighting stage names each
    // placement after its own tinted material, because there every mesh is told apart by
    // colour.
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
        ("name".to_owned(), Value::String("sunlight".to_owned())),
        (
            "cameras".to_owned(),
            // Low and back, looking along the line the boxes are spread on, so the near
            // shadow and the far shadow are in the same frame at very different distances.
            // That pairing is the whole test: a point light draws them at different lengths,
            // and only looking down the line can a reader see that they are not.
            //
            // The near plane is three units rather than a hand's width, for the reason the
            // other scenes give: depth slices divide the range between the near and far
            // planes, and a camera a hand's width from its near plane puts the whole floor
            // into the last few percent of the range.
            Value::List(vec![camera_placed(
                [6.6, 3.1, 8.4],
                [-0.2, 0.7, -1.6],
                0.95,
                3.0,
                34.0,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![directional(
                "sun",
                // Down, to the right and away from the camera, and **low**: the vertical
                // component is smaller than the horizontal ones, so every shadow is longer
                // than the box that casts it. A high sun makes a shadow about as long as its
                // object is wide, and a shadow that short cannot be measured against another.
                // The length of the vector does not matter; the compiler normalises it.
                [0.46, -0.60, -0.66],
                // Slightly warm and not very bright. There is no falloff to read, so the
                // intensity is only "bright enough that the floor is not mud after the
                // grade" — and a sun that clips the floor to white would hide the shadow's
                // own gradient, which is half of what this scene is for.
                [1.0, 0.95, 0.86],
                1.25,
                // **The whole drawn scene, and this is the number to change first.** The page
                // is a box of this half width fitted around the middle of everything drawn,
                // and the floor is `FLOOR_EXTENT` either side, so a smaller extent than the
                // floor's own half width leaves the outer boxes **outside the page** — where a
                // fragment reads as unshadowed, and the scene draws a sun with no shadow on
                // the edges of its own stage. A sun has to shadow the whole scene or the
                // comparison this scene exists for is only true of its middle.
                FLOOR_EXTENT + 0.5,
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
