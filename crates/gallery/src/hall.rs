//! A hall: one building derived from `derive::HALL`, and a facade the **style** decides.
//!
//! This is the style slice's proof scene. Nothing here chooses a wall thickness, a pitch or a window's
//! size: `HALL` states its relations — a wall is a twenty-sixth of the smaller span with a floor and a
//! ceiling, a rise is 0.85 of the half-span, a bay is three metres, a window is 0.52 of its bay with its
//! own sill and height — and `HALL::starter`, `HALL::bays` and `HALL::glazing` say what follows for this
//! footprint. What the file adds is the three things a frame needs that a building is not: a ground, a
//! sun, and a camera.
//!
//! # What the frame shows
//!
//! A tall single-storey hall with **three bays a face and one window in each**, all of them the style's
//! own glazing: the same relations that set the walls set the holes, which is what makes a style more
//! than a table of numbers.

use definition::{PackedList, Value};
use scene::standard::Stage;

use building::derive::{self, HALL, Starter};
use building::grid::Rhythm;
use building::identity::Identity;
use building::openings::{Inset, Opening};
use building::ops;
use building::{Building, BuildingMesh, MassId};

use crate::air::Atmosphere;
use crate::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use crate::kit::{
    camera, flux_for_irradiance, frame, instance, instances, light_with_radius, mesh,
    mesh_material, post_processes, screen_material, sky, surface,
};
use crate::shaders::{Projection, post_shader};

/// What the hall is: `HALL`'s relations at a 10 × 16 footprint, one tall storey.
///
/// # Invariants
///
/// **Every number in the document comes from the style**, which is the point of the scene: the wall, the
/// slab, the eave, the seat, the rise and the roof's thickness are `HALL::starter`'s answers, and the
/// windows are `HALL::glazing`'s. The footprint and the storey count are the scene's, because those are
/// what an author asks for rather than what a style decides.
///
/// **The bays are one count round the building.** A hall's rhythm is uniform, so the count `HALL::bays`
/// gives for the shorter span is used on every face; a scene that measured each face would be choosing a
/// second rhythm the style did not name.
pub fn document_at(width: f32, depth: f32, storeys: usize) -> Building {
    let plan = HALL.starter(width, depth, storeys);
    let mut skeleton = derive::starter(&plan).skeleton.clone();
    let mass = MassId(0);
    let bays = HALL.bays(plan.width);
    for face in skeleton.faces_of(mass) {
        ops::split_face(
            &mut skeleton,
            face,
            &Rhythm::Even { count: bays },
            &Rhythm::Even { count: 1 },
        );
    }
    let (left, right, sill, height) = HALL.glazing(plan.width / bays as f32, plan.height);
    for face in skeleton.faces_of(mass) {
        for column in 0..bays {
            let _ = ops::add_opening(
                &mut skeleton,
                Opening::new(face, 0, column, Inset::new(left, right, sill, height)),
            );
        }
    }
    Building::new(skeleton)
}

/// Returns the hall's document at the size the scene draws: 10 × 16, one tall storey.
pub fn document() -> Building {
    document_at(10.0, 16.0, 1)
}

/// Where the sun stands. The hall's own middle is the origin, so this is a direction as much as a place:
/// high and to the left, so the near wall is lit and the far one is not. High rather than grazing on
/// purpose — a low sun rakes the wall and the ground at the angle a shadow map is least able to resolve,
/// and the frame this scene exists to produce is not the place to discover that.
const SUN: [f64; 3] = [-26.0, 34.0, 24.0];

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

/// Builds the hall scene.
/// Builds the hall scene at the size the scene draws: 10 × 16, one tall storey.
pub fn hall() -> Value {
    hall_at(10.0, 16.0, 1)
}

/// Builds the hall scene at a **larger** footprint from the same style and the same code.
///
/// # Invariants
///
/// **Nothing but the footprint and the storey count is passed in**, and the frame follows the building: the
/// camera's distance, its height and the sun's shadow page are all derived from the size the style answers
/// with, so two sizes draw as two pictures of one building rather than one picture with a bigger box in it.
///
/// **The camera is this scene's own**, which is the one thing it does not take from
/// [`three_quarter`]. At 14 × 20 the shared view stands `ridge * 0.75` above and 29 units out, and the
/// frame it draws shows the roof and the two near facades and nothing of the floor — the plate the
/// building stands on covers no pixel of it at all. A camera has to be chosen for the subject rather than
/// scaled onto it, and this scene's subject is a room.
pub fn hall_large() -> Value {
    hall_framed(14.0, 20.0, 1, &close_and_low_view())
}

/// Builds the hall scene for a footprint and a storey count.
pub fn hall_at(width: f32, depth: f32, storeys: usize) -> Value {
    hall_framed(width, depth, storeys, &three_quarter_view())
}

/// The three-quarter view [`hall_at`] draws from, kept whole so the other scenes keep their pictures.
///
/// # Invariants
///
/// **It is a function of the size, not a number**, so a scene that calls [`hall_at`] for a third size gets
/// a camera that fits that building rather than one that happened to fit the first.
fn three_quarter(width: f32, depth: f32, ridge: f64) -> [f64; 3] {
    [
        f64::from(width) * 1.25,
        ridge * 0.75,
        f64::from(depth) * 0.95 + f64::from(width) * 0.3,
    ]
}

/// The eye [`hall_large`] draws from: lower and closer than [`three_quarter`].
///
/// # Invariants
///
/// **Lower and closer, and that is the whole of it.** The shared view stands `ridge * 0.75` above and
/// 29 units out at this size, which draws the roof and the two near facades and leaves the building small
/// in the frame; `ridge * 0.55` and a plan distance of 23 fill the frame with the hall instead.
///
/// **A reading, not a derivation, and the number under it is a tie.** This scene's plate is at the ground
/// and its ground is at the ground, so the two are coplanar and every pixel of the plate is a depth tie the
/// shading pass settles by rasterisation rather than by depth: `--where "slab 0, its top"` reads `0` at
/// [`three_quarter`] and `1,656` here, and at `ridge * 0.7` it reads `30`. **None of those three is a
/// statement about how much floor a reader can see, because there is none to see** — a buried plate cannot
/// be brought into a frame by moving the eye. What the eye can do is put the building in the frame at a
/// size worth looking at, and that is what this one does; the count is recorded because the record is
/// where a later reader looks, not because it is the thing that was chosen.
fn close_and_low(width: f32, depth: f32, ridge: f64) -> [f64; 3] {
    [
        f64::from(width) * 0.95,
        ridge * 0.55,
        f64::from(depth) * 0.75 + f64::from(width) * 0.25,
    ]
}

/// The aim `hall` looks from, and why it is not the building's own middle.
///
/// # Invariants
///
/// **A hall is sixteen metres deep and ten wide, and a camera that aims at its plan centre does not
/// put it in the middle of the frame.** Measured at this scene's own lens the silhouette stands
/// **148 px of a 1280 frame to the left of centre** — measured at 1280 by 784, which the frame above
/// then grew to 1328 by 816 — which is what left `hall` clipped on its left edge while the rest of
/// the frame fitted exactly. Only the yaw moves: the eye, the height and the distance are the ones
/// the square was composed with.
///
/// **The shift was fitted from two renders, because the geometry under-predicted it by 2.5x.** The
/// arithmetic says `0.2316 * 22.2183 * 0.314882` = 1.62 m along `[0.824289, 0, -0.566276]`, and
/// applying it moved the silhouette **356 px the wrong way** — the projection's right axis is the
/// negative of the one that was reasoned about. What is written here is the measured correction:
/// `-1.5473 * 148 / 356` = **0.6457 m along the other side**, and a reader who recomputes the
/// arithmetic will not arrive at it, which is the point of writing that it was measured.
///
/// **The height is 1.0643 m lower than the ridge puts it, for the same reason.** The silhouette sits
/// 29 px below the middle of an 816-row frame — the hall's ground line is below its ridge and the
/// aim is above both — and `57.8 / 408 * 22.2183 * 0.338055` brings it to the middle.
const HALL_AIM: [f64; 3] = [-0.5323, 2.09578, 0.36565];

/// The aim `hall_large` looks from, and why it is not the building's own middle.
///
/// # Invariants
///
/// **The same measurement on a longer building, and a longer correction.** At 14 by 20 the
/// silhouette stands **163.5 px of a 1264 frame to the left of centre**, so the correction is
/// `-2.5774 * 163.5 / 453` = **0.9303 m** along `[0.812045, 0, -0.583737]`, fitted the same way and
/// for the same reason. The target keeps the height the ridge puts it at, so the eye, the height
/// and the distance are untouched and only the yaw moves.
const HALL_LARGE_AIM: [f64; 3] = [-0.75545, 3.85308, 0.54306];

/// The image `hall` is drawn into, in pixels.
///
/// # Why this frame is this shape
///
/// A hall is the longest thing in this gallery, and the measured number says how long. Reading
/// every id the frame carries, at `FRAC_PI_3 * 1.30` into the pipeline's square the silhouette
/// stands `x 169..=750` by `y 361..=715`: **f_w 0.5679 by f_h 0.3460**, and the shape of the world
/// those describe is the frame's own aspect times their ratio — **1.641 to one**.
///
/// So the frame takes those proportions — **1.63 to one**, at **1328 by 816**, and the lens is worked
/// out against it. **The building is not drawn smaller in this frame: it is the same 0.89 of the
/// height it was asked for, out of a picture 1,083,648 pixels rather than 1,048,576**.
///
/// **784 rows was a row short and the render said so.** At 1280 by 784 the silhouette's own bottom —
/// the hall's wall base on the ground — carried **31 pixels on the frame's last row**, while the top
/// had 56 of margin. Raising the frame to 816 rather than shortening the lens is the cheaper of the
/// two corrections by a factor of eight in fill, and it is also the one that leaves the plate alone:
/// a lens change moves the sight line over every window sill, and a taller frame does not.
const HALL_FRAME: (i64, i64) = (1328, 816);

/// The full **vertical** angle `hall` is drawn through, in radians.
///
/// # The arithmetic
///
/// ```text
/// tan(new / 2) = tan(1.30 * PI/3 / 2) * 0.3460 / 0.89
///              = 0.810651 * 0.388764 = 0.315181
/// new = 2 * atan(0.315181) = 0.60998 rad = FRAC_PI_3 * 0.5825
/// ```
///
/// **0.6222 and not 0.5825**, and it is the render that says so rather than the arithmetic. The sweep
/// that fixed the target read the silhouette as 0.89 of the height; drawn at that lens the hall's own
/// wall base carried **31 px of the last row at 784 and 33 px at 816**, so the sweep under-read it by
/// about 3 % and a further turn of the lens closes the difference. `0.5825` scaled by `0.9314` puts
/// the silhouette's 677 rows between 69 and 746 of 816.
///
/// **89 % and not 90 %**, because the roof's soffit and its four fascias stand outside the skin they
/// belong to and a hall's own soffit ran off the **left edge** of the square this replaces — the
/// square cut `roof 0, the soffit under plane 1` at x 0 and carried **47 pixels of building** on
/// that edge. The eye did not move; only the yaw and the aim's height did, and the floor this frame
/// is about is seen through the hall's own windows from in front of them either way.
const HALL_FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.6222;

/// The image `hall_large` is drawn into, in pixels.
///
/// # Why this frame is this shape
///
/// The larger hall is longer and taller, and its proportion is not the smaller one's. Read the same
/// way at the same measurement lens it stands `x 59..=820` by `y 288..=778` — **f_w 0.7439 by
/// f_h 0.4790**, a world aspect of **1.553 to one**. So this frame is **1.55 to one** at 1264 by
/// 816, 1,031,424 pixels.
///
/// **The square it replaces was cutting it badly.** Its left column held **533 pixels of building**
/// and `mass 0, outer wall over edge 2` ran from x 0, so this is the frame that stops the cut.
const HALL_LARGE_FRAME: (i64, i64) = (1264, 816);

/// The full **vertical** angle `hall_large` is drawn through, in radians.
///
/// # The arithmetic
///
/// ```text
/// tan(new / 2) = tan(1.30 * PI/3 / 2) * 0.4790 / 0.89
///              = 0.810651 * 0.538202 = 0.436331
/// new = 2 * atan(0.436331) = 0.82201 rad = FRAC_PI_3 * 0.7850
/// ```
///
/// **The eye and the target are this scene's own and did not move** — `close_and_low` stands lower
/// and closer than the shared three-quarter view so that the building fills the frame rather than
/// the roof, and that reasoning is a statement about the subject, not about the frame's shape.
const HALL_LARGE_FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.7850;

/// The parts of a planned and expanded hall: every packed point list its document draws, and the
/// size its frame follows.
///
/// # Invariants
///
/// **Nothing here is a function of a dial of presentation.** Every field is the building's own,
/// which is what lets a lens or an aim edit read these back whole and re-run the document stage
/// alone; the ground is among them for the same reason, because it is cut from the plan and not
/// from the picture.
#[derive(Clone)]
pub struct HallParts {
    walls: PackedList,
    roof: PackedList,
    floors: PackedList,
    ground: PackedList,
    width: f32,
    depth: f32,
    ridge: f64,
    extent: f64,
}

/// Builds the parts of a planned hall from its expansion.
///
/// # Invariants
///
/// **The identity table is numbered from one**, so a corner's `authored_id` is its element's own
/// row and `0` stays reserved for the ground, and **the ridge and the extent are the starter's**,
/// which is what makes two sizes two pictures of one building rather than one picture with a
/// bigger box in it.
pub fn parts(starter: &Starter, building: &Building, built: &BuildingMesh) -> HallParts {
    // **Every mesh the frame draws carries its elements' ids**, so a pixel of this house can be asked what it
    // is: the walls name their edges, the hip names its skins and its fascia.
    let table = Identity::of(built, 1);
    let walls = packed_where(built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    // The starter's own foundation plate: a mass is a wall ring **and** a plate, so this hall has always
    // authored one and this is the mesh that draws it.
    let floors = packed_where(built, &table, |element| {
        matches!(element, building::Element::Slab(_, _))
    });
    let roof = packed_where(built, &table, |element| {
        matches!(element, building::Element::Roof(_, _))
    });
    HallParts {
        walls,
        roof,
        floors,
        ground: crate::ground::mesh(building, 140, 420.0, |_, _| 0.0),
        width: starter.width,
        depth: starter.depth,
        // The ridge is the number the frame has to fit: the wall top plus the style's own rise.
        ridge: f64::from(starter.height + starter.rise),
        extent: f64::from(starter.width.max(starter.depth)) * 1.3,
    }
}

/// The picture a hall is drawn into: the eye, the frame, the lens and the aim.
pub struct HallView {
    eye: fn(f32, f32, f64) -> [f64; 3],
    image: (i64, i64),
    field_of_view: f64,
    aim: [f64; 3],
}

/// The view [`hall_at`] draws from: [`three_quarter`] at this file's own frame, lens and aim.
fn three_quarter_view() -> HallView {
    HallView {
        eye: three_quarter,
        image: HALL_FRAME,
        field_of_view: HALL_FIELD_OF_VIEW,
        aim: HALL_AIM,
    }
}

/// The view [`hall_large`] draws from: [`close_and_low`] at that hall's own four numbers.
fn close_and_low_view() -> HallView {
    HallView {
        eye: close_and_low,
        image: HALL_LARGE_FRAME,
        field_of_view: HALL_LARGE_FIELD_OF_VIEW,
        aim: HALL_LARGE_AIM,
    }
}

/// The view [`hall_at`] draws from, at a scaled lens and a moved aim.
///
/// # Invariants
///
/// **`1.0` and `[0.0; 3]` are [`three_quarter_view`] exactly**, because a multiplication by one and
/// an addition of zero are the two operations that leave an `f64` alone, and a program that holds
/// its dials as `f32` cannot hand over this file's own constants without rounding them. The dials
/// are a scale and an offset for that reason, which is the shape `alpine_oak`'s `envelope` and
/// `key_position` dials already have.
pub fn hall_view(lens: f64, aim: [f64; 3]) -> HallView {
    let own = three_quarter_view();
    HallView {
        field_of_view: own.field_of_view * lens,
        aim: [
            own.aim[0] + aim[0],
            own.aim[1] + aim[1],
            own.aim[2] + aim[2],
        ],
        ..own
    }
}

/// Builds the hall scene for a footprint, a storey count and a view.
fn hall_framed(width: f32, depth: f32, storeys: usize, view: &HallView) -> Value {
    let starter = HALL.starter(width, depth, storeys);
    let building = document_at(width, depth, storeys);
    let built = building
        .build()
        .expect("the starter house expands: its outline is a quad and its roof's rows are paired");
    hall_document_over_parts(&parts(&starter, &building, &built), view)
}

/// Builds the hall scene's document from parts already made.
///
/// # Invariants
///
/// **Every hall document is written here** — by [`hall_at`], by [`hall_large`] and by a lichen
/// program alike. A document assembled anywhere else would be a second copy of this scene, and a
/// copy is exactly what a cached stage may not be.
pub fn hall_document_over_parts(parts: &HallParts, view: &HallView) -> Value {
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
        ("name".to_owned(), Value::String("hall".to_owned())),
        // The one field here that is about the picture rather than what is in it: a hall is a long
        // building, and a long building wants a long frame. A scene that names no frame is drawn
        // into the pipeline's own square, so this is a decision rather than a default.
        ("frame".to_owned(), frame(view.image.0, view.image.1)),
        (
            "cameras".to_owned(),
            // A three-quarter view from the near corner, high enough that the roof reads as a
            // volume rather than as a silhouette: the eave line and the ridge are both visible,
            // which is what says the roof has a soffit and a thickness. **The eye and the target did
            // not move**; the lens is the free variable, and it is free because the frame is now the
            // silhouette's own shape rather than the pipeline's square.
            Value::List(vec![camera(
                (view.eye)(parts.width, parts.depth, parts.ridge),
                view.aim,
                view.field_of_view,
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
                    parts.extent,
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
                mesh("walls", "Triangles", point.clone(), parts.walls.clone()),
                mesh("roof", "Triangles", point.clone(), parts.roof.clone()),
                mesh("floors", "Triangles", point, parts.floors.clone()),
                mesh(
                    "ground",
                    "Triangles",
                    plain.point_schema(),
                    parts.ground.clone(),
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

/// The air over the hall: a clear afternoon, and **the light the sun is not**.
///
/// The numbers are the ones this scene's own sky law carried — zenith, horizon, ground and the sun's
/// bloom — read out of it and handed to one `Atmosphere`, which bakes them into the map the shading reads
/// its sky term from. A hall whose shadows are black is a hall with no air, not a hall with the wrong
/// colours. The sun's bearing is the one the sky law carried, and it is not the sun light's own: the
/// glow is where the sun *is drawn*, and the light is where the shadows fall.
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
