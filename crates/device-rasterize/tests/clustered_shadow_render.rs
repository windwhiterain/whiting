//! The clustered lighting pass and the shadow pages, read back as pixels on a real gpu.
//!
//! # What this proves that a document level test cannot
//!
//! `crates/scene/tests/clustered_lights.rs` proves the assignment put the right lights in the
//! right clusters and the pool gave the right lights a page. `tests/shadow_pages.rs` proves the
//! pass count stopped moving with the light count. Neither can say what the *gpu* did with any
//! of it: a storage buffer bound at the wrong stride, a page index read as a mip level, a
//! cluster resolved from the wrong end of the frame — each produces a document that is
//! entirely correct and a frame that is entirely wrong.
//!
//! So this renders and reads pixels, and it uses the one technique that needs no arithmetic
//! about where a shadow ought to land: **two frames that differ**. A frame of an occluder and
//! a frame without one differ exactly where the occluder casts, so the difference *is* the
//! shadow and can be checked without knowing where it should be.
//!
//! # Why the assertions are about direction and not about amounts
//!
//! A shadow only ever darkens. So adding an occluder may change many pixels, and every one of
//! them must go **down**; a pixel that brightened is a comparison running backwards, which is
//! the failure a shadow term has when the depth is on the wrong side. And a light with no page
//! is lit unshadowed, so adding a second light may change many pixels and every one of them
//! must go **up**; a pixel that darkened is that light reading a neighbour's page, which is the
//! failure the page sentinel of minus one exists to prevent.
//!
//! Both assertions are therefore independent of how much light there is, how bright it is, and
//! where on the frame anything lands, which is what lets them be about the mechanism rather
//! than about a picture.

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier, Value};
use device::{Frame, Renderer};
use document::geometry::heightfield;
use document::kit::{
    camera_placed, gallery_custom_schema, gallery_instance_schema, instance, instances,
    light_with_radius, mesh, mesh_material, post_processes, screen_material, shaded_point_schema,
    shader, surface,
};
use document::shaders::{depthing, lighting, post_shader, shadowing};
use scene::standard::Stage;
use scene::{CompileContext, Compiler, STANDARD_SCENE_SCHEMA, StandardScene};
use whiting_device_rasterize::WgpuRenderer;

/// A note printed when there is nothing to test on.
const NO_ADAPTER: &str = "skipped: no gpu adapter is present";

/// How far the ground reaches either side of the origin.
const GROUND: f64 = 10.0;

/// How high the occluder stands above the ground, which is what makes a shadow at all.
const BLOCK_HEIGHT: f64 = 3.0;

/// Where the key lamp stands: on the far side of the block from the camera.
const KEY: [f64; 3] = [-30.0, 20.0, -24.0];

/// Where the second lamp stands, on the same far side and to the other hand.
const SECOND: [f64; 3] = [26.0, 18.0, -20.0];

/// The radiance each lamp is calibrated to put on the ground directly below it.
///
/// A light's `power` is a **radiant flux** and not a brightness, and the shading model turns a
/// flux into the radiance a surface sees with `power / (4 pi^2 d^2)`. So the flux that puts a
/// given radiance on a subject `d` away is `radiance * 4 pi^2 d^2`, and a document that states
/// any other number is describing a lamp in units no backend shares. See
/// `.agents/SPEC/render/light_units.md`.
///
/// The number below is short of the one that reads as white so that the whole ground keeps room
/// under the top of an eight bit present target, which this stage's empty grade does nothing to
/// pull back. Measured on this scene at 1024x1024, with the three channels summed: the key lamp
/// alone peaks at **586 of 765** over 447475 lit pixels and a mean of 83, with **no** pixel
/// clipped; the two lamp frame peaks at 765 over 505618 lit pixels and a mean of 175, with 422
/// pixels clipped. The block takes 70467 pixels to black, thirty five times the floor the
/// assertion below asks for.
const LAMP_RADIANCE: f64 = 0.45;

/// The radiant flux of the lamp standing at `position`, for the radiance [`LAMP_RADIANCE`].
fn flux(position: [f64; 3]) -> f64 {
    let distance =
        (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt();
    LAMP_RADIANCE * 4.0 * std::f64::consts::PI * std::f64::consts::PI * distance * distance
}

/// How many pixels of difference a shadow has to cover before it counts as one.
///
/// A shadow of a block this size on a ground this wide is a large part of the frame. The
/// number is a floor rather than a target: it only has to exclude the one pixel a rounding
/// error at a silhouette would move, which is what would make this pass on nothing at all.
const SHADOW_PIXELS: usize = 2_000;

/// An occluder between a light and the ground darkens exactly what it covers, and nothing else.
#[test]
fn an_occluder_darkens_the_ground_it_falls_on() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    let with = render(&renderer, &scene(true)).expect("the scene with an occluder renders");
    let without = render(&renderer, &scene(false)).expect("the same scene without one renders");

    let (darker, brighter) = compare(&with.rgba, &without.rgba);
    assert!(
        darker > SHADOW_PIXELS,
        "a block {} units above a ground {} wide, lit from one side, casts a shadow over a large \
         part of the frame, and this pair of frames differs in only {darker} darker pixels and \
         {brighter} brighter ones",
        BLOCK_HEIGHT,
        GROUND * 2.0,
    );
    assert_eq!(
        brighter, 0,
        "a shadow only ever darkens, so a pixel that brightened when the occluder was added is a \
         depth comparison running the wrong way round"
    );
}

/// A light the pool gave no page is lit unshadowed rather than darkened by a neighbour's page.
#[test]
fn a_light_with_no_page_lights_the_frame_rather_than_shadowing_it() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // One page and two lights: the pool ranked them and could only afford one, so the second
    // carries minus one and is lit unshadowed. If the sentinel were read as page zero, that
    // light would take the *first* light's shadow map and its light would be cut by shadows
    // cast for something else — which shows up here as pixels that got darker.
    let one = render(&renderer, &scene_of(1, 1, true)).expect("the single light scene renders");
    let two = render(&renderer, &scene_of(2, 1, true)).expect("the two light scene renders");

    let (darker, brighter) = compare(&two.rgba, &one.rgba);
    assert!(
        brighter > SHADOW_PIXELS,
        "a second light adds its own light to the frame, and this pair differs in only {brighter} \
         brighter pixels"
    );
    assert_eq!(
        darker, 0,
        "a light with no page is lit unshadowed, so it can only brighten. {darker} pixels got \
         darker, which is what reading a neighbour's page looks like"
    );
}

/// Counts the pixels of `after` that are darker than `before` and the ones that are brighter.
///
/// A pixel that is equal is counted as neither: the whole point is to find the ones the
/// mechanism moved, and a tolerance-free comparison of two independently rendered frames
/// would find rounding as readily as shadows.
fn compare(after: &[u8], before: &[u8]) -> (usize, usize) {
    assert_eq!(
        after.len(),
        before.len(),
        "two frames of the same scene are the same size"
    );
    let mut darker = 0;
    let mut brighter = 0;
    for (own, other) in after.chunks_exact(4).zip(before.chunks_exact(4)) {
        let luminance = |pixel: &[u8]| pixel[0] as i32 + pixel[1] as i32 + pixel[2] as i32;
        match luminance(own).cmp(&luminance(other)) {
            std::cmp::Ordering::Less => darker += 1,
            std::cmp::Ordering::Greater => brighter += 1,
            std::cmp::Ordering::Equal => {}
        }
    }
    (darker, brighter)
}

/// Builds the scene: a flat ground with a block standing on it, and one light off to one side.
///
/// `occluder` says whether the block is drawn in the **shadowing** stage, which is the only
/// stage whose passes write a light's page. Leaving it out of that stage while keeping it in
/// the other two gives the same picture with the shadow removed, and nothing else, which is
/// what makes the two frames comparable.
fn scene(occluder: bool) -> Value {
    scene_of(1, 64, occluder)
}

/// Builds the scene with a given number of lights and a pool of that many pages.
///
/// The two counts are separate arguments because that is the case worth testing: a scene can
/// hold more lights than pages, and the lights the pages did not reach are the ones the cheap
/// path exists for.
fn scene_of(lights: usize, pages: i64, occluder: bool) -> Value {
    let point = shaded_point_schema();
    let custom = gallery_custom_schema();
    let inputs = document::attributes::gallery_vertex_inputs(&point);
    let lit = surface([0.8, 0.8, 0.8], 0.0, 1.0, [0.0; 3]);
    let stone = surface([0.7; 3], 0.0, 1.0, [0.0; 3]);

    // A ground of a gentle grid rather than two triangles, because a heightfield's normals
    // are computed from its own gradient and a block cast onto a two triangle plane is a
    // shadow on a plane with no normal to lean away from.
    let flat = |_, _| 0.0;
    let raised = |x: f64, z: f64| {
        if x.abs() < 2.5 && z.abs() < 2.5 {
            BLOCK_HEIGHT
        } else {
            0.0
        }
    };

    let meshes = Value::List(vec![
        mesh(
            "ground",
            "Triangles",
            point.clone(),
            heightfield(32, GROUND, flat),
        ),
        mesh(
            "block",
            "Triangles",
            point.clone(),
            heightfield(32, GROUND, raised),
        ),
    ]);

    // The lights stand on the **far** side of the block from the camera, which is the whole
    // trick of making a shadow visible at all: a light beside the camera throws the block's
    // shadow directly away from the viewer, where the block itself hides it, and the two
    // frames come out identical for a reason that has nothing to do with the mechanism.
    //
    // Each light names a **radius** wide enough to reach the ground, because a light's radius
    // is what the cluster assignment culls it by and a light stood far enough off to throw a
    // visible shadow is further from the scene than one box radius of it. A light that
    // reaches nothing is lit by nothing, and a scene of ambient alone is a frame that
    // renders and casts no shadow — which is the same symptom as a broken page and the
    // opposite diagnosis. (A light that names **no** radius is a global light and is never
    // culled at all, which is the other way out and is what `kit::light` builds.)
    //
    // The **flux** is derived rather than written, and that is the whole of what was wrong with
    // this scene. It used to state `3.0` and `2.4`, which were brightness multipliers of a
    // shading model that divided by `d^2` and by nothing else. That model no longer exists —
    // `d36b4cb` put the sphere's `1 / (4 pi d^2)` and the Lambert lobe's `1 / pi` into the
    // shading, and re-derived every *gallery* scene's light against them — and a document that
    // states the old number against the new model is dark by the ratio between the two. At the
    // forty-odd units these lamps stand from the ground that ratio is 24687, so the brightest
    // pixel in the frame came out at 0.004 of 255: every one of the 1048576 pixels rounded to
    // zero and both of these tests spent their time comparing two black frames, which is why the
    // failure read as a broken page rather than as a scene with no light in it.
    let mut lights_list = Value::List(vec![light_with_radius(
        "key",
        KEY,
        [1.0, 0.96, 0.9],
        flux(KEY),
        20.0,
        70.0,
    )]);
    // A second light, on the far side too and to the other hand, so that its light lands on
    // ground the first one also reaches and the pair of frames differ wherever it acts.
    if lights > 1 {
        let mut extra = match lights_list {
            Value::List(list) => list,
            _ => Vec::new(),
        };
        extra.push(light_with_radius(
            "second",
            SECOND,
            [0.85, 0.9, 1.0],
            flux(SECOND),
            20.0,
            70.0,
        ));
        lights_list = Value::List(extra);
    }

    let mut buckets = vec![
        (
            Stage::Depthing,
            vec![
                instance(
                    "ground_depth",
                    "ground",
                    [0.0; 3],
                    [0.0; 3],
                    [1.0; 3],
                    "ground_depth",
                ),
                instance(
                    "block_depth",
                    "block",
                    [0.0; 3],
                    [0.0; 3],
                    [1.0; 3],
                    "block_depth",
                ),
            ],
        ),
        (
            Stage::Lighting,
            vec![
                instance(
                    "ground_lit",
                    "ground",
                    [0.0; 3],
                    [0.0; 3],
                    [1.0; 3],
                    "ground_lit",
                ),
                instance(
                    "block_lit",
                    "block",
                    [0.0; 3],
                    [0.0; 3],
                    [1.0; 3],
                    "block_lit",
                ),
            ],
        ),
    ];
    let mut casters = vec![instance(
        "ground_caster",
        "ground",
        [0.0; 3],
        [0.0; 3],
        [1.0; 3],
        "ground_caster",
    )];
    if occluder {
        casters.push(instance(
            "block_caster",
            "block",
            [0.0; 3],
            [0.0; 3],
            [1.0; 3],
            "block_caster",
        ));
    }
    buckets.insert(0, (Stage::Shadowing, casters));

    Value::structure([
        (
            "name".to_owned(),
            Value::String("clustered_shadow".to_owned()),
        ),
        (
            "frame".to_owned(),
            Value::structure([("shadow_pages".to_owned(), Value::Int(pages))]),
        ),
        (
            "cameras".to_owned(),
            Value::List(vec![camera_placed(
                [0.0, 9.0, 17.0],
                [0.0, 0.0, 0.0],
                std::f64::consts::FRAC_PI_3,
                0.5,
                60.0,
            )]),
        ),
        ("lights".to_owned(), lights_list),
        (
            "shaders".to_owned(),
            Value::List(vec![
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
                    &lighting(
                        &inputs,
                        "",
                        "",
                        document::shaders::Projection::Latlong,
                        &custom,
                    ),
                    scene::standard::light_pass_schema(),
                    point.clone(),
                    gallery_instance_schema(),
                    custom.clone(),
                ),
                post_shader("", "", None),
            ]),
        ),
        ("meshes".to_owned(), meshes),
        (
            "mesh_materials".to_owned(),
            // A material's label is unique across the whole scene, so a mesh drawn in three
            // stages carries three of them rather than one. The row is the same each time
            // here: what differs between the stages is the shader, not the surface.
            Value::List(vec![
                mesh_material("ground_caster", "shadowing", lit.clone()),
                mesh_material("ground_depth", "depthing", lit.clone()),
                mesh_material("ground_lit", "lighting", lit),
                mesh_material("block_caster", "shadowing", stone.clone()),
                mesh_material("block_depth", "depthing", stone.clone()),
                mesh_material("block_lit", "lighting", stone),
            ]),
        ),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        ("instances".to_owned(), instances(buckets)),
    ])
}

/// Returns a renderer, or nothing when the machine has no adapter.
fn renderer() -> Option<WgpuRenderer> {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    runtime.block_on(async {
        if !WgpuRenderer::probe().await {
            return None;
        }
        WgpuRenderer::new().await.ok()
    })
}

/// Compiles a scene the way a host would.
fn compile(scene: &Value, limits: Option<scene::DeviceLimits>) -> scene::CompileOutput {
    let mut registry = Registry::new();
    scene::register_standard_scene_schema(&mut registry);
    registry.put_schema(
        SchemaIdentifier::new(graphics::GRAPHICS_SCHEMA),
        graphics::graphics_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(graphics::INSTANCE_SCHEMA),
        graphics::instance_schema(),
    );
    let source = Definition::new(
        DefinitionIdentifier::new("clustered.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .map_err(|error| format!("the scene does not satisfy the scene schema: {error}"))
        .expect("the scene validates");
    StandardScene::new(
        DefinitionIdentifier::new("clustered.graphics"),
        DefinitionIdentifier::new("clustered.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
        limits,
    })
    .expect("the scene compiles")
}

/// Compiles a scene the way a host would and renders it.
fn render(renderer: &WgpuRenderer, scene: &Value) -> Result<Frame, String> {
    // The limits are the renderer's own, because this scene is compiled against the machine
    // that is about to draw it rather than against a floor.
    let compiled = compile(scene, renderer.limits());
    renderer
        .render(&compiled.graphics.value, &compiled.instance.value)
        .map_err(|error| format!("the frame does not render: {error}"))
}
