//! A pass with more than one colour attachment, and the debug channels, on a real gpu.
//!
//! # What these prove
//!
//! A rasterize pass can write **several** colour targets in one draw, each with its own format,
//! and a shader addresses them by `@location(i)`. Three things are checked, and each is
//! something that would be a silently wrong image rather than an error:
//!
//! - **both attachments come back**, each under the name the document declared it as and in the
//!   format it was drawn in — the shape a G-buffer needs, and a float normal beside an eight bit
//!   identifier is exactly the pair that could not be drawn before;
//! - **`@location(0)` and `@location(1)` do not swap.** The values are chosen so that a
//!   pipeline whose targets were built in the wrong order would still produce *two* images and
//!   still produce valid pixels — it would just be showing the caller the other one. That is
//!   the failure this file exists for, and it is invisible to a check that only counts images;
//! - **a pass with one attachment answers exactly as it did before.** The renderer used to take
//!   the first colour attachment and build a one element target list, so this is the case a
//!   regression would land in and the one nothing else here covers.
//!
//! The second half of the file is the **debug channels**, which are the reason a document
//! declares its outputs at all: a channel's image comes back by name because the emitter said
//! which target is which, and a channel whose image came back is still only half checked. What
//! these hold is that the numbers in it are the numbers the picture's own geometry implies —
//! that `Position` reports where the surface *is* and not merely a float in range, that `Depth`
//! reports the depth that pixel would be tested against, and that the two agree about which
//! pixels the scene drew on. The scene is built here rather than taken from the gallery for
//! exactly that reason: a dune field has geometry nobody can write down, so the only thing a
//! test could say about it is that its positions are finite, and a finite number is not a
//! measurement.
//!
//! Every test needs an adapter, and one that is missing is a **skip with a printed note** — the
//! same rule `texture_channel_render.rs` follows, for the same reason: a silent pass would make
//! this file worthless on a machine with no gpu.

use std::sync::OnceLock;

use definition::{
    Definition, DefinitionIdentifier, Path, Registry, Schema, SchemaIdentifier, SchemaKind, Value,
};
use gallery::geometry::heightfield;
// Aliased rather than imported plainly: the document builders below already have helpers called
// `instance` and `instances`, and two meanings in one file is a name that has to be read at
// every use rather than at its definition.
use device::{RenderedImage, Renderer};
use gallery::kit::{
    camera_placed, directional, instance as plane_instance, instances as stage_instances, mesh,
    mesh_material, shaded_point_schema, surface,
};
use gallery::shaders::mesh_shaders;
use scene::standard::{Bounds, Camera, Projection, Stage};
use scene::{CompileContext, Compiler, DebugChannel, STANDARD_SCENE_SCHEMA, StandardScene};
use whiting_device_rasterize::WgpuRenderer;

/// The size of the frame.
const FRAME: u32 = 64;

/// The value the first attachment carries, deliberately a value eight bits can hold exactly.
///
/// 0.25, 0.5 and 0.75 are all exactly representable in `rgba8unorm`, so the eight bit readback
/// is an exact check rather than a tolerance, and the numbers are far from the second
/// attachment's so a swap cannot produce anything that looks right.
const FIRST: [f32; 4] = [0.25, 0.5, 0.75, 1.0];

/// The value the second attachment carries, deliberately one eight bits **cannot** hold.
///
/// −2.75 in an `rgba8unorm` target would clamp to zero, so an attachment whose values came back
/// as `0.0` in that channel is a value that was never written where it was read from.
const SECOND: [f32; 4] = [1.5, -2.75, 3.25, 1.0];

/// A note printed when there is nothing to test on.
const NO_ADAPTER: &str = "skipped: no gpu adapter is present";

#[test]
fn both_colour_attachments_come_back_in_the_format_each_was_drawn_in() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    let images = renderer
        .render_outputs(&graphics(true), &instance(true))
        .expect("a pass with two colour attachments renders");

    let names: Vec<&str> = images.iter().map(|image| image.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["first", "present", "second"],
        "the frame and both attachments are returned, under the names the document declared \
         them as. They come back in **name** order rather than in the order the document listed \
         them, because `outputs` is a dictionary: a caller looks an image up by name, which is \
         what `RenderedFrame::present` does"
    );

    let first = images
        .iter()
        .find(|image| image.name == "first")
        .expect("the first attachment is returned");
    assert_eq!(
        first.format, "rgba8unorm",
        "the first attachment is read in the format it was drawn in, and not as the format of \
         whichever target happened to come back first"
    );
    let second = images
        .iter()
        .find(|image| image.name == "second")
        .expect("the second attachment is returned");
    assert_eq!(
        second.format, "rgba16float",
        "the second attachment is a float image beside an eight bit one, and each keeps its own \
         format through the readback"
    );
}

#[test]
fn the_two_attachments_hold_their_own_values_and_not_each_others() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    let images = renderer
        .render_outputs(&graphics(true), &instance(true))
        .expect("a pass with two colour attachments renders");
    let read = |name: &str| {
        images
            .iter()
            .find(|image| image.name == name)
            .unwrap_or_else(|| panic!("`{name}` is returned"))
            .texel(FRAME / 2, FRAME / 2)
            .unwrap_or_else(|| panic!("`{name}` holds a texel at the centre"))
    };

    // Every texel is the same value, so the centre is as good a place as any.
    let first = read("first");
    let second = read("second");
    for (axis, expected) in FIRST.iter().enumerate() {
        assert!(
            (first[axis] - expected).abs() < 0.01,
            "`@location(0)` holds the first value's axis {axis}: {:?} is not {expected}",
            first
        );
    }
    for (axis, expected) in SECOND.iter().enumerate() {
        assert!(
            (second[axis] - expected).abs() < 0.01,
            "`@location(1)` holds the second value's axis {axis}: {:?} is not {expected}. A value \
             that came back negative is the one only a float target can hold, so this is the \
             attachment that was drawn as a float — and a swapped pair would fail here rather \
             than merely look odd",
            second
        );
    }
}

#[test]
fn a_pass_with_one_attachment_is_answered_as_it_was_before() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // The same shader, the same document shape, one target. The renderer used to take the first
    // colour attachment and build exactly one `ColorTargetState`; this is the case that change
    // had to leave alone, and it is the one every scene in the workspace depends on.
    let frame = renderer
        .render(&graphics(false), &instance(false))
        .expect("a pass with one colour attachment renders");
    assert_eq!((frame.width, frame.height), (FRAME, FRAME));
    // 0.5 over 255 is 127.5 and the backend rounds it down, so the expectation is 127 rather than
    // 128. Written out rather than computed, because a test that recomputes the rounding rule it
    // is checking would agree with the backend by construction.
    assert_eq!(
        frame.rgba[0..4],
        [64, 127, 191, 255],
        "the frame is the first value, quantised into eight bits: {:?}",
        &frame.rgba[0..4]
    );
}

// ---------------------------------------------------------------------------
// The debug channels, on the same adapter.
//
// Everything above builds a graphics document by hand and asks what the device does with it.
// Everything below compiles a **scene** and asks what the channels say about what it drew, which
// is a different question: the document is the compiler's, and a number that comes back wrong
// is wrong in a way no error is raised for.
// ---------------------------------------------------------------------------

#[test]
fn a_covered_pixel_reports_the_surface_the_picture_shows() {
    let Some(channels) = channels() else {
        println!("{NO_ADAPTER}");
        return;
    };
    let mut checked = 0usize;
    for y in 0..FRAME {
        for x in 0..FRAME {
            let reported = covered(&channels.position, x, y);
            let Some(reported) = reported else {
                continue;
            };
            let expected = meets_plane(&channels.camera, ray_through(&channels.camera, x, y))
                .unwrap_or_else(|| {
                    panic!(
                        "({x}, {y}) is marked as a surface, so the plane covers it and the ray \
                        through it must meet the plane"
                    )
                });
            // The plane is the only thing the scene draws and it is at `PLANE`, so a reported
            // position is either on it or is the position of something the picture does not
            // show. This is the axis that needs no arithmetic at all: a position recovered from
            // the wrong rows of the matrix, from a mirrored pixel, or from a depth scaled as if
            // it were a distance all land somewhere else on this one number.
            assert!(
                (reported[1] as f64 - PLANE).abs() < 0.01,
                "({x}, {y}) reports {reported:?}, whose height is not the plane's {PLANE}. The \
                 scene draws one flat plane and this is the coordinate that says so"
            );
            // And the whole point, not just its height: the exact place on that plane the
            // camera's own ray through this pixel reaches. The tolerance is set by the half
            // float the image is drawn in — a value of forty units has a step of four
            // hundredths — and is four hundredths of one percent of the scene's own extent.
            let tolerance = 0.01 + expected.iter().map(|v| v.abs()).fold(0.0, f64::max) * 0.001;
            for (axis, want) in expected.iter().enumerate() {
                assert!(
                    (reported[axis] as f64 - want).abs() <= tolerance,
                    "({x}, {y}) axis {axis} is {}, and the camera's ray through that pixel meets \
                     the plane at {want} (tolerance {tolerance}): {reported:?}",
                    reported[axis]
                );
            }
            checked += 1;
        }
    }
    assert!(
        checked > FRAME as usize,
        "the frame is {FRAME} square and only {checked} of its pixels hold a surface, so this \
         assertion is about a scene that drew almost nothing"
    );
}

#[test]
fn a_pixel_the_scene_drew_nothing_on_reports_the_marker() {
    let Some(channels) = channels() else {
        println!("{NO_ADAPTER}");
        return;
    };
    let mut checked = 0usize;
    for y in 0..FRAME {
        for x in 0..FRAME {
            if covered(&channels.position, x, y).is_some() {
                continue;
            }
            let position = texel(&channels.position, x, y);
            let depth = texel(&channels.depth, x, y);
            // The marker is the alpha and it is the same rule in both images, so what is
            // checked here is that neither one reports a surface and neither one reports a
            // *value* in place of the marker. A position of zero is a perfectly good place to
            // be, so only the alpha can carry this; a channel that reported one in the colour
            // and left the alpha at one would put the sky a whole scene away.
            assert_eq!(
                (position[3], depth[3]),
                (0.0, 0.0),
                "({x}, {y}) is a pixel the scene drew nothing on and both channels mark it: \
                 position {position:?}, depth {depth:?}"
            );
            assert_eq!(
                position[0..3],
                [0.0, 0.0, 0.0],
                "({x}, {y}) has no surface, so the position channel has no position to report \
                 and says so with zeroes rather than inventing one"
            );
            // The depth channel's colour is the buffer **verbatim**, and a cleared depth buffer
            // holds one. That is the far plane, which is the truth about an empty pixel, and
            // the reason the two channels do not agree on what zero means.
            assert_eq!(
                depth[0..3],
                [1.0, 1.0, 1.0],
                "({x}, {y}) has no surface, so the depth channel reports what the buffer holds \
                 there, which is the cleared far plane: {depth:?}"
            );
            checked += 1;
        }
    }
    assert!(
        checked > FRAME as usize,
        "the frame is {FRAME} square and only {checked} of its pixels are empty, so the scene's \
         far edge is not in the picture and this assertion is about a frame with no sky in it"
    );
}

#[test]
fn the_depth_channel_reports_the_depth_that_pixel_would_be_tested_against() {
    let Some(channels) = channels() else {
        println!("{NO_ADAPTER}");
        return;
    };
    let mut checked = 0usize;
    for y in 0..FRAME {
        for x in 0..FRAME {
            if covered(&channels.position, x, y).is_none() {
                continue;
            }
            let reported = texel(&channels.depth, x, y);
            // The depth a fragment at this pixel was tested against is the one the camera gives
            // the point the ray through the pixel reaches, so the expectation is built from the
            // plane and the camera and never from the image: a depth channel that reported the
            // position, or the position's distance from the eye, or a range the camera was never
            // given, fails here rather than looking like a depth.
            let point = meets_plane(&channels.camera, ray_through(&channels.camera, x, y))
                .expect("a marked pixel is on the plane");
            let along = dot(sub(point, channels.camera.eye), channels.camera.forward);
            let expected = channels.camera.depth_at(along);
            assert!(
                (reported[0] as f64 - expected).abs() < 0.005,
                "({x}, {y}) holds depth {}, and the camera gives the plane at {along} units along \
                 its own axis a depth of {expected}: {reported:?}",
                reported[0]
            );
            assert_eq!(
                reported[0..3],
                [reported[0], reported[0], reported[0]],
                "({x}, {y}) holds one value in all three colours, because the depth buffer is one \
                 number rather than three"
            );
            checked += 1;
        }
    }
    assert!(
        checked > FRAME as usize,
        "only {checked} pixels of a {FRAME} square frame hold a surface, so there is nothing here \
         to be right about"
    );
}

#[test]
fn the_two_channels_agree_about_which_pixels_the_scene_drew() {
    let Some(channels) = channels() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // The shared contract, and the one most worth pinning: the two images are two readings of
    // one depth buffer, so a pixel is either reported by both or by neither. They agree about
    // coverage **and** about the number — the depth the camera gives the position one channel
    // reports is the depth the other one does — which is what makes a caller able to read a
    // position and a depth beside each other without having to trust either alone.
    let mut covered_texels = 0usize;
    let mut empty_texels = 0usize;
    for y in 0..FRAME {
        for x in 0..FRAME {
            let position = texel(&channels.position, x, y);
            let depth = texel(&channels.depth, x, y);
            assert_eq!(
                position[3] > 0.5,
                depth[3] > 0.5,
                "({x}, {y}) is a surface in one of the two images and not in the other: position \
                 {position:?}, depth {depth:?}. They are reconstructed from one depth buffer, so \
                 there is nothing one of them can see that the other cannot"
            );
            if position[3] <= 0.5 {
                empty_texels += 1;
                continue;
            }
            covered_texels += 1;
            let along = dot(
                sub(
                    [position[0] as f64, position[1] as f64, position[2] as f64],
                    channels.camera.eye,
                ),
                channels.camera.forward,
            );
            let expected = channels.camera.depth_at(along);
            assert!(
                (depth[0] as f64 - expected).abs() < 0.005,
                "({x}, {y}) reports the position {position:?} and the depth {}, and those two are \
                 one measurement: the camera gives that point a depth of {expected}. Reading the \
                 two files as separate facts is what a caller is invited to do, so this is the \
                 check that they are not",
                depth[0]
            );
        }
    }
    assert!(
        covered_texels > 0 && empty_texels > 0,
        "the frame holds {covered_texels} covered and {empty_texels} empty pixels, so it does not \
         have both kinds in it and the agreement above was decided by one of them alone"
    );
}

// ---------------------------------------------------------------------------
// The scene the channel tests read, and what is read out of it.
// ---------------------------------------------------------------------------

/// Where the test camera stands, and what it looks at.
///
/// # Invariants
///
/// The target is the **origin**, which is a point of the plane, so the middle of the frame is
/// the place the camera was aimed at. That is what lets a test write an expected answer down
/// instead of working one out: the picture's centre is the origin, and a channel reporting
/// anything else is wrong by exactly what it reports.
const EYE: [f64; 3] = [0.0, 6.0, 20.0];
const TARGET: [f64; 3] = [0.0, 0.0, 0.0];

/// The lens the camera looks through, and the two planes it holds.
///
/// The two planes are what make the eye the one written above rather than one pulled back until
/// the plane fits the lens: a camera that writes both keeps its own distance, and a test that
/// cannot say where the camera is cannot say what a pixel ought to report.
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3;
const NEAR: f64 = 1.0;
const FAR: f64 = 200.0;

/// The height the plane is drawn at, and how far it reaches either side of the origin.
///
/// # Invariants
///
/// The height is **zero**, so the expected position of every covered pixel is a number with a
/// closed form and the one coordinate a reported position can be checked against without any
/// camera arithmetic at all. The extent is finite and large enough to fill the lower frame,
/// which leaves the upper frame above the plane's far edge — a scene that covered the whole
/// frame would make every assertion about an empty pixel pass without a single empty pixel in
/// it.
const PLANE: f64 = 0.0;
const EXTENT: f64 = 40.0;

/// How many cells a side the plane is built from.
///
/// Eight is enough that the plane is not one triangle — a single quad's perspective-correct
/// depth and a subdivided plane's agree, and a test that could not tell them apart would be
/// testing the wrong thing — and small enough that the scene compiles in no time at all.
const DIVISIONS: usize = 8;

/// What one render of the test scene produced, and the camera it was drawn through.
struct Channels {
    position: RenderedImage,
    depth: RenderedImage,
    camera: Projection,
}

/// Holds the one render every channel test reads, or nothing when the machine has no adapter.
///
/// # Invariants
///
/// It is made **once**. The four tests run in parallel in one process, and each of them would
/// otherwise build its own adapter and compile its own copy of the same scene to read the same
/// two images; a test that each rebuilds the fixture is a test whose cost is paid four times
/// over for nothing.
static CHANNELS: OnceLock<Option<Channels>> = OnceLock::new();

/// Returns the one render every channel test reads, or nothing when there is no adapter.
fn channels() -> Option<&'static Channels> {
    // `get_or_init` borrows the static, so the borrow is for the life of the process and every
    // test reads the same two images rather than each of them paying for a render.
    CHANNELS.get_or_init(build_channels).as_ref()
}

/// Compiles the test scene, draws it through both channels, and reads the camera back out.
///
/// The camera is read from the same value the scene carries, and rebuilt through
/// [`Camera::read`] rather than taken out of the compiled document. That is deliberate: a test
/// that recovered the basis from the emitted uniform would agree with the shader by
/// construction whenever the emitter and the shader were wrong **together**, and a basis from
/// one camera with a matrix from another is the one thing `Camera::projection` names as the
/// failure worth catching.
fn build_channels() -> Option<Channels> {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    runtime.block_on(async {
        if !WgpuRenderer::probe().await {
            return None;
        }
        let renderer = WgpuRenderer::new().await.ok()?;

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
            DefinitionIdentifier::new("channels.source"),
            SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
            plane_scene(),
        );
        source.validate(&registry).unwrap_or_else(|error| {
            panic!("the test scene does not satisfy the scene schema: {error}")
        });
        let compiled = StandardScene::new(
            DefinitionIdentifier::new("channels.graphics"),
            DefinitionIdentifier::new("channels.instance"),
        )
        // Every channel, rather than the one a test happens to be about, so a change to one
        // pass is not invisible to a test that only reads the other's image.
        .with_debug(DebugChannel::ALL)
        .compile(&CompileContext {
            scene: &source,
            previous_graphics: None,
            previous_instance: None,
            registry: &registry,
            camera: 0,
            limits: renderer.limits(),
        })
        .unwrap_or_else(|error| panic!("the test scene does not compile: {error}"));
        let images = renderer
            .render_outputs(&compiled.graphics.value, &compiled.instance.value)
            .unwrap_or_else(|error| panic!("the test scene does not render: {error}"));
        let read = |channel: DebugChannel| {
            let name = channel.output();
            images
                .iter()
                .find(|image| image.name == name)
                .unwrap_or_else(|| panic!("`{name}` is returned, having been declared an output"))
                .clone()
        };
        let (position, depth) = (read(DebugChannel::Position), read(DebugChannel::Depth));
        assert_eq!(
            (position.width, position.height, depth.width, depth.height),
            (FRAME, FRAME, FRAME, FRAME),
            "both channels are the size of the frame the scene named, so a texel in one is the \
             same pixel as a texel in the other"
        );
        // The frame is square, and the camera is read against the aspect the scene declared, so
        // the ray this test builds is the ray the shader built.
        let camera = Camera::read(
            Some(&camera_value()),
            &Path::root(),
            &Bounds::around(TARGET, EXTENT),
            1.0,
        )
        .expect("the test camera reads")
        .projection();
        assert_eq!(
            camera.eye, EYE,
            "the camera kept the eye the scene wrote, because it wrote both depth planes. A test \
             that could not predict the eye could not say what a pixel ought to report"
        );
        Some(Channels {
            position,
            depth,
            camera,
        })
    })
}

/// Builds the scene the channel tests are made against: one flat plane and one placed camera.
///
/// # Invariants
///
/// **The plane is the only geometry, and it is flat.** That is what makes every expected
/// position a number rather than an opinion: a surface the picture shows is a surface at
/// [`PLANE`], and anything else in a position image is a position of something the frame does
/// not contain. A dune field has no such answer, which is why the fixture is built here.
///
/// **The plane is drawn in all three stages that want it.** A surface listed only under
/// `lighting` is in the picture and not in the depth buffer, and both channels cover the depth
/// buffer alone — so leaving the `depthing` entry out would make every image empty and every
/// assertion here vacuous rather than failing.
fn plane_scene() -> Value {
    let plane = || {
        plane_instance(
            "plane",
            "plane",
            [0.0; 3],
            [0.0; 3],
            [1.0; 3],
            "plane-depth",
        )
    };
    Value::structure([
        ("name".to_owned(), Value::String("channels".to_owned())),
        (
            "frame".to_owned(),
            Value::structure([
                ("width".to_owned(), Value::Int(FRAME as i64)),
                ("height".to_owned(), Value::Int(FRAME as i64)),
            ]),
        ),
        ("cameras".to_owned(), Value::List(vec![camera_value()])),
        (
            // One light, so the scene has something to draw. Its shadow is never read by these
            // tests; it is here because a scene with no light is refused rather than drawn.
            "lights".to_owned(),
            Value::List(vec![directional(
                "sun",
                [0.4, -1.0, 0.3],
                [1.0, 0.97, 0.92],
                3.0,
                2.0 * EXTENT,
            )]),
        ),
        ("shaders".to_owned(), Value::List(mesh_shaders("", ""))),
        (
            "meshes".to_owned(),
            Value::List(vec![mesh(
                "plane",
                "Triangles",
                shaded_point_schema(),
                heightfield(DIVISIONS, EXTENT, |_, _| PLANE),
            )]),
        ),
        (
            "mesh_materials".to_owned(),
            Value::List(vec![
                mesh_material(
                    "plane-shadowing",
                    "shadowing",
                    surface([0.5; 3], 0.0, 1.0, [0.0; 3]),
                ),
                mesh_material(
                    "plane-depth",
                    "depthing",
                    surface([0.5; 3], 0.0, 1.0, [0.0; 3]),
                ),
                mesh_material(
                    "plane-lighting",
                    "lighting",
                    surface([0.6, 0.58, 0.55], 0.0, 1.0, [0.0; 3]),
                ),
            ]),
        ),
        (
            "instances".to_owned(),
            stage_instances(vec![
                (
                    Stage::Shadowing,
                    vec![renamed(plane(), "shadowing", "plane-shadowing")],
                ),
                (Stage::Depthing, vec![plane()]),
                (
                    Stage::Lighting,
                    vec![renamed(plane(), "lighting", "plane-lighting")],
                ),
            ]),
        ),
    ])
}

/// Builds the camera the test scene is drawn through.
///
/// It is a function rather than a constant so that the scene and the expectation are built from
/// **one** value: a test that wrote the eye twice would be a test whose two copies could drift,
/// and a camera that moved while the test's arithmetic did not is a test that passes for the
/// wrong reason.
fn camera_value() -> Value {
    camera_placed(EYE, TARGET, FIELD_OF_VIEW, NEAR, FAR)
}

/// Returns one instance of the plane under a distinct label, drawing it with another material.
///
/// A label names one instance in the **whole** scene rather than one per pass, so the same
/// geometry drawn by three passes is three entries; this is the one that would otherwise be
/// written out three times.
fn renamed(mut entry: Value, label: &str, material: &str) -> Value {
    let Value::Struct(ref mut fields) = entry else {
        unreachable!("an instance is a structure value")
    };
    fields.insert("label".to_owned(), Value::String(label.to_owned()));
    fields.insert("material".to_owned(), Value::String(material.to_owned()));
    entry
}

/// Returns the texel at a pixel of an image.
fn texel(image: &RenderedImage, x: u32, y: u32) -> [f32; 4] {
    image
        .texel(x, y)
        .unwrap_or_else(|| panic!("`{}` reads at ({x}, {y})", image.name))
}

/// Returns the position a channel reported at a pixel, or none where it reported the marker.
///
/// The marker is `alpha`, and the test that owns it is the one that says what a pixel with no
/// surface holds; this only needs the rule, so it takes the alpha as the rule is written and
/// leaves the rest to the tests that check it.
fn covered(image: &RenderedImage, x: u32, y: u32) -> Option<[f32; 4]> {
    let texel = texel(image, x, y);
    (texel[3] > 0.5).then_some(texel)
}

/// Returns the ray through a pixel of the frame, built from the camera's own basis.
///
/// This is the arithmetic [`POSITION_SOURCE`] does, written out from the [`Projection`] the
/// scene's camera makes rather than from a uniform read back out of the compiled document. The
/// pixel's centre is what the ray is for, which is the same pixel the depth buffer was written
/// for, and the flip from texture space is the one the shader's vertex stage makes: clip space
/// runs from minus one to one with y up, texture space from zero to one with y down.
fn ray_through(camera: &Projection, x: u32, y: u32) -> [f64; 3] {
    let offset = [
        2.0 * (x as f64 + 0.5) / FRAME as f64 - 1.0,
        1.0 - 2.0 * (y as f64 + 0.5) / FRAME as f64,
    ];
    normalize3(add3(
        add3(
            camera.forward,
            scale3(camera.right, offset[0] * camera.half_extent[0]),
        ),
        scale3(camera.up, offset[1] * camera.half_extent[1]),
    ))
}

/// Returns where a ray meets the plane, or none for a ray pointing away from it.
///
/// None is a ray that never reaches the plane, which is every ray through the upper frame: it
/// is how this file knows a pixel above the far edge is a pixel the scene drew nothing on rather
/// than a pixel whose position it failed to work out.
fn meets_plane(camera: &Projection, ray: [f64; 3]) -> Option<[f64; 3]> {
    (ray[1] < -1e-9).then(|| add3(camera.eye, scale3(ray, (PLANE - camera.eye[1]) / ray[1])))
}

fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale3(v: [f64; 3], by: f64) -> [f64; 3] {
    [v[0] * by, v[1] * by, v[2] * by]
}

fn normalize3(v: [f64; 3]) -> [f64; 3] {
    scale3(v, 1.0 / dot(v, v).sqrt())
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
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

/// The shader a multi attachment pass is drawn with.
///
/// It declares nothing but the uniform, because the two targets are the only other binders the
/// pass has and **a target takes a binding number with no entry behind it** — so there is nothing
/// for a shader to declare. Both values are written to a named `@location`, and the test reads
/// them back per attachment rather than per draw.
fn source() -> String {
    let [fr, fg, fb, fa] = wgsl_vec(&FIRST);
    let [sr, sg, sb, sa] = wgsl_vec(&SECOND);
    format!(
        r#"
struct Uniforms {{
    time: f32,
}};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {{
    return vec4<f32>(position, 1.0);
}}

/// What one fragment writes, one entry per colour attachment.
///
/// The two are named rather than left to `@location(0)` and `@location(1)` in place, because a
/// shader that numbered them the other way round and a pipeline that ordered its targets the
/// other way round are the same defect, and a test that could not tell them apart would pass
/// on either.
struct Written {{
    @location(0) first: vec4<f32>,
    @location(1) second: vec4<f32>,
}};

@fragment
fn fragment_main() -> Written {{
    var out: Written;
    out.first = vec4<f32>({fr}, {fg}, {fb}, {fa});
    out.second = vec4<f32>({sr}, {sg}, {sb}, {sa});
    return out;
}}
"#
    )
}

/// Renders one value as the four literals a WGSL `vec4<f32>` takes.
fn wgsl_vec(value: &[f32; 4]) -> [String; 4] {
    value.map(|component| format!("{component}"))
}

/// Builds the graphics document: one rasterize pass writing one or two colour attachments.
///
/// With a single attachment the document declares one output, which is the shape every document
/// in the workspace has and the one the "declared outputs" machinery already answered.
fn graphics(second_attachment: bool) -> Value {
    let mut binders = vec![
        (
            "first".to_owned(),
            texture_declaration("first", "rgba8unorm"),
        ),
        (
            "row".to_owned(),
            buffer_declaration("row", INSTANCE_ROW_BYTES),
        ),
        (
            "triangle".to_owned(),
            buffer_declaration("triangle", TRIANGLE_BYTES),
        ),
    ];
    let mut targets = vec![Value::String("first".to_owned())];
    // Each declaration names a point, the `@location` its shader writes it at and the type of a
    // texel of it. The two are both `vec4<f32>` at locations zero and one, which is what the
    // shader's own `Written` struct declares, and the check in the device compares the two.
    let mut outputs = vec![
        (
            graphics::PRESENT_OUTPUT.to_owned(),
            declared("first", 0, "vec4<f32>"),
        ),
        ("first".to_owned(), declared("first", 0, "vec4<f32>")),
    ];
    if second_attachment {
        // `first` and `second` in name order, so the second target is the second attachment and
        // `@location(1)` has to be the one it was built as.
        binders.insert(
            1,
            (
                "second".to_owned(),
                texture_declaration("second", "rgba16float"),
            ),
        );
        targets.push(Value::String("second".to_owned()));
        outputs.push(("second".to_owned(), declared("second", 1, "vec4<f32>")));
    }

    let pass = Value::variant(
        "Rasterize",
        [
            ("label".to_owned(), Value::String("gbuffer".to_owned())),
            (
                "uniform".to_owned(),
                Value::structure([("time".to_owned(), Value::Float(0.0))]),
            ),
            ("shader".to_owned(), Value::String("shader".to_owned())),
            ("binders".to_owned(), Value::dictionary(binders)),
            ("targets".to_owned(), Value::List(targets)),
            (
                "clear_color".to_owned(),
                Value::List([0.0, 0.0, 0.0, 1.0].into_iter().map(Value::Float).collect()),
            ),
        ],
    );
    let mut fields = vec![("passes".to_owned(), Value::List(vec![pass]))];
    if second_attachment {
        // Without this the renderer would answer with the last colour target of the last colour
        // pass, which is the *second* attachment — and the test would then be checking a
        // convention rather than the declaration the whole feature is about.
        fields.push((
            graphics::OUTPUTS_FIELD.to_owned(),
            Value::dictionary(outputs),
        ));
    }
    fields.push((
        "instance_schema".to_owned(),
        graphics::example_graphics()
            .as_field_map()
            .and_then(|fields| fields.get("instance_schema"))
            .cloned()
            .expect("the example carries an instance schema"),
    ));
    Value::structure(fields)
}

/// Builds the instance the pass draws from.
fn instance(second_attachment: bool) -> Value {
    let mut entries = vec![
        ("row".to_owned(), instance_row()),
        // Both targets are written rather than uploaded, so neither carries pixels: giving a
        // target pixels would be uploading into a map the clear is about to overwrite.
        ("first".to_owned(), target_payload("rgba8unorm")),
        ("shader".to_owned(), shader(second_attachment)),
        ("triangle".to_owned(), triangle_vertices()),
    ];
    if second_attachment {
        entries.insert(1, ("second".to_owned(), target_payload("rgba16float")));
    }
    Value::structure([
        (
            "referencing_graphics".to_owned(),
            Value::String("test.graphics".to_owned()),
        ),
        ("entries".to_owned(), Value::dictionary(entries)),
    ])
}

/// The bytes three two dimensional points of three floats each occupy.
const TRIANGLE_BYTES: i64 = 3 * 3 * 4;

/// The bytes one instance row of three floats occupies.
const INSTANCE_ROW_BYTES: i64 = 3 * 4;

/// Builds a vertex buffer of three points that cover the whole viewport.
///
/// The third point is at three in both axes, so the triangle a rasteriser clips out of it covers
/// every pixel of the target rather than half of it — which is what makes every texel of both
/// attachments the value the shader wrote.
fn triangle_vertices() -> Value {
    packed_buffer(
        &[-1.0, -1.0, 0.0, 3.0, -1.0, 0.0, -1.0, 3.0, 0.0],
        "position",
        "vertex",
    )
}

/// Builds the single instance row, which carries nothing but has to exist to be counted.
fn instance_row() -> Value {
    packed_buffer(&[0.0, 0.0, 0.0], "filler", "instance")
}

/// Builds a buffer payload of three component points packed as floats.
fn packed_buffer(values: &[f32], field: &str, usage: &str) -> Value {
    let bytes: Vec<u8> = values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    Value::variant(
        "Buffer",
        [
            ("size_bytes".to_owned(), Value::Int(bytes.len() as i64)),
            (
                "usage".to_owned(),
                Value::List(vec![Value::String(usage.to_owned())]),
            ),
            ("data".to_owned(), Value::Bytes(bytes)),
            ("topology".to_owned(), Value::variant("Triangles", [])),
            (
                "schema".to_owned(),
                Schema::new(SchemaKind::structure([(
                    field.to_owned(),
                    graphics::layout::components_schema(3),
                )]))
                .to_value(),
            ),
        ],
    )
}

/// Builds the payload of a target, which is written by the pass and carries no pixels.
fn target_payload(format: &str) -> Value {
    Value::variant(
        "Texture",
        [
            ("width".to_owned(), Value::Int(FRAME as i64)),
            ("height".to_owned(), Value::Int(FRAME as i64)),
            ("format".to_owned(), Value::String(format.to_owned())),
        ],
    )
}

/// Builds the instance entry the pass names as its shader.
fn shader(second_attachment: bool) -> Value {
    Value::variant(
        "Shader",
        [
            ("source".to_owned(), Value::String(source())),
            ("schema".to_owned(), interface(second_attachment)),
        ],
    )
}

/// The interface the test shader declares.
///
/// The targets are listed because a binder field is one the pass **must** have, even though the
/// shader reads none of them: the targets are the only binders, and they take binding numbers
/// with no entries behind them. The list follows the pass, so the one attachment case declares
/// one and the two attachment case declares two — a schema naming a binder the pass does not
/// have would be a disagreement this file is not here to introduce.
fn interface(second_attachment: bool) -> Value {
    let mut binders = vec![
        ("first".to_owned(), graphics::texture_binding_schema()),
        ("row".to_owned(), graphics::buffer_binding_schema()),
        ("triangle".to_owned(), graphics::buffer_binding_schema()),
    ];
    if second_attachment {
        binders.insert(1, ("second".to_owned(), graphics::texture_binding_schema()));
    }
    Schema::new(SchemaKind::structure([
        (
            "uniform".to_owned(),
            Schema::new(SchemaKind::structure([(
                "time".to_owned(),
                Schema::new(SchemaKind::Float),
            )])),
        ),
        (
            "binders".to_owned(),
            Schema::new(SchemaKind::structure(binders)),
        ),
    ]))
    .to_value()
}

/// Builds one `outputs` entry: the point, the location and the type of a texel.
///
/// It is written through the shared helper rather than spelled out here, so that a document this
/// test builds and one the emitter builds are the same shape — the reader in `graphics` is the
/// only thing that has an opinion about it, and there is one.
fn declared(point: &str, location: u32, ty: &str) -> Value {
    Value::structure([
        ("point".to_owned(), Value::String(point.to_owned())),
        ("location".to_owned(), Value::Int(i64::from(location))),
        ("type".to_owned(), Value::String(ty.to_owned())),
    ])
}

/// Builds a texture declaration naming an entry of the instance.
fn texture_declaration(source: &str, format: &str) -> Value {
    Value::variant(
        "Resource",
        [(
            "resource".to_owned(),
            Value::variant(
                "Texture",
                [
                    ("source".to_owned(), Value::String(source.to_owned())),
                    ("width".to_owned(), Value::Int(FRAME as i64)),
                    ("height".to_owned(), Value::Int(FRAME as i64)),
                    ("format".to_owned(), Value::String(format.to_owned())),
                ],
            ),
        )],
    )
}

/// Builds a buffer declaration of the given size.
fn buffer_declaration(source: &str, size: i64) -> Value {
    Value::variant(
        "Resource",
        [(
            "resource".to_owned(),
            Value::variant(
                "Buffer",
                [
                    ("source".to_owned(), Value::String(source.to_owned())),
                    ("size_bytes".to_owned(), Value::Int(size)),
                ],
            ),
        )],
    )
}
