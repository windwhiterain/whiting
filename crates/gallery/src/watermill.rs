//! A **水碾房**: the water-driven mill of a south-western river village — a wheel in the millrace, a stone base, and a shed over the stones — the
//! twenty-first type.
//!
//! # What this scene is for
//!
//! Every river village in 滇西 and 黔东南 that grew rice grew it to be milled, and the mill is the one building in the village that **is a machine**: a
//! wheel standing in a millrace cut beside the stream, a shed over the stones it turns, and no ornament at all. It is also the only type here whose
//! **subject is a mechanism** — the wheel is the building, and the shed is what keeps the rain off the flour.
//!
//! # How it is built
//!
//! - **the millrace**: two banks of earth, `BANK_HIGH` tall, running past the wheel with the water between them;
//! - **the wheel**: a hub, `SPOKES` spokes and two rims, drawn entirely out of **members**, because a wheel is a ring of straight timber and not a solid;
//! - **the shed**: one mass with a flat tiled cap, standing on the bank where the stones are.
//!
//! # What it does not draw
//!
//! - **the stones themselves**: the runner and the bed stone are inside the shed, and this crate has no interior;
//! - **the wheel turning**: a document is one instant, and a wheel is the one type here whose whole meaning is that it does not stand still.

use building::{Building, BuildingSkeleton, Mass, ops};
use definition::Value;

/// The wheel: its radius, its hub, how many spokes and how thick, the two rims' section and how far apart they stand.
const WHEEL_R: f32 = 2.00;
const HUB: f32 = 0.26;
const SPOKES: usize = 12;
const SPOKE: f32 = 0.13;
const RIM: f32 = 0.12;
const RIM_APART: f32 = 0.62;
/// Where the wheel's axle stands above the race's own floor.
const AXLE_HIGH: f32 = 2.10;
/// The millrace: its banks' plan, their height, and how wide the water between them is.
const BANK_LONG: f32 = 19.0;
const BANK_DEEP: f32 = 3.60;
const BANK_HIGH: f32 = 0.85;
const RACE_HALF: f32 = 2.35;
/// The shed: where it stands, its plan and its height, and how far its cap oversails.
const SHED_AT: [f32; 2] = [4.60, 0.0];
const SHED_LONG: f32 = 5.40;
const SHED_DEEP: f32 = 4.60;
const SHED_HIGH: f32 = 3.15;
const CAP_EAVE: f32 = 0.55;

const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.62;

/// Builds the scene: from down the race, so the wheel is edge-on and the shed is behind it.
pub fn watermill() -> Value {
    crate::village::scene_of(
        &document(),
        &crate::settlement_plants::Extra::none(),
        &|_, _| 0.0,
        [-9.6, 4.4, 12.4],
        [1.2, 2.6, 0.0],
        FIELD_OF_VIEW,
        "watermill",
    )
}

/// Returns the document: the race's two banks, the wheel and the shed.
pub fn document() -> Building {
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());

    // **The millrace**: two banks of earth, the water running between them and past the wheel.
    for side in [-1.0_f32, 1.0] {
        let inner = side * RACE_HALF;
        let outer = side * (RACE_HALF + BANK_DEEP);
        let mut bank = Mass::new(
            ops::wound(vec![
                [-BANK_LONG * 0.5, inner.min(outer)],
                [BANK_LONG * 0.5, inner.min(outer)],
                [BANK_LONG * 0.5, inner.max(outer)],
                [-BANK_LONG * 0.5, inner.max(outer)],
            ])
            .into_iter()
            .map(|at| building::PlanNode::new(at, 0.0))
            .collect(),
            BANK_HIGH,
        );
        // **The ring's offset stays under half the narrow side** (rounds 74, 144, 171).
        bank.wall = BANK_DEEP * 0.22;
        bank.slab = 0.14;
        skeleton.masses.push(bank);
    }

    // **The wheel.** A hub, twelve spokes and two rims, all members: a wheel is a ring of straight timber.
    let axle = [0.0_f32, AXLE_HIGH, 0.0];
    // The hub runs across the wheel's own plane, so its `across` is the plane's normal turned a quarter.
    skeleton.members.push(building::Member::new(
        building::Anchor::Free {
            at: [axle[0], axle[1], -RIM_APART * 0.5 - 0.24],
        },
        building::Anchor::Free {
            at: [axle[0], axle[1], RIM_APART * 0.5 + 0.24],
        },
        [1.0, 0.0],
        HUB,
        HUB,
    ));
    for spoke in 0..SPOKES {
        let angle = core::f32::consts::TAU * spoke as f32 / SPOKES as f32;
        let (sin, cos) = angle.sin_cos();
        for side in [-1.0_f32, 1.0] {
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [axle[0], axle[1], side * RIM_APART * 0.5],
                },
                building::Anchor::Free {
                    at: [
                        axle[0] + WHEEL_R * cos,
                        axle[1] + WHEEL_R * sin,
                        side * RIM_APART * 0.5,
                    ],
                },
                [0.0, 1.0],
                SPOKE,
                SPOKE,
            ));
        }
    }
    // **The two rims**, each a ring of `SPOKES` straight pieces between the spokes' own ends.
    for side in [-1.0_f32, 1.0] {
        for piece in 0..SPOKES {
            let from = core::f32::consts::TAU * piece as f32 / SPOKES as f32;
            let to = core::f32::consts::TAU * (piece + 1) as f32 / SPOKES as f32;
            let (from_sin, from_cos) = from.sin_cos();
            let (to_sin, to_cos) = to.sin_cos();
            let middle = (from + to) * 0.5;
            // **A rim piece's `across` is its run turned a quarter** (round 134), laid in the wheel's own plane.
            let (turn_sin, turn_cos) = middle.sin_cos();
            skeleton.members.push(building::Member::new(
                building::Anchor::Free {
                    at: [
                        axle[0] + WHEEL_R * from_cos,
                        axle[1] + WHEEL_R * from_sin,
                        side * RIM_APART * 0.5,
                    ],
                },
                building::Anchor::Free {
                    at: [
                        axle[0] + WHEEL_R * to_cos,
                        axle[1] + WHEEL_R * to_sin,
                        side * RIM_APART * 0.5,
                    ],
                },
                [turn_cos, turn_sin],
                RIM,
                RIM,
            ));
        }
    }

    // **The shed** over the stones, on the bank, with a flat tiled cap.
    let mut shed = Mass::new(
        ops::wound(vec![
            [SHED_AT[0] - SHED_LONG * 0.5, SHED_AT[1] - SHED_DEEP * 0.5],
            [SHED_AT[0] + SHED_LONG * 0.5, SHED_AT[1] - SHED_DEEP * 0.5],
            [SHED_AT[0] + SHED_LONG * 0.5, SHED_AT[1] + SHED_DEEP * 0.5],
            [SHED_AT[0] - SHED_LONG * 0.5, SHED_AT[1] + SHED_DEEP * 0.5],
        ])
        .into_iter()
        .map(|at| building::PlanNode::new(at, BANK_HIGH))
        .collect(),
        BANK_HIGH + SHED_HIGH,
    );
    shed.wall = 0.32;
    shed.slab = 0.20;
    skeleton.masses.push(shed);
    let cap_x = SHED_LONG * 0.5 + CAP_EAVE;
    let cap_z = SHED_DEEP * 0.5 + CAP_EAVE;
    skeleton.slabs.push(building::Slab::roof(
        ops::wound(vec![
            [SHED_AT[0] - cap_x, SHED_AT[1] - cap_z],
            [SHED_AT[0] + cap_x, SHED_AT[1] - cap_z],
            [SHED_AT[0] + cap_x, SHED_AT[1] + cap_z],
            [SHED_AT[0] - cap_x, SHED_AT[1] + cap_z],
        ]),
        BANK_HIGH + SHED_HIGH,
        BANK_HIGH + SHED_HIGH + 0.16,
    ));

    Building::new(skeleton)
}
