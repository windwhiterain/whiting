//! The yard scene: a site on a slope — two buildings, a walled courtyard with a gate, and the ground
//! the whole thing was seated on.
//!
//! # What this frame exists to show
//!
//! The shed scene proved a facade. This one proves the **site**: that a building on a slope sits on a
//! terrace cut to the highest ground under it, that a wall **follows** the ground it crosses instead of
//! standing on a plane, that a courtyard is several closed boxes meeting at their corners, and that a gate
//! is a gap rather than a hole — all of it authored in one document and seated by one function.
//!
//! # One law for the ground
//!
//! `ground` is called twice in this file and it matters that it is the same function: once by the
//! terrain mesh this scene draws, and once by [`seat`] writing the ground into the document. Two
//! different laws would draw a slope with buildings hovering over it or buried in it, and the picture
//! would still look plausible — the one failure a render cannot show and a shared function cannot have.
//!
//! Run it with `cargo run -p whiting-device-rasterize --example render -- yard out/yard`.

use definition::Value;
use scene::standard::Stage;

use building::derive::{self, RoofStyle, Starter};
use building::ops;
use building::parts::{Pin, Placement, Target, Template};
use building::site::seat;
use building::wall::{Wall, WallId};
use building::{Building, BuildingSkeleton, Element, Mass, MassId, PlanNode};

use crate::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use crate::geometry::heightfield;
use crate::instancing::standing;
use crate::kit::{
    camera, flux_for_irradiance, instance, instances, light_with_radius, mesh, mesh_material,
    offset_by, post_processes, screen_material, shaded_point_schema, surface,
};
use crate::shaders::post_shader;
use building::identity::Identity;

/// The ground the site stands on, in metres: it falls one unit in eleven as `x` rises.
///
/// # Invariants
///
/// **The terrain and the seat both call this**, so what the frame draws is what the document was seated
/// on. The camera looks up the slope, which is why the fall reads as depth rather than as a tilt.
fn ground(x: f64, _z: f64) -> f64 {
    0.16 * x
}

/// The two buildings, as starters: a shed and a smaller store up the slope.
const SHED: Starter = Starter {
    width: 5.0,
    depth: 8.0,
    height: 2.8,
    wall: 0.25,
    // A deep foundation, because this site is a slope: the terrace is cut to the highest ground under
    // each building, so its downhill edge stands the relief above the ground and a slab carries it down.
    slab: 1.20,
    overhang: 0.40,
    sink: 0.10,
    rise: 1.30,
    roof_thickness: 0.16,
    roof: RoofStyle::Gable,
    seed: 5,
};

const STORE: Starter = Starter {
    width: 4.0,
    depth: 5.0,
    height: 2.4,
    wall: 0.25,
    slab: 1.20,
    overhang: 0.35,
    sink: 0.10,
    rise: 1.10,
    roof_thickness: 0.16,
    roof: RoofStyle::Gable,
    seed: 9,
};

/// How far up the slope the store stands from the shed.
const STORE_OFFSET: f32 = 14.0;

/// Where the sun stands: high and to the front-left, so the courtyard's inside faces are lit and the
/// ground's slope shows as shading rather than as a silhouette.
const SUN: [f64; 3] = [-16.0, 30.0, 24.0];

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

/// The site's own outline: a courtyard round the shed, open at one gate.
const YARD: f32 = 6.5;

/// The gate's own width and height: the 3.2 m opening, and a leaf a little lower than the 1.9 m wall.
const GATE_WIDTH: f32 = 3.2;
const GATE_HEIGHT: f32 = 1.7;

fn node(x: f32, z: f32) -> PlanNode {
    PlanNode::new([x, z], 0.0)
}

/// Returns the site's document: two derived buildings and a five-run courtyard with a gate, all in one
/// skeleton, none of it yet on the ground.
fn site() -> BuildingSkeleton {
    let shed = derive::starter(&SHED).skeleton;
    let mut store = derive::starter(&STORE).skeleton;
    // The second building moves up the slope: its mass, its roof, and the roof's host all shift together,
    // because a roof that named the wrong mass would stand on the first building's walls.
    for mass in &mut store.masses {
        for point in &mut mass.outline {
            point.position[0] += STORE_OFFSET;
        }
    }
    for roof in &mut store.roofs {
        for eave in &mut roof.eaves {
            eave.position[0] += STORE_OFFSET;
        }
        for crest in &mut roof.crest {
            crest.position[0] += STORE_OFFSET;
        }
    }
    let mut skeleton = shed;
    skeleton.masses.push(store.masses.remove(0));
    let mut store_roof = store.roofs.remove(0);
    store_roof.host = MassId(1);
    skeleton.roofs.push(store_roof);

    // The courtyard: four sides and a gate gap in the south one. The gate is where no wall was authored.
    skeleton.walls = vec![
        Wall::new(node(-YARD, -YARD), node(YARD, -YARD), 0.35, 1.9),
        Wall::new(node(YARD, -YARD), node(YARD, YARD), 0.35, 1.9),
        Wall::new(node(YARD, YARD), node(1.6, YARD), 0.35, 1.9),
        Wall::new(node(-1.6, YARD), node(-YARD, YARD), 0.35, 1.9),
        Wall::new(node(-YARD, YARD), node(-YARD, -YARD), 0.35, 1.9),
    ];
    // **The gate is a part hung on a wall, not the gap itself.** The leaf is one template placed at the end
    // of the run that stops at the opening, and it closes the 3.2 m beyond it — the one shape a bay cannot
    // hold, which is why the placement target had to learn to be a wall run.
    let stops_at_the_gate = skeleton.walls[2].length();
    let leaf = ops::add_template(&mut skeleton, gate_leaf(GATE_WIDTH, GATE_HEIGHT, 0.08));
    ops::place_part(
        &mut skeleton,
        Placement::on_wall(leaf, WallId(2), Pin::new(stops_at_the_gate, 0.0)),
    )
    .expect("the run that stops at the gate is in the document");
    skeleton
}

/// Returns a gate leaf: a panel `width` across, `height` tall, `thickness` through, standing at its own
/// origin and running `+x`.
///
/// # Invariants
///
/// Its plan starts at the origin so the placement's frame — the run's end, on the wall's centre line — puts
/// its hinge there and the panel swings across the opening beyond. The panel is **centred on the wall's
/// thickness** by the frame's `inward`, so a gate in a 0.35 m wall does not stand proud of either face.
fn gate_leaf(width: f32, height: f32, thickness: f32) -> Template {
    let slab = 0.03;
    let outline = [
        [0.0, -thickness * 0.5],
        [width, -thickness * 0.5],
        [width, thickness * 0.5],
        [0.0, thickness * 0.5],
    ]
    .into_iter()
    .map(|corner| PlanNode::new(corner, slab))
    .collect();
    Template::new(
        "gate",
        vec![Mass::new(outline, height).with_wall(0.02).with_slab(slab)],
        Vec::new(),
    )
}

/// Builds the yard scene.
pub fn yard() -> Value {
    let mut skeleton = site();
    let seated = seat(
        &mut skeleton,
        |plan| ground(plan[0] as f64, plan[1] as f64) as f32,
        0.0,
    );
    // **The scene refuses to draw a site it could not seat.** A mass the ground never answered for is
    // left at its authored elevation, which on this slope means floating or buried — and the frame is
    // exactly where that would hide. So the count is checked here rather than trusted.
    assert!(
        seated.len() == 2 && seated.walls == 5,
        "the site must be seated: {} masses and {} walls, not 2 and 5",
        seated.len(),
        seated.walls
    );
    let built = Building::new(skeleton.clone())
        .build()
        .expect("the seated site expands");

    // **Every mesh the frame draws carries its elements' ids** — the shells, the roofs, the walls and the gate —
    // so a pixel of this yard can be asked what it is. The gate is one template drawn once, so its mesh carries
    // the **template's** local ids and its material offsets them onto the row a bake would have written.
    let table = Identity::of(&built, 1);
    let shells = packed_where(&built, &table, |element| {
        matches!(element, Element::Mass(_, _))
    });
    let roofs = packed_where(&built, &table, |element| {
        matches!(element, Element::Roof(_, _))
    });
    let stone = packed_where(&built, &table, |element| {
        matches!(element, Element::Wall(_, _))
    });
    // **The gate is drawn from its template, once, and stood by an instance.** A scene that filtered the
    // baked mesh for parts would draw every leaf the document holds; this one draws the one shape and the
    // one transform that puts it in the opening.
    let gate = skeleton
        .parts
        .template(0)
        .expect("the gate's template is in the document");
    // **The gate's offset, read from the baked table**: the least id the table gives an element of the gate's
    // placement, less one, so the template's local ids land on exactly the rows a bake would have written.
    let gate_at = (0..skeleton.parts.placements().len())
        .find(|index| {
            skeleton
                .parts
                .placement(*index)
                .is_some_and(|placement| placement.target == Target::Wall { wall: WallId(2) })
        })
        .expect("the gate is placed on the run that stops at the opening");
    let gate_offset = i64::from(
        table
            .rows()
            .iter()
            .filter(
                |row| matches!(row.element, Element::Part { placement: at, .. } if at == gate_at),
            )
            .map(|row| row.id)
            .min()
            .expect("the gate draws at least one surface"),
    ) - 1;
    let gate_local = Identity::of(&gate.mesh().expect("the gate expands on its own"), 1);
    let gate_panel = packed_where(
        &gate.mesh().expect("the gate expands on its own"),
        &gate_local,
        |_| true,
    );
    let gate_standing = standing(
        &building::expand::part_frame(&skeleton, gate_at)
            .expect("the wall offsets")
            .expect("the gate's slot is in the domain"),
    );

    // Weatherboard, sheet metal, dry-stone and earth. Four materials for four things the frame has to tell
    // apart: what a surface *is* stays the shader's business.
    let board = || surface([0.36, 0.27, 0.18], 0.0, 0.86, [0.0; 3]);
    let metal = || surface([0.34, 0.37, 0.40], 0.0, 0.38, [0.0; 3]);
    let rubble = || surface([0.66, 0.65, 0.60], 0.0, 0.92, [0.0; 3]);
    let earth = || surface([0.24, 0.25, 0.16], 0.0, 0.95, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("board", board as fn() -> Value),
        ("metal", metal),
        ("rubble", rubble),
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
    for (stage, shader) in [
        ("cast", "authored_shadowing"),
        ("depth", "authored_depthing"),
        ("shade", "authored_lighting"),
    ] {
        materials.push(mesh_material(
            &format!("gate-{stage}"),
            shader,
            offset_by(board(), gate_offset as f64),
        ));
    }
    // The ground is not an element of the site, and it still writes the output — with the reserved zero.
    for (stage, shader) in [
        ("cast", "plain_shadowing"),
        ("depth", "plain_depthing"),
        ("shade", "plain_lighting"),
    ] {
        materials.push(mesh_material(&format!("ground-{stage}"), shader, earth()));
    }

    let placed = |stage: &str| {
        let mut rows = [
            ("shells", "board"),
            ("roofs", "metal"),
            ("stone", "rubble"),
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
        .collect::<Vec<_>>();
        rows.push(instance(
            &format!("gate-{stage}"),
            "gate",
            gate_standing.0,
            gate_standing.1,
            gate_standing.2,
            &format!("board-{stage}"),
        ));
        rows
    };

    // **Two point schemas**: every authored mesh carries uthored_id; the ground carries none and writes
    // the reserved zero from a constant.
    let extras = building_extras();
    let point = extras.point_schema();
    let plain = not_authored_extras();

    Value::structure([
        ("name".to_owned(), Value::String("yard".to_owned())),
        (
            "cameras".to_owned(),
            // Up the slope and to one side, high enough to see over the yard wall into the courtyard and
            // low enough that the gate gap reads as a way in rather than a step in the wall.
            Value::List(vec![camera(
                [-13.0, 6.4, 15.5],
                [2.0, 1.2, 0.0],
                std::f64::consts::FRAC_PI_3 * 0.82,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![light_with_radius(
                "sun",
                SUN,
                [1.0, 0.87, 0.70],
                flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                16.0,
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
                mesh("shells", "Triangles", point.clone(), shells),
                mesh("roofs", "Triangles", point.clone(), roofs),
                mesh("stone", "Triangles", point.clone(), stone),
                mesh("gate", "Triangles", point, gate_panel),
                mesh(
                    "ground",
                    "Triangles",
                    shaded_point_schema(),
                    heightfield(160, 42.0, |x, z| ground(x, z)),
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

/// The sky this scene draws, written once so the haze and the background agree.
const SKY: &str = r#"
fn sky_at(ray: vec3<f32>) -> vec3<f32> {
    let above = pow(clamp(ray.y, 0.0, 1.0), 0.55);
    let horizon = vec3<f32>(0.86, 0.73, 0.56);
    let zenith = vec3<f32>(0.14, 0.25, 0.47);
    let below = vec3<f32>(0.30, 0.27, 0.21);
    let sun = normalize(vec3<f32>(-16.0, 30.0, 24.0));
    let glow = pow(clamp(dot(normalize(ray), sun), 0.0, 1.0), 8.0) * 0.32;
    let sky = select(mix(horizon, below, clamp(-ray.y * 4.0, 0.0, 1.0)),
                     mix(horizon, zenith, above),
                     ray.y > 0.0);
    return sky + vec3<f32>(1.0, 0.86, 0.62) * glow;
}
"#;

/// Returns how far a light position stands from the scene's own middle.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// What the post stage does to the frame: the same shape as the shed's, with the air of a wider view.
const GRADE: &str = r#"
    if depth >= 1.0 {
        color = sky_at(ray_through(in.uv));
    }
    if depth < 1.0 {
        let ray = ray_through(in.uv);
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp(1.0 - exp(-along_ray * 0.005), 0.0, 0.60);
        color = mix(color, sky_at(vec3<f32>(ray.x, max(ray.y, -0.04), ray.z)), air);
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.20), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.40, 0.0, 1.0);
"#;

/// Returns the yard's document **as the scene builds it**: the site, seated on the ground it crosses.
///
/// # Invariants
///
/// It is the same document the scene draws — the seat is part of the picture, so a caller naming what a frame
/// shows must name it after the seating rather than before.
pub fn document() -> Building {
    let mut skeleton = site();
    seat(
        &mut skeleton,
        |plan| ground(plan[0] as f64, plan[1] as f64) as f32,
        0.0,
    );
    Building::new(skeleton)
}
