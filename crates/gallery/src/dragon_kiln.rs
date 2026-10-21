//! A **龙窑**: the climbing pottery kiln of a Sichuan or Yunnan clay village — a barrel that steps up the hillside, its firing mouths down the slope and its
//! chimney at the head — the twenty-fourth type.
//!
//! # What this scene is for
//!
//! A 龙窑 is the one building in this catalogue that is **a machine and a hill at once**: its chamber is thirty metres of masonry laid **up a slope**, so the
//! draught comes from the doors at the bottom and the heat climbs to the chimney at the top. The potter's shed over the doors is the only part that is a
//! building; the rest is a **kiln**, and a kiln is a solid with a fire in it.
//!
//! # How it is built
//!
//! - **the barrel**: `STEPS` masses, each standing on the one below with `Join::Storey` and each a little higher and a little narrower — which is how a
//!   long solid climbs a hill in this crate (rounds 169, 207 and 229);
//! - **the firing mouths**: an opening in the lowest chamber's own side, because a kiln is entered by a door and not by a lid;
//! - **the chimney** at the head, a mast of its own above the topmost chamber;
//! - **the shed**: posts and a cap over the doors, where the potter stands with the fire.
//!
//! # What it does not draw
//!
//! - **the fire and the pots**: the same two things round 224 could not draw, and for the same reason — this crate draws solids, and a firing is not one;
//! - **the hillside**: the barrel steps up **its own** basement and the ground under it is the harness's plane, so the kiln reads as a stair of masonry and
//!   not as a cut in a slope.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The barrel: how many chambers, how long each is, the width at the foot and how much each one tapers, each one's height, and how far each steps up.
const STEPS: usize = 5;
const STEP_LONG: f32 = 5.20;
const FOOT_WIDE: f32 = 4.20;
const TAPER: f32 = 0.24;
const STEP_HIGH: f32 = 1.55;
const STEP_UP: f32 = 1.20;
/// The firing mouth: its bay's place along the lowest chamber, its jamb and its head, and the chamber's own wall.
const MOUTH_JAMB: f32 = 0.58;
const MOUTH_HEAD: f32 = 0.46;
/// The chimney at the head: its section and how far it rises above the topmost chamber.
const CHIMNEY: f32 = 0.85;
const CHIMNEY_HIGH: f32 = 2.70;
/// The shed over the doors: its posts, their height and its cap's oversail.
const SHED_POST: f32 = 0.15;
const SHED_HIGH: f32 = 2.60;
const SHED_EAVE: f32 = 0.48;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.58;

/// Builds the scene: from the foot of the slope, so the barrel climbs away from the camera and the chimney stands at the head of the picture.
pub fn dragon_kiln() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [-16.6, 3.3, 14.2],
        [5.2, 3.6, 0.0],
        FIELD_OF_VIEW,
        "dragon_kiln",
    )
}

/// Returns the document: the barrel's five chambers, its chimney and the shed over the doors.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The barrel**: five chambers standing **side by side on the same ground**, each one's crown a step higher than the one before and each a little narrower.\n    // A chain that climbed by Join::Storey would have to nest each chamber inside the last ones plan, and a barrel thirty metres long **runs along x and\n    // not up a stack** (StoreyNotOnTop, StoreyOverhangs — measured this round). The climb is in the **crowns**, which is what a 龙窑s slope looks like\n    // from outside.
    for step in 0..STEPS {
        let from = STEP_LONG * step as f32;
        let to = from + STEP_LONG;
        let half = (FOOT_WIDE - TAPER * step as f32) * 0.5;
        let base = STEP_UP * step as f32;
        let mut chamber = Mass::new(
            ops::wound(vec![[from, -half], [to, -half], [to, half], [from, half]])
                .into_iter()
                .map(|at| building::PlanNode::new(at, 0.0))
                .collect(),
            base + STEP_HIGH,
        );
        // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171): the narrow side is `2 * half`.
        chamber.wall = half * 0.30;
        chamber.slab = 0.20;
        skeleton.masses.push(chamber);
    }

    // **The firing mouth** in the lowest chamber's own `-z` side, which is the side the potter works from.
    let face = building::FaceRef::new(building::MassId(0), 0);
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
                building::Inset::new(MOUTH_JAMB, MOUTH_JAMB, 0.10, MOUTH_HEAD),
            ),
        );
    }

    // **The chimney** above the topmost chamber's head.
    let head = STEP_LONG * STEPS as f32;
    let top = STEP_UP * (STEPS - 1) as f32 + STEP_HIGH;
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [head - CHIMNEY * 0.5, top - 0.10, 0.0],
        },
        building::Anchor::Free {
            at: [head - CHIMNEY * 0.5, top + CHIMNEY_HIGH, 0.0],
        },
        [1.0, 0.0],
        CHIMNEY,
        CHIMNEY,
    ));
    // **A cap on the chimney**, because a kiln's flue is kept from the rain when it is not firing.
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [head - CHIMNEY - 0.30, -CHIMNEY * 0.5 - 0.30],
            [head + 0.30, -CHIMNEY * 0.5 - 0.30],
            [head + 0.30, CHIMNEY * 0.5 + 0.30],
            [head - CHIMNEY - 0.30, CHIMNEY * 0.5 + 0.30],
        ]),
        top + CHIMNEY_HIGH,
        top + CHIMNEY_HIGH + 0.12,
    ));

    // **The shed** over the doors, where the potter stands with the fire: four posts and a cap on the low chamber's `-z` side.
    let shed_z = -FOOT_WIDE * 0.5 - 1.35;
    for x in [0.30, STEP_LONG - 0.30] {
        for side in [-1.0_f32, 1.0] {
            let z = shed_z + side * 0.10;
            skeleton.members.push(building::Member::new(
                building::Anchor::Free { at: [x, 0.0, z] },
                building::Anchor::Free {
                    at: [x, SHED_HIGH, z],
                },
                [1.0, 0.0],
                SHED_POST,
                SHED_POST,
            ));
        }
    }
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [-SHED_EAVE, shed_z - 1.30],
            [STEP_LONG + SHED_EAVE, shed_z - 1.30],
            [STEP_LONG + SHED_EAVE, shed_z + 1.30],
            [-SHED_EAVE, shed_z + 1.30],
        ]),
        SHED_HIGH,
        SHED_HIGH + 0.14,
    ));

    Building::new(skeleton)
}
