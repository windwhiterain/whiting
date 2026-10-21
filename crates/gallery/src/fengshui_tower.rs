//! A **风水塔**: a village's slim brick pagoda, seven storeys under a spire, standing where the village's mouth meets the valley — the twentieth type.
//!
//! # What this scene is for
//!
//! Every 白族 and 汉族 village in 滇西 and 黔北 that could afford one built a **tower at the low end of its site**, and the tower is not a building: it
//! has no floor to walk on, no door in use and no room. Its whole content is **its elevation** — a stack of battered storeys, each shorter than the
//! one below, each with its own corbelled eave, and a spire above them all.
//!
//! # How it is built
//!
//! `STOREYS` masses, one per storey, each with its own square plan narrower than the one below and one **eave slab** projecting from it. The whole is
//! a stack of small solids rather than one tapered mass, because a pagoda's eaves are **steps in its silhouette** and a tapered mass has a smooth
//! one.
//!
//! # What it does not draw
//!
//! - **the doors and the windows**: a 风水塔's openings are blind niches on seven of its eight faces and a single door at its foot. The crate's
//!   openings want a grid on a face and a bay, and a niche is neither — the round is a stack of eaves first and its faces second;
//! - **the 铁刹 and its chain**: the spire is a mast with two bands, not the cast-iron finial a real one carries.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The storeys: how many, the tower's width at its foot and at its top, how high a storey stands and how far its eave projects.
const STOREYS: usize = 7;
const FOOT: f32 = 3.20;
const HEAD: f32 = 1.45;
const STOREY_HIGH: f32 = 2.30;
const EAVE: f32 = 0.42;
const EAVE_THICK: f32 = 0.16;
/// The plinth, the spire's height and how wide the spire's bands are.
const PLINTH: f32 = 0.55;
const SPIRE: f32 = 2.90;
const SPIRE_WIDE: f32 = 0.34;
/// The blind niche on each storey's two faces: its frame's section and how far the frame stands proud of the wall.
const NICHE: f32 = 0.075;
const NICHE_PROUD: f32 = 0.035;
/// The 塔门: its jamb inset and the depth of wall above its head.
const DOOR_JAMB: f32 = 0.155;
const DOOR_HEAD: f32 = 0.42;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.52;

/// Builds the scene: from below and to one side, because a pagoda is a silhouette and its eaves read against the sky.
pub fn fengshui_tower() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [-16.0, 8.6, 16.8],
        [0.0, 9.8, 0.0],
        FIELD_OF_VIEW,
        "fengshui_tower",
    )
}

/// Returns the document: a plinth, seven battered storeys with their eaves, and the spire.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The plinth**, a low square the tower stands on: a 风水塔 is raised a step above the path it ends.
    let mut plinth = Mass::new(
        ops::wound(vec![
            [-FOOT * 0.5 - 0.30, -FOOT * 0.5 - 0.30],
            [FOOT * 0.5 + 0.30, -FOOT * 0.5 - 0.30],
            [FOOT * 0.5 + 0.30, FOOT * 0.5 + 0.30],
            [-FOOT * 0.5 - 0.30, FOOT * 0.5 + 0.30],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, 0.0))
        .collect(),
        PLINTH,
    );
    // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171).
    plinth.wall = FOOT * 0.10;
    plinth.slab = 0.12;
    skeleton.masses.push(plinth);

    // **The seven storeys**, each a square narrower than the one under it and **standing on it**: every storey is a mass of its own, attached to the
    // one below with `Join::Storey`, which is how the 土掌房's room stands inset on its terrace. A storey drawn as one thick slab is not a volume this
    // crate builds — its slabs are plates — and a storey drawn as an unattached mass is refused for nesting in plan (round 137). Attached, both
    // objections are answered.
    let mut host = building::MassId(0);
    for storey in 0..STOREYS {
        let low = PLINTH + STOREY_HIGH * storey as f32;
        let high = low + STOREY_HIGH;
        let half = 0.5 * (FOOT + (HEAD - FOOT) * storey as f32 / (STOREYS - 1) as f32);
        let mut body = Mass::new(
            ops::wound(vec![
                [-half, -half],
                [half, -half],
                [half, half],
                [-half, half],
            ])
            .into_iter()
            .map(|at| building::PlanNode::new(at, low))
            .collect(),
            high,
        );
        // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171).
        body.wall = half * 0.20;
        body.slab = 0.14;
        body.attachment = Some(building::Attachment {
            host,
            join: building::Join::Storey,
        });
        host = building::MassId(skeleton.masses.len());
        skeleton.masses.push(body);

        // **The 塔门 at the foot.** Round 209 recorded this as the one thing the type still owed: a pagoda's lowest storey has a **door** on the side
        // that faces the village, and it is a real opening — a grid on the face and a bay for the door to sit in — where the niches above it are only
        // frames. Face 2 is the `+z` side of a square wound from `[-h, -h]`, which is the side the plate looks at.
        if storey == 0 {
            let face = building::FaceRef::new(host, 2);
            if ops::split_face(
                &mut skeleton,
                face,
                &building::Rhythm::Even { count: 3 },
                &building::Rhythm::Even { count: 1 },
            )
            .is_some()
            {
                // The door fills its bay to within `DOOR_JAMB`, from the storey's own base up to `DOOR_HEAD` below its head.
                let bay = 2.0 * half / 3.0;
                let _ = ops::add_opening(
                    &mut skeleton,
                    building::Opening::new(
                        face,
                        0,
                        1,
                        building::Inset::new(DOOR_JAMB, DOOR_JAMB, 0.0, STOREY_HIGH - DOOR_HEAD),
                    ),
                );
                let _ = bay;
            }
        }

        // **The blind niche on each of the two faces the camera sees.** A 风水塔's walls are not blank: every storey carries a shallow **niche** on its
        // faces, and the village reads the tower's height by counting them. The crate has no recess, so the niche is drawn as its **frame** — four
        // thin members standing a centimetre proud of the wall — which is what a niche's edge looks like from the ground.
        let niche_wide = half * 0.46;
        let niche_low = low + STOREY_HIGH * 0.26;
        let niche_high = low + STOREY_HIGH * 0.74;
        for face in [-1.0_f32, 1.0] {
            let at = face * (half + NICHE_PROUD * 0.5);
            for side in [-1.0_f32, 1.0] {
                skeleton.members.push(building::Member::new(
                    building::Anchor::Free {
                        at: [side * niche_wide, niche_low, at],
                    },
                    building::Anchor::Free {
                        at: [side * niche_wide, niche_high, at],
                    },
                    [1.0, 0.0],
                    NICHE,
                    NICHE_PROUD,
                ));
            }
            for up in [niche_low, niche_high] {
                skeleton.members.push(building::Member::new(
                    building::Anchor::Free {
                        at: [-niche_wide, up, at],
                    },
                    building::Anchor::Free {
                        at: [niche_wide, up, at],
                    },
                    [0.0, 1.0],
                    NICHE,
                    NICHE_PROUD,
                ));
            }
        }

        // **The eave**: a slab standing out from the storey's own top, which is what steps the silhouette.
        let out = half + EAVE;
        skeleton.slabs.push(building::Slab::roof(
            ops::wound(vec![[-out, -out], [out, -out], [out, out], [-out, out]]),
            high,
            high + EAVE_THICK,
        ));
    }

    // **The spire**: a mast above the topmost eave with two bands on it.
    let foot = PLINTH + STOREY_HIGH * STOREYS as f32 + EAVE_THICK;
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [0.0, foot, 0.0],
        },
        building::Anchor::Free {
            at: [0.0, foot + SPIRE, 0.0],
        },
        [1.0, 0.0],
        SPIRE_WIDE,
        SPIRE_WIDE,
    ));
    for band in 0..2 {
        let up = foot + SPIRE * (0.34 + 0.22 * band as f32);
        let reach = SPIRE_WIDE * (1.8 - 0.4 * band as f32);
        skeleton.members.push(building::Member::new(
            building::Anchor::Free {
                at: [-reach, up, 0.0],
            },
            building::Anchor::Free {
                at: [reach, up, 0.0],
            },
            [0.0, 1.0],
            SPIRE_WIDE * 0.5,
            SPIRE_WIDE * 0.55,
        ));
    }

    Building::new(skeleton)
}
