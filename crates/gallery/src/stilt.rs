//! A stilt house: the floor levelled to the uphill grade, the downhill side standing on posts, and the
//! gallery that hangs off that side on brackets.
//!
//! This is the first scene of the Chinese vernacular level, and it draws five things the crate could
//! not draw before: **a building whose floor is not on the ground** (`site::posts` under a seated
//! mass), **a roof whose slope bends at every purlin** (a 举架 profile), **a gable wall that rises to
//! its roof** (a crown), **a 挑廊** — a plate outside the wall carried by members whose own ends are
//! anchored on the posts it braces against — and **tiles as geometry**, the covering's 瓦垄.
//!
//! What this scene deliberately does not draw, because the construct does not exist yet:
//!
//! - **no upper storey** — the house is one room deep and one storey tall, and a second floor is the
//!   crate's `derive::floor_slab` rather than this arc's business.
//!
//! A scene that faked it would be a picture of a building the document does not hold, which is the one
//! thing this gallery's scenes are for not doing.

use definition::Value;
use scene::standard::Stage;

use building::Building;
use building::derive::{self, RoofStyle, Starter};
use building::identity::Identity;
use building::math::Vec2;
use building::ops;
use building::roof::Step;
use building::site;

use document::air::Atmosphere;
use document::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use document::geometry;
use document::kit::{
    camera, flux_for_irradiance, frame, instance, instances, light_with_radius, mesh,
    mesh_material, post_processes, screen_material, sky, surface,
};
use document::shaders::{Projection, post_shader};

/// What the house is: three bays across a 10 m front, 6 m deep, one storey under a gable.
///
/// The numbers are the scene's, and they are **authored rather than surveyed** — the Chongqing note
/// prints a 4.5 m bay over a 1.0–1.3 m purlin spacing for the type, and this crate has neither a bay
/// grid at that spacing nor a profile to put the purlins on yet, so this body is a rectangle that the
/// *site* can be tested against. It is the same order as the derived house, deliberately: the point of
/// Arc 1 is the ground, not the plan.
const STARTER: Starter = Starter {
    width: 10.0,
    depth: 6.0,
    height: 3.2,
    wall: 0.24,
    slab: 0.30,
    overhang: 0.60,
    sink: 0.10,
    rise: 2.385,
    roof_thickness: 0.22,
    roof: RoofStyle::Gable,
    seed: 5,
};

/// The bank: 35 % of a metre of rise per metre of `x`, which is the middle of the **30–70 %** range
/// the Miao stilt house is built on (Guizhou note §3.4, two unverified sources agreeing).
const SLOPE: f64 = 0.35;

/// How far apart the posts stand, and how wide each one is.
///
/// A 1.5 m spacing is the scene's, and it is chosen against the frame rather than against a source: at
/// the distance this camera stands, 1.5 m is about 40 pixels, and a post is about 5 — enough that the
/// row of them reads as a row.
const POST_SPACING: f32 = 1.5;
const POST_SECTION: f32 = 0.2;

/// How far a post's top is sunk into the plate: a post is tenoned into the floor rather than butted
/// against it, and two coincident faces are two surfaces at one depth — the first render of this scene
/// carried a comb of stripes along the plate's edge until this number existed.
const POST_EMBED: f32 = 0.02;

/// The covering: one roll every 30 cm of plan, 62 % of that period wide, 6 cm proud of the skin.
///
/// The numbers are the scene's. A 筒瓦 is about 15 cm across with a 5 cm roll; what is drawn here is
/// coarser than that on purpose — at the distance this frame stands, a 15 cm period would be a
/// texture rather than geometry, and the point of the geometry is that it is there.
const COVERING: building::Covering = building::Covering {
    period: 0.30,
    fill: 0.62,
    height: 0.06,
};

/// The 步架 ladder: four strips of `0.9 m` of plan each, at **五举 0.50, 六举 0.60, 七五举 0.75 and
/// 八举 0.80** — the coefficients the Qing worked example attests arithmetically (see
/// `.agents/investigate/chinese-timber-frame-and-roof.md` §"举架"). The three entries here are the
/// purlins *below* the ridge; the ridge itself takes the fourth step, which is why the starter's
/// `rise` is `0.9 × (0.50 + 0.60 + 0.75 + 0.80) = 2.385 m` and not a round number.
const BEAM: f32 = 0.9;
const PROFILE: [Step; 3] = [
    Step {
        fraction: 0.25,
        rise: BEAM * 0.50,
    },
    Step {
        fraction: 0.50,
        rise: BEAM * 0.50 + BEAM * 0.60,
    },
    Step {
        fraction: 0.75,
        rise: BEAM * 0.50 + BEAM * 0.60 + BEAM * 0.75,
    },
];

/// The ground, as one law in two signatures: the site wants plan points in `f32`, the ground mesh wants
/// `f64` plan coordinates, and a scene that wrote the slope twice would be a scene whose picture and
/// whose site could disagree.
///
/// # The shape, and why it is not one plane
///
/// **A 35 % bank is cut flat where the lane runs**, which is what a builder does with a slope: the
/// uphill side is dug back to a level entrance and the downhill side is left falling away under the
/// posts. So the bank rises at `SLOPE` until `x = 4` and is level behind that, and a distant ridge
/// rises past `x = 25` so the horizon is not a plane meeting the sky.
fn height(x: f64, _z: f64) -> f64 {
    let bank = SLOPE * x.min(4.0);
    let ridge = 0.25 * (x - 25.0).max(0.0);
    bank + ridge
}

fn ground(at: Vec2) -> f32 {
    height(f64::from(at[0]), f64::from(at[1])) as f32
}

/// Where the sun stands: high, and on the camera's own side of the bank, so the downhill face — the
/// one the frame is about — is lit and the posts throw their shadows along the ground under the house.
const SUN: [f64; 3] = [-18.0, 26.0, 15.0];

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

/// How far the sun's shadow page has to cover: the house stands inside seven units of plan and its
/// shadow is thrown a long way down a 35 % bank, so the page is wider than the scene's own extent.
const SHADOW_EXTENT: f64 = 11.0;

/// The image this scene is drawn into, in pixels.
///
/// # Why this frame is this shape, and which numbers are measured
///
/// **The silhouette is measured and the frame is the decision.** Read from the `authored_id` image —
/// every pixel the building's four classes wrote, against a ground that writes the reserved zero — the
/// house stands `x 209..=902` by `y 128..=717` of this frame: **693 by 589 pixels, an aspect of 1.177**.
/// The first render was drawn into 1536 by 768, a 2:1 frame that held the same silhouette and spent
/// **45 % of its width on ground either side of it**; this frame takes **1.5 to one** — 1152 by 768 —
/// which leaves the bank visible (a stilt house is read against its slope, so the ground is part of the
/// subject and not margin) and costs 884,736 pixels against the square's 1,048,576.
///
/// The margins the measurement leaves are `left 209, right 250, top 128, bottom 51`: the silhouette
/// sits **38 pixels below the frame's middle**, which is the one number here a later arc can improve by
/// raising the target rather than by moving the lens.
const FRAME_WIDTH: i64 = 1152;
const FRAME_HEIGHT: i64 = 768;

/// The full **vertical** angle this scene is drawn through, in radians.
///
/// Wider than `FRAC_PI_3 * 0.52`, and for a measured reason: at that lens the **nearest posts were cut
/// off by the frame's bottom edge** — the id image carried building pixels at row 767 — because a post's
/// foot stands five metres below the eye and sixteen away, which is 22° below the axis against a 15.6°
/// half-angle. `0.56` puts them inside it.
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// How many divisions the ground heightfield is drawn with, and how far it reaches.
///
/// **1 m cells over 120 m**, which is the scale at which a 35 % bank reads as a bank: a bigger extent
/// would put seventy metres of slope in the frame and a smaller one would end the hill inside it.
const GROUND_DIVISIONS: usize = 120;
const GROUND_EXTENT: f64 = 60.0;

/// Builds the scene from down the bank, low, looking up at the underside.
pub fn stilt() -> Value {
    scene([-18.5, 4.4, 14.0], [-0.4, 2.4, 0.0], FIELD_OF_VIEW, "stilt")
}

/// Builds **the same house** from its own 挑廊, which is where the type is lived in.
///
/// # Invariants
///
/// **One document and two cameras.** The eye stands on the gallery deck, a person's height above it,
/// looking along the walk: the rail at hip height, the posts the deck hangs between, the brackets that
/// hold it and the eave above — the four things a 吊脚楼 is, in one frame, from where its owner stands.
pub fn stilt_gallery() -> Value {
    scene([-5.9, 3.5, -2.5], [-6.1, 2.9, 3.4], 1.15, "stilt_gallery")
}

/// Builds the scene from one camera.
fn scene(eye: [f64; 3], target: [f64; 3], field_of_view: f64, name: &str) -> Value {
    let building = document();
    let built = building
        .build()
        .expect("the stilt house expands: its outline is a quad and its roof's rows are paired");
    // A scene refuses to draw a document the crate's validator would not — the rule `house`, `shed`
    // and `workshop` already follow. The two things this one adds are the posts and the seat, and
    // both of them are the sort of geometry that can be wrong in a way only the validator sees.
    let violations = building.validate();
    assert!(
        violations.is_empty(),
        "the stilt house's own document must be clean, and it is not: {violations:?}"
    );

    // **Four classes of solid, four materials**, which is the crate's rule that a material belongs to
    // an instance and not to a face: the wall ring, the roof, the plate and the posts are four meshes
    // because they are four things a viewer can tell apart.
    let table = Identity::of(&built, 1);
    let walls = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Mass(_, _))
    });
    let roof = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Roof(_, _) | building::Element::Roll(_, _)
        )
    });
    // **The plate's two caps and not its rim.** `derive::starter` seats the plate on the mass's inner
    // loop on purpose — the plate is the room's inside and its sides stand clear of every *outer* wall
    // face — but the inner loop is also where the wall's **inner** face stands, so a camera below the
    // floor sees two coincident planes fighting along the plate's edge: the stilt house's first render
    // came out combed there. The wall's inner face already closes that band, so this scene draws the
    // caps and lets the wall be the rim.
    let floors = packed_where(&built, &table, |element| {
        matches!(
            element,
            building::Element::Slab(
                _,
                building::slab::SlabSurface::Top | building::slab::SlabSurface::Bottom
            )
        )
    });
    let posts = packed_where(&built, &table, |element| {
        matches!(element, building::Element::Member(_, _))
    });

    // A whitewashed wall, a grey tile roof, a dark floor, a darker post and dry earth. The five differ
    // in tint and roughness and in nothing else: what a surface *does* is the shader's business and this
    // scene adds no body to it.
    let plaster = || surface([0.70, 0.66, 0.58], 0.0, 0.86, [0.0; 3]);
    let tiles = || surface([0.23, 0.22, 0.22], 0.0, 0.64, [0.0; 3]);
    let floor = || surface([0.32, 0.24, 0.16], 0.0, 0.80, [0.0; 3]);
    let timber = || surface([0.19, 0.13, 0.09], 0.0, 0.72, [0.0; 3]);
    let earth = || surface([0.21, 0.19, 0.14], 0.0, 0.95, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("wall", plaster as fn() -> Value),
        ("roof", tiles as fn() -> Value),
        ("floor", floor as fn() -> Value),
        ("post", timber as fn() -> Value),
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
        ("cast", "plain_shadowing"),
        ("depth", "plain_depthing"),
        ("shade", "plain_lighting"),
    ] {
        materials.push(mesh_material(&format!("ground-{stage}"), shader, earth()));
    }

    let placed = |stage: &str| {
        [
            ("walls", "wall"),
            ("roof", "roof"),
            ("floors", "floor"),
            ("posts", "post"),
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

    // Two point schemas: the four authored classes carry `authored_id`, the ground carries none and
    // writes the reserved zero from a constant.
    let extras = building_extras();
    let point = extras.point_schema();
    let plain = not_authored_extras();

    // **The ground is drawn whole, and that is the whole point of the scene.** `ground::mesh` cuts the
    // ground away under every plate, which is right for a building seated on a terrace — the earth is
    // replaced by the building's own slab — and wrong here, where the hillside continues *under* the
    // floor and the posts stand on it. So the heightfield is built from `geometry::heightfield_over`
    // with the plan kept whole, and the ground the eye sees between the posts is the ground the site
    // sampled.
    let hillside = geometry::heightfield_over(GROUND_DIVISIONS, GROUND_EXTENT, height, |cell| {
        vec![cell.to_vec()]
    });
    // **One air, handed to two renderers**: the map a world reads and the light a surface sees. Without the
    // light the shading's sky term is zero, and the ground under the floor would be black.
    let (sky_image, sky_light) = sky(&atmosphere());

    Value::structure([
        ("name".to_owned(), Value::String(name.to_owned())),
        ("frame".to_owned(), frame(FRAME_WIDTH, FRAME_HEIGHT)),
        (
            "cameras".to_owned(),
            // From down the bank, low, looking up at the underside: the eye is **below the floor's own
            // level**, so the frame sees the posts against the ground under the house rather than the
            // ground behind the house through them.
            Value::List(vec![camera(eye, target, field_of_view)]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                light_with_radius(
                    "sun",
                    SUN,
                    [1.0, 0.88, 0.72],
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
        ("outputs".to_owned(), outputs_field()),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh("walls", "Triangles", point.clone(), walls),
                mesh("roof", "Triangles", point.clone(), roof),
                mesh("floors", "Triangles", point.clone(), floors),
                mesh("posts", "Triangles", point, posts),
                mesh("ground", "Triangles", plain.point_schema(), hillside),
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

/// Returns the stilt house's document: seated, and then stood on posts.
///
/// # Invariants
///
/// **The order is the contract**: `seat` decides where the floor is and `posts` reaches down from it, so
/// a document that called them the other way round would stand its posts on the old ground and then
/// move the house off them. Seating is idempotent for a mass and not for a post, which is why the two
/// are two calls and why the scene says which order they go in.
pub fn document() -> Building {
    let mut skeleton = derive::starter(&STARTER).skeleton;
    // The 举架 section, on every plane: a gable's two slopes are one 步架 ladder read twice, and the
    // ridge's own step is the starter's `rise` less the three purlins' rises.
    for plane in &mut skeleton.roofs[0].planes {
        plane.profile = PROFILE.to_vec();
    }
    // The tiles: rolls every 30 cm of plan, 62 % of that wide, 6 cm proud of the skin.
    skeleton.roofs[0] = skeleton.roofs[0].clone().with_covering(COVERING);
    // **The 山墙, which is a crown and a split.** A gable wall rises to its roof, and a per-node top
    // can only do that where there is a node under the ridge — so each gable end is split at its
    // middle before anything is seated, and the node that appears is crowned after, because the
    // height it takes is the roof's own underside once the house has been levelled.
    let gable = gable_stations(&skeleton.masses[0], STARTER.width * 0.5);
    debug_assert_eq!(gable.len(), 2, "a gable has two ends");
    for station in gable {
        ops::split_outline(&mut skeleton.masses[0], station)
            .expect("a gable end splits at its middle");
    }
    let seated = site::seat(&mut skeleton, ground, 0.0);
    // **The rafter tails under the eave that hangs over the bank**: the overhang is the whole point of a
    // 吊脚楼's roof on its downhill side, and what an eave shows from below is the ends of its rafters.
    for tail in derive::rafter_tails(&skeleton.roofs[0], &skeleton.masses[0], 0.40, 0.08, 0.10) {
        skeleton.members.push(tail);
    }
    debug_assert_eq!(seated.len(), 1, "the house is one mass");
    let report = site::posts(
        &mut skeleton,
        ground,
        POST_SPACING,
        POST_SECTION,
        POST_EMBED,
    );
    debug_assert!(
        report.posts > 0,
        "a house on a 35 % bank must stand on posts, and this one drew none"
    );
    // The apex is the roof's **underside** at the ridge less a centimetre: the wall stands under the
    // roof and not in it, and a crown set exactly at the underside would put two faces at one depth.
    let roof = &skeleton.roofs[0];
    let ridge = roof
        .crest
        .iter()
        .map(|point| point.elevation)
        .fold(f32::NEG_INFINITY, f32::max);
    let apex = ridge - roof.thickness - 0.01;
    let level = skeleton.masses[0].crown_top();
    let nodes = skeleton.masses[0].outline().len();
    let mut crowns = vec![level; nodes];
    for (node, crown) in crowns.iter_mut().enumerate() {
        if under_the_ridge(&skeleton.masses[0], node, STARTER.width * 0.5) {
            *crown = apex;
        }
    }
    {
        let top = skeleton.masses[0].crown_mut();
        for (node, crown) in top.outline.iter_mut().enumerate() {
            crown.elevation = crowns.get(node).copied().unwrap_or(level);
        }
    }
    gallery(&mut skeleton);
    Building::new(skeleton)
}

/// How far the gallery stands proud of the wall it hangs from, and how wide its plate is.
const GALLERY_DEPTH: f32 = 1.2;

/// How high the rail stands above its plate. The joists are placed **at the posts**, which is where a
/// bracket can be braced from: a joist between two posts would have nothing under it to push against.
const RAIL_HEIGHT: f32 = 0.95;

/// The section of every piece of the gallery's frame: a joist, a strut, a baluster and the rail.
const GALLERY_SECTION: f32 = 0.07;

/// Adds the **挑廊**: the plate that hangs outside the downhill wall, the brackets that carry it, and
/// the rail along its edge.
///
/// # Invariants
///
/// **Every piece is anchored to a piece that is already there.** A joist starts at a post's head — a
/// fraction of that member's own axis — and a strut starts lower on the same post and ends on the
/// joist's own outer end, so each bracket is a closed triangle in the document rather than three
/// pieces that happen to meet at a coordinate.
///
/// **The plate is a `Floor` outside the wall it belongs to**, which is what a 挑廊 is: a deck, not a
/// ground plate and not a roof. It names the mass as its host, and the coverage check reads `Plate`
/// roles only, so a floor that oversails its host is a deck and not a fault.
fn gallery(skeleton: &mut building::BuildingSkeleton) {
    let mass = &skeleton.masses[0];
    let floor = mass.lowest();
    let slab = mass.slab;
    let side = STARTER.width * 0.5;
    let depth = STARTER.depth * 0.5;
    let underside = floor - slab;
    // The posts that stand along the downhill wall, in the order the plan walks them.
    let mut posts: Vec<([f32; 2], usize)> = Vec::new();
    for (index, member) in skeleton.members.iter().enumerate() {
        let Some(foot) = foot_of_member(member) else {
            continue;
        };
        if (foot[0] + side).abs() < 1.0e-3 && foot[1].abs() <= depth + 1.0e-3 {
            posts.push((foot, index));
        }
    }
    posts.sort_by(|left, right| {
        left.0[1]
            .partial_cmp(&right.0[1])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if posts.len() < 2 {
        return;
    }

    // The plate, wound positively: a reversed loop is a slab whose faces point inward.
    let mut plan = vec![
        [-side - GALLERY_DEPTH, -depth],
        [-side, -depth],
        [-side, depth],
        [-side - GALLERY_DEPTH, depth],
    ];
    if loop_area(&plan) < 0.0 {
        plan.reverse();
    }
    skeleton
        .slabs
        .push(building::Slab::floor(plan, underside, floor).with_host(building::MassId(0)));

    // The joists: one from each post's head out to the plate's own edge, running at the heads' level.
    // A joist's axis is `x`, so `across = [0, 1]` puts its width along `z` and its depth along `y`, and
    // the plate's own thickness swallows it — which is what a floor structure is.
    let head = underside + POST_EMBED;
    let mut joists = Vec::new();
    for (foot, post) in &posts {
        let joist = building::Member::new(
            building::Anchor::OnMember {
                member: building::MemberId(*post),
                t: 1.0,
            },
            building::Anchor::Free {
                at: [-side - GALLERY_DEPTH, head, foot[1]],
            },
            [0.0, 1.0],
            GALLERY_SECTION,
            0.14,
        );
        joists.push(skeleton.members.len());
        skeleton.members.push(joist);
    }

    // The struts: a diagonal from each post, a fixed drop below its head, out to its joist's end. The
    // drop is a **length** rather than a fraction, because the posts are not one length and a fraction
    // of a short post is a strut with no room in it.
    for (at, joist) in joists.iter().enumerate() {
        let (_foot, post) = posts[at];
        let foot_height = match skeleton.members[post].from {
            building::Anchor::Free { at } => at[1],
            // A post is a free member, and the two other ways of naming an end are not posts: the
            // fallback is what keeps this reading total without pretending either of them is a foot.
            building::Anchor::OnMember { .. } | building::Anchor::OnFace { .. } => head - 1.0,
        };
        let post_length = (head - foot_height).max(0.1);
        let drop = 0.65_f32.min(post_length * 0.75);
        let t = 1.0 - drop / post_length;
        skeleton.members.push(building::Member::new(
            building::Anchor::OnMember {
                member: building::MemberId(post),
                t,
            },
            building::Anchor::OnMember {
                member: building::MemberId(*joist),
                t: 1.0,
            },
            [0.0, 1.0],
            GALLERY_SECTION,
            GALLERY_SECTION,
        ));
    }

    // The balusters and the rail: one baluster on the plate's own edge at every joist, and one rail
    // across their tops.
    let edge = -side - GALLERY_DEPTH + GALLERY_SECTION * 0.5;
    let mut tops: Vec<[f32; 3]> = Vec::new();
    for (foot, _) in &posts {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [edge, floor, foot[1]],
            },
            building::Anchor::Free {
                at: [edge, floor + RAIL_HEIGHT, foot[1]],
            },
            [1.0, 0.0],
            GALLERY_SECTION,
            GALLERY_SECTION,
        ));
        tops.push([edge, floor + RAIL_HEIGHT, foot[1]]);
        // **A middle rail in every bay**, because a rail at the top of a baluster and nothing above the
        // plate lets a child through: the 走马廊's rail is a frame, not a handrail on posts.
    }
    // **Three slats in every bay**, at its quarters: a 走马廊's rail is a frame of rails and uprights, and
    // the balusters it already had stand only at the posts, which is a fence and not a railing.
    for pair in posts.windows(2) {
        let (here, there) = (pair[0].0[1], pair[1].0[1]);
        for step in 1..=3 {
            let z = here + (there - here) * step as f32 / 4.0;
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [edge, floor, z],
                },
                building::Anchor::Free {
                    at: [edge, floor + RAIL_HEIGHT, z],
                },
                [1.0, 0.0],
                GALLERY_SECTION * 0.7,
                GALLERY_SECTION * 0.7,
            ));
        }
    }

    // The middle rail: one per bay, at half the rail's height, from one baluster to the next.
    let middle = floor + RAIL_HEIGHT * 0.45;
    for pair in posts.windows(2) {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [edge, middle, pair[0].0[1]],
            },
            building::Anchor::Free {
                at: [edge, middle, pair[1].0[1]],
            },
            [1.0, 0.0],
            GALLERY_SECTION,
            GALLERY_SECTION,
        ));
    }

    if let (Some(first), Some(last)) = (tops.first(), tops.last()) {
        skeleton.members.push(building::Member::new(
            building::Anchor::Free { at: *first },
            building::Anchor::Free { at: *last },
            [1.0, 0.0],
            GALLERY_SECTION,
            GALLERY_SECTION,
        ));
    }
}

/// Returns the plan point a member's foot stands at, which for a post is its `from` anchor's plan.
fn foot_of_member(member: &building::Member) -> Option<[f32; 2]> {
    match member.from {
        building::Anchor::Free { at } => Some([at[0], at[2]]),
        building::Anchor::OnMember { .. } | building::Anchor::OnFace { .. } => None,
    }
}

/// Answers whether a node stands under the ridge: the middle of one of the two gable ends.
///
/// A gable end is a wall at one of the two extremes of the ridge's own axis, and its middle is the
/// point where the gable's two rakes meet. The test reads the **plan**, so it survives seating.
fn under_the_ridge(mass: &building::Mass, node: usize, half_width: f32) -> bool {
    let at = mass.outline()[node].position;
    at[1].abs() < 1.0e-3 && (at[0].abs() - half_width).abs() < 1.0e-3
}

/// Returns the arc-length stations at which the two gable ends are, far one first.
///
/// A gable end's two nodes share an `x` of magnitude `half_width`; its middle is the station half way
/// along it. Both stations are measured on the **closed** loop before either insertion, and answered
/// far end first, because an insertion moves every station after it.
fn gable_stations(mass: &building::Mass, half_width: f32) -> Vec<f32> {
    let count = mass.outline().len();
    let lengths: Vec<f32> = (0..count)
        .map(|edge| {
            let from = mass.outline()[edge].position;
            let to = mass.outline()[(edge + 1) % count].position;
            ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt()
        })
        .collect();
    let perimeter: f32 = lengths.iter().sum();
    let mut stations = Vec::new();
    let mut walked = 0.0_f32;
    for (edge, length) in lengths.iter().enumerate() {
        let from = mass.outline()[edge].position;
        let to = mass.outline()[(edge + 1) % count].position;
        let gable = (from[0] - to[0]).abs() < 1.0e-4 && (from[0].abs() - half_width).abs() < 1.0e-4;
        if gable {
            stations.push((walked + length * 0.5) / perimeter);
        }
        walked += length;
    }
    stations.sort_by(|left, right| right.partial_cmp(left).unwrap_or(std::cmp::Ordering::Equal));
    stations
}

/// Returns the signed plan area of a loop, positive when it is wound so its up normal is not negative.
fn loop_area(plan: &[[f32; 2]]) -> f32 {
    let mut twice = 0.0_f32;
    for index in 0..plan.len() {
        let from = plan[index];
        let to = plan[(index + 1) % plan.len()];
        twice += from[0] * to[1] - to[0] * from[1];
    }
    twice * 0.5
}
/// Returns how far a light position stands from the scene's own middle.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// The air over the valley: a hazy river morning, and **the light the sun is not**.
///
/// The numbers are the ones this scene's own sky law carried — zenith, horizon, ground and the sun's bloom
/// — read out of it and handed to one `Atmosphere`, which bakes them into the map the shading reads its sky
/// term from.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.28, 0.42, 0.62])
        .horizon([0.86, 0.82, 0.74])
        .ground([0.30, 0.28, 0.22])
        .sun([-18.0, 26.0, 15.0], [1.0, 0.88, 0.66], 6.0, 0.30)
        .density(0.0011)
        .scale_height(200.0)
}

/// What the post stage does to the frame: the air, the curve, and the vignette, in that order.
const GRADE: &str = r#"
    // **The sky is read out of the scene's own environment map**, the same air the bounce reads.
    if depth >= 1.0 {
        color = environment_sky(ray_through(in.uv));
    }
    if depth < 1.0 {
        let ray = ray_through(in.uv);
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp((1.0 - exp(-along_ray * 0.004)) * 0.55, 0.0, 0.85);
        color = mix(
            color,
            environment_sky(vec3<f32>(ray.x, max(ray.y, -0.04), ray.z)),
            air,
        );
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.18), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.40, 0.0, 1.0);
"#;
