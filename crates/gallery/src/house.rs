//! A house: one authored building, standing on the ground, seen from outside.
//!
//! This is the shell phase's proof scene. The house is not modelled here — it is *derived* by
//! `building::derive` and expanded by `building::expand`, and what this file adds is the three
//! things a frame needs that a building is not: a ground, a sun, and a camera. The two meshes are
//! one solid each (the mass and the roof) because a material belongs to an instance and not to a
//! face, which is why walls and roof are drawn by different materials and are therefore different
//! meshes.
//!
//! What the scene is here to show, in the order the phase's contract names it:
//!
//! - **every solid is closed and outward** — the walls' inner faces and the floor exist, so the
//!   solid is watertight rather than a box of four planes, and the roof has a soffit, so a camera
//!   below the eave meets a face rather than a hole (the failure `gallery::built` records as
//!   "single-sided roofs");
//! - **one authored surface shades as one surface** — the normals come from the building's own
//!   corner normals, so a roof plane is smooth and the bends between planes are hard;
//! - **the roof is seated, not balanced** — its eaves stand `sink` below the wall top, an
//!   intersection the roof's `host` authorizes, which is the state `terraces` hand-builds today.

use definition::Value;
use scene::standard::Stage;

use building::Building;
use building::MassId;
use building::derive::{self, RoofStyle, Starter};
use building::grid::{FaceRef, Rhythm};
use building::identity::Identity;
use building::openings::{Inset, Opening};
use building::ops;

use crate::air::Atmosphere;
use crate::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use crate::kit::{
    camera, flux_for_irradiance, frame, instance, instances, light_with_radius, mesh,
    mesh_material, post_processes, screen_material, sky, surface,
};
use crate::shaders::{Projection, post_shader};

/// What the derived house is: a rectangular two-bay cottage with a hipped roof.
///
/// The numbers are the scene's, not the crate's: the starter's defaults are a house of the same
/// order and this one is a little longer and a little steeper, because the proof is a picture.
const STARTER: Starter = Starter {
    width: 7.0,
    depth: 11.0,
    height: 3.6,
    wall: 0.35,
    slab: 0.45,
    overhang: 0.7,
    sink: 0.15,
    rise: 2.1,
    roof_thickness: 0.28,
    roof: RoofStyle::Hip,
    seed: 3,
};

/// Where the sun stands. The house's own middle is the origin, so this is a direction as much as a
/// place: high and to the left, so the near wall is lit and the far one is not. High rather than
/// grazing on purpose — a low sun rakes the wall and the ground at the angle a shadow map is
/// least able to resolve, and the frame this scene exists to produce is not the place to discover
/// that (see the shadow notes in `.agents/proposals`).
const SUN: [f64; 3] = [-22.0, 30.0, 20.0];

/// The image this scene is drawn into, in pixels.
///
/// # Why this frame is this shape
///
/// A house seen from its near corner is **wider than it is tall in the picture**, and the measured
/// number says by how much: the silhouette this camera draws stands `x 10..=915` by `y 286..=775`
/// of the pipeline's 1024 square, which is **905 by 489 pixels, an aspect of 1.851**. A square frame
/// therefore spends about half its width on sky and ground beside a building that is not square, and
/// no lens undoes that: the shape is the ceiling.
///
/// So the frame takes the silhouette's own proportions — **1.851 to one**, here 1392 by 752 — and
/// the lens is then worked out against it. The picture is 1,046,784 pixels against the square's
/// 1,048,576, so the frame is also very slightly the cheaper of the two.
///
/// **One number here is the measured one and the other is the decision.** The silhouette's
/// proportions were read out of a render (`x 10..=915` by `y 286..=775` of a 1024 square); the 1392
/// by 752 is those proportions rounded to a whole number of pixels without growing the picture past
/// the square it replaces.
const FRAME_WIDTH: i64 = 1392;
const FRAME_HEIGHT: i64 = 752;

/// The full **vertical** angle this house is drawn through, in radians.
///
/// # The arithmetic, so a later reader does not have to re-derive it
///
/// The square this scene replaced held `FRAC_PI_3 * 0.85` = **0.89012 rad** down the frame, whose
/// half-angle tangent is **0.476976**, and the silhouette above took 489 of that frame's 1024 rows.
/// The building must now take **89 % of 752 rows**, so the half-angle's tangent is scaled by the
/// silhouette's own share of the old frame over the share the new one asks for:
///
/// ```text
/// tan(new / 2) = 0.476976 * 489 / (0.89 * 1024) = 0.256906
/// new = 2 * atan(0.256906) = 0.50254 rad = FRAC_PI_3 * 0.4799
/// ```
///
/// **The two frame heights cancel, which is the whole of a trap worth naming.** A reader who scales
/// the lens by the magnification alone — 489 px becoming 89 % of 752, a factor of 1.369 — arrives at
/// a lens 4 % shorter than this one and a picture that does not quite fill the frame it was composed
/// for. Pixels per metre go as `height / tan(angle / 2)`, so it is the **ratio** of the two that is
/// held and not the factor.
///
/// **89 % and not 90 %**, because the hip's fascia stands outside the skin it belongs to: at 90 % the
/// silhouette measured `x 0..=1257` of `0..=1391`, and the left column of the frame carried **22
/// pixels of it**. The roof's own bounding edge is the last thing to arrive at a silhouette's side,
/// and a frame that clips a fascia clips a building.
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.4799;

/// How far the sun's shadow page has to cover. The house stands inside twelve units and its shadow
/// reaches about twenty, so a tighter page than the scene's own extent is what puts texels where
/// the frame is looking.
const SHADOW_EXTENT: f64 = 14.0;

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

/// The wall the window is cut into: **edge 1**, the mass's own outline read from
/// `[(-3.5,-5.5), (3.5,-5.5), (3.5,5.5), (-3.5,5.5)]`, so edge 1 runs from `(3.5,-5.5)` to
/// `(3.5,5.5)` — the **east** face, and the one the eye at `[11.5, 4.2, 13.0]` stands outside of.
///
/// # Why this wall and not the other one the camera looks at
///
/// A three-quarter view sees two walls, and this scene's own frame is composed about both. Only one
/// of them can show the floor, and the difference is arithmetic rather than taste. The eye stands
/// `8.0` metres outside this wall's plane at `y = 4.2`, so a sight line from the eye to a point of
/// the plate's top crosses the plane at
/// `4.2 * (3.5 - x) / (11.5 - x)`, which is **`1.278 m` above the floor at `x = 0`** and falls to
/// `0.176 m` at the room's own east edge `x = 3.15`. Every one of those crossings is above a sill
/// and inside a head, so a window on this wall reaches the plate.
///
/// The **north** face cannot: the same line crosses that plane at `11.5 * (5.5 - z) / (13 - z)`, and
/// it lands inside the wall's own run — `x` between `-3.5` and `3.5` — only where the plate is
/// within `0.15 m` of its own north edge, at heights under `0.70 m`. A window with any sill at all
/// would sit above every one of them. That is why the opening is here and not there.
const EAST_EDGE: usize = 1;

/// How many bays that wall carries, and how many storeys the house has.
///
/// **Three and one, the numbers [`shed`](crate::shed) authors its facade with.** One storey on
/// purpose: a second storey line would divide a window into part of a window.
const BAYS: usize = 3;
const STOREYS: usize = 1;

/// Which bay across that wall the window stands in, counting from the edge's start.
///
/// **The third, which is the bay nearest the eye.** The sight lines above reach the wall between
/// `z = 0.37` and `z = 5.5`, and bay 2 is the one that spans `z 1.833..=5.5` — the whole northern
/// half of the crossing, with nothing of it cut off at either end.
const WINDOW_BAY: usize = 2;

/// The window's own insets: from each jamb, up from the sill, down from the head.
///
/// **The sill and the head are [`shed`](crate::shed)'s, unchanged and in metres** — a scene's
/// openings are the scene's numbers, and this scene's sibling is the one that already authored a
/// window: `1.05 m` up and `0.55 m` down, which on this wall's 3.6 m height reads a hole **`2.0 m`
/// tall**.
///
/// **The jambs are this wall's own, and that is not a preference.** A wall's two loops differ: the
/// offset loop's edge runs from `wall` to `length - wall`, because a right-angle mitre takes one
/// thickness off each end, and a hole whose far jamb lands at or past that end puts its reveal on a
/// corner the neighbouring wall still claims whole. `expand` calls that `past_offset` and refuses
/// the cut. Copied straight across, `shed`'s `0.30 m` jambs put this hole's far jamb at
/// `11.0 - 0.30 = 10.70` against an offset edge that ends at `10.65`, and the scene refused to
/// build with **`OpeningOutsideWall { …, past: 0.051000595 }`**. The jambs are therefore the wall's
/// own thickness plus the same `50 mm` `shed` leaves over its own `0.25 m`, which reads `0.40 m`
/// here and a hole **`2.867 m` across**.
///
/// A function and not a constant because [`Inset`]'s constructor is not a `const fn`, and the
/// scene's other builders here — [`shed`](crate::shed)'s leaf, [`yard`](crate::yard)'s gate — are
/// functions for the same reason.
fn window() -> Inset {
    let jamb = STARTER.wall + JAMB_CLEARANCE;
    Inset::new(jamb, jamb, 1.05, 0.55)
}

/// How far a jamb stands **past the wall's own thickness**, in metres.
///
/// The margin is `shed`'s, not this scene's: it leaves `0.05 m` over a `0.25 m` wall, and it is a
/// margin rather than the thickness itself because the thickness alone puts the hole's edge on
/// `GRID_TOLERANCE`, which the same check reads as reaching.
const JAMB_CLEARANCE: f32 = 0.05;

/// Builds the house scene.
pub fn house() -> Value {
    let building = document();
    let built = building
        .build()
        .expect("the starter house expands: its outline is a quad and its roof's rows are paired");
    // **The scene refuses to draw a document the crate's validator would not**, which is what `shed`
    // and `workshop` already do. A house that grows an opening grows the one thing most likely to
    // fault, so the check has to be in the scene that grows it and not in a probe that reports it.
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "the house's own document must be clean, and it is not: {violations:?}"
    );

    // **Every mesh the frame draws carries its elements' ids**, so a pixel of this house can be asked what it
    // is: the walls name their edges, the hip names its skins and its fascia.
    let table = Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    // The starter's own foundation plate: a mass is a wall ring **and** a plate, so this house has always
    // authored one and this is the mesh that draws it.
    let floors = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Slab(_, _))
    });
    let roof = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Roof(_, _))
    });

    // A plaster wall, a tiled roof and dry ground. The three differ in tint and roughness and in
    // nothing else: what a surface *does* is the shader's business and this scene adds no body to
    // it, so the materials are the standard ones with a colour apiece.
    let plaster = || surface([0.74, 0.70, 0.61], 0.0, 0.85, [0.0; 3]);
    let tiles = || surface([0.33, 0.14, 0.10], 0.0, 0.62, [0.0; 3]);
    let ground = || surface([0.23, 0.21, 0.15], 0.0, 0.95, [0.0; 3]);
    let floor = || surface([0.34, 0.26, 0.18], 0.0, 0.80, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("wall", plaster as fn() -> Value),
        ("roof", tiles as fn() -> Value),
        ("floor", floor as fn() -> Value),
    ] {
        materials.push(mesh_material(
            &format!("{name}-cast"),
            "authored_shadowing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{name}-depth"),
            "authored_depthing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{name}-shade"),
            "authored_lighting",
            body(),
        ));
    }
    // The ground is not an element of the house, and it still writes the output — with the reserved zero.
    for (stage, shader) in [
        ("cast", "plain_shadowing"),
        ("depth", "plain_depthing"),
        ("shade", "plain_lighting"),
    ] {
        materials.push(mesh_material(&format!("ground-{stage}"), shader, ground()));
    }

    // Every mesh is authored in world coordinates, so its three instances carry no transform: a
    // house placed by a translation here would be a second copy of where the house already is.
    let placed = |stage: &str| {
        [
            ("walls", "wall"),
            ("roof", "roof"),
            ("floors", "floor"),
            ("ground", "ground"),
        ]
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

    // **Two point schemas**: the walls and the hip carry `authored_id`; the ground carries none and writes
    // the reserved zero from a constant.
    let extras = building_extras();
    let point = extras.point_schema();
    let plain = not_authored_extras();
    // **One air, handed to two renderers**: the map a world reads and the light a surface sees. Without the
    // light the shading's sky term is zero, which is why a sun-only scene's shadowed side is black.
    let (sky_image, sky_light) = sky(&atmosphere());

    Value::structure([
        ("name".to_owned(), Value::String("house".to_owned())),
        // The one field here that is about the picture rather than what is in it: a cottage seen from
        // its near corner is nearly twice as wide as it is tall, and the frame takes that shape. A
        // scene that names no frame is drawn into the pipeline's own square, so this is a decision.
        ("frame".to_owned(), frame(FRAME_WIDTH, FRAME_HEIGHT)),
        (
            "cameras".to_owned(),
            // A three-quarter view from the near corner, high enough that the roof reads as a
            // volume rather than as a silhouette: the eave line and the ridge are both visible,
            // which is what says the roof has a soffit and a thickness.
            //
            // **The eye did not move and the target moved 0.43 m, for a measured reason.** A house
            // seven wide and eleven deep seen from its near corner does not project onto a frame
            // centred on its plan centre: the far corner pulls the silhouette's own centre
            // **0.0963 of a half-width to the left**, which is 67 px of a 1392 frame and which the
            // square was large enough to hide. `0.0963 * 17.4954 * 0.253028 = 0.4265 m` along the
            // camera's own right axis — `[0.749014, 0, -0.662587]` — brings the silhouette to the
            // middle, and the middle is what lets a lens be 4 % shorter without clipping it.
            //
            // **The lens is the other free variable**, and it is free because the frame is now the
            // silhouette's own shape rather than the pipeline's square.
            Value::List(vec![camera(
                [11.5, 4.2, 13.0],
                [-0.3195, 2.0, 0.2826],
                FIELD_OF_VIEW,
            )]),
        ),
        (
            "lights".to_owned(),
            // A `power` rather than a brightness, derived from the irradiance wanted at the
            // house's own distance from the light: see `.agents/proposals/physical-light-units.md`.
            Value::List(vec![
                light_with_radius(
                    "sun",
                    SUN,
                    [1.0, 0.87, 0.70],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                    SHADOW_EXTENT,
                    160.0,
                ),
                sky_light,
            ]),
        ),
        ("images".to_owned(), Value::List(vec![sky_image])),
        (
            "shaders".to_owned(),
            Value::List({
                let helpers = atmosphere().wgsl();
                let mut shaders = stage_set("authored", "", helpers.as_str(), &point, &extras);
                shaders.extend(stage_set(
                    "plain",
                    "",
                    helpers.as_str(),
                    &plain.point_schema(),
                    &plain,
                ));
                shaders.push(post_shader(GRADE, &helpers, Some(Projection::Latlong)));
                shaders
            }),
        ),
        // The one image this scene writes beside the frame: which element each pixel shows.
        ("outputs".to_owned(), outputs_field()),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh("walls", "Triangles", point.clone(), walls),
                mesh("roof", "Triangles", point.clone(), roof),
                mesh("floors", "Triangles", point, floors),
                mesh(
                    "ground",
                    "Triangles",
                    plain.point_schema(),
                    crate::ground::mesh(&building, 140, 420.0, |_, _| 0.0),
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
                (Stage::Shadowing, placed("cast")),
                (Stage::Depthing, placed("depth")),
                (Stage::Lighting, placed("shade")),
            ]),
        ),
    ])
}

/// Returns how far a light position stands from the scene's own middle.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// The air over the house: a clear afternoon, and **the light the sun is not**.
///
/// The numbers are the ones this scene's own sky law carried — zenith, horizon, ground and the sun's
/// bloom — read out of it and handed to one `Atmosphere`, which bakes them into the map the shading reads
/// its sky term from. A house whose shadows are black is a house with no air, not a house with the
/// wrong colours.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.16, 0.27, 0.50])
        .horizon([0.88, 0.74, 0.56])
        .ground([0.34, 0.30, 0.24])
        .sun([-22.0, 30.0, 20.0], [1.0, 0.86, 0.62], 8.0, 0.35)
        .density(0.0011)
        .scale_height(200.0)
}

/// What the post stage does to the frame.
///
/// # Invariants
///
/// **The air goes on before the curve and only over the geometry.** The sky is already the far
/// distance; what has to be crossed with air is everything between the eye and it, and the distance
/// is the post stage's own recovery of where the pixel is rather than a number assembled out of a
/// near and a far plane.
const GRADE: &str = r#"
    // **The sky is read out of the scene's own environment map**, the same air the bounce reads, so the
    // background and the light cannot be two skies.
    if depth >= 1.0 {
        color = environment_sky(ray_through(in.uv));
    }
    if depth < 1.0 {
        let ray = ray_through(in.uv);
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp(1.0 - exp(-along_ray * 0.006) * 0.85, 0.0, 0.92);
        // The air in front of a surface takes the colour of the sky *behind* it, so the ray's
        // height is held to the horizon rather than allowed to point into the ground.
        color = mix(
            color,
            environment_sky(vec3<f32>(ray.x, max(ray.y, -0.04), ray.z)),
            air,
        );
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.22), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.45, 0.0, 1.0);
"#;

/// Returns the house's document alone, for a caller that wants to name what a frame shows.
///
/// # Invariants
///
/// - **Every slot is named, never a coordinate**, so the facade moves if the grid does and nothing
///   here has to be rewritten — the same rule [`shed`](crate::shed)'s document states.
/// - **The rhythm is laid on every face and the window on one bay of one of them**, because an
///   opening names a slot and a slot only exists where a rhythm is. A rhythm with no hole on it
///   draws the same wall the starter drew, so the four bays this adds to the picture are the id
///   table's and not the frame's.
/// - **One opening, and it is a window rather than a door.** A door's sill is the ring's own base,
///   which is a different thing to cut: `shed` carries one and this scene does not need to, and the
///   arithmetic under [`EAST_EDGE`] is about a sill at `1.05 m`, not at the floor.
pub fn document() -> Building {
    let mut skeleton = derive::starter(&STARTER).skeleton;
    let mass = MassId(0);
    let columns = Rhythm::Even { count: BAYS };
    let rows = Rhythm::Even { count: STOREYS };
    for face in skeleton.faces_of(mass) {
        ops::split_face(&mut skeleton, face, &columns, &rows);
    }
    ops::add_opening(
        &mut skeleton,
        Opening::new(FaceRef::new(mass, EAST_EDGE), 0, WINDOW_BAY, window()),
    )
    .expect("the window names a bay of the grid");
    Building::new(skeleton)
}
