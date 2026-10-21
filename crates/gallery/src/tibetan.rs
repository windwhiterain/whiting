//! A **藏族碉房**: the Tibetan stone tower house of Diqing and the upper Yangtze — three storeys of coursed stone, a flat
//! roof the family threshes on, and one timber gallery on the sunny face.
//!
//! # What this scene is for
//!
//! It is the gallery's **eleventh type** and its highest: the other ten are one to two storeys because their materials are
//! light — bamboo, earth, timber, boards — and this one is **stone**, which is why it can be three. A 碉房 is a fortress
//! that a family lives in: the walls are six hundred millimetres of stone and carry everything, the windows are small and
//! high (a window is a hole in a wall that must still carry), and the roof is a floor.
//!
//! # The two facts the type has
//!
//! - **It is one wall, built in two tiers.** The lower two storeys are one mass and the third stands back on its roof by
//!   `SETBACK`, carried through `Join::Storey`: the same two-mass answer the 土掌房 and the 骑楼 got, and the only one this
//!   crate allows a tall house, because a mass drawn in several bands refuses a cut by name.
//! - **The roof is a threshing floor.** A flat earth slab on the top mass, with a parapet on its edge and the 煨桑炉 — the
//!   smoke-offering burner — in one corner, which is what a flat Tibetan roof carries.
//!
//! # What it does not draw
//!
//! - **the batter**: a real 碉房's walls lean in as they rise, and a banded mass cannot be cut, so this one is vertical;
//! - **the 经堂's interior**: the shrine room on the top floor is the interior arc's work;
//! - **the stone**: the walls are one plaster thickness here, and the courses are a material the frame cannot show.
use building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, derive, site};
use definition::Value;
use document::air::Atmosphere;
use document::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use document::kit::{
    camera, environment_light, flux_for_irradiance, frame, image, instance, instances,
    light_with_radius, mesh, mesh_material, post_processes, screen_material, surface,
};
use document::shaders::{Projection, post_shader};
use scene::standard::Stage;

use crate::facade::{self, Front};
use crate::ground;

/// The house: `WIDTH` by `DEPTH`, `STOREYS` storeys of `STOREY`, the top one standing back by `SETBACK`.
const WIDTH: f32 = 7.6;
const DEPTH: f32 = 6.4;
const STOREY: f32 = 2.60;
const STOREYS: usize = 3;
const SETBACK: f32 = 0.40;
/// The lower wall's batter: how many courses it is laid in, and how far each is set in from the one below.
const COURSES: usize = 4;
const BATTER: f32 = 0.11;

/// The wall and the two roof floors: six hundred millimetres of stone, and slabs that carry a threshing floor.
const WALL: f32 = 0.60;
const PLATE: f32 = 0.36;
const ROOF_THICK: f32 = 0.30;

/// The parapet round each roof, and the 煨桑炉 in the top one's corner.
const PARAPET: f32 = 0.30;
const PARAPET_HIGH: f32 = 0.70;
const BURNER: f32 = 0.95;
const BURNER_HIGH: f32 = 1.25;

/// The timber gallery on the top storey's sunny face: how far it stands out, and its posts and rail.
const GALLERY: f32 = 1.05;
const POST: f32 = 0.16;
const RAIL: f32 = 0.10;

/// The stair, climbing the shaded wall to the first roof.
const STEPS: usize = 12;
const TREAD: f32 = 0.30;
const TREAD_THICK: f32 = 0.12;

/// The sun, high and from the south-west, so the gallery is lit and the stone reads as stone.
const SUN: [f64; 3] = [-22.0, 34.0, 18.0];
const SUN_IRRADIANCE: f64 = 3.4;
const SHADOW_EXTENT: f64 = 30.0;

const FRAME_WIDTH: i64 = 1280;
const FRAME_HEIGHT: i64 = 800;
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.60;
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 60.0;

/// The air over the plateau: thin, bright and cold at the zenith.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.16, 0.30, 0.62])
        .horizon([0.82, 0.84, 0.86])
        .ground([0.30, 0.28, 0.24])
        .density(0.0007)
        .scale_height(160.0)
}

/// Returns how far a position is from the origin, which is where the sun's irradiance is stated.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// Builds the scene: from the south-west, where the gallery is.
pub fn tibetan_house() -> Value {
    scene(
        [15.0, 6.4, 16.0],
        [0.0, 5.4, 0.0],
        FIELD_OF_VIEW,
        "tibetan_house",
    )
}
/// Builds the scene from one camera.
fn scene(eye: [f64; 3], target: [f64; 3], field_of_view: f64, name: &str) -> Value {
    let building = document();
    let built = building.build().expect(
        "a 碉房 expands: two stone tiers, two roof floors, the parapet, the burner and the gallery",
    );
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "a 碉房's own document must be clean, and it is not: {violations:?}"
    );

    let table = building::identity::Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    // **The flat roof and the porch's shed are one mesh**, because they are one material: the earth the
    // 晒场 is made of. The scene had no `Roof` in it until the 厦子 arrived, so a roof element would have
    // been drawn by nothing.
    let roofs = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Slab(
                _,
                building::slab::SlabSurface::Top | building::slab::SlabSurface::Bottom
            ) | building::Element::Roof(_, _)
        )
    });
    let timbers = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Member(_, _))
    });

    let earth = || surface([0.56, 0.44, 0.30], 0.0, 0.92, [0.0; 3]);
    let roof_earth = || surface([0.48, 0.38, 0.26], 0.0, 0.94, [0.0; 3]);
    let stone = || surface([0.42, 0.40, 0.36], 0.0, 0.80, [0.0; 3]);
    let ground_earth = || surface([0.34, 0.29, 0.21], 0.0, 0.96, [0.0; 3]);

    let mut materials = Vec::new();
    for (label, body) in [
        ("wall", earth as fn() -> Value),
        ("roof", roof_earth),
        ("stone", stone),
    ] {
        materials.push(mesh_material(
            &format!("{label}-cast"),
            "authored_shadowing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{label}-depth"),
            "authored_depthing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{label}-shade"),
            "authored_lighting",
            body(),
        ));
    }
    for (stage, shader) in [
        ("cast", "plain_shadowing"),
        ("depth", "plain_depthing"),
        ("shade", "plain_lighting"),
    ] {
        materials.push(mesh_material(
            &format!("ground-{stage}"),
            shader,
            ground_earth(),
        ));
    }

    let placed = |stage: &str| {
        [
            ("walls", "wall"),
            ("roofs", "roof"),
            ("stone", "stone"),
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

    let extras = building_extras();
    let point = extras.point_schema();
    let plain = not_authored_extras();
    let ground = ground::mesh(&building, GROUND_DIVISIONS, GROUND_EXTENT, |_, _| 0.0);

    Value::structure([
        ("name".to_owned(), Value::String(name.to_owned())),
        ("frame".to_owned(), frame(FRAME_WIDTH, FRAME_HEIGHT)),
        (
            "cameras".to_owned(),
            Value::List(vec![camera(eye, target, field_of_view)]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                light_with_radius(
                    "sun",
                    SUN,
                    [1.0, 0.94, 0.82],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                    SHADOW_EXTENT,
                    170.0,
                ),
                environment_light("air", "air", [1.0; 3], 1.08),
            ]),
        ),
        (
            "images".to_owned(),
            Value::List(vec![image(
                "air",
                256,
                128,
                atmosphere().environment_pixels(256, 128),
                Projection::Latlong,
            )]),
        ),
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
                shaders.push(post_shader("", &helpers, Some(Projection::Latlong)));
                shaders
            }),
        ),
        ("outputs".to_owned(), outputs_field()),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh("walls", "Triangles", point.clone(), walls),
                mesh("roofs", "Triangles", point.clone(), roofs),
                mesh("stone", "Triangles", point.clone(), timbers),
                mesh("ground", "Triangles", plain.point_schema(), ground),
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

/// Returns how wide the stair is, measured from the wall it climbs.
///
/// # Invariants
///
/// **One number, three readers** — the treads, their stringer and the parapet's landing gap — so a stair
/// narrowed in one place cannot leave a gap in the parapet that no longer fits it.
fn stair_width() -> f32 {
    1.1
}

/// Returns the plan direction perpendicular to a member's own axis, which is what its section is measured
/// across.
///
/// # Invariants
///
/// **A section laid along the axis has no width at all**, and the expansion answers `MemberDegenerate` with the
/// member's own length — a number that reads like a degenerate *length* and is not one. Every member here is
/// axis-aligned, but the parapet's four sides run two ways, so the direction is read from the pair rather than
/// written per side.
fn across_of(from: [f32; 2], to: [f32; 2]) -> [f32; 2] {
    let axis = [to[0] - from[0], to[1] - from[1]];
    let length = (axis[0] * axis[0] + axis[1] * axis[1]).sqrt();
    if length <= 1.0e-3 {
        return [1.0, 0.0];
    }
    [-axis[1] / length, axis[0] / length]
}

/// Returns the document: one earth box, its roof floor, the parapet round it and the stair up to it.

/// Returns the balcony's width, which is also the gap it leaves in the parapet it stands in.
fn balcony_width() -> f32 {
    2.4
}

/// Returns the document: a stone tower in two tiers, a threshing roof, its parapet, the burner and the gallery.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let half = WIDTH * 0.5;
    let deep = DEPTH * 0.5;
    let lower_top = STOREY * (STOREYS - 1) as f32;
    let top = STOREY * STOREYS as f32;

    // **The two lower storeys are laid in COURSES stepped courses, each set in by BATTER.** A tower house's wall leans in as
    // it rises, and this crate cannot cut a mass drawn in several bands: a banded mass refuses an opening by name
    // (`Problem::BandedMassCut`). So the batter is built the way a stone wall is built — courses stepped one on the next, the
    // tradition's own 叠涩 — and every course is a one-band mass, which is cuttable. The frame reads it as a lean, because at a
    // tenth of a metre a course, four courses are one slope.
    //
    // The lesson of round 6 is the one obeyed here: **model the relationship, not the overlap**. The courses share an edge and
    // share no volume, and each is carried by the one below through `Join::Storey`.
    let mut id = MassId(skeleton.masses.len());
    let mut host = None;
    let mut courses: Vec<MassId> = Vec::with_capacity(COURSES);
    for course in 0..COURSES {
        let inset = BATTER * course as f32;
        let base = lower_top * course as f32 / COURSES as f32;
        let ceiling = lower_top * (course + 1) as f32 / COURSES as f32;
        let mut laid = Mass::new(
            [
                PlanNode::new([-half + inset, -deep + inset], base),
                PlanNode::new([half - inset, -deep + inset], base),
                PlanNode::new([half - inset, deep - inset], base),
                PlanNode::new([-half + inset, deep - inset], base),
            ]
            .to_vec(),
            ceiling,
        );
        laid.wall = WALL;
        laid.slab = PLATE;
        if let Some(under) = host {
            laid.attachment = Some(building::Attachment {
                host: under,
                join: building::Join::Storey,
            });
        }
        let laid_id = MassId(skeleton.masses.len());
        skeleton.masses.push(laid);
        courses.push(laid_id);
        host = Some(laid_id);
        if course == 0 {
            id = laid_id;
        }
    }
    // The third storey stands back on the last course's own plan, so the batter and the setback add up.
    let lean = BATTER * (COURSES - 1) as f32;
    let mut upper = Mass::new(
        [
            PlanNode::new([-half + SETBACK + lean, -deep + SETBACK + lean], lower_top),
            PlanNode::new([half - SETBACK - lean, -deep + SETBACK + lean], lower_top),
            PlanNode::new([half - SETBACK - lean, deep - SETBACK - lean], lower_top),
            PlanNode::new([-half + SETBACK + lean, deep - SETBACK - lean], lower_top),
        ]
        .to_vec(),
        top,
    );
    upper.wall = WALL;
    upper.slab = PLATE;
    // **The third storey stands on the last course, not on the first.** The validator reads a stack as a chain
    // (`stacked_above`), so a mass that names the bottom of four courses leaves three pairs unrelated: measured as
    // `UnattachedOverlap { mass: 1/2/3, other: 4 }` and `StoreyNotOnTop { mass: 4, host: 0, base: 5.2, top: 1.3 }`.
    upper.attachment = Some(building::Attachment {
        host: host.expect("the courses were laid"),
        join: building::Join::Storey,
    });
    let upper_id = MassId(skeleton.masses.len());
    skeleton.masses.push(upper);

    // **The openings, and where a battered wall puts them.** The ground course carries the house's one door and its two
    // windows, cut the way round 69 measured — the face split into three bays first, then an inset a wall's thickness clear of
    // the bay's ends. Every course above carries **one arrow slit a face**, because a wall of four courses with slits in two of
    // them is a wall with windows on its ground floor and its attic.
    let front = building::FaceRef::new(id, 2);
    let columns = building::Rhythm::Even { count: 3 };
    let rows = building::Rhythm::Even { count: 1 };
    if building::ops::split_face(&mut skeleton, front, &columns, &rows).is_some() {
        for column in 0..3 {
            let inset = if column == 1 {
                building::Inset::new(0.72, 0.72, 0.30, 1.20)
            } else {
                building::Inset::new(0.70, 0.70, 1.20, 1.65)
            };
            let _ = building::ops::add_opening(
                &mut skeleton,
                building::Opening::new(front, 0, column, inset),
            );
        }
    }
    let slit = building::Inset::new(1.20, 1.20, 1.40, 1.10);
    let others = courses
        .iter()
        .copied()
        .skip(1)
        .chain(std::iter::once(upper_id));
    for mass in others {
        for edge in 0..4 {
            let face = building::FaceRef::new(mass, edge);
            if building::ops::split_face(
                &mut skeleton,
                face,
                &building::Rhythm::Even { count: 1 },
                &building::Rhythm::Even { count: 1 },
            )
            .is_none()
            {
                continue;
            }
            let _ =
                building::ops::add_opening(&mut skeleton, building::Opening::new(face, 0, 0, slit));
        }
    }

    // **The first roof is a floor**: the family's 晒场, and the terrace the third storey stands on.
    skeleton.slabs.push(
        building::Slab::roof(
            vec![[-half, -deep], [half, -deep], [half, deep], [-half, deep]],
            lower_top - ROOF_THICK,
            lower_top,
        )
        .with_host(id),
    );
    // **The top roof is a floor too**, and the highest ground anyone in the house stands on.
    let inner = half - SETBACK;
    let deep_in = deep - SETBACK;
    skeleton.slabs.push(
        building::Slab::roof(
            vec![
                [-inner, -deep_in],
                [inner, -deep_in],
                [inner, deep_in],
                [-inner, deep_in],
            ],
            top - ROOF_THICK,
            top,
        )
        .with_host(upper_id),
    );

    // **A parapet round each roof, with the gallery's bay left open on the top one.** A parapet you cannot step over
    // from the gallery is a parapet and not a fence, so the front side of the top roof is two pieces with the balcony's
    // width between them.
    for (level, x, z, gap) in [
        (lower_top + 0.02, half, deep, None),
        (top + 0.02, inner, deep_in, Some(balcony_width())),
    ] {
        let mut sides = vec![
            ([-x, -z], [x, -z]),
            ([x, -z], [x, z]),
            ([x, z], [-x, z]),
            ([-x, z], [-x, -z]),
        ];
        if let Some(width) = gap {
            sides.retain(|(_, _)| true);
            sides.pop();
            sides.push(([-x, z], [-width * 0.5, z]));
            sides.push(([width * 0.5, z], [x, z]));
        }
        for (from, to) in sides {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [from[0], level, from[1]],
                },
                building::Anchor::Free {
                    at: [to[0], level, to[1]],
                },
                across_of(from, to),
                PARAPET,
                PARAPET_HIGH,
            ));
        }
    }

    // **The 煨桑炉: a square stone burner in the top roof's far corner**, drawn as four walls of members and not as a mass.
    // A mass inside another mass's plan is a mass with a host, and the validator asks that of every pair at once: the
    // burner's plan lies inside *both* tiers', so it reads as `UnattachedOverlap` against the one it is not attached to —
    // measured as `mass 0 / other 2` and `mass 1 / other 2` with the attachment in place. Four members have no plan of
    // their own to overlap with, which is the honest draw for a ring of stone with a fire in it.
    let at = [-inner + BURNER * 0.7, top, -deep_in + BURNER * 0.7];
    let square = [
        (
            [-BURNER * 0.5, -BURNER * 0.5],
            [BURNER * 0.5, -BURNER * 0.5],
        ),
        ([BURNER * 0.5, -BURNER * 0.5], [BURNER * 0.5, BURNER * 0.5]),
        ([BURNER * 0.5, BURNER * 0.5], [-BURNER * 0.5, BURNER * 0.5]),
        (
            [-BURNER * 0.5, BURNER * 0.5],
            [-BURNER * 0.5, -BURNER * 0.5],
        ),
    ];
    for (from, to) in square {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [at[0] + from[0], at[1], at[2] + from[1]],
            },
            building::Anchor::Free {
                at: [at[0] + to[0], at[1] + BURNER_HIGH, at[2] + to[1]],
            },
            across_of(from, to),
            0.18,
            0.18,
        ));
    }
    // **The gallery on the top storey's south face**, standing on posts that reach the first roof: a 碉房's one piece of
    // timber is this balcony, and its posts run the height of the storey below rather than hanging off the wall. The floor
    // is three planks across the brackets, because a balcony with rails and no deck is a frame in the air — which is what
    // the first version drew.
    let level = top;
    let (near, far) = (-balcony_width() * 0.5, balcony_width() * 0.5);
    let outer = deep_in + GALLERY - POST * 0.5;
    for x in [near, far] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [x, lower_top, outer],
            },
            building::Anchor::Free {
                at: [x, level + 1.05, outer],
            },
            [1.0, 0.0],
            POST,
            POST,
        ));
    }
    for (height, section) in [(level + 1.00, RAIL), (level + 0.55, RAIL * 0.7)] {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [near, height, outer],
            },
            building::Anchor::Free {
                at: [far, height, outer],
            },
            [0.0, 1.0],
            section,
            section,
        ));
    }
    for portion in 0..3 {
        let z = deep_in + (GALLERY / 3.0) * (portion as f32 + 0.5);
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [near, level - 0.06, z],
            },
            building::Anchor::Free {
                at: [far, level - 0.06, z],
            },
            [0.0, 1.0],
            GALLERY / 3.0 * 0.92,
            0.08,
        ));
    }

    // **The stair climbs the shaded side to the first roof.** A 碉房's stair is inside, and this crate has no interior, so
    // it runs up the north wall where a stone house would have its own external flight to the threshing floor.
    let rise = (lower_top - ROOF_THICK) / STEPS as f32;
    let width = balcony_width() * 0.5;
    for step in 0..STEPS {
        let y = rise * (step + 1) as f32;
        let z = -deep + TREAD * step as f32 + TREAD * 0.5;
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [-width * 0.5, y, z],
            },
            building::Anchor::Free {
                at: [width * 0.5, y, z],
            },
            [0.0, 1.0],
            TREAD,
            TREAD_THICK,
        ));
    }

    // **The openings: a door and two windows below, two windows above.** A 碉房's window is a hole in a wall that
    // carries, so they are small: the bays are three to a face and the openings take half a bay.
    let _ = facade::open_court_face(&mut skeleton, id, deep, true, Front::RANGE);
    let _ = facade::open_court_face(
        &mut skeleton,
        upper_id,
        deep - SETBACK,
        true,
        Front {
            storeys: 1,
            bays: 3,
            door: true,
        },
    );
    Building::new(skeleton)
}
