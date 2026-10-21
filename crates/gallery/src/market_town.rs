//! A **场镇**: a south-western market town — a paved market square, the **戏台** at its head, the **牌坊** at its entrance, and two rows of **铺面** with their awnings
//! down its sides — the fifteenth settlement.
//!
//! # What this scene is for
//!
//! The gallery has twelve villages and **not one town**. A 西南 场镇 is not a village with more houses: it is a **market with buildings round it**, and its ground is one
//! open paved square that exists for the day the country comes in to trade. Everything in it is addressed to the market: the stage is where the crowd is held,
//! the arch is where it is counted, and the shop fronts are **temporary stalls made permanent** — a wall with a counter and an awning over it.
//!
//! # What it composes
//!
//! - **one `opera_stage::document()`** at the square's head, facing down it;
//! - **one `paifang::document()`** at the square's entrance, where the road comes in;
//! - **two rows of shops**: five masses a side with a **lean-to awning** over each counter, because a shop front in a market town is a roof before it is a door;
//! - **the square's paving**, which is the whole point of the settlement.
//!
//! # What it does not draw
//!
//! - **the market itself**: awnings, baskets and stalls on the day are not buildings, and the plate is of the square on a day between markets — which is the
//!   same compromise round 235 made about its stage.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The square: its half width, its depth and its paving's thickness.
const SQUARE_HALF: f32 = 15.0;
const SQUARE_DEEP: f32 = 21.0;
const PAVING: f32 = 0.09;
/// The shops: how many a side, where the first stands, how far apart, each one's plan and height.
const SHOPS: usize = 5;
const SHOP_FROM: f32 = -14.0;
const SHOP_GAP: f32 = 6.40;
const SHOP_LONG: f32 = 5.40;
const SHOP_DEEP: f32 = 5.20;
const SHOP_HIGH: f32 = 3.55;
/// The awning over each counter: how far it stands out, how high, and how thick.
const AWNING: f32 = 1.85;
const AWNING_HIGH: f32 = 2.75;
const AWNING_THICK: f32 = 0.12;
/// The counter mouth in each shop's square-facing wall: its jamb and head insets, and how high the counter stands.
const COUNTER_JAMB: f32 = 0.52;
const COUNTER_SILL: f32 = 0.72;
const COUNTER_HEAD: f32 = 0.78;
/// The counter plank across the mouth: its thickness and how far it stands proud of the wall.
const PLANK: f32 = 0.14;
const PLANK_PROUD: f32 = 0.34;
/// The stage at the head and the arch at the entrance: where they stand, and the turn each takes.
const STAGE_AT: [f32; 2] = [0.0, -SQUARE_DEEP - 3.40];
const ARCH_AT: [f32; 2] = [0.0, SQUARE_DEEP + 2.10];

/// The 碉楼 at the square's south-west corner, which is where a 屯堡 market's tower stands: at the corner it can watch the road in and the square at once.
const TOWER_AT: [f32; 2] = [-19.6, 17.6];

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.56;

/// Builds the scene: from behind the arch, down the square, with the stage at its far end — the way a market day is entered.
pub fn market_town() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [11.0, 15.0, 41.0],
        [-4.2, 3.4, -6.0],
        FIELD_OF_VIEW * 1.12,
        "market_town",
    )
}

/// The **counters plate**: one shop front at a man's height, with its mouth and its awning.
///
/// # Why a second camera
///
/// Round 241 cut a **counter mouth** in every shop's square-facing wall and the town's own plate could not show one: that plate looks **along** the square, and
/// both rows stand with their counters turned across its line of sight. A shop front is read from the square, three metres from it, which is where a customer
/// stands — round 172's rule, taken for the ninth time.
pub fn market_shops() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [-6.2, 2.3, 7.4],
        [-14.6, 2.1, 6.6],
        FIELD_OF_VIEW * 0.72,
        "market_shops",
    )
}

/// Returns the document: the square, the two rows of shops with their awnings, the stage and the arch.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The square**: one paved ground, because in a market town the ground *is* the building.
    skeleton.slabs.push(building::Slab::paving(
        ops::wound(vec![
            [-SQUARE_HALF, -SQUARE_DEEP],
            [SQUARE_HALF, -SQUARE_DEEP],
            [SQUARE_HALF, SQUARE_DEEP],
            [-SQUARE_HALF, SQUARE_DEEP],
        ]),
        0.0,
        PAVING,
    ));

    // **The two rows of shops**, their counters facing the square.
    for side in [-1.0_f32, 1.0] {
        for shop in 0..SHOPS {
            let z = SHOP_FROM + SHOP_GAP * shop as f32;
            let inner = side * (SQUARE_HALF - 0.40);
            let outer = side * (SQUARE_HALF - 0.40 + SHOP_DEEP);
            let mut body = Mass::new(
                ops::wound(vec![
                    [inner.min(outer), z - SHOP_LONG * 0.5],
                    [inner.max(outer), z - SHOP_LONG * 0.5],
                    [inner.max(outer), z + SHOP_LONG * 0.5],
                    [inner.min(outer), z + SHOP_LONG * 0.5],
                ])
                .into_iter()
                .map(|at| building::PlanNode::new(at, PAVING))
                .collect(),
                PAVING + SHOP_HIGH,
            );
            // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171): the narrow side here is the shop's length across the square.
            body.wall = SHOP_LONG * 0.14;
            body.slab = 0.18;
            let id = building::MassId(skeleton.masses.len());
            skeleton.masses.push(body);
            // **The counter mouth.** A shop front in a market town is **a hole before it is a wall**: the goods are handed across a counter that stands open to
            // the square from morning to dark. The mouth is one bay of the square-facing face, cut from `COUNTER_SILL` up — the same `split_face` and
            // `Opening` the 塔门 and the 碉樓's loopholes use.
            //
            // The wound rectangle's faces are: `0` the `-z` side, `1` the `+x` side, `2` the `+z` side and `3` the `-x` side — so **the square-facing face is
            // `1` for the west row and `3` for the east one**.
            let facing = if side < 0.0 { 1 } else { 3 };
            let face = building::FaceRef::new(id, facing);
            if ops::split_face(
                &mut skeleton,
                face,
                &building::Rhythm::Even { count: 3 },
                &building::Rhythm::Even { count: 1 },
            )
            .is_some()
            {
                let cut = ops::add_opening(
                    &mut skeleton,
                    building::Opening::new(
                        face,
                        0,
                        1,
                        building::Inset::new(
                            COUNTER_JAMB,
                            COUNTER_JAMB,
                            COUNTER_SILL,
                            COUNTER_HEAD,
                        ),
                    ),
                );
                // **A refused mouth is a defect and not a detail**: the wide low counter tried in round 241 was refused and the `let _ =` swallowed it, which
                // left ten blank walls and a plate that had to catch them. The count in this gallery is **sixteen** other swallowed `add_opening` calls, listed
                // in the proposal as measured debt.
                assert!(
                    cut.is_some(),
                    "the counter mouth is a hole the face accepts"
                );
            }
            // **The counter plank** across the mouth at the sill's own height: the board the goods are handed over. It is one member, and it is what turns a
            // hole in a wall into a shop. It stands proud **into the square**, which is the wall's own place moved towards the square's middle.
            let face_x = side * (SQUARE_HALF - 0.40);
            let plank_x = face_x - side * PLANK_PROUD;
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [
                        plank_x,
                        PAVING + COUNTER_SILL - PLANK * 0.5,
                        z - SHOP_LONG * 0.5 + COUNTER_JAMB + 0.10,
                    ],
                },
                building::Anchor::Free {
                    at: [
                        plank_x,
                        PAVING + COUNTER_SILL - PLANK * 0.5,
                        z + SHOP_LONG * 0.5 - COUNTER_JAMB - 0.10,
                    ],
                },
                [1.0, 0.0],
                PLANK,
                PLANK,
            ));

            // **The shop's own cap**, a step over its walls.
            skeleton.slabs.push(building::Slab::roof(
                ops::wound(vec![
                    [inner.min(outer) - 0.30, z - SHOP_LONG * 0.5 - 0.30],
                    [inner.max(outer) + 0.30, z - SHOP_LONG * 0.5 - 0.30],
                    [inner.max(outer) + 0.30, z + SHOP_LONG * 0.5 + 0.30],
                    [inner.min(outer) - 0.30, z + SHOP_LONG * 0.5 + 0.30],
                ]),
                PAVING + SHOP_HIGH,
                PAVING + SHOP_HIGH + 0.16,
            ));
            // **The awning**: a lean-to over the counter, which is what turns a wall into a shop front.
            let edge = side * (SQUARE_HALF - 0.40 - AWNING);
            skeleton.slabs.push(building::Slab::roof(
                ops::wound(vec![
                    [inner.min(edge), z - SHOP_LONG * 0.5 + 0.20],
                    [inner.max(edge), z - SHOP_LONG * 0.5 + 0.20],
                    [inner.max(edge), z + SHOP_LONG * 0.5 - 0.20],
                    [inner.min(edge), z + SHOP_LONG * 0.5 - 0.20],
                ]),
                PAVING + AWNING_HIGH,
                PAVING + AWNING_HIGH + AWNING_THICK,
            ));
            // **Two posts** under the awning's outer edge, one at each end of the counter.
            for end in [-1.0_f32, 1.0] {
                let x = side * (SQUARE_HALF - 0.40 - AWNING);
                let z = z + end * (SHOP_LONG * 0.5 - 0.30);
                skeleton.members.push(building::Member::new(
                    building::Anchor::Free { at: [x, PAVING, z] },
                    building::Anchor::Free {
                        at: [x, PAVING + AWNING_HIGH, z],
                    },
                    [1.0, 0.0],
                    0.13,
                    0.13,
                ));
            }
        }
    }

    // **The stage at the square's head**, facing down it. Round 235's own document, translated, with none of its numbers restated.
    let mut stage = crate::opera_stage::document().skeleton;
    ops::translate(&mut stage, STAGE_AT);
    assert!(
        ops::merge(&mut skeleton, stage),
        "the stage places no parts, so the merge always happens"
    );

    // **The arch at the square's entrance**, across the road that comes in[^1].
    //
    // [^1]: the arch's own plane is `x`, which is the square's width, so it is **not** turned.
    let mut arch = crate::paifang::document().skeleton;
    ops::translate(&mut arch, ARCH_AT);
    assert!(
        ops::merge(&mut skeleton, arch),
        "the arch places no parts, so the merge always happens"
    );

    // **The 碉楼 at the square's south-west corner.** A 场镇 raised by 屯堡 people keeps their tower: at the corner it watches the road in and the square at
    // once, and it is the only building in this town taller than the shops. Round 239's own document, translated, with none of its numbers restated.
    let mut tower = crate::watchtower::document().skeleton;
    ops::translate(&mut tower, TOWER_AT);
    assert!(
        ops::merge(&mut skeleton, tower),
        "the tower places no parts, so the merge always happens"
    );

    Building::new(skeleton)
}
