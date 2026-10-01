//! Image based lighting, read back as pixels on a real gpu.
//!
//! # What this proves that a document level test cannot
//!
//! A compiler level test can prove the environment's map reached the instance as an entry and
//! that the pass bound it. It cannot say the shader read the **right texel for the right
//! direction**, because every way of being wrong here produces a document that is entirely
//! correct: a fold that reflects instead of subtracting puts the horizon in the wrong row, a
//! texel coordinate left unclamped reads one row past the end of the image at exactly the seam,
//! and a level that is the roughness without a clamp reads a level the map does not have. Each
//! of those is a frame that is merely wrong, and a wrong frame is still a frame.
//!
//! So the test renders and reads pixels, and it uses the one technique that needs no arithmetic
//! about where anything ought to land: **a surface facing up and a surface facing down**.
//!
//! # Why two surfaces and not one
//!
//! A single surface cannot tell a sampled environment from the two colour hemisphere that stood
//! in for one. Both are brighter towards the sky, so "the top is brighter than the bottom" holds
//! for either, and an assertion that stops there would pass on a shader that read nothing at
//! all. What the stand-in cannot do is move the two surfaces in **opposite directions** when the
//! environment is taken away, because it is a replacement and not an addition: the map's top row
//! is far brighter than the stand-in's sky and its bottom row is darker than the stand-in's
//! ground, so removing the environment drops the upward face and lifts the downward one. That is
//! the check that the image is being sampled by direction rather than merely added, and it is why
//! the first test here renders four frames rather than one.

#![cfg(feature = "wgpu")]

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier, Value};
use gallery::geometry::plane_facing;
use gallery::kit::{
    camera_placed, environment_light, gallery_custom_schema, gallery_instance_schema, image,
    instance, instances, mesh, mesh_material, post_processes, screen_material, shaded_point_schema,
    shader, surface,
};
use gallery::shaders::{depthing, lighting, octahedral_pixels, post_shader, shadowing};
use scene::standard::Stage;
use scene::{CompileContext, Compiler, STANDARD_SCENE_SCHEMA, StandardScene};
use whiting_device::{Frame, Renderer, WgpuRenderer};

/// A note printed when there is nothing to test on.
const NO_ADAPTER: &str = "skipped: no gpu adapter is present";

/// How far the two surfaces reach either side of the origin.
const EXTENT: f64 = 12.0;

/// How far above the ground the down-facing surface stands.
///
/// The camera goes between them, which is the only place from which both are visible at once.
const CEILING: f64 = 8.0;

/// The size of the environment map, in texels a side.
///
/// Sixteen is large enough that the top row and the bottom row are not neighbours, so a fold
/// that put them next to each other would be visible rather than subtle, and small enough that
/// the chain is five levels and the coarsest of them is one texel, which is what makes the
/// roughness test a real difference between two reads rather than two reads of one texel.
const MAP: i64 = 16;

/// The roughness a surface is drawn with to read the map's sharpest level.
///
/// The shader clamps a material's roughness into `0.04` to `1.0` and scales it by the chain, so
/// anything at the bottom of that range reads level zero. A scene testing the *direction* of a
/// sample wants level zero and a scene testing roughness wants the other end, so the two tests
/// pass different numbers rather than one number trying to be both.
const SHARP: f64 = 0.04;

/// The roughness that reads the coarsest level, where the map is a single averaged texel.
const BLURRED: f64 = 1.0;

/// How much of the difference between the two directions has to survive to count as a lookup.
///
/// The stand-in's two ends differ by about twelve of 765, and the map's by most of the range, so
/// a hundred is a floor that excludes the stand-in by a wide margin rather than a number tuned
/// to the frame.
const SEPARATION: u32 = 100;

/// The largest gap between the stand-in's two ends that still counts as the stand-in.
///
/// The stand-in's two colours are a twelfth and a fiftieth of one, and a surface multiplies
/// them by its own tint, so the gap it produces is around forty of seven hundred and sixty five.
/// The number is a floor above that rather than a measurement of it, so a stand-in that drifted
/// a little would still pass and a sampled map still could not.
const STAND_IN_GAP: u32 = 60;

/// Where the ground is probed, as a fraction down the frame.
const GROUND: f64 = 0.88;

/// Where the ceiling is probed, as a fraction down the frame.
const CEILING_PROBE: f64 = 0.12;

/// An up-facing and a down-facing surface are lit by different parts of the map, and removing
/// the environment moves them in opposite directions.
#[test]
fn the_two_directions_differ_and_the_environment_is_what_makes_them() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    let lit = render(&renderer, &scene(true, SHARP)).expect("the lit scene renders");
    let unlit =
        render(&renderer, &scene(false, SHARP)).expect("the same scene without one renders");

    // The bottom of the frame is the ground, which faces up and so faces the map's brightest
    // direction; the top of the frame is the ceiling, which faces down and faces its darkest.
    //
    // The ceiling is the interesting probe, and it is the one that exercises the seam. A normal
    // of exactly `(0, -1, 0)` maps to a square coordinate of exactly `1.0`, which multiplied by
    // the map's size is `16` — the first texel **past** the end. The clamp in the shader is what
    // turns that into the last texel, and the last texel is black because the map's bottom row
    // is. A shader without the clamp reads out of bounds here and returns whatever the backend
    // had, which is the whole reason this probe is the ceiling and not the ground.
    let up = probe(&lit, GROUND);
    let down = probe(&lit, CEILING_PROBE);
    assert!(
        up > down + SEPARATION,
        "a surface facing the top of the map is lit by its brightest texels and one facing the \
         bottom by its darkest, and this frame has {up} for the upward face against {down} for \
         the downward one"
    );

    // The half no single frame can give. The stand-in is a hemisphere that is brighter from
    // above too, so the ordering above is on its own consistent with the environment never
    // having been read; what the stand-in cannot do is move the two surfaces in opposite
    // directions when the environment is taken away.
    let (up_without, down_without) = (probe(&unlit, GROUND), probe(&unlit, CEILING_PROBE));
    assert!(
        up_without < up,
        "removing the environment takes the upward face from {up} down to {up_without}, because \
         the two colour stand-in is far dimmer at the top than the map's top row is"
    );
    assert!(
        down_without > down,
        "and it takes the downward face from {down} **up** to {down_without}, because the \
         stand-in is not black underneath while the map's bottom row is. Two surfaces moving in \
         opposite directions is what a lookup by direction looks like, and a constant or a \
         hemisphere cannot do it"
    );
}

/// A rough surface reads a coarser level of the map, and so a different value than a sharp one.
#[test]
fn a_rougher_surface_reads_a_coarser_level_of_the_map() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    let sharp = render(&renderer, &scene(true, SHARP)).expect("the sharp scene renders");
    let blurred = render(&renderer, &scene(true, BLURRED)).expect("the rough scene renders");

    // The ground, which is the strongest place to look for this. At the coarsest level the
    // whole map has been averaged into a **single** texel, so every direction reads the same
    // number and the ground comes back at the map's mean rather than at its top row. That is a
    // difference of most of the range on a surface that is in the frame either way, where a
    // probe at the horizon would have been a probe of the empty space between the two planes.
    let (sharp_ground, blurred_ground) = (probe(&sharp, GROUND), probe(&blurred, GROUND));
    assert!(
        sharp_ground > blurred_ground + SEPARATION,
        "a material's roughness is the level the map is read at. At the sharpest level the ground \
         faces the map's brightest row and reads {sharp_ground}; at the coarsest the whole map is \
         one averaged texel and it reads {blurred_ground}. A frame where the two agree is a \
         shader reading level zero whatever the roughness is"
    );
}

/// A scene naming no environment light compiles, renders, and renders the stand-in.
#[test]
fn a_scene_naming_no_environment_still_renders_the_stand_in() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // Rendered rather than borrowed from the first test, so this one fails on its own if a
    // scene with no environment light stops compiling — which is the failure a reader would look
    // for first, and the one a shared frame would hide.
    let frame = render(&renderer, &scene(false, SHARP))
        .expect("a scene with no environment light compiles");
    let (up, down) = (probe(&frame, GROUND), probe(&frame, CEILING_PROBE));
    assert!(
        up > down,
        "the stand-in is a hemisphere that is brighter from above, so the upward face at {up} is \
         still above the downward one at {down}"
    );
    // And the two are close together, which is the property that tells the stand-in from a map:
    // one dark colour across a whole hemisphere cannot be most of the range at the same time as
    // white at the top and black at the bottom.
    assert!(
        up - down < STAND_IN_GAP,
        "the stand-in's two ends are {up} and {down}, a gap of {} where a sampled map is most of \
         the range. A frame this far apart is a map being read in a scene that named none",
        up - down
    );
}

/// Returns the mean luminance of a patch of the frame at a fraction down it.
///
/// The sum of three eight bit channels, averaged over the patch, as an integer: a test about
/// whether one number is above another does not need a colour space, and an integer makes a
/// failure print as a number a reader can hold in their head.
fn probe(frame: &Frame, down: f64) -> u32 {
    let width = frame.width as i64;
    let height = frame.height as i64;
    // Half way across, and at the given fraction down. Nine pixels square, which is wide enough
    // that a triangle edge crossing the patch costs some of it rather than all of it, and small
    // enough to stay on one surface.
    let centre_x = width / 2;
    let centre_y = (down * height as f64).round() as i64;
    let mut total = 0u32;
    let mut count = 0u32;
    for offset in -4i64..=4 {
        let x = (centre_x + offset).clamp(0, width - 1) as usize;
        let y = (centre_y + offset).clamp(0, height - 1) as usize;
        let at = (y * frame.width as usize + x) * 4;
        total += u32::from(frame.rgba[at])
            + u32::from(frame.rgba[at + 1])
            + u32::from(frame.rgba[at + 2]);
        count += 1;
    }
    total / count.max(1)
}

/// Builds the scene: a ground, a ceiling above it, and an environment light if one is asked for.
///
/// `environment` says whether the scene names an `Environment` light and `roughness` is the
/// material's own. Everything else is the same in every frame these tests render, so the only
/// thing that differs between them is the thing being tested.
fn scene(environment: bool, roughness: f64) -> Value {
    let point = shaded_point_schema();
    let custom = gallery_custom_schema();
    let inputs = gallery::attributes::gallery_vertex_inputs(&point);
    let stone = surface([0.9; 3], 0.0, roughness, [0.0; 3]);

    let meshes = Value::List(vec![
        mesh(
            "ground",
            "Triangles",
            point.clone(),
            plane_facing(EXTENT, 0.0, true),
        ),
        mesh(
            "ceiling",
            "Triangles",
            point,
            plane_facing(EXTENT, CEILING, false),
        ),
    ]);

    // The environment is the **only** light in the scene. A clustered loop over an empty list is
    // a loop that does not run, so every pixel in the frame is the ambient term and nothing
    // else, which is what makes the difference between the two surfaces attributable to the map
    // rather than to a lamp. A scene with no environment light is a scene with *no* lights.
    let lights = if environment {
        Value::List(vec![environment_light("sky", "sky", [1.0; 3], 1.0)])
    } else {
        // A light of **no brightness**, and not no light at all. The clustered loop reads its
        // record, so a scene with an empty light list has a record buffer of nothing to read and
        // the draw is refused for a buffer that is four bytes where the shader wants a whole
        // record; a light of zero intensity gives the loop a record that contributes exactly
        // nothing, which is what these scenes want — every pixel in the frame is the ambient
        // term — without a scene that cannot be drawn at all.
        //
        // That it is needed is recorded rather than fixed: a scene naming no light of any kind
        // does not render, which is a pre-existing edge and not this stage's business. It is
        // written down here so the next reader knows the light is a stand-in and not a test of
        // anything.
        Value::List(vec![dark_light()])
    };

    Value::structure([
        (
            "name".to_owned(),
            Value::String("environment_map".to_owned()),
        ),
        (
            "images".to_owned(),
            Value::List(vec![image("sky", MAP, MAP, octahedral_pixels(MAP))]),
        ),
        (
            "cameras".to_owned(),
            Value::List(vec![camera_placed(
                [0.0, CEILING * 0.5, 18.0],
                [0.0, CEILING * 0.5, 0.0],
                std::f64::consts::FRAC_PI_2 - 0.15,
                0.5,
                60.0,
            )]),
        ),
        ("lights".to_owned(), lights),
        (
            "shaders".to_owned(),
            Value::List(vec![
                shader(
                    Stage::Shadowing,
                    "shadowing",
                    &shadowing(&inputs),
                    scene::standard::shadow_pass_schema(),
                    shaded_point_schema(),
                    gallery_instance_schema(),
                    custom.clone(),
                ),
                shader(
                    Stage::Depthing,
                    "depthing",
                    &depthing(&inputs),
                    scene::standard::depth_pass_schema(),
                    shaded_point_schema(),
                    gallery_instance_schema(),
                    custom.clone(),
                ),
                shader(
                    Stage::Lighting,
                    "lighting",
                    &lighting(&inputs, "", ""),
                    scene::standard::light_pass_schema(),
                    shaded_point_schema(),
                    gallery_instance_schema(),
                    custom.clone(),
                ),
                post_shader("", ""),
            ]),
        ),
        ("meshes".to_owned(), meshes),
        (
            "mesh_materials".to_owned(),
            Value::List(vec![
                mesh_material("ground_depth", "depthing", stone.clone()),
                mesh_material("ceiling_depth", "depthing", stone.clone()),
                mesh_material("ground_lit", "lighting", stone.clone()),
                mesh_material("ceiling_lit", "lighting", stone),
            ]),
        ),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        (
            "instances".to_owned(),
            instances(vec![
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
                            "ceiling_depth",
                            "ceiling",
                            [0.0; 3],
                            [0.0; 3],
                            [1.0; 3],
                            "ceiling_depth",
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
                            "ceiling_lit",
                            "ceiling",
                            [0.0; 3],
                            [0.0; 3],
                            [1.0; 3],
                            "ceiling_lit",
                        ),
                    ],
                ),
            ]),
        ),
    ])
}

/// Builds a point light that reaches everything and lights nothing.
///
/// # Why the intensity is zero and not the light absent
///
/// The clustered loop reads a record per light, so an empty light list is a record buffer of
/// nothing and a draw the backend refuses for a buffer four bytes wide where the shader wants a
/// whole ninety six byte record. A light of zero intensity is a record the loop can read and
/// adds nothing, which is what these scenes want: every pixel in the frame is the ambient term
/// and nothing else, so the difference between the two surfaces is the environment's.
///
/// The radius is the whole scene, because a light that reaches no cluster is a light in no
/// list, and a light in no list is not a record the loop reads at all — which would be the same
/// empty buffer by another route.
fn dark_light() -> Value {
    let mut light = gallery::kit::light("unlit", [0.0, 4.0, 0.0], [1.0; 3], 0.0, 1.0);
    if let Value::Enum { ref mut fields, .. } = light {
        fields.insert("radius".to_owned(), Value::Float(100.0));
    }
    light
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

/// Compiles a scene the way a host would and renders it.
fn render(renderer: &WgpuRenderer, scene: &Value) -> Result<Frame, String> {
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
        DefinitionIdentifier::new("environment.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .map_err(|error| format!("the scene does not satisfy the scene schema: {error}"))
        .expect("the scene validates");
    let compiled = StandardScene::new(
        DefinitionIdentifier::new("environment.graphics"),
        DefinitionIdentifier::new("environment.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
    })
    .expect("the scene compiles");
    renderer
        .render(&compiled.graphics.value, &compiled.instance.value)
        .map_err(|error| format!("the frame does not render: {error}"))
}
