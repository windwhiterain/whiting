//! The identity scene: a building drawn so that **every pixel says which element it shows**.
//!
//! # What this frame is for
//!
//! Every other building scene answers *"what does it look like"*. This one answers *"what is this"*: its
//! document declares an `authored_id` output, every point of the building's mesh carries the id of the
//! element it belongs to, and the renderer writes that image beside the frame — `<out>-authored_id.png`
//! for a viewer and `<out>-authored_id.txt` for the values, because a png cannot hold floats.
//!
//! So the loop closes: an agent looking at `out/identity.png` can ask the sidecar which element a pixel is,
//! and the number it gets is a row of [`building::identity::Identity`] — a name like *"mass 0, the left
//! reveal of opening 0 in edge 2"*. That is the join this crate's whole "the lines carry their
//! correspondence" premise exists to make possible, and this is the first building that shows it.
//!
//! # Two surfaces, and both must write the outputs
//!
//! The ground is not an element of the building, and it still declares the output — with the reserved
//! zero. A stage that drew geometry and did not declare the output would leave the image holding whatever
//! an earlier fragment wrote there, which is a picture that answers a question about somebody else's
//! triangle.
//!
//! Run it with `cargo run -p whiting-device-rasterize --example render -- identity out/identity`.

use definition::Value;
use scene::standard::Stage;

use building::derive::{self, RoofStyle, Starter, floor_slab};
use building::grid::{FaceRef, Rhythm};
use building::identity::Identity;
use building::openings::{Inset, Opening};
use building::ops;
use building::parts::{Pin, Placement, Template};
use building::wall::Wall;
use building::{Building, Mass, MassId, PlanNode};

use crate::air::Atmosphere;
use crate::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed, stage_set,
};
use crate::kit::{
    camera, flux_for_irradiance, frame, instance, instances, light_with_radius, mesh,
    mesh_material, post_processes, screen_material, sky, surface,
};
use crate::shaders::{Projection, post_shader};

/// The building this frame draws: a small two-storey shell, so the frame holds a mass, a roof, a plate, a
/// wall and a part — five kinds of element, each with an id of its own.
const STARTER: Starter = Starter {
    width: 5.0,
    depth: 8.0,
    height: 5.6,
    wall: 0.3,
    slab: 0.45,
    overhang: 0.45,
    sink: 0.12,
    rise: 1.4,
    roof_thickness: 0.18,
    roof: RoofStyle::Gable,
    seed: 7,
};

/// Where the floor sits inside the building.
const FLOOR_AT: f32 = 2.7;
const FLOOR_THICK: f32 = 0.22;

/// How many bays across and how many storeys, so the openings are bay-mounted per storey.
const BAYS: usize = 3;
const STOREYS: usize = 2;

/// Where the sun stands, front-left and high.
const SUN: [f64; 3] = [-13.0, 28.0, 21.0];

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

/// The image this scene is drawn into, in pixels.
///
/// # Why this frame is this shape
///
/// This building is a **small two-storey shell with a wall across its yard**, and it is the tallest
/// thing per metre of plan in the gallery: seven metres of ridge over eight of depth. The measured
/// number says so. Reading every id the frame carries at `FRAC_PI_3 * 1.25`, the silhouette stands
/// `x 132..=755` by `y 276..=674` of the pipeline's square — **f_w 0.6090 by f_h 0.3891**, a world
/// aspect of **1.565 to one**.
///
/// So the frame takes those proportions, **1.57 to one** at 1280 by 816: 1,044,480 pixels against
/// the square's 1,048,576.
///
/// **The square it replaces was cutting it.** Its left column carried **121 pixels of building**, and
/// `wall 0, its right side` — the wall the frame is named for — ran from x 0. A yard wall seen from
/// the near end is the longest thing in the picture and the first thing to leave it.
const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 816;

/// The full **vertical** angle this scene is drawn through, in radians.
///
/// # The arithmetic
///
/// ```text
/// tan(new / 2) = tan(1.25 * PI/3 / 2) * 0.3891 / 0.89
///              = 0.767327 * 0.437191 = 0.335442
/// new = 2 * atan(0.335442) = 0.64734 rad = FRAC_PI_3 * 0.6182
/// ```
///
/// **89 % and not 90 %**, because the roof's soffit stands outside the skin over it and the yard wall
/// runs past the building at one end.
///
/// **The eye did not move and the aim did, because the silhouette fits and is not in the middle.**
/// At this frame and this lens the scene measures `x 0..=1085` by `y 0..=705`: **0.8483 of the width
/// and 0.8650 of the height**, with its centre **97 px left of and 55 px above** the frame's — which
/// is why the yard wall stood on x 0 and the roof on row 0 while the right side carried 194 px of
/// margin. `97 / 639.5 * 15.7475 * 0.334840` = **0.8000 m** along `[0.796161, 0, 0.605078]` the other
/// way and `55 / 407.5 * 15.7475 * 0.334840` = **0.7117 m** of height put it in the middle with 55 px
/// of margin on every side, **and the lens did not move at all**. Shortening the lens until the wall
/// fitted would have cost a quarter of the fill — and a quarter is what it cost the first time this
/// was tried here.
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.6182;

/// The aim this scene looks from, and why it is not the building's own middle.
///
/// # Invariants
///
/// **This scene's one opening is a doorway whose head sits below its own plate**, so the floor is in
/// frame because of where the eye stands, how high it is and how far it is from the wall. **The eye
/// did not move and the pitch did**, and that is the one thing in this scene a correction was allowed
/// to spend: a narrower lens would have magnified the sight line over the same sill, and a shorter one
/// would have lost it. The render answers whether 0.71 m of pitch cost the plate.
const AIM: [f64; 3] = [-0.63693, 3.11174, -0.48432];

/// Returns a shutter: a panel `width` across, `height` tall, `thickness` through.
fn shutter(width: f32, height: f32, thickness: f32) -> Template {
    let slab = 0.02;
    let outline = [
        [0.0, 0.0],
        [width, 0.0],
        [width, thickness],
        [0.0, thickness],
    ]
    .into_iter()
    .map(|corner| PlanNode::new(corner, slab))
    .collect();
    Template::new(
        "shutter",
        vec![Mass::new(outline, height).with_wall(0.01).with_slab(slab)],
        Vec::new(),
    )
}

/// Returns the document: the shell, a plate, a wall across the yard, and a shutter in a bay.
pub fn document() -> Building {
    let mut skeleton = derive::starter(&STARTER).skeleton;
    let mass = MassId(0);
    let front = FaceRef::new(mass, 2);
    let plate = floor_slab(&skeleton.masses[0].clone(), mass, FLOOR_AT, FLOOR_THICK)
        .expect("the plate is inside the building");
    skeleton.slabs.push(plate);
    for face in skeleton.faces_of(mass) {
        ops::split_face(
            &mut skeleton,
            face,
            &Rhythm::Even { count: BAYS },
            &Rhythm::Even { count: STOREYS },
        );
    }
    skeleton.walls.push(Wall::new(
        PlanNode::new([-7.0, -6.5], 0.0),
        PlanNode::new([7.0, -6.5], 0.0),
        0.35,
        1.9,
    ));
    // A doorway and five windows, so the frame has a facade and the ids have something to name: the
    // doorway's head is below the plate, and the upper windows sit above it.
    ops::add_opening(
        &mut skeleton,
        Opening::new(front, 0, 1, Inset::new(0.34, 0.34, 0.0, 0.75)),
    )
    .expect("the middle bay of the ground storey is in the grid");
    for (row, column) in [(0, 0), (0, 2), (1, 0), (1, 1), (1, 2)] {
        ops::add_opening(
            &mut skeleton,
            Opening::new(front, row, column, Inset::new(0.34, 0.34, 1.05, 0.7)),
        )
        .expect("every bay of the grid is there");
    }
    let template = ops::add_template(&mut skeleton, shutter(0.9, 1.1, 0.05));
    ops::place_part(
        &mut skeleton,
        Placement::new(template, front, 0, 1, Pin::new(0.28, 0.25)),
    )
    .expect("the bay is in the grid");
    Building::new(skeleton)
}

/// Builds the identity scene.
pub fn identity() -> Value {
    let building = document();
    let built = building.build().expect("the identity document expands");
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "the identity scene's own document must be clean, and it is not: {violations:?}"
    );
    let numbering = Identity::of(&built, 1);

    // One mesh, one material: the point of the frame is the ids rather than the look, and a single mesh keeps
    // the picture and its sidecar from disagreeing about which material drew what.
    let clothed = packed(&built, &numbering);
    let wall = || surface([0.72, 0.68, 0.58], 0.0, 0.86, [0.0; 3]);
    let earth = || surface([0.30, 0.26, 0.19], 0.0, 0.95, [0.0; 3]);

    let mut materials = Vec::new();
    for (prefix, body) in [("wall", wall as fn() -> Value), ("earth", earth)] {
        for stage in ["shadowing", "depthing"] {
            materials.push(mesh_material(
                &format!("{prefix}-{stage}"),
                &format!("{prefix}_{stage}"),
                body(),
            ));
        }
        materials.push(mesh_material(
            &format!("{prefix}-shade"),
            &format!("{prefix}_lighting"),
            body(),
        ));
    }

    let placed = |stage: &str| {
        [("building", "wall"), ("ground", "earth")]
            .into_iter()
            .map(|(label, material)| {
                instance(
                    format!("{label}-{stage}"),
                    label,
                    [0.0; 3],
                    [0.0; 3],
                    [1.0; 3],
                    &format!("{material}-{stage}"),
                )
            })
            .collect::<Vec<_>>()
    };

    let point = building_extras().point_schema();
    let extras = building_extras();
    // **One air, handed to two renderers**: the map a world reads and the light a surface sees. Without the
    // light the shading's sky term is zero, which is why a sun-only scene's shadowed side is black.
    let (sky_image, sky_light) = sky(&atmosphere());
    let helpers = atmosphere().wgsl();
    let mut shaders = stage_set("wall", "", helpers.as_str(), &point, &extras);
    shaders.extend(stage_set(
        "earth",
        "",
        helpers.as_str(),
        &not_authored_extras().point_schema(),
        &not_authored_extras(),
    ));
    shaders.push(post_shader(GRADE, &helpers, Some(Projection::Latlong)));

    Value::structure([
        ("name".to_owned(), Value::String("identity".to_owned())),
        // The one field here that is about the picture rather than what is in it: a two-storey shell
        // and a yard wall, and a frame of that breadth. A scene that names no frame is drawn into the
        // pipeline's own square, so this is a decision rather than a default.
        ("frame".to_owned(), frame(FRAME_WIDTH, FRAME_HEIGHT)),
        // The one image this scene writes beside the frame. The building reads the ids from its points and
        // the ground writes the reserved zero.
        ("outputs".to_owned(), outputs_field()),
        (
            "cameras".to_owned(),
            // **The eye did not move and the lens did not either**; the aim is what this
            // scene's own frame asked for, and the frame's shape is what let the shell and its wall fill it.
            Value::List(vec![camera([-9.5, 3.6, 12.5], AIM, FIELD_OF_VIEW)]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                light_with_radius(
                    "sun",
                    SUN,
                    [1.0, 0.87, 0.70],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                    13.0,
                    160.0,
                ),
                sky_light,
            ]),
        ),
        ("images".to_owned(), Value::List(vec![sky_image])),
        ("shaders".to_owned(), Value::List(shaders)),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh("building", "Triangles", point.clone(), clothed),
                // The ground writes the reserved zero from a **constant**, so its points carry no id field at
                // all: a constant is not a field, and the set of stages it is drawn by is built over the
                // three-field schema the heightfield already packs.
                mesh(
                    "ground",
                    "Triangles",
                    not_authored_extras().point_schema(),
                    crate::ground::mesh(&building, 160, 200.0, |_, _| 0.0),
                ),
            ]),
        ),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        ("mesh_materials".to_owned(), Value::List(materials)),
        (
            "instances".to_owned(),
            instances(vec![
                (Stage::Shadowing, placed("shadowing")),
                (Stage::Depthing, placed("depthing")),
                (Stage::Lighting, placed("shade")),
            ]),
        ),
    ])
}

/// Returns how far a light position stands from the scene's own middle.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// The air over the identity scene: a clear afternoon, and **the light the sun is not**.
///
/// The numbers are the ones this scene's own sky law carried — zenith, horizon, ground and the sun's
/// bloom — read out of it and handed to one `Atmosphere`, which bakes them into the map the shading reads
/// its sky term from. A frame whose shadows are black is a frame with no air, not one with the wrong
/// colours.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.15, 0.26, 0.48])
        .horizon([0.86, 0.73, 0.56])
        .ground([0.30, 0.27, 0.21])
        .sun([-13.0, 28.0, 21.0], [1.0, 0.86, 0.62], 8.0, 0.32)
        .density(0.0011)
        .scale_height(200.0)
}

/// What the post stage does to the frame; a fragment, spliced into the post shader's body.
const GRADE: &str = r#"
    // **The sky is read out of the scene's own environment map**, the same air the bounce reads, so the
    // background and the light cannot be two skies.
    if depth >= 1.0 {
        color = environment_sky(ray_through(in.uv));
    }
    if depth < 1.0 {
        let ray = ray_through(in.uv);
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp(1.0 - exp(-along_ray * 0.005), 0.0, 0.55);
        color = mix(
            color,
            environment_sky(vec3<f32>(ray.x, max(ray.y, -0.04), ray.z)),
            air,
        );
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.20), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.40, 0.0, 1.0);
"#;
