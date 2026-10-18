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
//! floor_probe, not argued by the picture.
//!
//! # The one part that stands inside the building
//!
//! A part can stand on **both** faces of a wall ring, and this scene now carries one of each: the five
//! shutters on the **outer** face the camera looks at, and one **workbench** on the **inner** face of the
//! east wall, named by [`FaceRef::inner`]. **The bench is in the picture, and the doorway is the reason**:
//! the camera stands off the building's north-west corner, so the west and north walls show their outer
//! faces and the south wall is nine metres behind the door, while every sight line through the open
//! doorway runs east across the room and arrives at the east wall. A fitting on any other wall's inner
//! face would be in the document and in no frame at all.
//!
//! Run it with `cargo run -p whiting-device-rasterize --example render -- workshop out/workshop`.

use definition::{PackedList, Value};
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
use crate::instancing::{Standing, standing};
use crate::kit::{
    camera, flux_for_irradiance, frame, instance, instances, light_with_radius, mesh,
    mesh_material, offset_by, post_processes, screen_material, shaded_point_schema, surface,
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

/// The wall the bench stands against, as **the rooms** see it rather than as the street does.
///
/// # Invariants
///
/// **It is the one wall the open doorway looks across, and that is the whole of why it was chosen.**
/// `rectangle` winds the plan with the solid on the walker's right, so `edge 0` runs along `+x` at
/// `z = -4.5`, `edge 1` along `+z` at `x = 3.0`, `edge 2` back along `-x` at `z = 4.5` and `edge 3`
/// along `-z` at `x = -3.0`. The camera stands at `[-10.5, 3.2, 13.5]`, off the **north-west**
/// corner, so `edge 3` and `edge 2` are the two walls whose outer faces are in the frame and `edge 0`
/// is nine metres behind the door. The sight lines that get through the door are the remaining
/// direction, and they run **east**: through the hole at `z = 4.18` they reach the inner face of
/// `edge 1` and nothing else. **`Edge 3` is the one face that would be in the document and in no
/// frame at all**, because the camera is looking away from it.
const BENCH_EDGE: usize = 1;

/// How far along its bay the bench stands, in metres, and how far up it.
///
/// # Invariants
///
/// **Both are measured against the room.** The bay is the whole inner loop of `edge 1` — one column
/// and one row, so the bay's own sill is the room's floor and its own start is the inner loop's
/// first node at `z = -4.18` — and `BENCH_ALONG` is read along it. The number in the report is
/// therefore `bay.from + 5.38 = 5.38`, and the bench stands from `z = 1.20` to `z = 2.30`.
///
/// **It sits in the band the doorway can see.** A ray from the eye through the hole's near edge
/// (`x = -0.72`, `z = 4.18`) reaches the inner face at `z = 0.94` and one through the hole's far edge
/// (`x = 0.72`) reaches it at `z = 2.55`, so the wall's inner face is visible between those two and
/// up to `y = 1.65` at the near end. **A 1.10 m bench inside a 1.61 m band** is the fitting's own
/// length the numbers chose, and it leaves a hand's width at each end rather than sitting on an edge
/// of the reveal.
const BENCH_ALONG: f32 = 5.38;

/// The bench's own dimensions, in metres: how long it is along the wall, how deep it stands into the
/// room, and how tall it is.
///
/// **Its boards are built the way a shutter's are** — a mass whose outline stands at the elevation of
/// its own 0.02 m slab, so the template's local origin is its floor and a pin at `up = 0` lands it on
/// the room's — and it is that shared origin which makes one template shape serve both faces of a
/// ring.
const BENCH_LENGTH: f32 = 1.10;
const BENCH_DEPTH: f32 = 0.60;
/// How tall the bench stands, which is the top of its topmost board.
const BENCH_TOP: f32 = 0.94;

const BAYS: usize = 3;

/// How tall a shutter is, and so how tall every window it fills.
const SHUTTER_HEIGHT: f32 = 0.85;
const STOREYS: usize = 2;

/// Where the sun stands: front-left and high, so the gable end is the lit face and the openings read as
/// depth rather than as paint.
const SUN: [f64; 3] = [-15.0, 30.0, 22.0];

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

/// The image this scene is drawn into, in pixels.
///
/// # Why this frame is this shape
///
/// The workshop is **two storeys under a shallow gable**, six metres wide and nine deep with a
/// mezzanine inside it, and the measured number says what that projects as. Reading every id the
/// frame carries at `FRAC_PI_3 * 1.30`, the silhouette stands `x 318..=750` by `y 307..=666` of the
/// pipeline's square — **f_w 0.4223 by f_h 0.3509**, a world aspect of **1.204 to one**.
///
/// So the frame takes those proportions, **1.21 to one** at 1120 by 928: 1,039,360 pixels against
/// the square's 1,048,576.
///
/// **This is the only scene in the gallery whose frame is taller than a square's half**, and the
/// reason is the building rather than the lens: two storeys and a gable read as a block from the
/// ground plate to the ridge, and a block is not a row.
const FRAME_WIDTH: i64 = 1120;
const FRAME_HEIGHT: i64 = 928;

/// The full **vertical** angle this workshop is drawn through, in radians.
///
/// # The arithmetic
///
/// ```text
/// tan(new / 2) = tan(1.30 * PI/3 / 2) * 0.3509 / 0.89
///              = 0.810651 * 0.394270 = 0.319609
/// new = 2 * atan(0.319609) = 0.61809 rad = FRAC_PI_3 * 0.5902
/// ```
///
/// **89 % and not 90 %**, because the workbench standing on the mass's own inner wall face and the
/// roof's soffit both reach past the wall they belong to.
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.5902;

/// The aim this workshop looks from, and why it is not the building's own middle.
///
/// # Invariants
///
/// **The eye did not move and the aim did, because the silhouette fits and is not in the middle.**
/// At this frame and this lens the building measures `x 115..=1109` by `y 0..=806`: **0.8883 of the
/// width and 0.8695 of the height**, with its centre **52.5 px right of and 60.5 px above** the
/// frame's — which is why the roof's own fascia sat on row 0 while 121 rows of ground stood below.
/// `52.5 / 559.5 * 17.1137 * 0.319062` = **0.5124 m** along `[0.789342, 0, 0.613966]` and
/// `60.5 / 463.5 * 17.1137 * 0.319062` = **0.7126 m** of height put it in the middle with 60 rows of
/// margin on every side, **and the lens did not move at all** — a lens correction would have cost a
/// tenth of the fill to fix a fault that was never about the lens.
///
/// **That costs the pitch, and the pitch is what the plate is seen through.** This scene's plate is a
/// mezzanine under a doorway whose head sits below its own underside; the render answers whether it
/// survives.
const AIM: [f64; 3] = [0.40446, 3.3126, 0.31462];

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

/// Returns a board: a mass whose outline runs `x0..x1` by `z0..z1`, standing at elevation `floor` and
/// walled up to `top`.
///
/// # Invariants
///
/// - **`top` is the elevation of the wall's top and not a height above it.** `Mass::new`'s second
///   argument is documented *"The elevation of the wall top"*, so a board whose outline stands above
///   its own `top` is a ring walked **downwards** and every one of its faces comes back wound the other
///   way round — which is not a shape the author sees but a signed volume that reads negative, and the
///   scene's own `validate` assert is what named it.
/// - **A board's bounds reach one slab thickness below its outline**, because `slab` is documented as
///   *"measured down from the outline's elevations"* and `reach` measures the template. A part that is
///   to sit flush on a bay's sill therefore has its outline stand **at** its own slab thickness, which
///   is the shutter's own convention and the reason that convention exists.
fn board(x0: f32, x1: f32, z0: f32, z1: f32, floor: f32, top: f32) -> Mass {
    let outline = [[x0, z0], [x1, z0], [x1, z1], [x0, z1]]
        .into_iter()
        .map(|corner| PlanNode::new(corner, floor))
        .collect();
    Mass::new(outline, top).with_wall(0.01).with_slab(0.02)
}

/// Returns a workbench: a top on two end panels, standing `BENCH_LENGTH` along the wall and
/// `BENCH_DEPTH` into the room.
///
/// # Invariants
///
/// - **Its plan runs `+x` along the wall and `+z` away from it, which is what [`FaceRef::inner`]
///   asks of a template.** The shutter's plan runs the same way and [`FaceRef::new`] puts its `+z`
///   **into** the masonry; **the same spelling on a mass's own inner face puts its `+z` into the
///   room**, because `inward` is the walker's left on every face of a ring and only which way it
///   points differs. One template shape therefore serves both faces, and the face alone decides
///   where the depth goes.
/// - **The two panels stop where the top board's own slab begins**, at `0.84`, so the three solids
///   touch along a plane and none of them overlaps another — a bench built out of volumes that
///   overlapped would be reported for it rather than read as a bench.
/// - **Its own bounds are `x [0, 1.10]`, `z [0, 0.60]`, `y [0, 0.94]`**, and the `0.0` is the point:
///   the template's origin is its floor, which is the room's floor, because the ground plate's top
///   stands at `y = 0` and a pin at `up = 0` lands it there.
fn workbench() -> Template {
    const PANEL: f32 = 0.12;
    const REACH: f32 = 0.10;
    /// The elevation the top board's own outline stands at. Its slab is the 0.02 m below this, and
    /// that slab's underside is where the two panels stop.
    const UNDER_TOP: f32 = 0.86;
    Template::new(
        "workbench",
        vec![
            board(
                0.0,
                PANEL,
                REACH,
                BENCH_DEPTH - REACH,
                0.02,
                UNDER_TOP - 0.02,
            ),
            board(
                BENCH_LENGTH - PANEL,
                BENCH_LENGTH,
                REACH,
                BENCH_DEPTH - REACH,
                0.02,
                UNDER_TOP - 0.02,
            ),
            board(0.0, BENCH_LENGTH, 0.0, BENCH_DEPTH, UNDER_TOP, BENCH_TOP),
        ],
        Vec::new(),
    )
}

/// Builds the workshop's document, and returns it with the templates its parts are drawn from.
///
/// # Invariants
///
/// **The doorway is the only opening without a shutter**, and that is the scene's argument: an open doorway
/// is the one place a viewer can see into the building, and what is inside it is the plate the floor slice
/// derived — and, since this slice, the bench standing on that plate against the wall the doorway looks
/// across.
///
/// **The two faces of the ring get their parts from the two constructors that name them.** The five
/// shutters hang on [`FaceRef::new`] and the bench stands on [`FaceRef::inner`], so every part in this
/// document is on a face the crate can name and the one that is inside is named rather than reached
/// through a struct literal.
/// Returns the workshop's document, the template its shutters come from, and the template its bench
/// comes from.
pub fn workshop_document() -> (Building, usize, usize) {
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

    // **The bench, and the fourth face of a ring.** The grid above covers the outline and nothing
    // else, so the inner loop has no bay to pin into and one is laid on it here — a single column
    // and a single row, which makes the bay the whole inner loop and puts its sill on the room's
    // floor. **A rhythm emits no geometry**, so this half of the document changes what can be
    // pinned and nothing about what is drawn until the part below is placed.
    let inside = FaceRef::inner(mass, BENCH_EDGE);
    ops::split_face(
        &mut skeleton,
        inside,
        &Rhythm::Even { count: 1 },
        &Rhythm::Even { count: 1 },
    );
    let bench = ops::add_template(&mut skeleton, workbench());
    ops::place_part(
        &mut skeleton,
        Placement::new(bench, inside, 0, 0, Pin::new(BENCH_ALONG, 0.0)),
    )
    .expect("the inner loop's own bay is in the grid the split laid on it");

    (Building::new(skeleton), template, bench)
}

/// Builds the workshop scene.
pub fn workshop() -> Value {
    let (building, shutters, bench) = workshop_document();
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
    // **Every part template in the frame is one mesh, drawn once and stood by one instance per
    // placement.** The shutters and the bench differ in how many placements they have and not in
    // what a placement needs, so they go through one list: five rows for the shutters and one for
    // the bench, each carrying its template's local ids and each offset onto the rows a bake wrote.
    let mut parts: Vec<(usize, &str, &str, PackedList, Vec<(Standing, i64)>)> =
        [("shutters", "shutter", shutters), ("bench", "bench", bench)]
            .into_iter()
            .map(|(mesh, label, template)| {
                let held = skeleton
                    .parts
                    .template(template)
                    .expect("every template this scene instances is in its own document")
                    .mesh()
                    .expect("the template expands on its own");
                let local = Identity::of(&held, 1);
                (
                    template,
                    mesh,
                    label,
                    packed_where(&held, &local, |_| true),
                    Vec::new(),
                )
            })
            .collect();
    // **The bases are read from the baked table**, not copied from a probe's printout: a placement's offset is
    // the least id the table gives an element of that placement, so the template's local ids land on exactly
    // the rows a bake would have written.
    for (index, placement) in skeleton.parts.placements().iter().enumerate() {
        let Some((_, _, _, _, hung)) = parts
            .iter_mut()
            .find(|(template, ..)| *template == placement.template)
        else {
            continue;
        };
        let frame = part_frame(skeleton, index)
            .expect("the wall offsets")
            .expect("every placement's slot is in the grid");
        let base = table
            .rows()
            .iter()
            .filter(|row| matches!(row.element, building::Element::Part { placement: at, .. } if at == index))
            .map(|row| row.id)
            .min()
            .expect("the placement draws at least one surface");
        hung.push((standing(&frame), i64::from(base) - 1));
    }
    // The rows the materials and the instances are both written from: one per placement, carrying the
    // mesh it draws, the prefix its label carries, how many of its template's placements it is, and
    // the offset its own material carries.
    let hung: Vec<(&str, &str, usize, Standing, i64)> = parts
        .iter()
        .flat_map(|(_, mesh, label, _, placed)| {
            placed
                .iter()
                .enumerate()
                .map(move |(index, (at, offset))| (*mesh, *label, index, *at, *offset))
        })
        .collect();

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
    // makes a placement's pixels answer that placement's own elements lives on the row the instance names, and
    // two placements of one mesh name two rows that all draw with one shader. The bench's row is here for the
    // same reason as the shutters' five and not because the bench is a different kind of part — it is one
    // placement of one template, which is all a row is.
    for (_, label, index, _, offset) in &hung {
        let offset = *offset as f64;
        for (stage, shader) in [
            ("cast", "authored_shadowing"),
            ("depth", "authored_depthing"),
            ("shade", "authored_lighting"),
        ] {
            materials.push(mesh_material(
                &format!("{label}-{index}-{stage}"),
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

    let placed = |stage: &str| {
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
        // **One row per placement, over the mesh its own template draws**, and each names **its own**
        // material so the offset that separates five shutters from one another lives on the row rather
        // than on the geometry.
        rows.extend(
            hung.iter()
                .map(|(mesh, label, index, (translation, rotation, scale), _)| {
                    instance(
                        &format!("{label}-{index}-{stage}"),
                        mesh,
                        *translation,
                        *rotation,
                        *scale,
                        &format!("{label}-{index}-{stage}"),
                    )
                }),
        );
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
        // The one field here that is about the picture rather than what is in it: two storeys and a
        // gable, and a frame of that block. A scene that names no frame is drawn into the pipeline's
        // own square, so this is a decision rather than a default.
        ("frame".to_owned(), frame(FRAME_WIDTH, FRAME_HEIGHT)),
        (
            "cameras".to_owned(),
            // Close and low, square on to the gable end: both rows of openings have to be in frame, and a
            // high camera would show the roof and lose the storey the frame is about. **The eye did not
            // move and the lens did not either**; the aim is what this scene's own frame asked for, and
            // the frame's shape is what let the building fill it.
            Value::List(vec![camera([-10.5, 3.2, 13.5], AIM, FIELD_OF_VIEW)]),
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
            // **One mesh per part template**, drawn once and stood by an instance per placement, so the
            // template's own geometry is uploaded a single time however many of them there are. The order
            // is the one this scene has always written: the building's own surfaces, then the parts, then
            // the ground that is not an element of anything.
            Value::List(
                [
                    mesh("shells", "Triangles", point.clone(), shells),
                    mesh("roofs", "Triangles", point.clone(), roofs),
                    mesh("floors", "Triangles", point.clone(), floors),
                ]
                .into_iter()
                .chain(parts.iter().map(|(_, name, _, points, _)| {
                    mesh(name, "Triangles", point.clone(), points.clone())
                }))
                .chain([mesh(
                    "ground",
                    "Triangles",
                    shaded_point_schema(),
                    crate::ground::mesh(&building, 160, 200.0, |_, _| 0.0),
                )])
                .collect(),
            ),
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
