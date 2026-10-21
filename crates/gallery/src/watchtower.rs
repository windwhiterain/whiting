//! A **碉楼**: the stone watchtower of a 屯堡 — square, battered, loopholes on every face and a timber gallery on its top — the twenty-seventh type.
//!
//! # What this scene is for
//!
//! A 屯堡 is a **garrison that became a village**: its founders were Ming soldiers settled on a frontier road, and the one building every one of their villages
//! kept is the tower at the end of the street that could see the road for twenty kilometres. It is not a 羌碉 (round 120's type is a highland house that
//! fortifies itself): a 碉楼 is **only a tower** — no rooms, no hearth, a stair and loopholes — and its whole form is a **square stone stack with a timber top**.
//!
//! # How it is built
//!
//! - **four stone storeys**, each standing on the one below with `Join::Storey` and each a little narrower, which is a tower's batter;
//! - **a loophole on each storey's front face**, cut in the face that looks down the street — a 碉楼 watches **one way**, and that way is the road;
//! - **the timber gallery** on the top storey: four posts and a cap, which is what a stone tower is crowned with in this tradition.
//!
//! # What it does not draw
//!
//! - **the stair**: this crate has no interior, and a tower's stair is inside its walls;
//! - **the garrison**: the tower is drawn on a day between alarms, which is the same compromise the stage and the salt works carry.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The storeys: how many, the tower's width at the foot, how much each one tapers, and how high each stands.
const STOREYS: usize = 4;
const FOOT: f32 = 4.60;
const TAPER: f32 = 0.30;
const STOREY_HIGH: f32 = 3.30;
/// The plinth the tower stands on, and its own height.
const PLINTH: f32 = 5.80;
const PLINTH_HIGH: f32 = 0.55;
/// The loophole: its sill above each storeys floor, and its jamb and heads inset into the bay the face is split into.
const LOOP_SILL: f32 = 1.05;
const LOOP_JAMB: f32 = 0.42;
const LOOP_HEAD: f32 = 1.05;
/// The timber gallery: its posts, their height and its cap's oversail.
const GALLERY_POST: f32 = 0.18;
const GALLERY_HIGH: f32 = 2.55;
const GALLERY_EAVE: f32 = 0.55;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.62;

/// Builds the scene: from the street, low, so the tower is read as a height and not as a plan.
pub fn watchtower() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [-11.6, 4.2, 15.8],
        [0.0, 8.4, 0.0],
        FIELD_OF_VIEW,
        "watchtower",
    )
}

/// Returns the document: the plinth, four stone storeys with a loophole each, and the timber gallery.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The plinth**: the tower's foot, wider than the tower, which is what a tower's footing is.
    let mut plinth = Mass::new(
        ops::wound(vec![
            [-PLINTH * 0.5, -PLINTH * 0.5],
            [PLINTH * 0.5, -PLINTH * 0.5],
            [PLINTH * 0.5, PLINTH * 0.5],
            [-PLINTH * 0.5, PLINTH * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, 0.0))
        .collect(),
        PLINTH_HIGH,
    );
    // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171).
    plinth.wall = PLINTH * 0.12;
    plinth.slab = 0.16;
    skeleton.masses.push(plinth);

    // **The four stone storeys**, each on the one below and each a little narrower: a tower's batter, and the crate's answer for it is `Join::Storey`.
    let mut host = building::MassId(0);
    for storey in 0..STOREYS {
        let low = PLINTH_HIGH + STOREY_HIGH * storey as f32;
        let high = low + STOREY_HIGH;
        let half = 0.5 * (FOOT - TAPER * storey as f32);
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
        // **The ring's offset stays under half the narrow side**: the narrow side is `2 * half`, so this is a sixth of it.
        body.wall = half * 0.33;
        body.slab = 0.18;
        body.attachment = Some(building::Attachment {
            host,
            join: building::Join::Storey,
        });
        let id = building::MassId(skeleton.masses.len());
        skeleton.masses.push(body);
        host = id;

        // **The loophole** on the storey's `+z` face, which is the face that looks down the street.
        let face = building::FaceRef::new(id, 2);
        if ops::split_face(
            &mut skeleton,
            face,
            &building::Rhythm::Even { count: 3 },
            &building::Rhythm::Even { count: 1 },
        )
        .is_some()
        {
            let _ = ops::add_opening(
                &mut skeleton,
                building::Opening::new(
                    face,
                    0,
                    1,
                    building::Inset::new(LOOP_JAMB, LOOP_JAMB, LOOP_SILL, LOOP_HEAD),
                ),
            );
        }
    }

    // **The timber gallery** on the top storey: four posts and a cap, which is what crowns a stone tower in this tradition.
    let top = PLINTH_HIGH + STOREY_HIGH * STOREYS as f32;
    let half = 0.5 * (FOOT - TAPER * STOREYS as f32);
    for x in [-half + GALLERY_POST, half - GALLERY_POST] {
        for z in [-half + GALLERY_POST, half - GALLERY_POST] {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, top, z] },
                building::Anchor::Free {
                    at: [x, top + GALLERY_HIGH, z],
                },
                [1.0, 0.0],
                GALLERY_POST,
                GALLERY_POST,
            ));
        }
    }
    let cap = half + GALLERY_EAVE;
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![[-cap, -cap], [cap, -cap], [cap, cap], [-cap, cap]]),
        top + GALLERY_HIGH,
        top + GALLERY_HIGH + 0.18,
    ));

    Building::new(skeleton)
}
