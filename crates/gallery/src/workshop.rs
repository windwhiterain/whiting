//! The workshop scene: two storeys, one building, and a floor between them.
//!
//! # What this frame exists to show
//!
//! The shed proved a facade and the yard proved a site, and both were **one storey**. This one is the
//! crate's first building with a second storey: a 6 × 9 workshop whose gable end carries **two rows of
//! openings**, and whose first floor is a **plate derived from the mass's own inner loop**.
//!
//! # What the frame shows, and what it does not
//!
//! It shows the two storeys: three windows above and a window, an **open doorway** and a window below. It
//! does **not** show the plate: the mezzanine sits at 2.2 m, so its underside is the doorway's own ceiling
//! and about seven centimetres of it are visible through the opening at this angle. That the plate exists,
//! is closed, and is inset by the wall's own thickness on all four sides is measured by
//! loor_probe, not argued by the picture.
//!
//! Run it with `cargo run -p whiting-device-rasterize --example render -- workshop out/workshop`.

use definition::Value;
use scene::standard::Stage;

use building::derive::{self, RoofStyle, Starter, floor_slab};
use building::grid::{FaceRef, Rhythm};
use building::openings::{Inset, Opening};
use building::ops;
use building::parts::{Pin, Placement, Template};
use building::{Building, Mass, MassId, PlanNode};

use crate::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use crate::instancing::standing;
use crate::kit::{
    camera, flux_for_irradiance, instance, instances, light_with_radius, mesh, mesh_material,
    offset_by, post_processes, screen_material, shaded_point_schema, surface,
};
use crate::shaders::post_shader;
use building::expand::part_frame;
use building::identity::Identity;

/// The workshop: two storeys, a shallow gable, and enough wall for three bays a storey.
const STARTER: Starter = Starter {
    width: 6.0,
    depth: 9.0,
    height: 5.6,
    wall: 0.32,
    slab: 0.5,
    overhang: 0.45,
    sink: 0.12,
    rise: 1.5,
    roof_thickness: 0.2,
    roof: RoofStyle::Gable,
    seed: 11,
};

/// Where the mezzanine sits, and how thick the plate is. **Its underside is below the doorway's head on
/// purpose**: a first floor above the door cannot be seen through it, and the plate is the one interior
/// surface this frame exists to show.
const FLOOR_AT: f32 = 2.2;
const FLOOR_THICK: f32 = 0.22;

/// The wall the openings are cut into: the gable end, which the camera looks at.
const FRONT_EDGE: usize = 2;

/// How tall a shutter is, and so how tall the windows it fills are.\nconst SHUTTER_HEIGHT: f32 = 0.85;\n\n/// How many bays across and how many storeys, which is what makes the openings bay-mounted per storey.
const BAYS: usize = 3;

/// How tall a shutter is, and so how tall every window it fills.
const SHUTTER_HEIGHT: f32 = 0.85;
const STOREYS: usize = 2;

/// Where the sun stands: front-left and high, so the gable end is the lit face and the openings read as
/// depth rather than as paint.
const SUN: [f64; 3] = [-15.0, 30.0, 22.0];

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

/// Returns a shutter: a panel `width` across, `height` tall, `thickness` through, standing at its own
/// origin.
///
/// # Invariants
///
/// **It is the one template the scene instances**, so its plan starts at the origin and runs `+x` along the
/// wall and `+z` through it: a placement's frame puts that origin on the wall's outer face at the hole's sill,
/// and the panel then stands in the hole itself.
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

/// Builds the workshop's document, and returns it with the template its shutters are drawn from.
///
/// # Invariants
///
/// **The doorway is the only opening without a shutter**, and that is the scene's argument: an open doorway
/// is the one place a viewer can see into the building, and what is inside it is the plate the floor slice
/// derived.
/// Returns the workshop's document and the template its shutters come from.
pub fn workshop_document() -> (Building, usize) {
    let mut skeleton = derive::starter(&STARTER).skeleton;
    let mass = MassId(0);
    let front = FaceRef::new(mass, FRONT_EDGE);

    // The plate first: it is derived from the mass's inner loop, so it must be authored before the walls are
    // cut — the derivation reads the outline and the wall thickness, not the openings.
    let mass_now = skeleton.masses[0].clone();
    let floor = floor_slab(&mass_now, mass, FLOOR_AT, FLOOR_THICK)
        .expect("the first floor is inside the building");
    skeleton.slabs.push(floor);

    for face in skeleton.faces_of(mass) {
        ops::split_face(
            &mut skeleton,
            face,
            &Rhythm::Even { count: BAYS },
            &Rhythm::Even { count: STOREYS },
        );
    }

    // The ground floor: a doorway in the middle bay and a window either side of it.
    ops::add_opening(
        &mut skeleton,
        Opening::new(front, 0, 1, Inset::new(0.28, 0.28, 0.0, 0.75)),
    )
    .expect("the middle bay of the ground storey is in the grid");
    for column in [0, 2] {
        ops::add_opening(
            &mut skeleton,
            Opening::new(
                front,
                0,
                column,
                Inset::new(0.34, 0.34, 1.15, SHUTTER_HEIGHT),
            ),
        )
        .expect("the side bays of the ground storey are in the grid");
    }
    // The first floor: three windows, the middle one over the doorway.
    for column in 0..BAYS {
        ops::add_opening(
            &mut skeleton,
            Opening::new(
                front,
                1,
                column,
                Inset::new(0.34, 0.34, 0.95, SHUTTER_HEIGHT),
            ),
        )
        .expect("every bay of the first storey is in the grid");
    }

    // One shutter per window, and **one template behind all five**: the scene draws it once and stands it
    // five times, which is what this slice exists to show. Each placement's pin is the window's own inset and
    // sill, so a shutter fills the hole it is hung in.
    let template = ops::add_template(&mut skeleton, shutter(1.0, SHUTTER_HEIGHT, 0.05));
    for (row, column, sill) in [
        (0usize, 0usize, 1.15f32),
        (0, 2, 1.15),
        (1, 0, 0.95),
        (1, 1, 0.95),
        (1, 2, 0.95),
    ] {
        ops::place_part(
            &mut skeleton,
            Placement::new(template, front, row, column, Pin::new(0.34, sill)),
        )
        .expect("every window's bay is in the grid");
    }

    (Building::new(skeleton), template)
}

/// Builds the workshop scene.
pub fn workshop() -> Value {
    let (building, shutters) = workshop_document();
    let built = building
        .build()
        .expect("the workshop expands: its outline is a quad, its grid tiles every face, and its plate is closed");
    // The scene is the phase's own acceptance in a picture, so it refuses to draw a document the crate's
    // validator would not.
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "the workshop's own document must be clean, and it is not: {violations:?}"
    );

    // **Every mesh the frame draws carries its elements' ids**, so a pixel of this workshop can be asked what
    // it is. The shutters are one mesh drawn five times, so they carry their **template's** local ids and each
    // placement's material offsets them onto the rows a bake would have written.
    let table = Identity::of(&built, 1);
    let shells = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    let roofs = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Roof(_, _))
    });
    let floors = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Slab(_, _))
    });
    let skeleton = &building.skeleton;
    let held = skeleton
        .parts
        .template(shutters)
        .expect("the shutter template is in the document")
        .mesh()
        .expect("the shutter expands on its own");
    let local = Identity::of(&held, 1);
    let panel = packed_where(&held, &local, |_| true);
    // **The bases are read from the baked table**, not copied from a probe's printout: a placement's offset is
    // the least id the table gives an element of that placement, so the template's local ids land on exactly
    // the rows a bake would have written.
    let mut hung: Vec<(([f64; 3], [f64; 3], [f64; 3]), i64)> = Vec::new();
    for (index, placement) in skeleton.parts.placements().iter().enumerate() {
        if placement.template != shutters {
            continue;
        }
        let frame = part_frame(skeleton, index)
            .expect("the wall offsets")
            .expect("every shutter's slot is in the grid");
        let base = table
            .rows()
            .iter()
            .filter(|row| matches!(row.element, building::Element::Part { placement: at, .. } if at == index))
            .map(|row| row.id)
            .min()
            .expect("the placement draws at least one surface");
        hung.push((standing(&frame), i64::from(base) - 1));
    }

    // Rendered wall, sheet metal, a boarded first floor and dry earth. The floor is its own material because
    // it is the building's one interior surface a viewer can see.
    let plaster = || surface([0.72, 0.68, 0.58], 0.0, 0.86, [0.0; 3]);
    let metal = || surface([0.36, 0.38, 0.40], 0.0, 0.42, [0.0; 3]);
    let board = || surface([0.33, 0.23, 0.14], 0.0, 0.78, [0.0; 3]);
    let earth = || surface([0.30, 0.26, 0.19], 0.0, 0.95, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("plaster", plaster as fn() -> Value),
        ("metal", metal),
        ("board", board),
        ("earth", earth),
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
    // **One material set per placement**, because a scene's instance carries no custom values: the offset that
    // makes five shutter pixels answer five different elements lives on the row the instance names, and five
    // rows name five materials that all draw with one shader.
    for (index, (_, offset)) in hung.iter().enumerate() {
        let offset = *offset as f64;
        for (stage, shader) in [
            ("cast", "authored_shadowing"),
            ("depth", "authored_depthing"),
            ("shade", "authored_lighting"),
        ] {
            materials.push(mesh_material(
                &format!("shutter-{index}-{stage}"),
                shader,
                offset_by(board(), offset),
            ));
        }
    }
    // The ground is not an element of the building, and it still writes the output — with the reserved zero.
    for (stage, shader) in [
        ("cast", "plain_shadowing"),
        ("depth", "plain_depthing"),
        ("shade", "plain_lighting"),
    ] {
        materials.push(mesh_material(&format!("ground-{stage}"), shader, earth()));
    }

    let placed =
        |stage: &str| {
            let mut rows = [
                ("shells", "plaster"),
                ("roofs", "metal"),
                ("floors", "board"),
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
            // **Five rows for five shutters, over one mesh**, and each names **its own** material so the offset that
            // separates the five lives on the row rather than on the geometry.
            rows.extend(hung.iter().enumerate().map(
                |(index, ((translation, rotation, scale), _))| {
                    instance(
                        &format!("shutter-{index}-{stage}"),
                        "shutters",
                        *translation,
                        *rotation,
                        *scale,
                        &format!("shutter-{index}-{stage}"),
                    )
                },
            ));
            rows
        };

    // **Two point schemas, because there are two kinds of surface.** Every authored mesh — the shell, the
    // roofs, the plate and the shutters — carries `authored_id`; the ground carries no id at all and writes the
    // reserved zero from a constant.
    let extras = building_extras();
    let point = extras.point_schema();
    let plain = not_authored_extras();

    Value::structure([
        ("name".to_owned(), Value::String("workshop".to_owned())),
        (
            "cameras".to_owned(),
            // Close and low, square on to the gable end: both rows of openings have to be in frame, and a
            // high camera would show the roof and lose the storey the frame is about.
            Value::List(vec![camera(
                [-10.5, 3.2, 13.5],
                [0.0, 2.6, 0.0],
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
                13.0,
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
                mesh("floors", "Triangles", point.clone(), floors),
                mesh("shutters", "Triangles", point, panel),
                mesh(
                    "ground",
                    "Triangles",
                    shaded_point_schema(),
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
    let horizon = vec3<f32>(0.87, 0.74, 0.57);
    let zenith = vec3<f32>(0.15, 0.26, 0.48);
    let below = vec3<f32>(0.31, 0.28, 0.21);
    let sun = normalize(vec3<f32>(-15.0, 30.0, 22.0));
    let glow = pow(clamp(dot(normalize(ray), sun), 0.0, 1.0), 8.0) * 0.33;
    let sky = select(mix(horizon, below, clamp(-ray.y * 4.0, 0.0, 1.0)),
                     mix(horizon, zenith, above),
                     ray.y > 0.0);
    return sky + vec3<f32>(1.0, 0.86, 0.62) * glow;
}
"#;

/// What the post stage does to the frame; a fragment, spliced into the post shader's body.
const GRADE: &str = r#"
    if depth >= 1.0 {
        color = sky_at(ray_through(in.uv));
    }
    if depth < 1.0 {
        let ray = ray_through(in.uv);
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp(1.0 - exp(-along_ray * 0.005), 0.0, 0.55);
        color = mix(color, sky_at(vec3<f32>(ray.x, max(ray.y, -0.04), ray.z)), air);
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.20), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.40, 0.0, 1.0);
"#;

/// Returns the workshop's document alone, for a caller that wants to name what a frame shows.
pub fn document() -> Building {
    workshop_document().0
}
