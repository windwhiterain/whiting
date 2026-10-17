//! The shed scene: the proof that a facade is a grid, holes in it, and parts standing in them.
//!
//! The house beside this one proves a building is a volume with a roof. This one proves the other
//! half of the phase: the same derivation, then a **rhythm** authored on its faces, **two windows and
//! a doorway cut through** the wall by the openings, and **two parts** — a door leaf and a canopy —
//! standing in the bays by slot. Every number a viewer sees in this frame came out of the document:
//! the window reveals are the wall's own thickness, the door's head is the hole's head, and the
//! canopy's underside is a part's own face.
//!
//! Run it with `cargo run -p whiting-device-rasterize --example render -- shed out/shed`.

use definition::Value;
use scene::standard::Stage;

use building::derive::{self, RoofStyle, Starter};
use building::grid::{FaceRef, Rhythm};
use building::openings::{Inset, Opening};
use building::ops;
use building::parts::{Anchor, Joint, Pin, Placement, Socket, Template};
use building::{Building, Mass, MassId, PlanNode};

use crate::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use crate::geometry::heightfield;
use crate::kit::{
    camera, flux_for_irradiance, instance, instances, light_with_radius, mesh, mesh_material,
    post_processes, screen_material, surface,
};
use crate::shaders::post_shader;
use building::identity::Identity;

/// What the shed is: a small utilitarian box with a gabled sheet-metal roof.
const STARTER: Starter = Starter {
    width: 5.0,
    depth: 8.0,
    height: 2.8,
    wall: 0.25,
    slab: 0.30,
    overhang: 0.40,
    sink: 0.10,
    rise: 1.30,
    roof_thickness: 0.16,
    roof: RoofStyle::Gable,
    seed: 5,
};

/// The wall the door and the windows are cut into: the **south** face, which is the one the camera
/// looks at, and whose three bays are the facade the frame is about.
const FRONT_EDGE: usize = 2;

/// How many bays the front is divided into, and how many storeys the shed has. One storey on purpose:
/// a shed is a single volume, and a second storey line would divide the door into a part of a door.
const BAYS: usize = 3;
const STOREYS: usize = 1;

/// Where the sun stands, high and to the front so the gable end that carries the door and the two
/// windows is the lit face: the facade is what this scene exists to show.
const SUN: [f64; 3] = [-9.0, 28.0, 24.0];

/// How far the sun's shadow page has to cover. A shed is smaller than a house and its shadow shorter,
/// so the page is tighter and its texels land where the frame is looking.
const SHADOW_EXTENT: f64 = 9.0;

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

/// Returns the door leaf: a panelled door `width` across, `height` tall, `thickness` through.
///
/// The frame is the placement's: `x` across the doorway, `y` up it, `z` into the wall. Its own origin
/// is its floor, so a pin of `(along, 0.0)` stands it on the doorway's own sill.
fn door_leaf(width: f32, height: f32, thickness: f32) -> Template {
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
        "door leaf",
        vec![Mass::new(outline, height).with_wall(0.01).with_slab(slab)],
        Vec::new(),
    )
}

/// Returns the canopy: a slab `width` across, `thickness` thick, reaching `reach` **out of** the wall.
///
/// A placement maps the template's `z` **into** the wall, so a canopy that overhangs the door is
/// authored with its `z` negative: the part hangs from the wall's outer face outwards, which is what
/// a canopy is.
///
/// # Invariants
///
/// **A slab is a room with a very shallow floor, not a slab.** `Mass` is the crate's room model — walls
/// between a soffit and a top, a floor disc between them — so a body whose floor coincides with its
/// top is not a thin solid but a degenerate one: measured, eight facets of no area and the validator
/// saying so. The body here therefore stands its floor strictly between the two, which is what keeps
/// a plate plate-shaped and legal at the same time.
fn canopy(width: f32, thickness: f32, reach: f32) -> Template {
    let slab = thickness * 0.5;
    let floor = slab;
    let outline = [[0.0, -reach], [width, -reach], [width, 0.0], [0.0, 0.0]]
        .into_iter()
        .map(|corner| PlanNode::new(corner, floor))
        .collect();
    Template::new(
        "canopy",
        vec![
            Mass::new(outline, thickness)
                .with_wall(0.02)
                .with_slab(slab),
        ],
        Vec::new(),
    )
    .with_socket(Socket::new("back", (width * 0.5, thickness)))
}

/// Builds the shed's document: the derived shell, the front's rhythm, the door and the two windows,
/// and the two parts standing in it.
///
/// # Invariants
///
/// **Every slot is named, never a coordinate**, so the whole facade moves if the grid changes and
/// nothing here has to be rewritten. The one coordinate the document cannot avoid is the pin that
/// says where inside its bay a part stands, and even that is in metres from the bay's own corner.
pub fn document() -> Building {
    let mut skeleton = derive::starter(&STARTER).skeleton;
    let mass = MassId(0);
    let front = FaceRef::new(mass, FRONT_EDGE);
    let columns = Rhythm::Even { count: BAYS };
    let rows = Rhythm::Even { count: STOREYS };
    for face in skeleton.faces_of(mass) {
        ops::split_face(&mut skeleton, face, &columns, &rows);
    }

    // The middle bay carries the door, the two beside it carry a window each. The insets are the
    // frame's own taste, and they are the only numbers in this scene that are not structural.
    let door = Opening::new(front, 0, 1, Inset::new(0.25, 0.25, 0.0, 0.45));
    let left = Opening::new(front, 0, 0, Inset::new(0.30, 0.30, 1.05, 0.55));
    let right = Opening::new(front, 0, 2, Inset::new(0.30, 0.30, 1.05, 0.55));
    for opening in [door, left, right] {
        ops::add_opening(&mut skeleton, opening).expect("each opening names a bay of the grid");
    }

    // Where the middle bay is, so the leaf and its canopy can be pinned inside it. This is the one
    // place a scene reads a bay: a document would leave it to the expansion.
    let bay = skeleton.bay(front, 0, 1).expect("the grid is there");
    let (_, hole) = skeleton.aperture(0).expect("the doorway was just cut");
    let leaf = ops::add_template(&mut skeleton, door_leaf(1.10, 2.10, 0.05));
    ops::place_part(
        &mut skeleton,
        Placement::new(leaf, front, 0, 1, Pin::new(hole.from - bay.from, 0.0)),
    )
    .expect("the doorway is in a bay of the grid");
    let shelter = ops::add_template(&mut skeleton, canopy(1.45, 0.10, 0.70));
    ops::place_part(
        &mut skeleton,
        // **Hung on the doorway's head, not pinned above it**: the canopy's `back` socket sits on the
        // opening's head line, so moving the door's head moves the canopy with it — the difference
        // between a part that is placed and one that is connected.
        Placement::new(shelter, front, 0, 1, Pin::new(0.0, 0.0))
            .with_joint(Joint::new("back", Anchor::OpeningHead { opening: 0 })),
    )
    .expect("the canopy hangs over the same bay");

    Building::new(skeleton)
}

/// Builds the shed scene.
pub fn shed() -> Value {
    let building = document();
    let built = building
        .build()
        .expect("the shed expands: its outline is a quad, its grid tiles every face, and its parts are closed");

    // **Every mesh the frame draws carries its elements' ids**, so a pixel of this shed can be asked what it
    // is: the walls, the roof's skins and fascia, the door leaf and the canopy all name themselves.
    let table = Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    let roof = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Roof(_, _))
    });
    let door = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Part { placement: 0, .. })
    });
    let canopy = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Part { placement: 1, .. })
    });
    // The scene is the phase's own acceptance in a picture, so it refuses to draw a document the
    // crate's validator would not: an unreported fault is worse than a blank scene.
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "the shed's own document must be clean, and it is not: {violations:?}"
    );

    // Weathered board, sheet metal, and the same board again for the parts: the door is a door
    // because it is wood in a hole, not because a shader knows what a door is.
    let board = || surface([0.45, 0.36, 0.25], 0.0, 0.86, [0.0; 3]);
    let metal = || surface([0.34, 0.37, 0.40], 0.0, 0.38, [0.0; 3]);
    let timber = || surface([0.21, 0.14, 0.09], 0.0, 0.70, [0.0; 3]);
    let earth = || surface([0.30, 0.26, 0.19], 0.0, 0.95, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("board", board as fn() -> Value),
        ("metal", metal),
        ("timber", timber),
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
    // The ground is not an element of the shed, and it still writes the output — with the reserved zero.
    for (stage, shader) in [
        ("cast", "plain_shadowing"),
        ("depth", "plain_depthing"),
        ("shade", "plain_lighting"),
    ] {
        materials.push(mesh_material(&format!("ground-{stage}"), shader, earth()));
    }

    // Every mesh is authored in world coordinates, so its instances carry no transform: the parts
    // were placed by the document, not by the scene.
    let placed = |stage: &str| {
        [
            ("walls", "board"),
            ("roof", "metal"),
            ("door", "timber"),
            ("canopy", "timber"),
            ("ground", "ground"),
        ]
        .into_iter()
        .map(|(label, material)| {
            instance(
                &format!("{label}-{stage}"),
                label,
                [0.0; 3],
                [0.0; 3],
                [1.0; 3],
                &format!("{material}-{stage}"),
            )
        })
        .collect::<Vec<_>>()
    };

    // **Two point schemas**: every authored mesh carries uthored_id, the ground carries none and writes
    // the reserved zero from a constant.
    let extras = building_extras();
    let point = extras.point_schema();
    let plain = not_authored_extras();

    Value::structure([
        ("name".to_owned(), Value::String("shed".to_owned())),
        (
            "cameras".to_owned(),
            // Low and close, so the door, both windows and the canopy are all in frame: the facade is
            // what this scene exists to show, and a high camera would show the roof instead.
            Value::List(vec![camera(
                [-9.4, 4.4, 11.2],
                [0.0, 1.4, 0.0],
                std::f64::consts::FRAC_PI_3 * 0.80,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![light_with_radius(
                "sun",
                SUN,
                [1.0, 0.87, 0.70],
                flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                SHADOW_EXTENT,
                160.0,
            )]),
        ),
        (
            "shaders".to_owned(),
            Value::List({
                let mut shaders = stage_set("authored", "", "", &point, &extras);
                shaders.extend(stage_set("plain", "", "", &plain.point_schema(), &plain));
                shaders.push(post_shader(GRADE, SKY, None));
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
                mesh("door", "Triangles", point.clone(), door),
                mesh("canopy", "Triangles", point, canopy),
                mesh(
                    "ground",
                    "Triangles",
                    plain.point_schema(),
                    heightfield(120, 360.0, |_, _| 0.0),
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

/// The sky this scene draws, written once so the haze and the background agree.
const SKY: &str = r#"
fn sky_at(ray: vec3<f32>) -> vec3<f32> {
    let above = pow(clamp(ray.y, 0.0, 1.0), 0.55);
    let horizon = vec3<f32>(0.86, 0.72, 0.54);
    let zenith = vec3<f32>(0.14, 0.24, 0.46);
    let below = vec3<f32>(0.32, 0.28, 0.22);
    let sun = normalize(vec3<f32>(-9.0, 28.0, 24.0));
    let glow = pow(clamp(dot(normalize(ray), sun), 0.0, 1.0), 8.0) * 0.35;
    let sky = select(mix(horizon, below, clamp(-ray.y * 4.0, 0.0, 1.0)),
                     mix(horizon, zenith, above),
                     ray.y > 0.0);
    return sky + vec3<f32>(1.0, 0.86, 0.62) * glow;
}
"#;

/// What the post stage does to the frame.
///
/// # Invariants
///
/// A **fragment**, not a function: it is spliced into the post shader's own body, so `depth`,
/// `color`, `in.uv` and the kit's `ray_through`/`world_position` are already in scope. The air goes on
/// before the curve and only over the geometry, for the reason the house's grade gives.
const GRADE: &str = r#"
    if depth >= 1.0 {
        color = sky_at(ray_through(in.uv));
    }
    if depth < 1.0 {
        let ray = ray_through(in.uv);
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        // **The air is an exponential and nothing else.** `1 - exp(-L·k)·c` has an asymptote at
        // `1 - c`, not at zero, so a factor written that way fogs a twelve-metre shed by a fifth
        // whatever the distance; measured, that is what made the first frame of this scene read
        // monochrome. Bounded below by the clamp only to keep a very long ray sane.
        let air = clamp(1.0 - exp(-along_ray * 0.006), 0.0, 0.55);
        color = mix(color, sky_at(vec3<f32>(ray.x, max(ray.y, -0.04), ray.z)), air);
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.18), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.38, 0.0, 1.0);
"#;
