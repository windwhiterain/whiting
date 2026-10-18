//! The site: one Hengduan slope, drawn at the height of each belt.
//!
//! # What this module is
//!
//! [`.agents/proposals/southwest-china-trees.md`](../../../.agents/proposals/southwest-china-trees.md)
//! §7: a hero specimen is a tree and an epic frame is a place. Every site frame here stands on **one**
//! heightfield — a ridge descending south-west into a valley, with a gully running along it — so the
//! four frames are one mountain seen at four heights rather than four inventions, and E1's check is a
//! comparison of the frame's sidecar against the species' notes rather than against a new ground per
//! scene.
//!
//! # What a site frame carries that a specimen frame does not
//!
//! Companions, at a coarser [`Detail`] and the spray foliage, seated on the slope by
//! [`vegetation::ground::seat`]; a standing stone of a stated height, because E2 wants a known size in
//! frame and a stone belongs in every one of these belts while a person does not; and a light that is
//! the belt's own — a hot noon in the valley, a raking morning on the montane slope, a cold high sun
//! over the subalpine stand.

use definition::Value;
use vegetation::derive::Species;
use vegetation::expand::Detail;
use vegetation::ground::seat;
use vegetation::plant::Placement;
use vegetation::rng::Rng;

use crate::air::Atmosphere;
use crate::geometry::cuboid;
use crate::southwest::{Foliage, pine_palette, yunnan_pine};
use crate::southwest_flag::{dove_palette, dove_tree};
use crate::southwest_oak::{alpine_oak, oak_palette};
use crate::vegetation_picture::{
    Framing, Ground, Lighting, Palette, Planting, Prop, Skin, plant_scene_lit, planting_handle,
};

/// The name the montane site frame is built and rendered under.
pub const SITE_MONTANE: &str = "site_montane";

/// The slope: a ridge falling south-west at about eleven degrees, a broad swell along it, and a gully
/// running with it at `z` near eleven, which is where the dove trees stand because that is where the
/// moisture is.
///
/// # Invariants
///
/// **One law, two readers.** The floor is drawn from this at `f64` and every planting is seated on it at
/// `f32`, and the two are the same function, so a tree cannot stand where the floor is not — the same
/// rule [`vegetation::ground`] states for a plant and its base.
pub fn slope_height(x: f64, z: f64) -> f64 {
    -0.20 * x + 0.8 * (x * 0.045).sin() * (z * 0.038).cos()
        - 1.6 * (-((z - 11.0) / 5.0).powi(2)).exp()
}

/// The slope at `f32`, for seating.
fn slope(p: [f32; 2]) -> f32 {
    slope_height(f64::from(p[0]), f64::from(p[1])) as f32
}

/// The detail a companion is drawn at: the hero's dial is the crate's default, and a companion on the
/// far ridge is the same rule coarser, which is what [`Planting::with_detail`] is for.
const COMPANION: Detail = Detail {
    stations: 4,
    around: 8,
    spacing: 1.5,
};

/// A planting of one species at a plan position, seated on the slope and turned by its own seed.
fn seated(species: Species, palette: Palette, at: [f32; 2], turn: f32, detail: Detail) -> Planting {
    Planting::new(species)
        .at(seat(
            Placement {
                origin: [at[0], 0.0, at[1]],
                x: [turn.cos(), 0.0, -turn.sin()],
                y: [0.0, 1.0, 0.0],
                z: [turn.sin(), 0.0, turn.cos()],
            },
            slope,
        ))
        .with_palette(palette)
        .with_detail(detail)
}

/// The montane slope's plantings: a hero pine, a hero oak in the draw, a hero dove tree in the gully,
/// and companions of all three at the coarser dial, placed by one seeded stream so the stand is the
/// same stand every time.
fn montane_plantings() -> Vec<Planting> {
    let mut rng = Rng::new(4_200);
    let mut plantings = Vec::new();
    plantings.push(seated(
        yunnan_pine(21, Foliage::Spray),
        pine_palette(),
        [0.0, 0.0],
        0.4,
        Detail::DEFAULT,
    ));
    plantings.push(seated(
        alpine_oak(31),
        oak_palette(),
        [-7.0, 6.0],
        2.2,
        Detail::DEFAULT,
    ));
    plantings.push(seated(
        dove_tree(41),
        dove_palette(),
        [3.0, 11.0],
        1.1,
        Detail::DEFAULT,
    ));
    for index in 0..5 {
        let across = -14.0 + 6.0 * index as f32 + 3.0 * rng.signed();
        let deep = -16.0 + 6.0 * rng.unit() + 0.30 * across;
        plantings.push(seated(
            yunnan_pine(100 + index, Foliage::Stand),
            pine_palette(),
            [across, deep],
            rng.unit() * 6.28,
            COMPANION,
        ));
    }
    plantings
}

/// The standing stone: 1.8 metres of it, which is the known size E2 asks the frame to carry.
fn montane_stone() -> Prop {
    let at = [5.0, slope_height(5.0, -4.0), -4.0];
    Prop {
        name: "stone".to_owned(),
        points: cuboid(0.9, 1.8, 0.7),
        tint: [0.34, 0.33, 0.31],
        roughness: 0.95,
        at: [at[0], at[1] + 0.7, at[2]],
    }
}

/// A raking morning on the montane slope: the sun low in the east along the valley, a cool sky, and
/// enough haze to put the far ridge behind the near stand.
fn montane_lighting() -> Lighting {
    Lighting {
        key_position: [60.0, 26.0, -34.0],
        key_colour: [1.0, 0.93, 0.80],
        key_irradiance: 3.0,
        key_distance: 73.4,
        key_reach: 700.0,
        key_extent: 60.0,
        fill_position: [-30.0, 18.0, 26.0],
        fill_colour: [0.44, 0.52, 0.70],
        fill_irradiance: 0.5,
        fill_distance: 43.7,
        floor: Skin::new([0.32, 0.28, 0.20], 0.95),
        sky: Atmosphere::new()
            .zenith([0.09, 0.19, 0.42])
            .horizon([0.52, 0.60, 0.68])
            .ground([0.30, 0.28, 0.24])
            .density(0.0016)
            .scale_height(900.0),
        sky_radiance: 0.7,
        haze: 0.005,
    }
}

/// The montane frame: standing off down-slope so the stand is seen against the ridge behind it.
const MONTANE_FRAME: Framing = Framing {
    width: 1600,
    height: 1000,
    eye: [34.0, 6.0, 30.0],
    target: [-2.0, 7.0, 2.0],
    field_of_view: 0.72,
    near: 0.3,
    far: 900.0,
    floor: 220.0,
    ground: Ground::Fielded {
        height: slope_height,
        divisions: 120,
    },
};

/// Builds the montane site frame: three heroes, fourteen companions, one stone, one morning.
pub fn site_montane() -> Value {
    plant_scene_lit(
        &montane_plantings(),
        &MONTANE_FRAME,
        SITE_MONTANE,
        &montane_lighting(),
        &[montane_stone()],
    )
}

/// Resolves a site frame's `authored_id` into the planting and the handle it names.
pub fn site_montane_handle_of(id: u32) -> Option<String> {
    planting_handle(&montane_plantings(), id)
}
